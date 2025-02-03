use eframe::egui;
use egui::{emath, CollapsingHeader, Frame, Painter, Pos2, Rect, Shape, Stroke, Ui, Vec2};

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Spyplot Demo",
        native_options,
        Box::new(|cc| Ok(Box::new(DemoApp::new(cc)))),
    )
}

#[derive(Default)]
struct DemoApp {
    spyplot: Spyplot,
}

impl DemoApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");

            ui.scope(|ui| {
                ui.set_max_height(200.0);
                self.spyplot.ui(ui);
            });

            // ui.debug_paint_cursor();

            use egui_plot::{Line, Plot, PlotPoints};
            let sin: PlotPoints = (0..1000)
                .map(|i| {
                    let x = i as f64 * 0.01;
                    [x, x.sin()]
                })
                .collect();
            let line = Line::new(sin);
            Plot::new("my_plot")
                .view_aspect(2.0)
                .label_formatter(|name, value| {
                    if !name.is_empty() {
                        format!("{}: {:.*}%", name, 1, value.y)
                    } else {
                        "".to_owned()
                    }
                })
                .show(ui, |plot_ui| plot_ui.line(line));
        });
    }
}

#[derive(PartialEq, Debug)]
struct Spyplot {
    bounds: PlotBounds,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct PlotBounds {
    x_range: [f64; 2],
    y_range: [f64; 2],
}

impl Default for Spyplot {
    fn default() -> Self {
        Self {
            bounds: PlotBounds {
                x_range: [-100.0, 100.0],
                y_range: [-100.0, 100.0],
            },
        }
    }
}

impl Spyplot {
    pub fn ui(&mut self, ui: &mut Ui) {
        let painter = Painter::new(
            ui.ctx().clone(),
            ui.layer_id(),
            ui.available_rect_before_wrap(),
        );
        self.paint(&painter);
        // Make sure we allocate what we used (everything)
        ui.expand_to_include_rect(painter.clip_rect());
        let (id, rect) = ui.allocate_space(painter.clip_rect().size());

        // let (id, rect) = ui.allocate_space(ui.available_size_before_wrap());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());

        if response.dragged() {
            let x_drag = response.drag_delta().x as f64;
            println!("dragged: {:?}", x_drag);
            self.bounds.x_range = [
                self.bounds.x_range[0] + x_drag,
                self.bounds.x_range[1] + x_drag,
            ];
            println!("new bounds: {:?}", self.bounds);
        }

        if response.hovered() {
            let zoom_delta = ui.ctx().input(|i| i.zoom_delta()) as f64;
        }

        if response.double_clicked() {}

        Frame::popup(ui.style())
            .stroke(Stroke::NONE)
            .show(ui, |ui| {
                ui.set_max_width(270.0);
                CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui));
            });
    }

    fn options_ui(&mut self, ui: &mut Ui) {
        ui.label("No options yet");
    }

    fn paint(&mut self, painter: &Painter) {
        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_max(
                Pos2::new(self.bounds.x_range[0] as f32, self.bounds.y_range[0] as f32),
                Pos2::new(self.bounds.x_range[1] as f32, self.bounds.y_range[1] as f32),
            ),
            painter.clip_rect(),
        );

        let x_lines = (self.bounds.x_range[0] as i64..self.bounds.x_range[1] as i64)
            .step_by(10)
            .map(|x| {
                Shape::line_segment(
                    [
                        to_screen.transform_pos(Pos2::new(x as f32, self.bounds.y_range[0] as f32)),
                        to_screen.transform_pos(Pos2::new(x as f32, self.bounds.y_range[1] as f32)),
                    ],
                    Stroke::new(1.0, egui::Color32::from_white_alpha(128)),
                )
            });

        painter.extend(x_lines);

        let y_lines = (self.bounds.y_range[0] as i64..self.bounds.y_range[1] as i64)
            .step_by(10)
            .map(|y| {
                Shape::line_segment(
                    [
                        to_screen.transform_pos(Pos2::new(self.bounds.x_range[0] as f32, y as f32)),
                        to_screen.transform_pos(Pos2::new(self.bounds.x_range[1] as f32, y as f32)),
                    ],
                    Stroke::new(1.0, egui::Color32::from_white_alpha(128)),
                )
            });

        painter.extend(y_lines);
    }
}
