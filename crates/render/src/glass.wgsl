struct ClockUniform {
    digits: vec4<u32>,
    size: vec2<f32>,
    padding: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> clock: ClockUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var position = vec2<f32>(-1.0, -1.0);
    switch index {
        case 1u, 4u: { position = vec2<f32>(1.0, -1.0); }
        case 2u, 3u: { position = vec2<f32>(-1.0, 1.0); }
        case 5u: { position = vec2<f32>(1.0, 1.0); }
        default: {}
    }
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    return output;
}

fn digit_mask(value: u32) -> u32 {
    switch value {
        case 0u: { return 63u; }
        case 1u: { return 6u; }
        case 2u: { return 91u; }
        case 3u: { return 79u; }
        case 4u: { return 102u; }
        case 5u: { return 109u; }
        case 6u: { return 125u; }
        case 7u: { return 7u; }
        case 8u: { return 127u; }
        case 9u: { return 111u; }
        default: { return 0u; }
    }
}

fn segment_coverage(point: vec2<f32>, mask: u32) -> f32 {
    let edge = 0.055;
    var coverage = 0.0;
    if ((mask & 1u) != 0u && point.x > 0.20 && point.x < 0.80 && point.y > 0.04 && point.y < 0.14) {
        coverage = 1.0;
    }
    if ((mask & 2u) != 0u && point.x > 0.82 && point.x < 0.96 && point.y > 0.10 && point.y < 0.45) {
        coverage = 1.0;
    }
    if ((mask & 4u) != 0u && point.x > 0.82 && point.x < 0.96 && point.y > 0.55 && point.y < 0.90) {
        coverage = 1.0;
    }
    if ((mask & 8u) != 0u && point.x > 0.20 && point.x < 0.80 && point.y > 0.86 && point.y < 0.96) {
        coverage = 1.0;
    }
    if ((mask & 16u) != 0u && point.x > 0.04 && point.x < 0.18 && point.y > 0.55 && point.y < 0.90) {
        coverage = 1.0;
    }
    if ((mask & 32u) != 0u && point.x > 0.04 && point.x < 0.18 && point.y > 0.10 && point.y < 0.45) {
        coverage = 1.0;
    }
    if ((mask & 64u) != 0u && point.x > 0.20 && point.x < 0.80 && point.y > 0.45 && point.y < 0.55) {
        coverage = 1.0;
    }
    return smoothstep(0.0, edge, coverage);
}

fn rounded_box_distance(point: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(point) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn digit_x(index: u32, start_x: f32, digit_width: f32, gap: f32, colon_width: f32) -> f32 {
    switch index {
        case 0u: { return start_x; }
        case 1u: { return start_x + digit_width + gap; }
        case 2u: { return start_x + 2.0 * digit_width + 3.0 * gap + colon_width; }
        case 3u: { return start_x + 3.0 * digit_width + 4.0 * gap + colon_width; }
        default: { return start_x; }
    }
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let size = max(clock.size, vec2<f32>(1.0));
    let point = fragment.xy;
    let shortest = min(size.x, size.y);
    let panel_distance = rounded_box_distance(
        point - size * 0.5,
        size * 0.5 - vec2<f32>(2.0),
        shortest * 0.16,
    );
    let panel = 1.0 - smoothstep(-1.0, 1.0, panel_distance);
    let border = 1.0 - smoothstep(1.0, 3.0, abs(panel_distance));
    let highlight = 1.0 - smoothstep(0.0, size.y * 0.85, point.y);
    var color = vec3<f32>(0.72, 0.83, 1.0) * (0.25 + 0.06 * highlight);
    var alpha = panel * (0.34 + 0.08 * highlight);
    color = mix(color, vec3<f32>(0.94, 0.97, 1.0), border * 0.55);
    alpha = max(alpha, border * 0.42);

    let digit_width = min(size.x * 0.135, size.y * 0.52);
    let digit_height = min(size.y * 0.50, digit_width * 1.72);
    let gap = digit_width * 0.17;
    let colon_width = digit_width * 0.18;
    let total_width = 4.0 * digit_width + 4.0 * gap + colon_width;
    let start_x = (size.x - total_width) * 0.5;
    let start_y = (size.y - digit_height) * 0.5;

    var text = 0.0;
    for (var index = 0u; index < 4u; index += 1u) {
        let local = (point - vec2<f32>(
            digit_x(index, start_x, digit_width, gap, colon_width),
            start_y,
        )) / vec2<f32>(digit_width, digit_height);
        if (all(local >= vec2<f32>(0.0)) && all(local <= vec2<f32>(1.0))) {
            text = max(text, segment_coverage(local, digit_mask(clock.digits[index])));
        }
    }

    let colon_x = start_x + 2.0 * digit_width + 2.0 * gap;
    let colon_point = (point - vec2<f32>(colon_x, start_y)) /
        vec2<f32>(colon_width, digit_height);
    if (all(colon_point >= vec2<f32>(0.0)) && all(colon_point <= vec2<f32>(1.0))) {
        let upper_dot = distance(colon_point, vec2<f32>(0.5, 0.34));
        let lower_dot = distance(colon_point, vec2<f32>(0.5, 0.66));
        text = max(text, 1.0 - smoothstep(0.08, 0.13, min(upper_dot, lower_dot)));
    }

    let grain = fract(sin(dot(point, vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5;
    color += vec3<f32>(grain * 0.025);
    color = mix(color, vec3<f32>(0.98, 0.99, 1.0), text);
    alpha = max(alpha, text * 0.96);
    return vec4<f32>(color, alpha);
}
