struct VertexOut {
    @location(0) color: vec4<f32>,
    @location(1) normal: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

struct Uniforms {
    x_range: vec2<f32>,
    y_range: vec2<f32>,
    line_width: f32,
    feather: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>
) -> VertexOut {
    var out: VertexOut;

    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    out.color = vec4<f32>(0.82, 0.23, 0.31, 1.0);
    
    // TODO(danny): consider using a vec2 for min and the size
    // move the viewport around
    let width = uniforms.x_range[1] - uniforms.x_range[0];
    let height = uniforms.y_range[1] - uniforms.y_range[0];
    let x = mix(-1.0, 1.0, (out.position.x - uniforms.x_range[0]) / width);
    let y = mix(-1.0, 1.0, (out.position.y - uniforms.y_range[0]) / height);

    // https://blog.mapbox.com/drawing-antialiased-lines-with-opengl-8766f34192dc
    out.position = vec4<f32>(x, y, 0.0, 1.0) + vec4(uniforms.line_width * normal, 0.0, 0.0);

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let alpha = smoothstep(0.0, 1.0, (1.0 - length(in.normal) / uniforms.feather));
    return vec4(in.color.xyz, alpha);
}
