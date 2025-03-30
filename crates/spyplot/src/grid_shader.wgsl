struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// struct Uniforms {
//     x_range: vec2<f32>,
//     y_range: vec2<f32>,
// };

// @group(0) @binding(0)
// var<uniform> uniforms: Uniforms;

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

    return out;
}

const pitch: vec2<f32> = vec2<f32>(50., 50.);

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let grid_active = f32(i32(in.position.x % pitch.x) == 0 || i32(in.position.y % pitch.y) == 0);
    return vec4(grid_active, grid_active, grid_active, 1.0);
}
