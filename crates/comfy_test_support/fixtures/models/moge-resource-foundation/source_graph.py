import json
import math
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
DINO_ORACLE = ROOT / "crates/comfy_test_support/fixtures/models/dinov2-backbone-owner-foundation/oracle.json"


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def bits(value):
    return struct.unpack("<I", struct.pack("<f", f32(value)))[0]


def from_bits(value):
    return struct.unpack("<f", struct.pack("<I", value))[0]


def fma(left, right, addend):
    return f32(f32(left) * f32(right) + f32(addend))


def fadd(left, right):
    return f32(f32(left) + f32(right))


def fmul(left, right):
    return f32(f32(left) * f32(right))


def fdiv(left, right):
    return f32(f32(left) / f32(right))


def recover_forced_shift(points, confidence, width, height, focal):
    samples = []
    for output_y in range(64):
        source_y = output_y * height // 64
        for output_x in range(64):
            source_x = output_x * width // 64
            pixel = source_y * width + source_x
            if confidence[pixel] > 0.5:
                samples.append((pixel, source_x, source_y))
    if len(samples) < 2:
        return f32(0.0)
    diagonal_pixels = math.hypot(float(width), float(height))

    def evaluate(shift):
        cost = 0.0
        gradient = 0.0
        hessian = 0.0
        for pixel, x, y in samples:
            base = pixel * 3
            z = fadd(points[base + 2], shift)
            if not math.isfinite(z) or abs(z) <= 1.0e-6:
                continue
            u = f32(((x + 0.5) * 2.0 - width) / diagonal_pixels)
            v = f32(((y + 0.5) * 2.0 - height) / diagonal_pixels)
            projected_x = fdiv(points[base], z)
            projected_y = fdiv(points[base + 1], z)
            residual_x = fadd(fmul(focal, projected_x), -u)
            residual_y = fadd(fmul(focal, projected_y), -v)
            z_squared = fmul(z, z)
            derivative_x = fdiv(fmul(-focal, points[base]), z_squared)
            derivative_y = fdiv(fmul(-focal, points[base + 1]), z_squared)
            cost += float(fadd(fmul(residual_x, residual_x), fmul(residual_y, residual_y)))
            gradient += float(fadd(fmul(residual_x, derivative_x), fmul(residual_y, derivative_y)))
            hessian += float(fadd(fmul(derivative_x, derivative_x), fmul(derivative_y, derivative_y)))
        return cost, gradient, hessian

    shift = f32(0.0)
    damping = f32(1.0e-3)
    current_cost = math.inf
    for _ in range(32):
        cost, gradient, hessian = evaluate(shift)
        step = fdiv(f32(-gradient), fadd(f32(hessian), damping))
        if not math.isfinite(step):
            break
        candidate = fadd(shift, step)
        candidate_cost, _, _ = evaluate(candidate)
        if candidate_cost < cost:
            improvement = cost - candidate_cost
            shift = candidate
            current_cost = candidate_cost
            damping = max(f32(fmul(damping, 0.1)), f32(1.0e-9))
            if abs(step) <= 1.0e-6 or improvement <= 1.0e-3 * max(cost, 1.0):
                break
        else:
            damping = min(f32(fmul(damping, 10.0)), f32(1.0e9))
            if math.isfinite(current_cost) and abs(current_cost - cost) <= 1.0e-3 * max(cost, 1.0):
                break
    return shift


