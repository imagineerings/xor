import hashlib
import math
import struct
from fractions import Fraction


STYLE_WIDTH = 8
STYLE_CONTEXT = 6
STYLE_HEADS = 2
STYLE_TOKENS = 2
STYLE_LAYERS = 3
REDUX_INPUT = 4
REDUX_HIDDEN = 12
REDUX_OUTPUT = 6
INPUT_TOKENS = 2


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def bits(value):
    return struct.unpack("<I", struct.pack("<f", f32(value)))[0]


def from_bits(value):
    return struct.unpack("<f", struct.pack("<I", value))[0]


def fadd(left, right):
    return f32(f32(left) + f32(right))


def fmul(left, right):
    return f32(f32(left) * f32(right))


def fdiv(left, right):
    return f32(f32(left) / f32(right))


def round_ratio_ties_even(numerator, denominator):
    quotient, remainder = divmod(numerator, denominator)
    doubled_remainder = remainder * 2
    if doubled_remainder > denominator or (
        doubled_remainder == denominator and quotient & 1
    ):
        quotient += 1
    return quotient


def fraction_to_f32(value):
    if value == 0:
        return 0.0
    sign = 0x80000000 if value < 0 else 0
    value = abs(value)
    numerator = value.numerator
    denominator = value.denominator
    exponent = numerator.bit_length() - denominator.bit_length()
    if exponent >= 0:
        if numerator < denominator << exponent:
            exponent -= 1
    elif numerator << -exponent < denominator:
        exponent -= 1
    if exponent > 127:
        return from_bits(sign | 0x7F800000)
    if exponent >= -126:
        shift = 23 - exponent
        if shift >= 0:
            significand = round_ratio_ties_even(numerator << shift, denominator)
        else:
            significand = round_ratio_ties_even(numerator, denominator << -shift)
        if significand == 1 << 24:
            significand >>= 1
            exponent += 1
            if exponent > 127:
                return from_bits(sign | 0x7F800000)
        return from_bits(sign | ((exponent + 127) << 23) | (significand - (1 << 23)))
    significand = round_ratio_ties_even(numerator << 149, denominator)
    if significand >= 1 << 23:
        return from_bits(sign | (1 << 23))
    return from_bits(sign | significand)


def fmadd(left, right, addend):
    exact = Fraction(f32(left)) * Fraction(f32(right)) + Fraction(f32(addend))
    return fraction_to_f32(exact)


def sigmoid(value):
    exponential = f32(math.exp(f32(-value)))
    return fdiv(1.0, fadd(1.0, exponential))


def silu(value):
    exponential = f32(math.exp(f32(-value)))
    return fdiv(value, fadd(1.0, exponential))


def storage_bits(value, dtype):
    value = f32(value)
    if dtype == "float32":
        return bits(value)
    if dtype == "float16":
        return struct.unpack("<H", struct.pack("<e", value))[0]
    raw = bits(value)
    return ((raw + 0x7FFF + ((raw >> 16) & 1)) >> 16) & 0xFFFF


def project_storage(raw, dtype):
    if dtype == "float32":
        return from_bits(raw)
    if dtype == "float16":
        return f32(struct.unpack("<e", struct.pack("<H", raw))[0])
    return from_bits(raw << 16)


def product(shape):
    result = 1
    for dimension in shape:
        result *= dimension
    return result


def style_manifest():
    definitions = [
        ("style_embedding", [1, STYLE_TOKENS, STYLE_WIDTH]),
        ("proj", [STYLE_WIDTH, STYLE_CONTEXT]),
    ]
    for layer in range(STYLE_LAYERS):
        prefix = f"transformer_layes.{layer}"
        definitions.extend([
            (f"{prefix}.attn.in_proj_weight", [STYLE_WIDTH * 3, STYLE_WIDTH]),
            (f"{prefix}.attn.in_proj_bias", [STYLE_WIDTH * 3]),
            (f"{prefix}.attn.out_proj.weight", [STYLE_WIDTH, STYLE_WIDTH]),
            (f"{prefix}.attn.out_proj.bias", [STYLE_WIDTH]),
            (f"{prefix}.ln_1.weight", [STYLE_WIDTH]),
            (f"{prefix}.ln_1.bias", [STYLE_WIDTH]),
            (f"{prefix}.mlp.c_fc.weight", [STYLE_WIDTH * 4, STYLE_WIDTH]),
            (f"{prefix}.mlp.c_fc.bias", [STYLE_WIDTH * 4]),
            (f"{prefix}.mlp.c_proj.weight", [STYLE_WIDTH, STYLE_WIDTH * 4]),
            (f"{prefix}.mlp.c_proj.bias", [STYLE_WIDTH]),
            (f"{prefix}.ln_2.weight", [STYLE_WIDTH]),
            (f"{prefix}.ln_2.bias", [STYLE_WIDTH]),
        ])
    definitions.extend([
        ("ln_post.weight", [STYLE_WIDTH]),
        ("ln_post.bias", [STYLE_WIDTH]),
        ("ln_pre.weight", [STYLE_WIDTH]),
        ("ln_pre.bias", [STYLE_WIDTH]),
    ])
    return definitions


