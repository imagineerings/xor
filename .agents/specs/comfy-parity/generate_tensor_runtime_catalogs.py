#!/usr/bin/env python3
"""Generate static tensor, autograd, and RNG conformance catalogs for ComfyUI."""

import ast
import csv
import hashlib
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path


SPEC = Path(__file__).resolve().parent
CATALOGS = SPEC / "catalogs"
SOURCE = SPEC.parents[2] / "projects" / "comfy" / "ComfyUI"
SOURCE_COVERAGE = CATALOGS / "backend-source-coverage.csv"
BACKEND_RECONCILIATION = CATALOGS / "backend-reconciliation.json"

TENSOR_ROOTS = {
    "torch",
    "torchvision",
    "torchaudio",
    "torchsde",
    "einops",
    "einops_exts",
    "kornia",
    "xformers",
    "flash_attn",
    "sageattention",
    "sageattn3",
    "alias_free_torch",
    "torch_directml",
    "torch_npu",
    "torch_mlu",
}

TENSOR_METHODS = {
    "abs", "absolute", "acos", "acosh", "add", "add_", "all", "amax", "amin",
    "any", "argmax", "argmin", "argsort", "as_strided", "atan2", "bfloat16",
    "bool", "broadcast_to", "byte", "ceil", "chunk", "clamp", "clamp_", "clone",
    "contiguous", "copy_", "cos", "cpu", "cuda", "cumprod", "cumsum", "data_ptr",
    "detach", "detach_", "dim", "div", "div_", "double", "element_size", "eq", "exp",
    "expand", "expand_as", "expm1", "flatten", "flip", "float", "floor", "gather",
    "ge", "gt", "half", "index_add", "index_add_", "index_copy_", "index_put_", "int",
    "is_contiguous", "is_floating_point", "item", "le", "lerp", "log", "log1p", "long",
    "lt", "masked_fill", "masked_fill_", "matmul", "max", "mean", "min", "movedim",
    "mul", "mul_", "narrow", "ndimension", "ne", "neg", "nonzero", "norm", "numel",
    "numpy", "permute", "pow", "prod", "repeat", "repeat_interleave", "reshape", "resize_",
    "roll", "round", "scatter", "scatter_", "select", "sigmoid", "sign", "sin", "size",
    "softmax", "split", "sqrt", "square", "squeeze", "std", "stride", "sub", "sub_", "sum",
    "t", "tanh", "to", "tolist", "transpose", "type", "type_as", "unbind", "unfold",
    "unique", "unsqueeze", "var", "view", "view_as", "zero_", "requires_grad_",
    "retain_grad", "register_hook", "backward",
}

AUTOGRAD_METHODS = {
    "backward", "detach", "detach_", "requires_grad_", "retain_grad", "register_hook",
    "save_for_backward", "mark_dirty", "mark_non_differentiable", "set_materialize_grads",
}
AUTOGRAD_STATE = {"requires_grad", "grad", "grad_fn", "data", "is_leaf"}
AUTOGRAD_CONTEXT_STATE = {"needs_input_grad", "saved_tensors", "saved_variables"}
RNG_NAMES = {
    "Generator", "manual_seed", "seed", "initial_seed", "get_rng_state", "set_rng_state",
    "get_state", "set_state", "rand", "rand_like", "randn", "randn_like", "randint",
    "randint_like", "randperm", "random", "random_", "choice", "choices", "shuffle", "permutation",
    "sample", "uniform", "uniform_", "normal", "normal_", "multinomial", "bernoulli",
    "bernoulli_", "poisson", "default_rng", "SobolEngine", "BrownianTree",
}
RNG_METHODS = {
    "manual_seed", "seed", "get_state", "set_state", "get_rng_state", "set_rng_state",
    "random", "random_", "rand", "randn", "randint", "choice", "shuffle", "permutation",
    "uniform", "uniform_", "normal", "normal_", "multinomial", "bernoulli", "bernoulli_",
    "poisson", "draw",
}
TYPE_NAMES = {
    "Tensor", "Parameter", "Module", "ModuleList", "ModuleDict", "Sequential", "Generator",
    "device", "dtype", "Size", "SymInt", "MemoryFormat", "Optimizer",
}
MODULE_SEGMENTS = {
    "nn", "functional", "fft", "linalg", "special", "autograd", "amp", "cuda", "mps",
    "xpu", "compiler", "distributed", "utils", "checkpoint", "optim", "quasirandom", "ops",
}


@dataclass(frozen=True)
class Usage:
    symbol: str
    source_file: str
    line: int
    column: int
    usage_kind: str
    resolution: str
    source_classification: str
    source_tier: str
    enclosing_symbol: str
    expression: str

    def site(self):
        enclosing = " ({})".format(self.enclosing_symbol) if self.enclosing_symbol else ""
        return "{}:{}:{}{}".format(self.source_file, self.line, self.column + 1, enclosing)


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def stable_id(prefix, key):
    return "{}-{}".format(prefix, sha256_bytes(key.encode("utf-8"))[:12].upper())


def safe_unparse(node):
    try:
        return ast.unparse(node).replace("\n", " ")
    except Exception:
        return "<unparse-unavailable>"


def sorted_join(values):
    return " | ".join(sorted(set(value for value in values if value)))


def read_csv(path):
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def write_csv(path, fieldnames, rows):
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def source_tier(classification, source_file=""):
    if classification == "test-only":
        return "test"
    if classification == "documented-only":
        return "support"
    if classification == "infrastructure-only" and source_file.startswith((".ci/", ".github/", "script_examples/")):
        return "support"
    return "production"


def source_availability(usages):
    production = [usage for usage in usages if usage.source_tier == "production"]
    if production:
        paths = [usage.source_file for usage in production]
        if all(path.startswith(("comfy_extras/", "comfy_api_nodes/")) for path in paths):
            return "conditional"
        return "active"
    if any(usage.source_tier == "test" for usage in usages):
        return "developer-only"
    return "infrastructure-only"


class Resolver:
    def __init__(self, tree):
        self.aliases = {}
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for item in node.names:
                    binding = item.asname or item.name.split(".")[0]
                    target = item.name if item.asname else item.name.split(".")[0]
                    self.aliases[binding] = target
            elif isinstance(node, ast.ImportFrom) and node.module:
                for item in node.names:
                    if item.name != "*":
                        self.aliases[item.asname or item.name] = "{}.{}".format(node.module, item.name)

        for _ in range(4):
            changed = False
            for node in getattr(tree, "body", []):
                if not isinstance(node, (ast.Assign, ast.AnnAssign)):
                    continue
                targets = node.targets if isinstance(node, ast.Assign) else [node.target]
                value = node.value
                if value is None:
                    continue
                resolved = self.resolve(value)
                if not resolved or "()" in resolved:
                    continue
                for target in targets:
                    if isinstance(target, ast.Name) and self.aliases.get(target.id) != resolved:
                        self.aliases[target.id] = resolved
                        changed = True
            if not changed:
                break

    def resolve(self, node):
        if isinstance(node, ast.Name):
            return self.aliases.get(node.id, node.id)
        if isinstance(node, ast.Attribute):
            base = self.resolve(node.value)
            return "{}.{}".format(base, node.attr) if base else None
        if isinstance(node, ast.Call):
            function = self.resolve(node.func)
            return "{}()".format(function) if function else None
        return None

    def imported_non_tensor_receiver(self, node):
        root = node
        while isinstance(root, (ast.Attribute, ast.Subscript)):
            root = root.value
        if not isinstance(root, ast.Name):
            return False
        imported = self.aliases.get(root.id)
        return imported is not None and not is_tensor_symbol(imported)


