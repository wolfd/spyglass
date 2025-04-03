use anyhow::Result;
use slang::DataType;
use slang::LazyFrame;
use slang::PolarsError;
use slang::PolarsResult;
use spyplot::Spyplot;
use std::collections::HashMap;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    file_io: crate::file_io::SpyglassFileDialog,

    #[serde(skip)]
    df: PolarsResult<LazyFrame>,

    x_expr: String,
    y_exprs: Vec<String>,

    error: Option<String>,

    use_spyplot: bool,

    #[serde(skip)]
    xy_plot: crate::xy_plot::XYPlot,

    #[serde(skip)]
    spyplot: Option<spyplot::Spyplot>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            file_io: crate::file_io::SpyglassFileDialog::default(),
            df: Err(PolarsError::NoData("No data".into())),
            x_expr: "utime".to_owned(),
            y_exprs: vec!["position.data[0]".to_owned()],
            error: None,
            use_spyplot: false,
            xy_plot: Default::default(),
            spyplot: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let stored: TemplateApp =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            let stored = if let Some(opened_file) = &stored.file_io.opened_file {
                let df = slang::read_data(opened_file.clone());
                Self {
                    file_io: stored.file_io.copy_for_save(),
                    df,
                    ..stored
                }
            } else {
                stored
            };
            return Self {
                spyplot: Spyplot::new(cc),
                ..stored
            };
        }

        Self {
            spyplot: Spyplot::new(cc),
            ..Default::default()
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                egui::widgets::global_theme_preference_switch(ui);

                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    // maybe get new dataframe
                    if let Some(df) = self.file_io.ui(ui, ctx) {
                        self.df = df;
                    }
                }

                self.xy_plot.options_ui(ui);
            });
        });

        if let Ok(df) = &self.df {
            egui::SidePanel::left("plot_options").show(ctx, |ui| {
                ui.vertical(|ui| {
                    if let Ok(schema) = df.clone().collect_schema() {
                        schema.iter().for_each(|(name, data_type)| {
                            render_schema(ui, name.clone().into_string(), data_type);
                        });
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                self.editor_ui(ui);
            });
            ui.separator();

            ui.checkbox(&mut self.use_spyplot, "Use spyplot viewer");

            if self.use_spyplot {
                self.spyplot.as_mut().unwrap().ui(ui);
            } else {
                self.xy_plot.ui(ui);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

impl TemplateApp {
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        ui.text_edit_singleline(&mut self.x_expr);
        ui.horizontal(|ui| {
            if ui.small_button("-").clicked() {
                self.y_exprs.pop();
            }
            if ui.small_button("+").clicked() {
                if self.y_exprs.is_empty() {
                    self.y_exprs.push("".to_owned());
                } else {
                    self.y_exprs.push(self.y_exprs.last().unwrap().clone());
                }
            }
        });
        for y_expr in &mut self.y_exprs {
            ui.text_edit_singleline(y_expr);
        }

        if ui
            .button("Run")
            .on_hover_text("Run the expression")
            .clicked()
        {
            self.error = self.eval_and_plot().err().map(|e| e.to_string());
        }

        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
    }

    fn eval_and_plot(&mut self) -> Result<()> {
        if let PolarsResult::Ok(df) = &self.df {
            let mut y_series: HashMap<String, Vec<f64>> = HashMap::new();
            for y_expr in self.y_exprs.iter() {
                for y_trace in slang::eval(df, &y_expr)?.into_iter() {
                    y_series.insert(y_trace.name, y_trace.data);
                }
            }

            let x_data = slang::eval(df, &self.x_expr)?
                .into_iter()
                .next()
                .ok_or(anyhow::anyhow!("No x_expr trace"))?
                .data;
            self.xy_plot.set_data(&x_data, &y_series);
            if y_series.len() != 0 {
                let points: Vec<[f64; 2]> = x_data
                    .iter()
                    .zip(y_series.iter().next().unwrap().1)
                    .map(|(x, y)| [*x, *y])
                    .collect();

                let spyplot = self.spyplot.as_mut().expect("Spyplot not initialized!");
                spyplot.line = spyplot::to_vertices(points);
                spyplot.dirty = true;
            }
        }

        Ok(())
    }
}

fn describe_data_type(data_type: &DataType) -> String {
    match data_type {
        DataType::Struct(_inner) => "struct".to_string(),
        DataType::List(_inner) => "list".to_string(),
        DataType::Boolean => "bool".to_string(),
        DataType::Int8 => "int8".to_string(),
        DataType::Int16 => "int16".to_string(),
        DataType::Int32 => "int32".to_string(),
        DataType::Int64 => "int64".to_string(),
        DataType::UInt8 => "uint8".to_string(),
        DataType::UInt16 => "uint16".to_string(),
        DataType::UInt32 => "uint32".to_string(),
        DataType::UInt64 => "uint64".to_string(),
        DataType::Float32 => "float32".to_string(),
        DataType::Float64 => "float64".to_string(),
        DataType::Date => "Date".to_string(),
        DataType::Datetime(time_unit, time_zone) => format!(
            "DateTime({}{})",
            time_unit.to_ascii(),
            if let Some(time_zone) = time_zone {
                format!(", {}", time_zone)
            } else {
                "".to_string()
            }
        ),
        data_type => format!("{:?} [unsupported]", data_type),
    }
}

fn render_schema(ui: &mut egui::Ui, name: String, data_type: &DataType) {
    if data_type.is_nested() {
        match data_type {
            DataType::Struct(fields) => {
                ui.collapsing(
                    format!("{}: {}", name, describe_data_type(data_type)),
                    |ui| {
                        for field in fields {
                            render_schema(ui, field.name.clone().into_string(), field.dtype());
                        }
                    },
                );
            }
            DataType::List(inner) => {
                if inner.leaf_dtype().is_nested() {
                    render_schema(ui, format!("{}[]", name), inner.leaf_dtype());
                } else {
                    ui.label(format!(
                        "{}[]: {}",
                        name,
                        describe_data_type(inner.leaf_dtype())
                    ));
                }
            }
            _ => unreachable!(),
        }
    } else {
        ui.label(format!("{}: {}", name, describe_data_type(data_type)));
    }
}