def redux_manifest():
    return [
        ("redux_up.weight", [REDUX_HIDDEN, REDUX_INPUT]),
        ("redux_up.bias", [REDUX_HIDDEN]),
        ("redux_down.weight", [REDUX_OUTPUT, REDUX_HIDDEN]),
        ("redux_down.bias", [REDUX_OUTPUT]),
    ]


def fixture_value(profile, state_index, value_index, key, shape):
    if key == "style_embedding" and value_index == 0:
        return from_bits(0x80000000)
    if ".attn.in_proj_weight" in key:
        output_row = value_index // shape[1]
        if output_row < STYLE_WIDTH * 2:
            return f32(0.0)
    if ".attn.in_proj_bias" in key and value_index < STYLE_WIDTH * 2:
        return f32(0.0)
    if key.endswith(".weight") and (".ln_" in key or key.startswith("ln_")):
        lane = ((state_index + value_index * 3) % 7) - 3
        return fadd(1.0, fmul(lane, 0.015625))
    if key.endswith(".bias") and (".ln_" in key or key.startswith("ln_")):
        lane = ((state_index * 5 + value_index * 7) % 9) - 4
        return fmul(lane, 0.0078125)
    lane = ((state_index * 19 + value_index * 13 + (3 if profile == "redux" else 0)) % 31) - 15
    scale = 0.00625 if key.endswith(".bias") else 0.0125
    return fmul(lane, scale)


def source_state(profile, dtype):
    definitions = style_manifest() if profile == "style" else redux_manifest()
    state = []
    for state_index, (key, shape) in enumerate(definitions):
        raw = [
            storage_bits(fixture_value(profile, state_index, index, key, shape), dtype)
            for index in range(product(shape))
        ]
        state.append({"key": key, "shape": shape, "storage_bits": raw})
    return state


def projected_state(state, dtype):
    return {entry["key"]: [project_storage(value, dtype) for value in entry["storage_bits"]] for entry in state}


def input_values(width, batch=1):
    values = []
    for index in range(batch * INPUT_TOKENS * width):
        if index == 0:
            values.append(from_bits(0x80000000))
        else:
            values.append(fmul(((index * 11) % 17) - 8, 0.03125))
    return values


def linear(values, rows, input_width, weight, output_width, bias):
    output = []
    for row in range(rows):
        for output_channel in range(output_width):
            total = f32(0.0 if bias is None else bias[output_channel])
            for input_channel in range(input_width):
                total = fmadd(
                    values[row * input_width + input_channel],
                    weight[output_channel * input_width + input_channel],
                    total,
                )
            output.append(total)
    return output


def layer_norm(values, width, weight, bias):
    output = []
    for start in range(0, len(values), width):
        row = values[start:start + width]
        total = 0.0
        for value in row:
            total += float(value)
        mean = f32(total / width)
        variance_total = 0.0
        for value in row:
            difference = float(value) - float(mean)
            variance_total += difference * difference
        variance = variance_total / width
        inverse = f32(1.0 / math.sqrt(variance + float(f32(1.0e-5))))
        for index, value in enumerate(row):
            normalized = fmul(fadd(value, -mean), inverse)
            output.append(fadd(fmul(normalized, weight[index]), bias[index]))
    return output


def add(left, right):
    return [fadd(a, b) for a, b in zip(left, right)]