def normalize_match_syntax(text):
    text = re.sub(r"^(\s*)match\s+.*:\s*$", r"\1if True:", text, flags=re.MULTILINE)
    return re.sub(r"^(\s*)case\s+.*:\s*$", r"\1if True:", text, flags=re.MULTILINE)


def parse_source(path):
    text = path.read_text(encoding="utf-8")
    try:
        return ast.parse(text, filename=str(path)), "native-ast"
    except SyntaxError as original_error:
        normalized = normalize_match_syntax(text)
        if normalized == text:
            raise original_error
        return ast.parse(normalized, filename=str(path)), "syntax-normalized-ast"


def is_tensor_symbol(symbol):
    if not symbol:
        return False
    if symbol.startswith("comfy.ops"):
        return True
    return symbol.split(".")[0] in TENSOR_ROOTS


def canonical_chain_symbol(symbol):
    if not symbol or "()" not in symbol:
        return symbol
    if symbol.startswith("torch.Generator()."):
        return symbol.replace("torch.Generator().", "torch.Generator.", 1)
    if symbol.startswith("numpy.random.default_rng()."):
        return symbol.replace("numpy.random.default_rng().", "numpy.random.Generator.", 1)
    return symbol


def is_rng_symbol(symbol):
    if not symbol:
        return False
    canonical = canonical_chain_symbol(symbol)
    parts = canonical.replace("()", "").split(".")
    if canonical.startswith(("random.", "numpy.random.", "torchsde.BrownianTree")):
        return parts[-1] in RNG_NAMES or "BrownianTree" in canonical or "Generator" in canonical
    if canonical.startswith("torch."):
        return parts[-1] in RNG_NAMES or ".distributions." in canonical or ".quasirandom." in canonical
    return False


def annotation_is_tensor(node, resolver):
    if node is None:
        return False
    for child in ast.walk(node):
        resolved = resolver.resolve(child)
        if resolved and (resolved.endswith(".Tensor") or resolved == "Tensor"):
            return True
    return False


class TensorNameCollector(ast.NodeVisitor):
    def __init__(self, resolver):
        self.resolver = resolver
        self.scope = []
        self.names = defaultdict(set)

    def scope_name(self):
        return ".".join(self.scope)

    def mark_targets(self, target):
        if isinstance(target, ast.Name):
            self.names[self.scope_name()].add(target.id)
        elif isinstance(target, (ast.Tuple, ast.List)):
            for item in target.elts:
                self.mark_targets(item)

    def likely_tensor(self, node):
        if isinstance(node, ast.Name):
            return node.id in self.names[self.scope_name()]
        if isinstance(node, ast.Subscript):
            return self.likely_tensor(node.value)
        if isinstance(node, (ast.BinOp, ast.UnaryOp)):
            return any(self.likely_tensor(child) for child in ast.iter_child_nodes(node))
        if isinstance(node, ast.Call):
            symbol = canonical_chain_symbol(self.resolver.resolve(node.func))
            if symbol and is_tensor_symbol(symbol):
                if any(part in symbol for part in (".device", ".dtype", ".Size", ".is_", ".compile")):
                    return False
                if ".nn." in symbol and ".functional." not in symbol and symbol.split(".")[-1][:1].isupper():
                    return False
                return True
            if isinstance(node.func, ast.Attribute) and node.func.attr in TENSOR_METHODS:
                return self.likely_tensor(node.func.value)
        return False

    def visit_FunctionDef(self, node):
        self.scope.append(node.name)
        for argument in list(node.args.posonlyargs) + list(node.args.args) + list(node.args.kwonlyargs):
            if annotation_is_tensor(argument.annotation, self.resolver):
                self.names[self.scope_name()].add(argument.arg)
        if node.args.vararg and annotation_is_tensor(node.args.vararg.annotation, self.resolver):
            self.names[self.scope_name()].add(node.args.vararg.arg)
        if node.args.kwarg and annotation_is_tensor(node.args.kwarg.annotation, self.resolver):
            self.names[self.scope_name()].add(node.args.kwarg.arg)
        for statement in node.body:
            self.visit(statement)
        self.scope.pop()

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_ClassDef(self, node):
        self.scope.append(node.name)
        for statement in node.body:
            self.visit(statement)
        self.scope.pop()

    def visit_Assign(self, node):
        self.visit(node.value)
        if self.likely_tensor(node.value):
            for target in node.targets:
                self.mark_targets(target)

    def visit_AnnAssign(self, node):
        if node.value:
            self.visit(node.value)
        if annotation_is_tensor(node.annotation, self.resolver) or (node.value and self.likely_tensor(node.value)):
            self.mark_targets(node.target)


def custom_autograd_scopes(tree, resolver):
    scopes = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.ClassDef):
            continue
        if "torch.autograd.Function" not in {resolver.resolve(base) for base in node.bases}:
            continue
        for child in node.body:
            if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)):
                scopes.add("{}.{}".format(node.name, child.name))
    return scopes