def source_lm_recovery(points, confidence, width, height, forced_focal=None):
    samples = []
    diagonal = math.hypot(width, height)
    for output_y in range(64):
        y = output_y * height // 64
        for output_x in range(64):
            x = output_x * width // 64
            pixel = y * width + x
            if confidence[pixel] > 0.5:
                samples.append((pixel, ((x + 0.5) * 2.0 - width) / diagonal,
                                ((y + 0.5) * 2.0 - height) / diagonal))
    if len(samples) < 2:
        return (1.0 if forced_focal is None else forced_focal), 0.0, 0, 0

    def residuals(shift):
        projected = []
        numerator = denominator = 0.0
        for pixel, u, v in samples:
            x, y, z = points[pixel * 3:pixel * 3 + 3]
            px, py = x / (z + shift), y / (z + shift)
            projected.append((px, py, u, v))
            numerator += px * u + py * v
            denominator += px * px + py * py
        focal = forced_focal if forced_focal is not None else numerator / denominator
        return focal, [item for px, py, u, v in projected for item in (focal * px - u, focal * py - v)]

    shift, damping, accepted, rejected = 0.0, 1.0e-3, 0, 0
    focal, residual = residuals(shift)
    cost = sum(value * value for value in residual)
    for _ in range(64):
        epsilon = math.sqrt(2.220446049250313e-16) * (1.0 + abs(shift))
        _, displaced = residuals(shift + epsilon)
        jacobian = [(right - left) / epsilon for left, right in zip(residual, displaced)]
        gradient = sum(value * derivative for value, derivative in zip(residual, jacobian))
        hessian = sum(derivative * derivative for derivative in jacobian)
        step = -gradient / (hessian + damping)
        candidate_shift = shift + step
        candidate_focal, candidate_residual = residuals(candidate_shift)
        candidate_cost = sum(value * value for value in candidate_residual)
        if candidate_cost < cost:
            improvement = cost - candidate_cost
            shift, focal, residual, cost = candidate_shift, candidate_focal, candidate_residual, candidate_cost
            damping = max(damping * 0.1, 1.0e-12)
            accepted += 1
            if abs(step) <= 1.0e-9 or improvement <= 1.0e-3 * max(cost, 1.0):
                break
        else:
            damping = min(damping * 10.0, 1.0e12)
            rejected += 1
    return focal, shift, accepted, rejected


def geometry_discriminators():
    width = height = 8
    diagonal = math.hypot(width, height)
    expected_focal, expected_shift = 0.8, 0.25
    points = []
    for y in range(height):
        for x in range(width):
            pixel = y * width + x
            z = 1.0 + pixel * 0.01
            u = ((x + 0.5) * 2.0 - width) / diagonal
            v = ((y + 0.5) * 2.0 - height) / diagonal
            points.extend([u * (z + expected_shift) / expected_focal,
                           v * (z + expected_shift) / expected_focal, z])
    confidence = [1.0] * (width * height)
    focal, shift, accepted, rejected = source_lm_recovery(points, confidence, width, height)
    negative_points = [(-value if index % 3 != 2 else value) for index, value in enumerate(points)]
    initial_focal, _, _, _ = source_lm_recovery(negative_points, confidence, width, height)
    fallback_focal = (width / height) / math.sqrt(2.0) / math.tan(math.radians(30.0))
    _, fallback_shift, fallback_accepted, fallback_rejected = source_lm_recovery(
        negative_points, confidence, width, height, fallback_focal)
    sparse_points = [0.0] * (64 * 64 * 3)
    sparse_confidence = [1.0] + [0.0] * (64 * 64 - 1)
    sparse_focal, sparse_shift, _, _ = source_lm_recovery(
        sparse_points, sparse_confidence, 64, 64)
    return {
        "shape": [1, height, width, 3],
        "auto_focal": focal,
        "auto_shift": shift,
        "auto_accepted": accepted,
        "auto_rejected": rejected,
        "invalid_initial_focal": initial_focal,
        "fallback_focal": fallback_focal,
        "fallback_shift": fallback_shift,
        "fallback_accepted": fallback_accepted,
        "fallback_rejected": fallback_rejected,
        "fallback_relative_tolerance": 7.0e-7,
        "fallback_relative_max_plus_one": 7.0001e-7,
        "less_than_two_focal": sparse_focal,
        "less_than_two_shift": sparse_shift,
        "absolute_tolerance": 2.5e-3,
        "max_plus_one": 2.5001e-3,
    }


def project_storage(value, dtype):
    value = f32(value)
    if dtype == "f32":
        return value
    if dtype == "f16":
        return f32(struct.unpack("<e", struct.pack("<e", value))[0])
    raw = bits(value)
    return from_bits((raw + 0x7FFF + ((raw >> 16) & 1)) & 0xFFFF0000)


def fixture_value(state_index, value_index, key):
    if key.endswith(".weight") and ("norm" in key or ".layers.0." in key or ".layers.3." in key):
        return f32(1.0)
    if key.endswith(".lambda1"):
        return f32(0.125)
    if key.endswith(".bias"):
        return f32(0.0)
    lane = ((state_index * 17 + value_index * 13) % 29) - 14
    return f32(f32(lane) * f32(0.0025))


