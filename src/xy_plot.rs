use egui::epaint::Hsva;
use egui::{Color32, ComboBox, Response, TextWrapMode};
use egui_plot::{
    CoordinatesFormatter, Corner, Legend, Line, LineStyle, Plot, PlotPoint, PlotPoints,
};
use rand::distributions::Uniform;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};

#[derive(Clone, PartialEq)]
pub struct XYPlot {
    proportional: bool,
    coordinates: bool,
    show_axes: bool,
    line_style: LineStyle,

    plot_points: HashMap<String, Vec<PlotPoint>>,
}

impl Default for XYPlot {
    fn default() -> Self {
        Self {
            proportional: false,
            coordinates: true,
            show_axes: true,
            line_style: LineStyle::Solid,

            plot_points: HashMap::new(),
        }
    }
}

impl XYPlot {
    pub fn options_ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            proportional,
            coordinates,
            show_axes,
            line_style,

            plot_points: _,
        } = self;

        ui.menu_button("View", |ui| {
            ui.checkbox(show_axes, "Show axes");
            ui.checkbox(coordinates, "Show coordinates on hover")
                .on_hover_text("Can take a custom formatting function.");

            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
            ui.checkbox(proportional, "Proportional data axes")
                .on_hover_text("Tick are the same size on both axes.");

            ComboBox::from_label("Line style")
                .selected_text(line_style.to_string())
                .show_ui(ui, |ui| {
                    for style in &[
                        LineStyle::Solid,
                        LineStyle::dashed_dense(),
                        LineStyle::dashed_loose(),
                        LineStyle::dotted_dense(),
                        LineStyle::dotted_loose(),
                    ] {
                        ui.selectable_value(line_style, *style, style.to_string());
                    }
                });
        });
    }
}

impl XYPlot {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        let mut plot = Plot::new("xy_plot")
            .legend(Legend::default())
            .show_axes(self.show_axes)
            .show_grid(true);

        if self.proportional {
            plot = plot.data_aspect(1.0);
        }
        if self.coordinates {
            plot = plot.coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default());
        }
        plot.show(ui, |plot_ui| {
            for (label, y_data) in self.plot_points.iter() {
                let mut h = DefaultHasher::new();
                h.write(label.as_bytes());
                let hash = h.finish();
                // let color = Color32::from_rgb(hash as u8, (hash >> 8) as u8, (hash >> 16) as u8);

                use rand::prelude::*;
                let mut rng = SmallRng::seed_from_u64(hash);
                let hue = rng.sample(Uniform::new(0.0, 1.0));
                let color: Color32 = Hsva::new(hue, 0.8, 0.8, 1.0).into();

                plot_ui.line(
                    Line::new(PlotPoints::Borrowed(y_data))
                        .color(color)
                        .style(self.line_style)
                        .name(label),
                );
            }
        })
        .response
    }

    pub fn set_data(&mut self, x_data: &Vec<f64>, y_series: &HashMap<String, Vec<f64>>) {
        self.plot_points.clear();
        for (label, y_data) in y_series.iter() {
            let points: Vec<PlotPoint> = x_data
                .iter()
                .zip(y_data.iter())
                .map(|(x, y)| PlotPoint { x: *x, y: *y })
                .collect();

            self.plot_points.insert(label.clone(), points);
        }
    }
}