class UsageVisitor(ast.NodeVisitor):
    def __init__(self, source_file, classification, parser_mode, tree, resolver, tensor_names, autograd_scopes):
        self.source_file = source_file
        self.classification = classification
        self.tier = source_tier(classification, source_file)
        self.parser_mode = parser_mode
        self.tree = tree
        self.resolver = resolver
        self.tensor_names = tensor_names
        self.autograd_scopes = autograd_scopes
        self.scope = []
        self.tensor_usages = []
        self.autograd_usages = []
        self.rng_usages = []
        self.parents = {}
        for parent in ast.walk(tree):
            for child in ast.iter_child_nodes(parent):
                self.parents[id(child)] = parent
        self.decorator_nodes = set()
        self.annotation_nodes = set()
        self.type_base_nodes = set()
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                for decorator in node.decorator_list:
                    self.decorator_nodes.update(id(child) for child in ast.walk(decorator))
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                annotations = [argument.annotation for argument in list(node.args.posonlyargs) + list(node.args.args) + list(node.args.kwonlyargs)]
                annotations.extend([node.args.vararg.annotation if node.args.vararg else None, node.args.kwarg.annotation if node.args.kwarg else None, node.returns])
                for annotation in annotations:
                    if annotation:
                        self.annotation_nodes.update(id(child) for child in ast.walk(annotation))
            elif isinstance(node, ast.AnnAssign):
                self.annotation_nodes.update(id(child) for child in ast.walk(node.annotation))
            elif isinstance(node, ast.ClassDef):
                for base in node.bases:
                    self.type_base_nodes.update(id(child) for child in ast.walk(base))

    def scope_name(self):
        return ".".join(self.scope)

    def resolution(self, base):
        if isinstance(base, ast.Call):
            symbol = canonical_chain_symbol(self.resolver.resolve(base.func))
            if symbol and is_tensor_symbol(symbol):
                return "static-call-result"
        if isinstance(base, ast.Name) and base.id in self.tensor_names.get(self.scope_name(), set()):
            return "flow-inferred-tensor"
        if isinstance(base, (ast.Attribute, ast.Subscript)):
            root = base
            while isinstance(root, (ast.Attribute, ast.Subscript)):
                root = root.value
            if isinstance(root, ast.Name) and root.id in self.tensor_names.get(self.scope_name(), set()):
                return "flow-inferred-tensor"
        return "receiver-unverified"

    def add(self, destination, symbol, node, usage_kind, resolution, expression=None):
        destination.append(Usage(
            symbol=symbol,
            source_file=self.source_file,
            line=getattr(node, "lineno", 0),
            column=getattr(node, "col_offset", 0),
            usage_kind=usage_kind,
            resolution="{}+{}".format(resolution, self.parser_mode) if self.parser_mode != "native-ast" else resolution,
            source_classification=self.classification,
            source_tier=self.tier,
            enclosing_symbol=self.scope_name(),
            expression=expression if expression is not None else safe_unparse(node),
        ))

    def visit_FunctionDef(self, node):
        self.scope.append(node.name)
        self.generic_visit(node)
        self.scope.pop()

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_ClassDef(self, node):
        self.scope.append(node.name)
        self.generic_visit(node)
        self.scope.pop()

    def visit_Call(self, node):
        resolved = self.resolver.resolve(node.func)
        canonical = canonical_chain_symbol(resolved)
        decorator = id(node) in self.decorator_nodes
        if canonical and is_tensor_symbol(canonical) and "()" not in canonical:
            kind = "decorator-call" if decorator else "call"
            self.add(self.tensor_usages, canonical, node, kind, "static-import-resolution")
        elif isinstance(node.func, ast.Attribute) and node.func.attr in TENSOR_METHODS:
            if self.resolver.imported_non_tensor_receiver(node.func.value):
                self.add(
                    self.tensor_usages,
                    "torch.Tensor.{}".format(node.func.attr),
                    node,
                    "reclassified-external-call",
                    "imported-non-tensor:{}".format(canonical),
                )
            else:
                receiver_resolution = self.resolution(node.func.value)
                kind = "decorator-call" if decorator else (
                    "receiver-inferred-call" if receiver_resolution != "receiver-unverified" else "receiver-unverified-call-candidate"
                )
                self.add(self.tensor_usages, "torch.Tensor.{}".format(node.func.attr), node, kind, receiver_resolution)

        if isinstance(node.func, ast.Attribute) and node.func.attr in AUTOGRAD_METHODS:
            receiver_resolution = self.resolution(node.func.value)
            symbol = "torch.Tensor.{}".format(node.func.attr)
            if node.func.attr in {"save_for_backward", "mark_dirty", "mark_non_differentiable", "set_materialize_grads"}:
                symbol = "torch.autograd.FunctionCtx.{}".format(node.func.attr)
                if isinstance(node.func.value, ast.Name) and node.func.value.id in {"ctx", "context"}:
                    receiver_resolution = "custom-function-context"
            self.add(self.autograd_usages, symbol, node, "method-call", receiver_resolution)

        if canonical and ("autograd" in canonical or canonical in {
            "torch.no_grad", "torch.enable_grad", "torch.inference_mode", "torch.set_grad_enabled",
            "torch.autocast",
        } or canonical.startswith(("torch.utils.checkpoint.", "torch.amp.", "torch.cuda.amp.", "torch.optim."))):
            self.add(self.autograd_usages, canonical, node, "decorator-or-context-call" if decorator else "api-call", "static-import-resolution")

        if isinstance(node.func, ast.Attribute) and node.func.attr in {"step", "zero_grad", "scale", "unscale_", "update"}:
            receiver = safe_unparse(node.func.value).lower()
            if "optim" in receiver or "scaler" in receiver or "grad_scaler" in receiver:
                self.add(
                    self.autograd_usages,
                    "torch.training.{}".format(node.func.attr),
                    node,
                    "optimizer-or-scaler-call",
                    "receiver-name-inferred",
                )

        for keyword in node.keywords:
            if keyword.arg == "requires_grad":
                self.add(self.autograd_usages, "torch.Tensor.requires_grad keyword", keyword.value, "constructor-state", "keyword-syntax", safe_unparse(keyword.value))

        rng_symbol = canonical
        rng_resolution = "static-import-resolution"
        if not is_rng_symbol(rng_symbol) and isinstance(node.func, ast.Attribute) and node.func.attr in RNG_METHODS:
            receiver_text = safe_unparse(node.func.value)
            receiver_name = receiver_text.lower()
            if any(marker in receiver_name for marker in ("generator", "rng", "random", "sobol")):
                rng_symbol = "generator.{}".format(node.func.attr)
                rng_resolution = "receiver-name-inferred"
            else:
                rng_symbol = None
        if rng_symbol and (is_rng_symbol(rng_symbol) or rng_resolution == "receiver-name-inferred"):
            seed_expression = ""
            if node.args and (rng_symbol.endswith(("manual_seed", ".seed", "default_rng", "Generator")) or "BrownianTree" in rng_symbol):
                seed_expression = safe_unparse(node.args[0])
            for keyword in node.keywords:
                if keyword.arg in {"seed", "entropy"}:
                    seed_expression = safe_unparse(keyword.value)
            generator_expression = ""
            device_expression = ""
            for keyword in node.keywords:
                if keyword.arg == "generator":
                    generator_expression = safe_unparse(keyword.value)
                elif keyword.arg == "device":
                    device_expression = safe_unparse(keyword.value)
            detail = json.dumps({
                "device": device_expression,
                "generator": generator_expression,
                "seed": seed_expression,
            }, sort_keys=True, separators=(",", ":"))
            self.add(self.rng_usages, canonical_chain_symbol(rng_symbol), node, "rng-call", rng_resolution, detail)
        self.generic_visit(node)

    def visit_Attribute(self, node):
        parent = self.parents.get(id(node))
        if isinstance(parent, ast.Attribute) and parent.value is node:
            self.generic_visit(node)
            return
        if isinstance(parent, ast.Call) and parent.func is node:
            self.generic_visit(node)
            return
        resolved = self.resolver.resolve(node)
        if resolved and is_tensor_symbol(resolved):
            final = resolved.split(".")[-1]
            if id(node) in self.decorator_nodes:
                kind = "decorator-reference"
            elif id(node) in self.annotation_nodes or id(node) in self.type_base_nodes or final in TYPE_NAMES:
                kind = "type-reference"
            elif final in MODULE_SEGMENTS:
                kind = "namespace-reference"
            else:
                kind = "value-reference"
            self.add(self.tensor_usages, resolved, node, kind, "static-import-resolution")

        if isinstance(node.ctx, (ast.Load, ast.Store)) and node.attr in AUTOGRAD_STATE:
            receiver_resolution = self.resolution(node.value)
            kind = "state-write" if isinstance(node.ctx, ast.Store) else "state-reference"
            self.add(self.autograd_usages, "torch.Tensor.{}".format(node.attr), node, kind, receiver_resolution)
        if (
            isinstance(node.value, ast.Name)
            and node.value.id in {"ctx", "context"}
            and node.attr in AUTOGRAD_CONTEXT_STATE
            and self.scope_name() in self.autograd_scopes
        ):
            kind = "state-write" if isinstance(node.ctx, ast.Store) else "state-reference"
            self.add(
                self.autograd_usages,
                "torch.autograd.FunctionCtx.{}".format(node.attr),
                node,
                kind,
                "custom-function-context",
            )
        self.generic_visit(node)


