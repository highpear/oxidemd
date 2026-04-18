use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{self, CentralPanel, RichText, ScrollArea, TextEdit, TopBottomPanel};
use rfd::FileDialog;

use crate::i18n::{Language, tr};

pub struct OxideMdApp {
    language: Language,
    current_file: Option<PathBuf>,
    file_content: String,
    status_message: String,
}

impl Default for OxideMdApp {
    fn default() -> Self {
        let language = Language::En;

        Self {
            language,
            current_file: None,
            file_content: String::new(),
            status_message: tr(language, "status.no_file").to_owned(),
        }
    }
}

impl OxideMdApp {
    fn switch_language(&mut self) {
        self.language = match self.language {
            Language::En => Language::Ja,
            Language::Ja => Language::En,
        };

        if self.current_file.is_none() {
            self.status_message = tr(self.language, "status.no_file").to_owned();
        }
    }

    fn open_markdown_file(&mut self) {
        let selected_file = FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file();

        if let Some(path) = selected_file {
            match self.load_file(&path) {
                Ok(content) => {
                    self.current_file = Some(path.clone());
                    self.file_content = content;
                    self.status_message =
                        format!("{} {}", tr(self.language, "status.loaded"), path.display());
                }
                Err(error) => {
                    self.status_message =
                        format!("{} {}", tr(self.language, "status.load_failed"), error);
                }
            }
        }
    }

    fn load_file(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| error.to_string())
    }

    fn current_file_label(&self) -> String {
        self.current_file
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| tr(self.language, "label.no_file").to_owned())
    }
}

impl eframe::App for OxideMdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr(self.language, "action.open")).clicked() {
                    self.open_markdown_file();
                }

                if ui
                    .button(tr(self.language, "action.switch_language"))
                    .clicked()
                {
                    self.switch_language();
                }

                ui.separator();
                ui.label(format!(
                    "{} {}",
                    tr(self.language, "label.current_file"),
                    self.current_file_label()
                ));
            });

            ui.label(&self.status_message);
        });

        CentralPanel::default().show(ctx, |ui| {
            if self.file_content.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(RichText::new(tr(self.language, "message.empty")).heading());
                    ui.add_space(8.0);
                    ui.label(tr(self.language, "message.open_prompt"));
                });
                return;
            }

            ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.file_content)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            });
        });
    }
}
