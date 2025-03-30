#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub viewport_size: [f32; 2],
    pub x_bounds: [f32; 2],
    pub y_bounds: [f32; 2],

    pub grid_pitch: [f32; 2],

    pub line_width: f32,
    pub feather: f32,
}