def add_conv(definitions, prefix, input_channels, output_channels, kernel, transposed=False):
    shape = ([input_channels, output_channels, kernel, kernel] if transposed
             else [output_channels, input_channels, kernel, kernel])
    definitions.extend([(prefix + ".weight", shape), (prefix + ".bias", [output_channels])])


def add_residual(definitions, prefix, channels):
    definitions.extend([(prefix + ".layers.0.weight", [channels]), (prefix + ".layers.0.bias", [channels])])
    add_conv(definitions, prefix + ".layers.2", channels, channels, 3)
    definitions.extend([(prefix + ".layers.3.weight", [channels]), (prefix + ".layers.3.bias", [channels])])
    add_conv(definitions, prefix + ".layers.5", channels, channels, 3)


def head_manifest(profile):
    definitions = []
    if profile == "v1":
        for index in range(4):
            add_conv(definitions, f"head.projects.{index}", 4, 4, 1)
        add_conv(definitions, "head.upsample_blocks.0.0.0", 6, 4, 2, True)
        add_conv(definitions, "head.upsample_blocks.0.0.1", 4, 4, 3)
        add_residual(definitions, "head.upsample_blocks.0.1", 4)
        for index, output in [(0, 3), (1, 1)]:
            add_conv(definitions, f"head.output_block.{index}.0", 6, 4, 3)
            add_conv(definitions, f"head.output_block.{index}.2", 4, output, 1)
        return definitions
    for index in range(4):
        add_conv(definitions, f"encoder.output_projections.{index}", 4, 4, 1)
    for prefix, output in [("neck", 4), ("points_head", 3), ("mask_head", 1), ("normal_head", 3)]:
        for level in range(5):
            input_channels = (6 if level == 0 else 2) if prefix == "neck" else 4
            add_conv(definitions, f"{prefix}.input_blocks.{level}", input_channels, 4, 1)
            add_residual(definitions, f"{prefix}.res_blocks.{level}.0", 4)
            if level == 4:
                add_conv(definitions, f"{prefix}.output_blocks.{level}", 4, output, 1)
            if level < 4:
                add_conv(definitions, f"{prefix}.resamplers.{level}.1", 4, 4, 3)
    definitions.extend([
        ("scale_head.0.weight", [4, 4]), ("scale_head.0.bias", [4]),
        ("scale_head.2.weight", [1, 4]), ("scale_head.2.bias", [1]),
    ])
    return definitions


def product(shape):
    result = 1
    for dimension in shape:
        result *= dimension
    return result


def head_state(profile, dtype, mutation=None):
    state = {}
    for offset, (key, shape) in enumerate(head_manifest(profile)):
        values = [project_storage(fixture_value(79 + offset, lane, key), dtype) for lane in range(product(shape))]
        if mutation and mutation["state_key"] == key:
            values[mutation["lane"]] = f32(values[mutation["lane"]] + mutation["delta"])
        state[key] = {"shape": shape, "values": values}
    return state


def tensor(shape, values):
    assert product(shape) == len(values)
    return {"shape": list(shape), "values": list(values)}


def add(left, right):
    assert left["shape"] == right["shape"]
    return tensor(left["shape"], [f32(a + b) for a, b in zip(left["values"], right["values"])])


def relu(value):
    return tensor(value["shape"], [max(item, 0.0) for item in value["values"]])


def feature_tensor(route):
    patches, channels = route["patch_shape"][1:]
    source = [from_bits(value) for value in route["patch_bits"]]
    values = [0.0] * len(source)
    for patch in range(patches):
        for channel in range(channels):
            values[channel * patches + patch] = source[patch * channels + channel]
    side = int(math.sqrt(patches))
    return tensor([1, channels, side, side], values)


def pad_replicate(value, padding=1):
    batch, channels, height, width = value["shape"]
    output_height, output_width = height + 2 * padding, width + 2 * padding
    output = [0.0] * (batch * channels * output_height * output_width)
    for b in range(batch):
        for c in range(channels):
            for y in range(output_height):
                source_y = min(max(y - padding, 0), height - 1)
                for x in range(output_width):
                    source_x = min(max(x - padding, 0), width - 1)
                    output[((b * channels + c) * output_height + y) * output_width + x] = value["values"][((b * channels + c) * height + source_y) * width + source_x]
    return tensor([batch, channels, output_height, output_width], output)


