// Desire: module that allows me to efficiently plot live data in the GPU
// The core idea: As we receive data, we rechunk it occasionally, so that we don't have
// too many draw operations.
// I'm thinking using 4096 points per chunk, and then we can draw 4096 points at a time.
// The final chunk will be allocated to that size, but will basically get frozen once it reaches that size.
extern crate nalgebra as na;
extern crate nalgebra_glm as glm;

pub mod bounds;
pub mod lines;

use bounds::Bounds;
use eframe::{
    egui_wgpu::wgpu::util::DeviceExt,
    egui_wgpu::{self, wgpu},
};
use egui::Vec2;
use lines::{Uniform, Vertex};

pub struct Spyplot {
    line_width: f32,
    feather: f32,
    viewport_size: Option<egui::Vec2>,
    bounds: bounds::Bounds,
    reorigin: na::Similarity2<f64>,

    pub dirty: bool,
    pub line: Vec<Vertex>,
}

impl Spyplot {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        // Get the WGPU render state from the eframe creation context. This can also be retrieved
        // from `eframe::Frame` when you don't have a `CreationContext` available.
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        let device = &wgpu_render_state.device;

        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("spyplot_line_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./spyplot_shader.wgsl").into()),
        });

        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("spyplot_background_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("./grid_shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spyplot_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                    // min_binding_size: NonZeroU64::new(
                    //     bytemuck::bytes_of(&[Uniform::default()]).len() as u64,
                    // ),
                },
                count: None,
            }],
        });

        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("spyplot_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let background_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("spyplot_pipeline_background"),
            layout: Some(&background_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &background_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &background_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu_render_state.target_format.into())], // need to specify alpha blending?
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("spyplot_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("spyplot_pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu_render_state.target_format.into())], // need to specify alpha blending?
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("spyplot_uniforms"),
            contents: bytemuck::cast_slice(&[Uniform::default()]),
            // Mapping at creation (as done by the create_buffer_init utility) doesn't require us to to add the MAP_WRITE usage
            // (this *happens* to workaround this bug )
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("spyplot_vertex_buffer"),
            contents: bytemuck::cast_slice(&vec![Vertex::default(); 16000000]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spyplot_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Because the graphics pipeline must have the same lifetime as the egui render pass,
        // instead of storing the pipeline in our `Custom3D` struct, we insert it into the
        // `paint_callback_resources` type map, which is stored alongside the render pass.
        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(SpyplotRenderResources {
                background_pipeline,
                line_pipeline,
                bind_group,
                uniform_buffer,
                vertex_buffer,
                vertex_count: 0,
            });

        let mut line = Vec::with_capacity(100000 * 2);
        for x in 0..100000 {
            let x = x as f32 / 100.0;
            let normal = Vec2::new(-f32::cos(x), 1.0).normalized();

            line.push(Vertex {
                position: [x, f32::sin(x)],
                normal: [normal.x, normal.y],
            });
            line.push(Vertex {
                position: [x, f32::sin(x)],
                normal: [-normal.x, -normal.y],
            });
        }

        Some(Self {
            line_width: 0.006,
            feather: 0.1,
            viewport_size: None, // autosize
            bounds: Bounds::from(&[[-5.0, -5.0], [5.0, 5.0]]),
            reorigin: na::Similarity::identity(),
            dirty: true,
            line,
        })
    }
}

impl eframe::App for Spyplot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("The plot is being painted using ");
                    ui.hyperlink_to("WGPU", "https://wgpu.rs");
                    ui.label(" (Portable Rust graphics API awesomeness)");
                });
                ui.label(format!("{:?}", self.bounds));

                ui.add(egui::Slider::new(&mut self.line_width, 0.0..=0.01).text("Line width"));
                ui.add(egui::Slider::new(&mut self.feather, 0.0..=2.5).text("Line feather"));

                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.custom_painting(
                        ui,
                        self.line_width,
                        self.feather,
                        if self.dirty {
                            Some(self.line.clone())
                        } else {
                            None
                        },
                    );
                    self.dirty = false;
                });
                ui.label("Drag to pan!");
            });
        });
    }
}

fn get_normal(a: &na::Vector2<f64>, b: &na::Vector2<f64>) -> na::Vector2<f64> {
    let tangent = b - a;
    na::Vector2::new(tangent.y, -tangent.x)
}

pub fn to_vertices(points: Vec<[f64; 2]>) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let n = points.len();

    if n < 2 {
        return vertices;
    }

    let mut normal = get_normal(&points[0].into(), &points[1].into());
    for i in 0..n - 1 {
        let current: na::Vector2<f64> = points[i].into();
        let next: na::Vector2<f64> = points[i + 1].into();

        normal = get_normal(&current, &next).normalize();

        // TODO(danny): angle bisector, extrude vectors, maybe bitpacking things
        vertices.push(Vertex {
            position: [points[i][0] as f32, points[i][1] as f32],
            normal: [normal.x as f32, normal.y as f32],
        });
        vertices.push(Vertex {
            position: [points[i][0] as f32, points[i][1] as f32],
            normal: [-normal.x as f32, -normal.y as f32],
        });
    }

    // add one more set of vertices
    vertices.push(Vertex {
        position: [points[n - 1][0] as f32, points[n - 1][1] as f32],
        normal: [normal.x as f32, normal.y as f32],
    });
    vertices.push(Vertex {
        position: [points[n - 1][0] as f32, points[n - 1][1] as f32],
        normal: [-normal.x as f32, -normal.y as f32],
    });

    vertices
}

