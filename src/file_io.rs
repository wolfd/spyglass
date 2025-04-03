use egui_file::FileDialog;
use lazy_static::lazy_static;
use slang::{LazyFrame, PolarsResult};
use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

lazy_static! {
    static ref FILE_FORMATS: HashMap<&'static OsStr, usize> = {
        let mut m = HashMap::new();
        m.insert(OsStr::new("parquet"), 1);
        m.insert(OsStr::new("csv"), 2);
        m.insert(OsStr::new("mcap"), 3);
        m
    };
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub(crate) struct SpyglassFileDialog {
    pub(crate) opened_file: Option<PathBuf>,
    #[serde(skip)]
    open_file_dialog: Option<FileDialog>,
}

impl SpyglassFileDialog {
    pub(crate) fn copy_for_save(&self) -> Self {
        Self {
            opened_file: self.opened_file.clone(),
            open_file_dialog: None,
        }
    }

    #[must_use]
    pub(crate) fn ui(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) -> Option<PolarsResult<LazyFrame>> {
        ui.menu_button("File", |ui| {
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.button("Load").clicked() {
                let filter = Box::new({
                    move |path: &Path| -> bool {
                        path.extension()
                            .is_some_and(|ext| FILE_FORMATS.contains_key(ext))
                    }
                });
                let mut dialog =
                    FileDialog::open_file(self.opened_file.clone()).show_files_filter(filter);
                dialog.open();
                self.open_file_dialog = Some(dialog);

                ui.close_menu();
            }
        });

        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(file) = dialog.path() {
                    self.opened_file = Some(file.to_path_buf());
                    return Some(slang::read_data(self.opened_file.clone().unwrap()));
                }
            }
        }

        None
    }
}