def conv(value, state, prefix, transposed=False):
    weight, bias = state[prefix + ".weight"], state[prefix + ".bias"]["values"]
    batch, input_channels, height, width = value["shape"]
    kernel = weight["shape"][2]
    if transposed:
        output_channels = weight["shape"][1]
        output_height, output_width = height * 2, width * 2
        output = [bias[c] for _b in range(batch) for c in range(output_channels) for _ in range(output_height * output_width)]
        for b in range(batch):
            for ic in range(input_channels):
                for y in range(height):
                    for x in range(width):
                        source = value["values"][((b * input_channels + ic) * height + y) * width + x]
                        for oc in range(output_channels):
                            for ky in range(kernel):
                                for kx in range(kernel):
                                    destination = ((b * output_channels + oc) * output_height + y * 2 + ky) * output_width + x * 2 + kx
                                    weight_index = ((ic * output_channels + oc) * kernel + ky) * kernel + kx
                                    output[destination] = fma(source, weight["values"][weight_index], output[destination])
        return tensor([batch, output_channels, output_height, output_width], output)
    padded = pad_replicate(value) if kernel == 3 else value
    _, _, padded_height, padded_width = padded["shape"]
    output_channels = weight["shape"][0]
    output = [0.0] * (batch * output_channels * height * width)
    for b in range(batch):
        for oc in range(output_channels):
            for y in range(height):
                for x in range(width):
                    result = bias[oc]
                    for ic in range(input_channels):
                        for ky in range(kernel):
                            for kx in range(kernel):
                                source = padded["values"][((b * input_channels + ic) * padded_height + y + ky) * padded_width + x + kx]
                                weight_index = ((oc * input_channels + ic) * kernel + ky) * kernel + kx
                                result = fma(source, weight["values"][weight_index], result)
                    output[((b * output_channels + oc) * height + y) * width + x] = result
    return tensor([batch, output_channels, height, width], output)


def resize(value, output_height, output_width):
    batch, channels, height, width = value["shape"]
    output = [0.0] * (batch * channels * output_height * output_width)
    def axis_weights(source_extent, output_extent, output_index):
        inverse_scale = f32(source_extent / output_extent)
        coordinate = fma(f32(output_index + 0.5), inverse_scale, -0.5)
        coordinate = min(max(coordinate, 0.0), f32(source_extent - 1))
        lower = int(math.floor(coordinate))
        upper = lower + 1
        fraction = f32(coordinate - lower)
        result = [(lower, f32(1.0 - fraction))]
        if upper < source_extent:
            result.append((upper, fraction))
        return result
    for b in range(batch):
        for c in range(channels):
            for y in range(output_height):
                for x in range(output_width):
                    destination = ((b * channels + c) * output_height + y) * output_width + x
                    for source_y, weight_y in axis_weights(height, output_height, y):
                        for source_x, weight_x in axis_weights(width, output_width, x):
                            source = value["values"][((b * channels + c) * height + source_y) * width + source_x]
                            weight = fmul(weight_y, weight_x)
                            output[destination] = fma(source, weight, output[destination])
    return tensor([batch, channels, output_height, output_width], output)


def view_plane(batch, height, width, aspect):
    diagonal = math.sqrt(1.0 + aspect * aspect)
    span_x, span_y = aspect / diagonal, 1.0 / diagonal
    def linspace(start, end, steps):
        if steps <= 1:
            return [f32(start)] * steps
        return [f32(end if index + 1 == steps else start * (1.0 - index / (steps - 1)) + end * (index / (steps - 1))) for index in range(steps)]
    horizontal = linspace(-span_x * (width - 1) / width, span_x * (width - 1) / width, width)
    vertical = linspace(-span_y * (height - 1) / height, span_y * (height - 1) / height, height)
    values = [0.0] * (batch * 2 * height * width)
    for b in range(batch):
        for y in range(height):
            for x in range(width):
                values[(b * 2) * height * width + y * width + x] = horizontal[x]
                values[(b * 2 + 1) * height * width + y * width + x] = vertical[y]
    return tensor([batch, 2, height, width], values)