def attention(values, state, layer, batch, tokens):
    prefix = f"transformer_layes.{layer}.attn"
    fused_weight = state[f"{prefix}.in_proj_weight"]
    fused_bias = state[f"{prefix}.in_proj_bias"]
    width = STYLE_WIDTH
    rows = batch * tokens
    query = linear(values, rows, width, fused_weight[:width * width], width, fused_bias[:width])
    key = linear(values, rows, width, fused_weight[width * width:2 * width * width], width,
                 fused_bias[width:2 * width])
    value = linear(values, rows, width, fused_weight[2 * width * width:], width,
                   fused_bias[2 * width:])
    head_width = width // STYLE_HEADS
    scale = f32(1.0 / math.sqrt(head_width))
    output = [f32(0.0)] * (rows * width)
    for batch_index in range(batch):
        batch_start = batch_index * tokens
        for head in range(STYLE_HEADS):
            for query_token in range(tokens):
                query_row = batch_start + query_token
                scores = []
                for key_token in range(tokens):
                    key_row = batch_start + key_token
                    score = f32(0.0)
                    for component in range(head_width):
                        q = query[query_row * width + head * head_width + component]
                        k = key[key_row * width + head * head_width + component]
                        score = fadd(score, fmul(q, k))
                    scores.append(fmul(score, scale))
                maximum = max(scores)
                probabilities = [f32(math.exp(fadd(score, -maximum))) for score in scores]
                denominator = f32(0.0)
                for probability in probabilities:
                    denominator = fadd(denominator, probability)
                probabilities = [fdiv(probability, denominator) for probability in probabilities]
                for component in range(head_width):
                    result = f32(0.0)
                    for key_token, probability in enumerate(probabilities):
                        key_row = batch_start + key_token
                        lane = value[key_row * width + head * head_width + component]
                        result = fadd(result, fmul(probability, lane))
                    output[query_row * width + head * head_width + component] = result
    return output


def matmul_f64(left, rows, inner, right, columns):
    output = []
    for row in range(rows):
        for column in range(columns):
            total = 0.0
            for lane in range(inner):
                total += float(left[row * inner + lane]) * float(right[lane * columns + column])
            output.append(f32(total))
    return output


def execute_style(state, input_data, batch=1):
    sequence = INPUT_TOKENS + STYLE_TOKENS
    rows = batch * sequence
    embedding = state["style_embedding"]
    embedding = [fadd(value, 0.0) for value in embedding]
    values = []
    input_batch_values = INPUT_TOKENS * STYLE_WIDTH
    for batch_index in range(batch):
        start = batch_index * input_batch_values
        values.extend(input_data[start:start + input_batch_values])
        values.extend(embedding)
    values = layer_norm(values, STYLE_WIDTH, state["ln_pre.weight"], state["ln_pre.bias"])
    for layer in range(STYLE_LAYERS):
        prefix = f"transformer_layes.{layer}"
        normalized = layer_norm(values, STYLE_WIDTH, state[f"{prefix}.ln_1.weight"],
                                state[f"{prefix}.ln_1.bias"])
        attended = attention(normalized, state, layer, batch, sequence)
        projected = linear(attended, rows, STYLE_WIDTH, state[f"{prefix}.attn.out_proj.weight"],
                           STYLE_WIDTH, state[f"{prefix}.attn.out_proj.bias"])
        values = add(values, projected)
        normalized = layer_norm(values, STYLE_WIDTH, state[f"{prefix}.ln_2.weight"],
                                state[f"{prefix}.ln_2.bias"])
        expanded = linear(normalized, rows, STYLE_WIDTH, state[f"{prefix}.mlp.c_fc.weight"],
                          STYLE_WIDTH * 4, state[f"{prefix}.mlp.c_fc.bias"])
        activated = [fmul(value, sigmoid(fmul(1.702, value))) for value in expanded]
        projected = linear(activated, rows, STYLE_WIDTH * 4,
                           state[f"{prefix}.mlp.c_proj.weight"], STYLE_WIDTH,
                           state[f"{prefix}.mlp.c_proj.bias"])
        values = add(values, projected)
    selected = []
    for batch_index in range(batch):
        start = (batch_index * sequence + INPUT_TOKENS) * STYLE_WIDTH
        selected.extend(values[start:start + STYLE_TOKENS * STYLE_WIDTH])
    selected = layer_norm(selected, STYLE_WIDTH, state["ln_post.weight"], state["ln_post.bias"])
    return matmul_f64(
        selected, batch * STYLE_TOKENS, STYLE_WIDTH, state["proj"], STYLE_CONTEXT
    )