def call_group(symbol, usages):
    kinds = {usage.usage_kind for usage in usages}
    final = symbol.split(".")[-1]
    if not any("call" in kind for kind in kinds):
        if "type-reference" in kinds:
            return "type-contract"
        if "namespace-reference" in kinds:
            return "namespace-contract"
        return "value-or-constant-contract"
    if final in RNG_NAMES:
        return "random-number-generation"
    if ".fft." in symbol or symbol.startswith("torch.fft"):
        return "spectral-transform"
    if ".linalg." in symbol or final in {"matmul", "mm", "bmm", "einsum", "tensordot", "svd", "qr"}:
        return "linear-algebra"
    if final in {"reshape", "view", "view_as", "permute", "transpose", "movedim", "flatten", "squeeze", "unsqueeze", "expand", "expand_as", "repeat", "repeat_interleave", "chunk", "split", "unbind", "cat", "stack", "pad"}:
        return "shape-layout-transform"
    if final in {"sum", "mean", "prod", "std", "var", "max", "min", "amax", "amin", "all", "any", "norm", "argmax", "argmin"}:
        return "reduction"
    if final in {"gather", "scatter", "scatter_", "index_add", "index_add_", "index_copy_", "index_put_", "select", "narrow", "nonzero", "where", "masked_select", "masked_fill", "masked_fill_"}:
        return "indexing-masking"
    if ".nn.functional." in symbol:
        if any(name in final for name in ("conv", "pool", "interpolate", "grid_sample", "affine_grid")):
            return "spatial-functional-kernel"
        if any(name in final for name in ("norm", "softmax", "activation", "relu", "gelu", "silu")):
            return "activation-normalization-functional"
        return "neural-network-functional"
    if ".nn." in symbol and final[:1].isupper():
        return "neural-network-module"
    if symbol.startswith(("torchvision.", "torchaudio.", "kornia.", "einops.", "einops_exts.")):
        return "external-tensor-kernel"
    if symbol.startswith(("xformers.", "flash_attn.", "sageattention.", "sageattn3.")):
        return "accelerated-attention-kernel"
    if final in {"to", "cpu", "cuda", "half", "float", "double", "bfloat16", "type", "type_as", "contiguous", "clone", "copy_", "numpy"}:
        return "storage-dtype-device"
    if final in {"tensor", "as_tensor", "from_numpy", "empty", "zeros", "ones", "full", "arange", "linspace", "eye"}:
        return "tensor-creation"
    if symbol.startswith("comfy.ops"):
        return "comfy-operator-indirection"
    return "elementwise-or-runtime-operation"


def operation_requirements(group):
    shape = {
        "shape-layout-transform": "Match rank, axis normalization, inferred dimensions, broadcasting, view-versus-copy behavior, and invalid-shape failures for every listed call.",
        "reduction": "Match reduced axes, keep-dimension behavior, empty-domain behavior, output rank, and scalar-versus-tensor results.",
        "indexing-masking": "Match index bounds, negative indices, mask broadcasting, repeated-index behavior, output shape, and invalid-index failures.",
        "spatial-functional-kernel": "Match batch/channel/spatial conventions, padding, stride, dilation, groups, interpolation coordinates, and invalid geometry failures.",
        "linear-algebra": "Match batch broadcasting, contraction dimensions, transpose conventions, singular/degenerate cases, and result ranks.",
    }.get(group, "Match PyTorch-visible rank, shape, broadcasting, axis, scalar, empty-input, and invalid-shape behavior at all cataloged call sites.")
    dtype = "Match accepted dtypes, scalar coercion, promotion, accumulation dtype, cast/quantization rules, overflow, and unsupported-dtype errors; never silently widen or narrow differently."
    layout = "Match strides, contiguity requirements, aliasing, in-place mutation, view/copy identity, memory format, and overlap rejection where externally observable."
    device = "Provide the operation on CPU and each certified native backend used by the source call sites; match transfer, co-location, fallback prohibition, synchronization, and device-error behavior."
    numerics = "Define deterministic and fast numeric modes; match NaN/Inf/signed-zero handling, reduction order obligations, approximation tolerance, and backend-specific variance fixtures."
    vjp = "Implement and verify the VJP/JVP when the operation is reachable from a cataloged autograd path; otherwise prove and record forward-only reachability instead of assuming it."
    cancellation = "Check cooperative cancellation before dispatch and at bounded kernel/tile boundaries; cancelled work must publish no partial tensor, cache entry, model mutation, or output."
    return shape, dtype, layout, device, numerics, vjp, cancellation


def build_tensor_rows(usages):
    grouped = defaultdict(list)
    for usage in usages:
        grouped[usage.symbol].append(usage)
    rows = []
    for symbol in sorted(grouped):
        items = grouped[symbol]
        call_items = [item for item in items if "call" in item.usage_kind]
        production_calls = [item for item in call_items if item.source_tier == "production"]
        test_calls = [item for item in call_items if item.source_tier == "test"]
        support_calls = [item for item in call_items if item.source_tier == "support"]
        references = [item for item in items if "call" not in item.usage_kind]
        kinds = sorted(set(item.usage_kind for item in items))
        group = call_group(symbol, items)
        shape, dtype, layout, device, numerics, vjp, cancellation = operation_requirements(group)
        confidence = "high"
        if any(item.usage_kind == "receiver-unverified-call-candidate" for item in items):
            confidence = "low"
        elif any(item.usage_kind == "receiver-inferred-call" for item in items):
            confidence = "medium"
        evidence = "test-backed" if test_calls and confidence != "low" else "code-inferred"
        inventory_kind = "reclassified-external-operation" if call_items and all(
            item.usage_kind == "reclassified-external-call" for item in call_items
        ) else "callable-operation" if call_items else (
            "type-reference" if "type-reference" in kinds else "namespace-or-value-reference"
        )
        rows.append({
            "operation_id": stable_id("COMFY-TENSOR-OP", symbol),
            "symbol": symbol,
            "semantic_group": group,
            "inventory_kind": inventory_kind,
            "usage_kinds": " | ".join(kinds),
            "production_call_count": len(production_calls),
            "test_call_count": len(test_calls),
            "support_call_count": len(support_calls),
            "decorator_call_count": sum(item.usage_kind == "decorator-call" for item in items),
            "type_reference_count": sum(item.usage_kind == "type-reference" for item in items),
            "namespace_reference_count": sum(item.usage_kind == "namespace-reference" for item in items),
            "value_reference_count": sum(item.usage_kind in {"value-reference", "decorator-reference"} for item in items),
            "production_call_sites": sorted_join(item.site() for item in production_calls),
            "test_call_sites": sorted_join(item.site() for item in test_calls),
            "support_call_sites": sorted_join(item.site() for item in support_calls),
            "non_call_reference_sites": sorted_join(item.site() for item in references),
            "resolution": sorted_join(item.resolution for item in items),
            "source_classifications": sorted_join(item.source_classification for item in items),
            "availability": source_availability(items),
            "evidence_level": evidence,
            "confidence": confidence,
            "shape_requirement": shape,
            "dtype_requirement": dtype,
            "layout_requirement": layout,
            "device_requirement": device,
            "numerics_requirement": numerics,
            "vjp_jvp_requirement": vjp,
            "cancellation_requirement": cancellation,
            "native_rust_decision": "Implement behind the Sim-owned comfy_tensor facade; third-party crate APIs and backend-specific handles must not become workflow or plugin ABI.",
            "limitations": "Receiver-unverified candidates require call-graph/type confirmation before implementation closure." if confidence == "low" else "Static resolution does not prove runtime branch reachability or exact overload selection.",
        })
    return rows