def concatenate(left, right):
    batch, left_channels, height, width = left["shape"]
    assert right["shape"] == [batch, 2, height, width]
    values = []
    for b in range(batch):
        values.extend(left["values"][b * left_channels * height * width:(b + 1) * left_channels * height * width])
        values.extend(right["values"][b * 2 * height * width:(b + 1) * 2 * height * width])
    return tensor([batch, left_channels + 2, height, width], values)


def group_norm(value, state, prefix, groups):
    batch, channels, height, width = value["shape"]
    weight, bias = state[prefix + ".weight"]["values"], state[prefix + ".bias"]["values"]
    output = [0.0] * len(value["values"])
    per_group = channels // groups
    for b in range(batch):
        for group in range(groups):
            indexes = [((b * channels + c) * height + y) * width + x for c in range(group * per_group, (group + 1) * per_group) for y in range(height) for x in range(width)]
            indexes = list(indexes)
            mean = f32(sum(float(value["values"][index]) for index in indexes) / len(indexes))
            variance = sum((float(value["values"][index]) - float(mean)) ** 2 for index in indexes) / len(indexes)
            inverse = f32(1.0 / math.sqrt(variance + float(f32(1e-5))))
            for index in indexes:
                channel = (index // (height * width)) % channels
                normalized = fmul(f32(value["values"][index] - mean), inverse)
                output[index] = fadd(fmul(normalized, weight[channel]), bias[channel])
    return tensor(value["shape"], output)


def residual(value, state, prefix):
    output = relu(group_norm(value, state, prefix + ".layers.0", 1))
    output = conv(output, state, prefix + ".layers.2")
    output = relu(group_norm(output, state, prefix + ".layers.3", max(output["shape"][1] // 32, 1)))
    return add(conv(output, state, prefix + ".layers.5"), value)


def conv_stack(inputs, state, prefix):
    outputs, current = [], None
    for level, input_value in enumerate(inputs):
        feature = conv(input_value, state, f"{prefix}.input_blocks.{level}")
        current = feature if current is None else add(current, feature)
        current = residual(current, state, f"{prefix}.res_blocks.{level}.0")
        output_prefix = f"{prefix}.output_blocks.{level}"
        outputs.append(conv(current, state, output_prefix) if output_prefix + ".weight" in state else current)
        if level + 1 < len(inputs):
            current = conv(resize(current, current["shape"][2] * 2, current["shape"][3] * 2), state, f"{prefix}.resamplers.{level}.1")
    return outputs


def linear(values, state, prefix):
    weight, bias = state[prefix + ".weight"], state[prefix + ".bias"]["values"]
    output = []
    for row in range(weight["shape"][0]):
        result = bias[row]
        for column, value in enumerate(values):
            result = fma(value, weight["values"][row * weight["shape"][1] + column], result)
        output.append(result)
    return output


def execute(profile, dtype="f32", mutation=None):
    dino = json.loads(DINO_ORACLE.read_text())
    routes = dino["ordinary_routes"][dtype]
    state = head_state(profile, dtype, mutation)
    features = [feature_tensor(route) for route in routes]
    projected = None
    projection_prefix = "head.projects" if profile == "v1" else "encoder.output_projections"
    for index, feature in enumerate(features):
        value = conv(feature, state, f"{projection_prefix}.{index}")
        projected = value if projected is None else add(projected, value)
    if profile == "v1":
        value = concatenate(projected, view_plane(1, 2, 2, 1.0))
        value = conv(value, state, "head.upsample_blocks.0.0.0", True)
        value = conv(value, state, "head.upsample_blocks.0.0.1")
        value = residual(value, state, "head.upsample_blocks.0.1")
        value = concatenate(resize(value, 4, 4), view_plane(1, 4, 4, 1.0))
        point_logits = conv(relu(conv(value, state, "head.output_block.0.0")), state, "head.output_block.0.2")
        mask_logits = conv(relu(conv(value, state, "head.output_block.1.0")), state, "head.output_block.1.2")
        normal_logits, scale, scale_logit = None, [1.0], None
    else:
        levels = [concatenate(projected, view_plane(1, 2, 2, 1.0))]
        levels.extend(view_plane(1, 2 << level, 2 << level, 1.0) for level in range(1, 5))
        neck = conv_stack(levels, state, "neck")
        point_logits = resize(conv_stack(neck, state, "points_head")[-1], 4, 4)
        mask_logits = resize(conv_stack(neck, state, "mask_head")[-1], 4, 4)
        normal_logits = resize(conv_stack(neck, state, "normal_head")[-1], 4, 4)
        class_values = [from_bits(value) for value in routes[-1]["class_bits"]]
        scale_logit = linear([max(value, 0.0) for value in linear(class_values, state, "scale_head.0")], state, "scale_head.2")[0]
        scale = [f32(math.exp(scale_logit))]
    points_raw, mask_raw, normal_raw = point_logits["values"], mask_logits["values"], None if normal_logits is None else normal_logits["values"]
    points, depth, mask, normal = [], [], [], [] if normal_raw is not None else None
    aspect = 1.0
    diagonal = math.sqrt(1.0 + aspect * aspect)
    focal = f32(aspect / diagonal / math.tan(math.radians(30.0)))
    fx = f32((focal * 0.5 * diagonal) / aspect)
    fy = f32(focal * 0.5 * diagonal)
    remapped_points = [item for pixel in range(16) for item in [
        f32(points_raw[pixel] * f32(math.exp(points_raw[2 * 16 + pixel]))),
        f32(points_raw[16 + pixel] * f32(math.exp(points_raw[2 * 16 + pixel]))),
        f32(math.exp(points_raw[2 * 16 + pixel])),
    ]]
    confidence_values = [
        value if profile == "v1" else f32(1.0 / (1.0 + math.exp(-value)))
        for value in mask_raw
    ]
    shift = recover_forced_shift(remapped_points, confidence_values, 4, 4, focal)
    for pixel in range(16):
        z = fmul(fadd(remapped_points[pixel * 3 + 2], shift), scale[0])
        u, v = f32((pixel % 4 + 0.5) / 4), f32((pixel // 4 + 0.5) / 4)
        points.extend([
            fmul(fdiv(fadd(u, -0.5), fx), z),
            fmul(fdiv(fadd(v, -0.5), fy), z),
            z,
        ])
        depth.append(z)
        confidence = confidence_values[pixel]
        mask.append(1.0 if confidence > 0.5 and (profile == "v1" or z > 0.0) else 0.0)
        if normal is not None:
            lanes = [normal_raw[channel * 16 + pixel] for channel in range(3)]
            length = f32(math.sqrt(sum(float(value) ** 2 for value in lanes)))
            normal.extend([f32(value / length) for value in lanes])
    exp_inputs = list(points_raw[2 * 16:3 * 16])
    if profile == "v2":
        exp_inputs.extend(-value for value in mask_raw)
        exp_inputs.append(scale_logit)
    return {
        "points_shape": [1, 4, 4, 3], "points_bits": [bits(value) for value in points],
        "depth_shape": [1, 4, 4], "depth_bits": [bits(value) for value in depth],
        "intrinsics_shape": [1, 3, 3], "intrinsics_bits": [bits(value) for value in [fx, 0.0, 0.5, 0.0, fy, 0.5, 0.0, 0.0, 1.0]],
        "mask_shape": [1, 4, 4], "mask_bits": [bits(value) for value in mask],
        "normal_shape": [1, 4, 4, 3] if normal is not None else None,
        "normal_bits": [bits(value) for value in normal] if normal is not None else None,
        "exp_input_min_bits": bits(min(exp_inputs)),
        "exp_input_max_bits": bits(max(exp_inputs)),
        "exp_input_bits": [bits(value) for value in exp_inputs],
    }


def build_oracle():
    profile_ulp_bounds = {
        "v1": {
            "f32": {"depth": 0, "points": 0, "intrinsics": 0},
            "f16": {"depth": 0, "points": 0, "intrinsics": 0},
            "bf16": {"depth": 0, "points": 0, "intrinsics": 0},
        },
        "v2": {
            "f32": {"depth": 6, "points": 11, "intrinsics": 0, "normal": 1},
            "f16": {"depth": 2, "points": 4, "intrinsics": 0, "normal": 1},
            "bf16": {"depth": 8, "points": 14, "intrinsics": 0, "normal": 1},
        },
    }
    mutation_ulp_bounds = {
        "v1_projection": {"depth": 0, "points": 0, "intrinsics": 0},
        "v1_points": {"depth": 0, "points": 0, "intrinsics": 0},
        "v1_mask": {"depth": 0, "points": 0, "intrinsics": 0},
        "v2_projection": {"depth": 1, "points": 2, "intrinsics": 0, "normal": 1},
        "v2_neck": {"depth": 2, "points": 4, "intrinsics": 0, "normal": 1},
        "v2_points": {"depth": 1, "points": 2, "intrinsics": 0, "normal": 1},
        "v2_mask": {"depth": 16, "points": 18, "intrinsics": 0, "normal": 1},
        "v2_normal": {"depth": 6, "points": 11, "intrinsics": 0, "normal": 1},
        "v2_scale": {"depth": 6, "points": 10, "intrinsics": 0, "normal": 1},
    }
    mutations = {
        "v1_projection": {"profile": "v1", "state_key": "head.projects.0.weight", "lane": 0, "delta": 0.5},
        "v1_points": {"profile": "v1", "state_key": "head.output_block.0.0.weight", "lane": 0, "delta": 0.5},
        "v1_mask": {"profile": "v1", "state_key": "head.output_block.1.0.weight", "lane": 0, "delta": 0.5},
        "v2_projection": {"profile": "v2", "state_key": "encoder.output_projections.0.weight", "lane": 0, "delta": 0.5},
        "v2_neck": {"profile": "v2", "state_key": "neck.input_blocks.0.weight", "lane": 0, "delta": 0.5},
        "v2_points": {"profile": "v2", "state_key": "points_head.input_blocks.0.weight", "lane": 0, "delta": 0.5},
        "v2_mask": {"profile": "v2", "state_key": "mask_head.input_blocks.0.weight", "lane": 0, "delta": 0.5},
        "v2_normal": {"profile": "v2", "state_key": "normal_head.input_blocks.0.weight", "lane": 0, "delta": 0.5},
        "v2_scale": {"profile": "v2", "state_key": "scale_head.0.weight", "lane": 0, "delta": 0.5},
    }
    dino = json.loads(DINO_ORACLE.read_text())
    profiles = {
        profile: {
            dtype: {
                **execute(profile, dtype),
                "output_ulp_bounds": profile_ulp_bounds[profile][dtype],
            }
            for dtype in ["f32", "f16", "bf16"]
        }
        for profile in ["v1", "v2"]
    }
    mutation_outputs = {
        name: {
            **mutation,
            "output": {
                **execute(mutation["profile"], "f32", mutation),
                "output_ulp_bounds": mutation_ulp_bounds[name],
            },
        }
        for name, mutation in mutations.items()
    }
    exp_input_bits = set()
    for profile in profiles.values():
        for output in profile.values():
            exp_input_bits.update(output.pop("exp_input_bits"))
    for mutation in mutation_outputs.values():
        exp_input_bits.update(mutation["output"].pop("exp_input_bits"))
    ordered_exp_input_bits = sorted(exp_input_bits, key=from_bits)
    return {
        "format": "moge-resource-foundation-v2",
        "numeric_contract": {
            "canonical_exp_max_ulp": 2,
            "max_plus_one_must_reject": 3,
            "derived_geometry_distinct_nonzero_ulp_bounds": [1, 2, 4, 6, 8, 10, 11, 14, 16, 18],
            "derived_geometry_contract": "Per-case depth bounds cover canonical f32 analytic bounded-LM versus the independent f64 finite-difference source-equation oracle. Point bounds separately cover downstream projection multiplication amplification. V1 outputs and intrinsics remain bit exact; normalized V2 normals retain a one-ULP bound; Bool masks remain exact.",
            "canonical_exp_probe_input_bits": ordered_exp_input_bits,
            "source_exp_probe_output_bits": [
                bits(f32(math.exp(from_bits(value)))) for value in ordered_exp_input_bits
            ],
        },
        "input_bits": dino["input_bits"], "input_shape": [1, 4, 4, 3],
        "profiles": profiles,
        "mutations": mutation_outputs,
        "geometry_discriminators": geometry_discriminators(),
    }