// Callbacks in egui_wgpu have 3 stages:
// * prepare (per callback impl)
// * finish_prepare (once)
// * paint (per callback impl)
//
// The prepare callback is called every frame before paint and is given access to the wgpu
// Device and Queue, which can be used, for instance, to update buffers and uniforms before
// rendering.
// If [`egui_wgpu::Renderer`] has [`egui_wgpu::FinishPrepareCallback`] registered,
// it will be called after all `prepare` callbacks have been called.
// You can use this to update any shared resources that need to be updated once per frame
// after all callbacks have been processed.
//
// On both prepare methods you can use the main `CommandEncoder` that is passed-in,
// return an arbitrary number of user-defined `CommandBuffer`s, or both.
// The main command buffer, as well as all user-defined ones, will be submitted together
// to the GPU in a single call.
//
// The paint callback is called after finish prepare and is given access to egui's main render pass,
// which can be used to issue draw commands.
struct SpyplotCallback {
    viewport_size: egui::Vec2,
    bounds: Bounds,
    line_width: f32,
    feather: f32,

    dirty: bool,
    line: Vec<Vertex>,
}

impl egui_wgpu::CallbackTrait for SpyplotCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &mut SpyplotRenderResources = resources.get_mut().unwrap();
        resources.prepare(device, queue, &self);
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let resources: &SpyplotRenderResources = resources.get().unwrap();
        resources.paint(render_pass);
    }
}

impl Spyplot {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.custom_painting(
            ui,
            self.line_width,
            self.feather,
            if self.dirty {
                Some(self.line.clone())
            } else {
                None
            },
        );
        self.dirty = false;
    }

    fn custom_painting(
        &mut self,
        ui: &mut egui::Ui,
        line_width: f32,
        feather: f32,
        new_line: Option<Vec<Vertex>>,
    ) {
        let real_size = egui::Vec2::splat(500.0);
        let (rect, response) = ui.allocate_exact_size(real_size, egui::Sense::drag());
        self.viewport_size = Some(rect.size());

        assert_eq!(real_size, rect.size());

        let size_vec: na::Vector2<f64> = na::Vector2::new(real_size.x as f64, real_size.y as f64);
        let motion_scale = self.bounds.size().component_div(&size_vec);
        let mut delta: f32 = 0.;
        if response.hovered() {
            ui.input(|i| {
                delta = i
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::MouseWheel {
                            unit: _,
                            delta,
                            modifiers: _,
                        } => Some(delta.y),
                        _ => None,
                    })
                    .sum();
                delta /= 100.0; // TODO(danny): I recall that different platforms have wildly different ideas for scrolling counts

                if delta != 0.0 {
                    let zoom_center = if let Some(pointer_pos) = i.pointer.latest_pos() {
                        let rect_relative_pos = (pointer_pos - rect.min) / rect.size();
                        self.bounds.unit_position_to_plot_position(&na::Point2::new(
                            rect_relative_pos.x as f64,
                            1.0 - rect_relative_pos.y as f64, // y is inverted relative to plot
                        ))
                    } else {
                        self.bounds.center()
                    };

                    self.bounds = self.bounds.scale_from_point(
                        &zoom_center,
                        &na::Vector2::new(1.0 + delta as f64, 1.0 + delta as f64),
                    );
                }

                // reset to origin with same zoom when A is pressed
                if i.key_pressed(egui::Key::A) {
                    self.bounds = self.bounds.translate(&-self.bounds.center().coords);
                }

                if i.key_pressed(egui::Key::O) {
                    self.reorigin = self.bounds.transform_to_origin();
                    self.dirty = true;
                }
            });
        }

        use float_cmp::Ulps;
        ui.label(format!("{:?}", self.bounds));
        ui.label(format!(
            "ULPs f64 {} f32 {}",
            self.bounds.min.x.ulps(&self.bounds.max.x),
            (self.bounds.min.x as f32).ulps(&(self.bounds.max.x as f32))
        ));

        let drag: [f32; 2] = (response.drag_motion() * egui::Vec2 { x: -1.0, y: 1.0 }).into();
        let translate_by = na::Vector2::from(drag).cast().component_mul(&motion_scale);
        self.bounds = self.bounds.translate(&translate_by);

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            SpyplotCallback {
                viewport_size: self.viewport_size.unwrap_or(Vec2::new(1.0, 1.0)),
                bounds: self.bounds,
                line_width,
                feather,

                dirty: new_line.is_some(),
                line: new_line.unwrap_or_default(),
            },
        ));
    }
}

struct SpyplotRenderResources {
    background_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl SpyplotRenderResources {
    fn prepare(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue, data: &SpyplotCallback) {
        let bounds = &data.bounds;
        let line_width = data.line_width;
        let feather = data.feather;

        let grid_pitch_x = 10.0_f64.powf((bounds.max.x - bounds.min.x).log10().round() - 1.0);
        let grid_pitch_y = 10.0_f64.powf((bounds.max.y - bounds.min.y).log10().round() - 1.0);

        // update uniform buffer
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniform {
                viewport_size: [data.viewport_size.x, data.viewport_size.y],
                x_bounds: [bounds.min.x as f32, bounds.max.x as f32],
                y_bounds: [bounds.min.y as f32, bounds.max.y as f32],
                grid_pitch: [grid_pitch_x as f32, grid_pitch_y as f32],
                line_width,
                feather,
            }]),
        );

        if data.dirty {
            self.vertex_count = data.line.len() as u32;
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&data.line));
        }
    }

    fn paint(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.background_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        render_pass.set_pipeline(&self.line_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}
