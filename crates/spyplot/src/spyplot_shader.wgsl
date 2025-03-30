struct VertexOut {
    @location(0) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

struct Uniforms {
    x_range: vec2<f32>,
    y_range: vec2<f32>,
    @size(8) angle: f32, // pad to total of 24
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

var<private> v_positions: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
    vec2<f32>(0.0, 10.0),
    vec2<f32>(10.0, -10.0),
    vec2<f32>(-10.0, -10.0),
);

var<private> v_colors: array<vec4<f32>, 3> = array<vec4<f32>, 3>(
    vec4<f32>(1.0, 0.0, 0.0, 1.0),
    vec4<f32>(0.0, 1.0, 0.0, 1.0),
    vec4<f32>(0.0, 0.0, 1.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOut {
    var out: VertexOut;

    let width = uniforms.x_range[1] - uniforms.x_range[0];
    let height = uniforms.y_range[1] - uniforms.y_range[0];

    out.position = vec4<f32>(v_positions[v_idx], 0.0, 1.0);
    out.color = v_colors[v_idx];

    // TODO(danny): consider using a vec2 for min and the size
    let x = mix(-1.0, 1.0, (out.position.x - uniforms.x_range[0]) / width);
    let y = mix(-1.0, 1.0, (out.position.y - uniforms.y_range[0]) / height);
    out.position = vec4<f32>(x, y, 0.0, 1.0);


    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