def function_signature(node):
    arguments = []
    for argument in list(node.args.posonlyargs) + list(node.args.args):
        arguments.append(argument.arg)
    if node.args.vararg:
        arguments.append("*{}".format(node.args.vararg.arg))
    for argument in node.args.kwonlyargs:
        arguments.append(argument.arg)
    if node.args.kwarg:
        arguments.append("**{}".format(node.args.kwarg.arg))
    return "{}({})".format(node.name, ", ".join(arguments))


def discover_custom_autograd(file_records):
    rows = []
    for record in file_records:
        tree = record["tree"]
        resolver = record["resolver"]
        source_file = record["source_file"]
        classification = record["classification"]
        tier = source_tier(classification, source_file)
        for node in ast.walk(tree):
            if not isinstance(node, ast.ClassDef):
                continue
            bases = [resolver.resolve(base) for base in node.bases]
            if "torch.autograd.Function" not in bases:
                continue
            methods = {child.name: child for child in node.body if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef))}
            method_sites = ["{}:{} ({})".format(source_file, method.lineno, function_signature(method)) for method in methods.values()]
            apply_sites = []
            for candidate in ast.walk(tree):
                if isinstance(candidate, ast.Call) and isinstance(candidate.func, ast.Attribute):
                    if candidate.func.attr == "apply" and isinstance(candidate.func.value, ast.Name) and candidate.func.value.id == node.name:
                        apply_sites.append("{}:{}:{}".format(source_file, candidate.lineno, candidate.col_offset + 1))
            usages = [Usage(
                symbol=node.name, source_file=source_file, line=node.lineno, column=node.col_offset,
                usage_kind="custom-function", resolution="static-base-class", source_classification=classification,
                source_tier=tier, enclosing_symbol=node.name, expression=node.name,
            )]
            rows.append({
                "autograd_id": stable_id("COMFY-AUTOGRAD", "custom:{}:{}".format(source_file, node.name)),
                "construct": "custom-autograd-function",
                "symbol": node.name,
                "production_use_count": 1 if tier == "production" else 0,
                "test_use_count": 1 if tier == "test" else 0,
                "support_use_count": 1 if tier == "support" else 0,
                "production_sites": "{}:{}".format(source_file, node.lineno) if tier == "production" else "",
                "test_sites": "{}:{}".format(source_file, node.lineno) if tier == "test" else "",
                "support_sites": "{}:{}".format(source_file, node.lineno) if tier == "support" else "",
                "method_or_state_sites": " | ".join(sorted(method_sites)),
                "forward_contract": function_signature(methods["forward"]) if "forward" in methods else "missing forward definition",
                "reverse_contract": sorted_join(function_signature(methods[name]) for name in ("backward", "vjp", "jvp") if name in methods),
                "apply_sites": " | ".join(sorted(apply_sites)),
                "resolution": "static subclass of torch.autograd.Function",
                "availability": source_availability(usages),
                "evidence_level": "code-inferred",
                "confidence": "high",
                "native_requirement": "Implement the exact saved-tensor/context lifecycle, forward result structure, backward arity, None-gradient behavior, higher-order-gradient policy, device/dtype propagation, and error behavior; add finite-difference and source-oracle VJP fixtures.",
                "state_and_lifetime_requirement": "Saved tensors and metadata remain owned until the graph releases them; cancellation or worker failure drops the graph without publishing partial parameter gradients.",
                "limitations": "Static definitions and same-file apply sites do not prove every dynamic invocation or higher-order differentiation path.",
            })
    return rows


def autograd_construct(symbol):
    if symbol.startswith("torch.autograd.FunctionCtx"):
        return "custom-function-context"
    if "checkpoint" in symbol:
        return "activation-checkpointing"
    if ".amp." in symbol or symbol == "torch.autocast":
        return "mixed-precision-autograd"
    if symbol.startswith(("torch.optim.", "torch.training.")):
        return "optimizer-or-gradient-scaler"
    if symbol in {"torch.no_grad", "torch.enable_grad", "torch.inference_mode", "torch.set_grad_enabled"}:
        return "gradient-mode"
    if "requires_grad" in symbol or symbol.endswith(("grad_fn", "is_leaf")):
        return "gradient-state"
    if symbol.endswith("backward") or symbol.endswith("grad") or ".autograd.grad" in symbol:
        return "reverse-mode-execution"
    if symbol.endswith(("detach", "detach_", "data")):
        return "graph-detachment-or-storage-alias"
    if symbol.endswith(("register_hook", "retain_grad")):
        return "gradient-hook-or-retention"
    return "autograd-api"


def build_autograd_rows(usages, custom_rows):
    grouped = defaultdict(list)
    for usage in usages:
        grouped[usage.symbol].append(usage)
    rows = list(custom_rows)
    for symbol in sorted(grouped):
        items = grouped[symbol]
        production = [item for item in items if item.source_tier == "production"]
        tests = [item for item in items if item.source_tier == "test"]
        support = [item for item in items if item.source_tier == "support"]
        confidence = "low" if any("unverified" in item.resolution for item in items) else (
            "medium" if any("inferred" in item.resolution for item in items) else "high"
        )
        rows.append({
            "autograd_id": stable_id("COMFY-AUTOGRAD", "usage:{}".format(symbol)),
            "construct": autograd_construct(symbol),
            "symbol": symbol,
            "production_use_count": len(production),
            "test_use_count": len(tests),
            "support_use_count": len(support),
            "production_sites": sorted_join(item.site() for item in production),
            "test_sites": sorted_join(item.site() for item in tests),
            "support_sites": sorted_join(item.site() for item in support),
            "method_or_state_sites": sorted_join(item.site() for item in items),
            "forward_contract": "Preserve forward values, tensor aliasing, dtype/device, gradient-mode nesting, and source-visible validation at every site.",
            "reverse_contract": "Provide the exact VJP accumulation, broadcasting reduction, None-gradient, in-place versioning, hook order, and repeated-backward behavior required by reachable sites.",
            "apply_sites": "",
            "resolution": sorted_join(item.resolution for item in items),
            "availability": source_availability(items),
            "evidence_level": "test-backed" if tests and confidence != "low" else "code-inferred",
            "confidence": confidence,
            "native_requirement": "The native comfy_tensor autograd engine SHALL expose this construct without Python, preserve source error/cancellation behavior, and pass analytical, finite-difference, graph-lifetime, and conformance-oracle tests.",
            "state_and_lifetime_requirement": "Gradient state is worker-owned, versioned across in-place mutation, released deterministically, excluded from inference caches unless declared, and discarded atomically on cancellation or recovery.",
            "limitations": "Generic receiver identity remains unverified and may include a same-named non-tensor method." if confidence == "low" else "Static evidence does not prove branch reachability or numerical gradient correctness.",
        })
    return sorted(rows, key=lambda row: row["autograd_id"])