def execute_redux(state, input_data):
    expanded = linear(input_data, INPUT_TOKENS, REDUX_INPUT, state["redux_up.weight"],
                      REDUX_HIDDEN, state["redux_up.bias"])
    activated = [silu(value) for value in expanded]
    return linear(activated, INPUT_TOKENS, REDUX_HIDDEN, state["redux_down.weight"],
                  REDUX_OUTPUT, state["redux_down.bias"])


def state_identity(state, dtype, projected=False):
    digest = hashlib.sha256()
    digest.update(b"conditioning-auxiliary-state-v1\0")
    digest.update(("float32" if projected else dtype).encode())
    for entry in state:
        key = entry["key"].encode()
        digest.update(len(key).to_bytes(8, "little"))
        digest.update(key)
        for dimension in entry["shape"]:
            digest.update(dimension.to_bytes(8, "little"))
        for raw in entry["storage_bits"]:
            value = bits(project_storage(raw, dtype)) if projected else raw
            width = 4 if projected or dtype == "float32" else 2
            digest.update(value.to_bytes(width, "little"))
    return digest.hexdigest()


def profile_oracle(profile):
    cases = {}
    input_data = input_values(STYLE_WIDTH if profile == "style" else REDUX_INPUT)
    for dtype in ["float32", "float16", "bfloat16"]:
        state = source_state(profile, dtype)
        projected = projected_state(state, dtype)
        output = execute_style(projected, input_data) if profile == "style" else execute_redux(projected, input_data)
        cases[dtype] = {
            "state": state,
            "source_identity_sha256": state_identity(state, dtype),
            "projected_identity_sha256": state_identity(state, dtype, projected=True),
            "output_shape": [1, STYLE_TOKENS, STYLE_CONTEXT] if profile == "style" else [1, INPUT_TOKENS, REDUX_OUTPUT],
            "output_bits": [bits(value) for value in output],
        }
    return {
        "input_shape": [1, INPUT_TOKENS, STYLE_WIDTH if profile == "style" else REDUX_INPUT],
        "input_bits": [bits(value) for value in input_data],
        "dtypes": cases,
    }


def mutation_oracle(profile, name, key, index, delta):
    state = source_state(profile, "float32")
    for entry in state:
        if entry["key"] == key:
            value = project_storage(entry["storage_bits"][index], "float32")
            entry["storage_bits"][index] = bits(fadd(value, delta))
            break
    projected = projected_state(state, "float32")
    input_data = input_values(STYLE_WIDTH if profile == "style" else REDUX_INPUT)
    output = execute_style(projected, input_data) if profile == "style" else execute_redux(projected, input_data)
    return {
        "profile": profile,
        "key": key,
        "index": index,
        "delta_bits": bits(delta),
        "source_identity_sha256": state_identity(state, "float32"),
        "output_bits": [bits(value) for value in output],
    }


