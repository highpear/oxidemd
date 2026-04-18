use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, CentralPanel, Frame, Margin, RichText, ScrollArea, TopBottomPanel};
use rfd::FileDialog;

use crate::i18n::{Language, tr};
use crate::parser::{MarkdownDocument, parse_markdown};
use crate::renderer::render_markdown_document;
use crate::theme::default_theme;
use crate::watcher::{FileWatchEvent, FileWatcherHandle, watch_file};

#[derive(Clone, Copy)]
enum ReloadStatus {
    Idle,
    Reloading,
    Error,
}

pub struct OxideMdApp {
    ui_context: egui::Context,
    language: Language,
    current_file: Option<PathBuf>,
    document: Option<MarkdownDocument>,
    status_message: String,
    reload_status: ReloadStatus,
    watcher: Option<FileWatcherHandle>,
    pending_reload_at: Option<Instant>,
}

impl OxideMdApp {
    pub fn new(ui_context: egui::Context) -> Self {
        let language = Language::En;

        Self {
            ui_context,
            language,
            current_file: None,
            document: None,
            status_message: tr(language, "status.no_file").to_owned(),
            reload_status: ReloadStatus::Idle,
            watcher: None,
            pending_reload_at: None,
        }
    }

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
            self.load_selected_file(path);
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

    fn load_selected_file(&mut self, path: PathBuf) {
        match self.load_markdown_document(&path) {
            Ok(document) => {
                self.current_file = Some(path.clone());
                self.document = Some(document);
                self.pending_reload_at = None;
                self.reload_status = ReloadStatus::Idle;
                self.start_watching_file(&path);
                self.status_message =
                    format!("{} {}", tr(self.language, "status.loaded"), path.display());
            }
            Err(error) => {
                self.document = None;
                self.current_file = None;
                self.reload_status = ReloadStatus::Error;
                self.watcher = None;
                self.pending_reload_at = None;
                self.status_message =
                    format!("{} {}", tr(self.language, "status.load_failed"), error);
            }
        }
    }

    fn load_markdown_document(&self, path: &Path) -> Result<MarkdownDocument, String> {
        self.load_file(path).map(|content| parse_markdown(&content))
    }

    fn start_watching_file(&mut self, path: &Path) {
        match watch_file(path, self.ui_context.clone()) {
            Ok(handle) => {
                self.watcher = Some(handle);
            }
            Err(error) => {
                self.watcher = None;
                self.reload_status = ReloadStatus::Error;
                self.status_message =
                    format!("{} {}", tr(self.language, "status.watch_failed"), error);
            }
        }
    }

    fn process_watch_events(&mut self) {
        let mut watch_error = None;
        let mut saw_change = false;

        if let Some(watcher) = &self.watcher {
            while let Ok(event) = watcher.receiver.try_recv() {
                match event {
                    FileWatchEvent::Changed => {
                        saw_change = true;
                    }
                    FileWatchEvent::Error(error) => {
                        watch_error = Some(error);
                    }
                }
            }
        }

        if saw_change {
            self.pending_reload_at = Some(Instant::now());
            self.reload_status = ReloadStatus::Reloading;
            self.status_message = tr(self.language, "reload.reloading").to_owned();
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
        }

        if let Some(error) = watch_error {
            self.reload_status = ReloadStatus::Error;
            self.status_message = format!("{} {}", tr(self.language, "status.watch_failed"), error);
        }
    }

    fn reload_if_ready(&mut self) {
        let Some(last_change_at) = self.pending_reload_at else {
            return;
        };

        if last_change_at.elapsed() < Duration::from_millis(200) {
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
            return;
        }

        let Some(path) = self.current_file.clone() else {
            self.pending_reload_at = None;
            return;
        };

        self.pending_reload_at = None;

        match self.load_markdown_document(&path) {
            Ok(document) => {
                self.document = Some(document);
                self.reload_status = ReloadStatus::Idle;
                self.status_message = format!(
                    "{} {}",
                    tr(self.language, "status.reloaded"),
                    path.display()
                );
            }
            Err(error) => {
                self.reload_status = ReloadStatus::Error;
                self.status_message =
                    format!("{} {}", tr(self.language, "status.reload_failed"), error);
            }
        }
    }

    fn reload_status_label(&self) -> &'static str {
        match self.reload_status {
            ReloadStatus::Idle => tr(self.language, "reload.idle"),
            ReloadStatus::Reloading => tr(self.language, "reload.reloading"),
            ReloadStatus::Error => tr(self.language, "reload.error"),
        }
    }
}

impl eframe::App for OxideMdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_watch_events();
        self.reload_if_ready();

        let theme = default_theme();

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

                ui.separator();
                let (status_bg, status_text) = match self.reload_status {
                    ReloadStatus::Idle => (theme.status_idle_background, theme.status_idle_text),
                    ReloadStatus::Reloading => {
                        (theme.status_loading_background, theme.status_loading_text)
                    }
                    ReloadStatus::Error => (theme.status_error_background, theme.status_error_text),
                };

                Frame::new()
                    .fill(status_bg)
                    .corner_radius(egui::CornerRadius::same(255))
                    .inner_margin(Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(self.reload_status_label())
                                .color(status_text)
                                .strong(),
                        );
                    });
            });

            ui.label(&self.status_message);
        });

        CentralPanel::default().show(ctx, |ui| {
            let Some(document) = self.document.as_ref() else {
                ui.vertical_centered(|ui| {
                    ui.add_space(32.0);
                    ui.label(RichText::new(tr(self.language, "message.empty")).heading());
                    ui.add_space(8.0);
                    ui.label(tr(self.language, "message.open_prompt"));
                });
                return;
            };

            ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                Frame::new()
                    .fill(theme.content_background)
                    .stroke(egui::Stroke::new(1.0, theme.content_border))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 6],
                        blur: 24,
                        spread: 0,
                        color: theme.content_shadow,
                    })
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(24, 20))
                    .show(ui, |ui| {
                        ui.set_max_width(860.0);
                        render_markdown_document(ui, document, &theme);
                    });
                ui.add_space(16.0);
            });
        });
    }
}
