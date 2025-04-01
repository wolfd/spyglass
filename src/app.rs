use anyhow::Result;
use egui_file::FileDialog;
use slang::DataType;
use slang::LazyFrame;
use slang::PolarsError;
use slang::PolarsResult;
use spyplot::Spyplot;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    opened_file: Option<PathBuf>,
    #[serde(skip)]
    open_file_dialog: Option<FileDialog>,

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
            opened_file: None,
            open_file_dialog: None,
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
            if let Some(opened_file) = &stored.opened_file {
                let df = slang::read_data(opened_file.clone());
                return Self {
                    opened_file: stored.opened_file.clone(),
                    df,
                    spyplot: Spyplot::new(cc),
                    ..stored
                };
            }
            return stored;
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
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Load").clicked() {
                            let filter = Box::new({
                                let ext = Some(OsStr::new("parquet"));
                                move |path: &Path| -> bool { path.extension() == ext }
                            });
                            let mut dialog = FileDialog::open_file(self.opened_file.clone())
                                .show_files_filter(filter);
                            dialog.open();
                            self.open_file_dialog = Some(dialog);

                            ui.close_menu();
                        }
                    });

                    if let Some(dialog) = &mut self.open_file_dialog {
                        if dialog.show(ctx).selected() {
                            if let Some(file) = dialog.path() {
                                self.opened_file = Some(file.to_path_buf());
                                self.df = slang::read_data(self.opened_file.clone().unwrap());
                            }
                        }
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
            self.xy_plot.x_data.clear();
            self.xy_plot.y_series.clear();

            for y_expr in self.y_exprs.iter() {
                for y_trace in slang::eval(df, &y_expr)?.into_iter() {
                    self.xy_plot.y_series.insert(y_trace.name, y_trace.data);
                }
            }

            self.xy_plot.x_data = slang::eval(df, &self.x_expr)?
                .into_iter()
                .next()
                .ok_or(anyhow::anyhow!("No x_expr trace"))?
                .data;

            let points: Vec<[f32; 2]> = self
                .xy_plot
                .x_data
                .iter()
                .zip(self.xy_plot.y_series.iter().next().unwrap().1)
                .map(|(x, y)| [*x as f32, *y as f32])
                .collect();

            let spyplot = self.spyplot.as_mut().unwrap();
            spyplot.line = spyplot::to_vertices(points);
            spyplot.dirty = true;
        }

        Ok(())
    }
}

fn describe_data_type(data_type: &DataType) -> String {
    match data_type {
        DataType::Struct(_inner) => "struct",
        DataType::List(_inner) => "list",
        DataType::Boolean => "bool",
        DataType::Int8 => "int8",
        DataType::Int16 => "int16",
        DataType::Int32 => "int32",
        DataType::Int64 => "int64",
        DataType::UInt8 => "uint8",
        DataType::UInt16 => "uint16",
        DataType::UInt32 => "uint32",
        DataType::UInt64 => "uint64",
        DataType::Float32 => "float32",
        DataType::Float64 => "float64",
        _ => "unknown",
    }
    .to_string()
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
