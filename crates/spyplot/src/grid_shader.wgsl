struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) plot_position: vec2<f32>,
};

struct Uniforms {
    viewport_size: vec2<f32>,
    x_range: vec2<f32>,
    y_range: vec2<f32>,

    grid_pitch: vec2<f32>,

    line_width: f32,
    feather: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// TODO(danny): maybe use these or get rid of them
// they're useful for rendering 3D grids but I'm interested in 2D
// https://iquilezles.org/articles/filterableprocedurals/
// https://eliotbo.github.io/glsl2wgsl/
fn filtered_grid(p: vec2<f32>, dpdx: vec2<f32>, dpdy: vec2<f32>) -> f32 {
    let N: f32 = 10.; // grid ratio
    let w: vec2<f32> = max(abs(dpdx), abs(dpdy));
    let a: vec2<f32> = p + 0.5 * w;
    let b: vec2<f32> = p - 0.5 * w;
    let i: vec2<f32> = (floor(a) + min(fract(a) * N, vec2(1.)) - floor(b) - min(fract(b) * N, vec2(1.))) / (N * w);
    return (1. - i.x) * (1. - i.y);
}

fn grid_texture(p: vec2<f32>) -> f32 {
    let N: f32 = 10.0; // grid ratio
    let i: vec2<f32> = step(fract(p), vec2<f32>(1.0 / N));
    return (1. - i.x) * (1. - i.y);
} 

// cover the screen with one triangle
var<private> v_positions: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0)
);

var<private> v_colors: array<vec4<f32>, 3> = array<vec4<f32>, 3>(
    vec4<f32>(0.0, 0.0, 0.0, 1.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOut {
    var out: VertexOut;

    out.position = vec4<f32>(v_positions[v_idx], 0.0, 1.0);
    out.color = v_colors[v_idx];

    let x = mix(uniforms.x_range[0], uniforms.x_range[1], (out.position.x + 1.0) * 0.5);
    let y = mix(uniforms.y_range[0], uniforms.y_range[1], (out.position.y + 1.0) * 0.5);
    out.plot_position = vec2(x, y);

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let plot_size = vec2(uniforms.x_range[1] - uniforms.x_range[0], uniforms.y_range[1] - uniforms.y_range[0]);
    let plot_distance_one_pixel = (vec2(1.0) / uniforms.viewport_size) * plot_size;
    let plot_distance_to_grid_start = in.plot_position.xy % uniforms.grid_pitch;

    if abs(plot_distance_to_grid_start.x) < plot_distance_one_pixel.x || abs(plot_distance_to_grid_start.y) < plot_distance_one_pixel.y {
        return vec4(1.0);
    }

    return vec4(0.0);
}