def rng_phase(usage):
    path = usage.source_file
    enclosing = usage.enclosing_symbol.lower()
    if usage.source_tier == "test":
        return "test-fixture"
    if "dataset" in path or "train" in path:
        return "training-and-data-order"
    if path.endswith("float.py") or "quant" in path:
        return "stochastic-quantization"
    if any(marker in path for marker in ("k_diffusion", "samplers.py", "sample.py", "extra_samplers", "nodes_custom_sampler")):
        return "sampling-noise-and-solver"
    if "context_windows" in path:
        return "context-window-selection"
    if any(marker in path for marker in ("ldm/", "text_encoders/", "model_base.py", "image_encoders/")):
        return "model-internal-stochasticity"
    if "prefix" in enclosing or path in {"nodes.py", "comfy_api/latest/_ui.py"}:
        return "temporary-output-naming"
    if "noise" in enclosing or "noise" in path:
        return "node-level-noise"
    if "api" in path:
        return "api-media-conversion"
    return "runtime-utility"


def rng_details(usage):
    try:
        return json.loads(usage.expression)
    except Exception:
        return {"device": "", "generator": "", "seed": ""}


def seededness(symbol, details):
    if symbol.endswith(("manual_seed", ".seed", "set_rng_state", "set_state")):
        return "state-mutator"
    if details.get("seed") or details.get("generator"):
        return "explicit-seed-or-generator"
    if symbol.endswith("default_rng"):
        return "entropy-default"
    if symbol.endswith(("Generator", "get_rng_state", "get_state")):
        return "state-constructor-or-snapshot"
    return "implicit-global-or-object-state"


def build_rng_rows(usages):
    grouped = defaultdict(list)
    for usage in usages:
        grouped[(rng_phase(usage), usage.symbol, usage.resolution)].append(usage)
    rows = []
    for (phase, symbol, resolution), items in sorted(grouped.items()):
        production = [item for item in items if item.source_tier == "production"]
        tests = [item for item in items if item.source_tier == "test"]
        support = [item for item in items if item.source_tier == "support"]
        details = [rng_details(item) for item in items]
        seed_expressions = sorted_join(detail.get("seed", "") for detail in details)
        generator_expressions = sorted_join(detail.get("generator", "") for detail in details)
        device_expressions = sorted_join(detail.get("device", "") for detail in details)
        modes = sorted_join(seededness(symbol, detail) for detail in details)
        rows.append({
            "rng_id": stable_id("COMFY-RNG", "{}:{}:{}".format(phase, symbol, resolution)),
            "phase": phase,
            "symbol": symbol,
            "resolution": resolution,
            "seededness": modes,
            "seed_expressions": seed_expressions,
            "generator_expressions": generator_expressions,
            "device_expressions": device_expressions,
            "production_call_count": len(production),
            "test_call_count": len(tests),
            "support_call_count": len(support),
            "production_call_sites": sorted_join(item.site() for item in production),
            "test_call_sites": sorted_join(item.site() for item in tests),
            "support_call_sites": sorted_join(item.site() for item in support),
            "availability": source_availability(items),
            "evidence_level": "test-backed" if production and tests else ("test-backed" if tests and not production else "code-inferred"),
            "confidence": "medium" if "inferred" in resolution else "high",
            "phase_identity_requirement": "Derive an independent versioned stream from workflow seed, node identity, execution ordinal, phase '{}', sample/batch index, and declared retry policy; unrelated phases must not perturb this stream.".format(phase),
            "seed_mapping_requirement": "Match signedness, width, modulo/wrap rules, per-batch expansion, Brownian entropy lists, and CPU-versus-device generator selection represented by the recorded expressions.",
            "state_requirement": "Make RNG state explicit, serializable where restart parity requires it, and excluded from cache identity only when the node declares deterministic replay from the same phase key.",
            "device_requirement": "Define the algorithm and output sequence per certified backend; CPU-seeded device transfer and native-device generation are distinct contracts and may not be silently interchanged.",
            "cancellation_retry_requirement": "Cancellation, validation failure, OOM retry, and worker recovery must not commit partial RNG advancement; retry SHALL either replay the same phase stream or use an explicitly versioned retry substream.",
            "native_rust_decision": "Implement in the native worker with named phase streams; no Python, NumPy, PyTorch, browser JavaScript, or process-global RNG is permitted in production.",
            "limitations": "Static argument extraction does not prove runtime seed values, external torchsde algorithms, or branch reachability.",
        })
    return rows


def count_rows(rows, key):
    return dict(sorted(Counter(row[key] for row in rows).items()))


def markdown_table(mapping, left, right):
    lines = ["| {} | {} |".format(left, right), "|---|---:|"]
    lines.extend("| {} | {} |".format(key, value) for key, value in mapping.items())
    return "\n".join(lines)


def validate_receiver_classification():
    tree = ast.parse("import numpy as np\nnp.roll(values, 1, axis=0)\nvalues.roll(1, 0)\n")
    resolver = Resolver(tree)
    calls = [node for node in ast.walk(tree) if isinstance(node, ast.Call)]
    numpy_call = next(call for call in calls if resolver.resolve(call.func) == "numpy.roll")
    unknown_call = next(call for call in calls if resolver.resolve(call.func) == "values.roll")
    if not resolver.imported_non_tensor_receiver(numpy_call.func.value):
        raise RuntimeError("imported NumPy receivers must not become torch.Tensor candidates")
    if resolver.imported_non_tensor_receiver(unknown_call.func.value):
        raise RuntimeError("unresolved value receivers must remain visible for type-flow closure")