def attention_discriminator_oracle():
    state = source_state("style", "float32")
    modifications = []
    for layer in range(STYLE_LAYERS):
        weight_key = f"transformer_layes.{layer}.attn.in_proj_weight"
        bias_key = f"transformer_layes.{layer}.attn.in_proj_bias"
        layer_scale = f32((layer + 1) * 0.125)
        weight_values = [
            (0 * STYLE_WIDTH + 0, fadd(0.5, layer_scale)),
            (0 * STYLE_WIDTH + 1, -0.25),
            (4 * STYLE_WIDTH + 2, fadd(-0.375, layer_scale)),
            (4 * STYLE_WIDTH + 3, 0.625),
            ((STYLE_WIDTH + 0) * STYLE_WIDTH + 1, fadd(-0.75, layer_scale)),
            ((STYLE_WIDTH + 0) * STYLE_WIDTH + 2, 0.375),
            ((STYLE_WIDTH + 4) * STYLE_WIDTH + 3, fadd(0.875, -layer_scale)),
            ((STYLE_WIDTH + 4) * STYLE_WIDTH + 4, -0.5),
        ]
        bias_values = [
            (0, fadd(0.0625, layer_scale)),
            (4, fadd(-0.09375, layer_scale)),
            (STYLE_WIDTH + 0, fadd(0.15625, -layer_scale)),
            (STYLE_WIDTH + 4, fadd(-0.21875, layer_scale)),
        ]
        for key, values in [(weight_key, weight_values), (bias_key, bias_values)]:
            entry = next(candidate for candidate in state if candidate["key"] == key)
            for index, value in values:
                value_bits = bits(value)
                entry["storage_bits"][index] = value_bits
                modifications.append({"key": key, "index": index, "value_bits": value_bits})
    projected = projected_state(state, "float32")
    batch = 2
    input_data = input_values(STYLE_WIDTH, batch)
    output = execute_style(projected, input_data, batch)
    batch_output_values = STYLE_TOKENS * STYLE_CONTEXT
    return {
        "state_modifications": modifications,
        "source_identity_sha256": state_identity(state, "float32"),
        "input_shape": [batch, INPUT_TOKENS, STYLE_WIDTH],
        "input_bits": [bits(value) for value in input_data],
        "output_shape": [batch, STYLE_TOKENS, STYLE_CONTEXT],
        "output_bits": [bits(value) for value in output],
        "batch_outputs_differ": output[:batch_output_values] != output[batch_output_values:],
        "query_key_are_asymmetric": any(
            modification["index"] < STYLE_WIDTH * STYLE_WIDTH
            for modification in modifications
        ) and any(
            STYLE_WIDTH * STYLE_WIDTH
            <= modification["index"]
            < STYLE_WIDTH * STYLE_WIDTH * 2
            for modification in modifications
        ),
    }


def build_oracle():
    quick_input = f32(0.375)
    quick_scaled = fmul(1.702, quick_input)
    quick_sigmoid = sigmoid(quick_scaled)
    mutations = {
        "style_embedding": mutation_oracle("style", "style_embedding", "style_embedding", 1, 0.125),
        "style_value_projection": mutation_oracle(
            "style", "style_value_projection", "transformer_layes.0.attn.in_proj_weight",
            STYLE_WIDTH * STYLE_WIDTH * 2 + 3, 0.125),
        "style_quick_gelu": mutation_oracle(
            "style", "style_quick_gelu", "transformer_layes.1.mlp.c_fc.bias", 5, 0.25),
        "style_projection": mutation_oracle("style", "style_projection", "proj", 7, 0.125),
        "redux_up": mutation_oracle("redux", "redux_up", "redux_up.bias", 2, 0.25),
        "redux_down": mutation_oracle("redux", "redux_down", "redux_down.weight", 9, 0.125),
    }
    return {
        "format": "conditioning-auxiliary-resource-foundation-v1",
        "reduced_profiles_are_source_exact": False,
        "source_dimensions": {
            "style": {"width": 1024, "context": 768, "heads": 8, "layers": 3, "tokens": 8, "state_count": 42},
            "redux": {"input": 1152, "hidden": 12288, "output": 4096, "state_count": 4},
        },
        "reduced_dimensions": {
            "style": {"width": STYLE_WIDTH, "context": STYLE_CONTEXT, "heads": STYLE_HEADS,
                      "layers": STYLE_LAYERS, "tokens": STYLE_TOKENS, "state_count": 42},
            "redux": {"input": REDUX_INPUT, "hidden": REDUX_HIDDEN, "output": REDUX_OUTPUT,
                      "state_count": 4},
        },
        "style": profile_oracle("style"),
        "redux": profile_oracle("redux"),
        "attention_discriminator": attention_discriminator_oracle(),
        "mutations": mutations,
        "discriminators": {
            "signed_zero_input_bits": 0x80000000,
            "signed_zero_after_add_bits": bits(fadd(from_bits(0x80000000), 0.0)),
            "quick_gelu": {
                "coefficient_bits": bits(1.702),
                "input_bits": bits(quick_input),
                "scaled_bits": bits(quick_scaled),
                "sigmoid_bits": bits(quick_sigmoid),
                "output_bits": bits(fmul(quick_input, quick_sigmoid)),
            },
            "detection_precedence": ["style_embedding", "redux_down.weight"],
        },
    }
