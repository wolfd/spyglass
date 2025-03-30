#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub x_bounds: [f32; 2],
    pub y_bounds: [f32; 2],
    pub angle: f32,
    pub _padding: f32,
}