def main():
    validate_receiver_classification()
    coverage_rows = read_csv(SOURCE_COVERAGE)
    coverage = {row["source_file"]: row["classification"] for row in coverage_rows}
    backend = json.loads(BACKEND_RECONCILIATION.read_text(encoding="utf-8"))
    python_files = sorted(path for path in SOURCE.rglob("*.py") if path.is_file())
    missing_coverage = [str(path.relative_to(SOURCE)) for path in python_files if str(path.relative_to(SOURCE)) not in coverage]
    if missing_coverage:
        raise RuntimeError("Python files absent from backend source coverage: {}".format(missing_coverage))

    file_records = []
    tensor_usages = []
    autograd_usages = []
    rng_usages = []
    parser_modes = Counter()
    for path in python_files:
        relative = str(path.relative_to(SOURCE))
        tree, parser_mode = parse_source(path)
        parser_modes[parser_mode] += 1
        resolver = Resolver(tree)
        name_collector = TensorNameCollector(resolver)
        name_collector.visit(tree)
        visitor = UsageVisitor(
            relative,
            coverage[relative],
            parser_mode,
            tree,
            resolver,
            name_collector.names,
            custom_autograd_scopes(tree, resolver),
        )
        visitor.visit(tree)
        tensor_usages.extend(visitor.tensor_usages)
        autograd_usages.extend(visitor.autograd_usages)
        rng_usages.extend(visitor.rng_usages)
        file_records.append({
            "classification": coverage[relative],
            "parser_mode": parser_mode,
            "resolver": resolver,
            "source_file": relative,
            "tree": tree,
        })

    tensor_rows = build_tensor_rows(tensor_usages)
    custom_autograd = discover_custom_autograd(file_records)
    autograd_rows = build_autograd_rows(autograd_usages, custom_autograd)
    rng_rows = build_rng_rows(rng_usages)

    tensor_fields = [
        "operation_id", "symbol", "semantic_group", "inventory_kind", "usage_kinds",
        "production_call_count", "test_call_count", "support_call_count", "decorator_call_count",
        "type_reference_count", "namespace_reference_count", "value_reference_count",
        "production_call_sites", "test_call_sites", "support_call_sites", "non_call_reference_sites",
        "resolution", "source_classifications", "availability", "evidence_level", "confidence",
        "shape_requirement", "dtype_requirement", "layout_requirement", "device_requirement",
        "numerics_requirement", "vjp_jvp_requirement", "cancellation_requirement",
        "native_rust_decision", "limitations",
    ]
    autograd_fields = [
        "autograd_id", "construct", "symbol", "production_use_count", "test_use_count",
        "support_use_count", "production_sites", "test_sites", "support_sites",
        "method_or_state_sites", "forward_contract", "reverse_contract", "apply_sites", "resolution",
        "availability", "evidence_level", "confidence", "native_requirement",
        "state_and_lifetime_requirement", "limitations",
    ]
    rng_fields = [
        "rng_id", "phase", "symbol", "resolution", "seededness", "seed_expressions",
        "generator_expressions", "device_expressions", "production_call_count", "test_call_count",
        "support_call_count", "production_call_sites", "test_call_sites", "support_call_sites",
        "availability", "evidence_level", "confidence", "phase_identity_requirement",
        "seed_mapping_requirement", "state_requirement", "device_requirement",
        "cancellation_retry_requirement", "native_rust_decision", "limitations",
    ]
    write_csv(CATALOGS / "backend-tensor-operations.csv", tensor_fields, tensor_rows)
    write_csv(CATALOGS / "backend-autograd.csv", autograd_fields, autograd_rows)
    write_csv(CATALOGS / "backend-rng.csv", rng_fields, rng_rows)

    tensor_site_totals = {
        "production_calls": sum(int(row["production_call_count"]) for row in tensor_rows),
        "test_calls": sum(int(row["test_call_count"]) for row in tensor_rows),
        "support_calls": sum(int(row["support_call_count"]) for row in tensor_rows),
        "decorator_calls": sum(int(row["decorator_call_count"]) for row in tensor_rows),
        "type_references": sum(int(row["type_reference_count"]) for row in tensor_rows),
        "namespace_references": sum(int(row["namespace_reference_count"]) for row in tensor_rows),
        "value_references": sum(int(row["value_reference_count"]) for row in tensor_rows),
    }
    autograd_totals = {
        "production_uses": sum(int(row["production_use_count"]) for row in autograd_rows),
        "test_uses": sum(int(row["test_use_count"]) for row in autograd_rows),
        "support_uses": sum(int(row["support_use_count"]) for row in autograd_rows),
    }
    rng_totals = {
        "production_calls": sum(int(row["production_call_count"]) for row in rng_rows),
        "test_calls": sum(int(row["test_call_count"]) for row in rng_rows),
        "support_calls": sum(int(row["support_call_count"]) for row in rng_rows),
    }
    manifest = {}
    for name, rows in (
        ("backend-tensor-operations.csv", tensor_rows),
        ("backend-autograd.csv", autograd_rows),
        ("backend-rng.csv", rng_rows),
    ):
        data = (CATALOGS / name).read_bytes()
        manifest[name] = {"rows": len(rows), "sha256": sha256_bytes(data)}

    reconciliation = {
        "schema_version": 1,
        "generated_by": "generate_tensor_runtime_catalogs.py",
        "generator_sha256": sha256_bytes(Path(__file__).read_bytes()),
        "source_baseline": backend["baseline"],
        "source_closure": {
            "backend_source_coverage_rows": len(coverage_rows),
            "python_files_scanned": len(python_files),
            "missing_from_backend_source_coverage": len(missing_coverage),
            "parser_modes": dict(sorted(parser_modes.items())),
            "python_file_classifications": dict(sorted(Counter(coverage[str(path.relative_to(SOURCE))] for path in python_files).items())),
            "python_file_execution_tiers": dict(sorted(Counter(
                source_tier(coverage[str(path.relative_to(SOURCE))], str(path.relative_to(SOURCE)))
                for path in python_files
            ).items())),
            "note": "This catalog references the canonical backend-source-coverage.csv closure and does not duplicate its 949 per-file rows.",
        },
        "tensor_operations": {
            "rows": len(tensor_rows),
            "inventory_kind": count_rows(tensor_rows, "inventory_kind"),
            "semantic_group": count_rows(tensor_rows, "semantic_group"),
            "availability": count_rows(tensor_rows, "availability"),
            "evidence_level": count_rows(tensor_rows, "evidence_level"),
            "confidence": count_rows(tensor_rows, "confidence"),
            "site_totals": tensor_site_totals,
            "receiver_unverified_rows": sum(row["confidence"] == "low" for row in tensor_rows),
        },
        "autograd": {
            "rows": len(autograd_rows),
            "construct": count_rows(autograd_rows, "construct"),
            "availability": count_rows(autograd_rows, "availability"),
            "evidence_level": count_rows(autograd_rows, "evidence_level"),
            "confidence": count_rows(autograd_rows, "confidence"),
            "custom_function_rows": len(custom_autograd),
            "use_totals": autograd_totals,
        },
        "rng": {
            "rows": len(rng_rows),
            "phase": count_rows(rng_rows, "phase"),
            "availability": count_rows(rng_rows, "availability"),
            "evidence_level": count_rows(rng_rows, "evidence_level"),
            "confidence": count_rows(rng_rows, "confidence"),
            "call_totals": rng_totals,
        },
        "generated_catalogs": manifest,
        "limitations": [
            "This is static code evidence, not runtime observation; branch reachability, dynamic dispatch, overload selection, backend kernels, and numeric equivalence remain unverified.",
            "Tensor method calls without a statically recoverable receiver are retained as low-confidence receiver-unverified candidates instead of being guessed or omitted.",
            "One Python 3.10 match statement file is parsed after syntax-only match/case header normalization; line numbers and call expressions are preserved.",
            "NumPy arithmetic, SciPy, PIL, OpenCV, model container parsing, and media codecs are outside this PyTorch/tensor-runtime surface; NumPy and Python random calls are included only in the RNG catalog.",
            "Test call sites show direct use of the same API symbol, not necessarily semantic coverage of every production call, shape, dtype, device, or failure mode.",
        ],
    }
    reconciliation_path = CATALOGS / "backend-tensor-runtime-reconciliation.json"
    reconciliation_path.write_text(json.dumps(reconciliation, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    report = """# Native tensor-runtime evidence

## Scope and baseline

This evidence pack statically inventories the tensor/operator, autograd, and random-number surfaces that the pinned ComfyUI source uses and turns each surface into an explicit native Rust conformance obligation. Production Sim may use ComfyUI only as a development-time oracle. None of these rows authorizes a Python runtime, a PyTorch process, JavaScript execution, or an external ComfyUI dependency in production.

The source baseline is ComfyUI `{version}` with `{files}` regular files and all-file fingerprint `{fingerprint}`. The generator scanned `{python_files}` Python files and reconciled every one against [`catalogs/backend-source-coverage.csv`](catalogs/backend-source-coverage.csv); it does not duplicate the canonical 949-row source closure. `{native_ast}` files parsed directly with the host AST. `{normalized_ast}` file used syntax-only normalization of Python 3.10 `match`/`case` headers so Python 3.9 could preserve and inspect the original call expressions and line numbers. The canonical source catalog's `infrastructure-only` label can mean an internal implementation-support module with no independently named feature row. Calls in executable product paths are therefore counted as production execution evidence while the original source classification remains preserved on each row; only `.ci`, `.github`, and `script_examples` Python is placed in the support tier.

## Evidence method

The generator resolves imports, aliases, direct PyTorch/ecosystem calls, decorators, types, namespaces, constants, and a bounded Tensor-method vocabulary. A direct imported call is high-confidence static evidence. A method whose receiver flows from an annotated Tensor or a resolved tensor-producing call is medium-confidence. A same-named method whose receiver cannot be proven is retained as a low-confidence candidate. This prevents both silent omission and false claims of certainty.

Existing tests are linked when they directly call the same symbol. Such a link raises the row to `test-backed`, but does not prove that a test covers every production shape, dtype, device, numeric, gradient, cancellation, or error variant. No tensor/model runtime was loaded and no row is classified `observed`.

## Tensor/operator reconciliation

[`catalogs/backend-tensor-operations.csv`](catalogs/backend-tensor-operations.csv) contains `{tensor_rows}` symbol rows. It separates `{callable_rows}` callable operations from `{type_rows}` type rows and `{reference_rows}` namespace/value rows. Its recorded call sites reconcile to `{tensor_prod}` production, `{tensor_test}` test, and `{tensor_support}` support calls. There are `{unverified}` low-confidence receiver-unverified rows; their exact candidate sites remain visible and require type/call-graph confirmation before implementation closure.

{tensor_groups}

Every row carries native shape, dtype, layout, device, numerics, VJP/JVP, and cancellation requirements. The implementation boundary is a Sim-owned `comfy_tensor` facade. A selected compute crate may sit behind that facade, but its types, handles, serialization, and backend assumptions cannot become workflow or Rust/WASM plugin ABI.

## Autograd reconciliation

[`catalogs/backend-autograd.csv`](catalogs/backend-autograd.csv) contains `{autograd_rows}` rows, including `{custom_functions}` explicit `torch.autograd.Function` subclasses. The catalog records forward/reverse method signatures, same-file `.apply` sites, gradient modes, graph detachment, gradient state, hook/retention behavior, and receiver uncertainty. Uses reconcile to `{autograd_prod}` production, `{autograd_test}` test, and `{autograd_support}` support sites.

{autograd_constructs}

Native parity requires graph ownership, saved-tensor lifetimes, broadcasting reduction in VJPs, in-place version checks, None gradients, hook order, repeated backward, finite-difference checks, worker cancellation, and recovery without partial gradient publication. Forward-only implementations are acceptable only after reachability evidence proves a row cannot participate in a cataloged autograd path.

## Phase-scoped RNG reconciliation

[`catalogs/backend-rng.csv`](catalogs/backend-rng.csv) contains `{rng_rows}` rows keyed by mechanism, resolution, and semantic phase. Calls reconcile to `{rng_prod}` production, `{rng_test}` test, and `{rng_support}` support sites.

{rng_phases}

Sim must not use a process-global RNG as an implicit compatibility mechanism. Each row requires a versioned phase identity derived from workflow seed, node identity, execution ordinal, phase, sample or batch index, and declared retry policy. Cancellation, validation failure, OOM retry, and worker recovery may not commit partial RNG advancement. CPU-seeded transfer and native-device generation remain distinct contracts because they can produce different observable sequences.

## Boundaries and limitations

- The inventory is static `code-inferred` or direct-symbol `test-backed` evidence. It does not demonstrate runtime branch reachability, dynamic monkey-patching, actual accelerator kernels, overload selection, or numerical equivalence.
- Dynamic operator selection and calls through arbitrary variables cannot be resolved in general. Receiver-unverified Tensor-method candidates are explicit rows with low confidence.
- NumPy arithmetic, SciPy, PIL, OpenCV, media codecs, and model-container parsing are handled by other parity domains. Python and NumPy random calls are included here because phase RNG affects deterministic behavior.
- External accelerated attention, torchvision, torchaudio, kornia, einops, torchsde, and device-extension calls are obligations to reproduce or reject with source-compatible errors; their presence does not authorize those Python packages in production.
- Direct test-symbol matches are not semantic coverage. Native conformance fixtures must exercise success, boundaries, invalid shape/dtype/device, special values, cancellation, retry, worker crash, persistence, and backend variance.

## Generated artifacts

| Artifact | Rows | SHA-256 |
|---|---:|---|
| [`catalogs/backend-tensor-operations.csv`](catalogs/backend-tensor-operations.csv) | {tensor_rows} | `{tensor_sha}` |
| [`catalogs/backend-autograd.csv`](catalogs/backend-autograd.csv) | {autograd_rows} | `{autograd_sha}` |
| [`catalogs/backend-rng.csv`](catalogs/backend-rng.csv) | {rng_rows} | `{rng_sha}` |
| [`catalogs/backend-tensor-runtime-reconciliation.json`](catalogs/backend-tensor-runtime-reconciliation.json) | reconciliation | generated deterministically |
""".format(
        version=backend["baseline"]["package_version"],
        files=backend["baseline"]["all_file_count"],
        fingerprint=backend["baseline"]["all_file_fingerprint_sha256"],
        python_files=len(python_files),
        native_ast=parser_modes.get("native-ast", 0),
        normalized_ast=parser_modes.get("syntax-normalized-ast", 0),
        tensor_rows=len(tensor_rows),
        callable_rows=sum(row["inventory_kind"] == "callable-operation" for row in tensor_rows),
        type_rows=sum(row["inventory_kind"] == "type-reference" for row in tensor_rows),
        reference_rows=sum(row["inventory_kind"] == "namespace-or-value-reference" for row in tensor_rows),
        tensor_prod=tensor_site_totals["production_calls"],
        tensor_test=tensor_site_totals["test_calls"],
        tensor_support=tensor_site_totals["support_calls"],
        unverified=reconciliation["tensor_operations"]["receiver_unverified_rows"],
        tensor_groups=markdown_table(reconciliation["tensor_operations"]["semantic_group"], "Semantic group", "Rows"),
        autograd_rows=len(autograd_rows),
        custom_functions=len(custom_autograd),
        autograd_prod=autograd_totals["production_uses"],
        autograd_test=autograd_totals["test_uses"],
        autograd_support=autograd_totals["support_uses"],
        autograd_constructs=markdown_table(reconciliation["autograd"]["construct"], "Autograd construct", "Rows"),
        rng_rows=len(rng_rows),
        rng_prod=rng_totals["production_calls"],
        rng_test=rng_totals["test_calls"],
        rng_support=rng_totals["support_calls"],
        rng_phases=markdown_table(reconciliation["rng"]["phase"], "RNG phase", "Rows"),
        tensor_sha=manifest["backend-tensor-operations.csv"]["sha256"],
        autograd_sha=manifest["backend-autograd.csv"]["sha256"],
        rng_sha=manifest["backend-rng.csv"]["sha256"],
    )
    (SPEC / "evidence-tensor-runtime.md").write_text(report, encoding="utf-8")


if __name__ == "__main__":
    main()
