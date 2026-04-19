use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, CentralPanel, Frame, Key, KeyboardShortcut, Layout, Margin, Modifiers, RichText,
    ScrollArea, SidePanel, Slider, TopBottomPanel,
};
use rfd::FileDialog;

use crate::document_loader::load_markdown_document;
use crate::i18n::{Language, TranslationKey, tr};
use crate::metrics;
use crate::parser::{HeadingNavItem, MarkdownDocument};
use crate::reload_worker::{ReloadResponse, ReloadWorkerHandle, spawn_reload_worker};
use crate::renderer::render_markdown_document;
use crate::theme::{DEFAULT_THEME_ID, ThemeId, apply_theme, available_themes, theme};
use crate::watcher::{FileWatchEvent, FileWatcherHandle, watch_file};

#[derive(Clone, Copy)]
enum ReloadStatus {
    Idle,
    Reloading,
    Error,
}

const DEFAULT_ZOOM_FACTOR: f32 = 1.0;
const MIN_ZOOM_FACTOR: f32 = 0.8;
const MAX_ZOOM_FACTOR: f32 = 1.8;
const ZOOM_STEP: f32 = 0.1;

pub struct OxideMdApp {
    ui_context: egui::Context,
    language: Language,
    theme_id: ThemeId,
    zoom_factor: f32,
    current_file: Option<PathBuf>,
    document: Option<MarkdownDocument>,
    status_message: String,
    reload_status: ReloadStatus,
    reload_worker: ReloadWorkerHandle,
    watcher: Option<FileWatcherHandle>,
    pending_reload_at: Option<Instant>,
    queued_reload_id: u64,
    in_flight_reload_id: Option<u64>,
    pending_heading_scroll: Option<usize>,
    startup_started: Option<Instant>,
}

impl OxideMdApp {
    pub fn new(ui_context: egui::Context, startup_started: Instant) -> Self {
        let language = Language::En;
        debug_assert!(available_themes().contains(&DEFAULT_THEME_ID));

        Self {
            reload_worker: spawn_reload_worker(ui_context.clone()),
            ui_context,
            language,
            theme_id: DEFAULT_THEME_ID,
            zoom_factor: DEFAULT_ZOOM_FACTOR,
            current_file: None,
            document: None,
            status_message: tr(language, TranslationKey::StatusNoFile).to_owned(),
            reload_status: ReloadStatus::Idle,
            watcher: None,
            pending_reload_at: None,
            queued_reload_id: 0,
            in_flight_reload_id: None,
            pending_heading_scroll: None,
            startup_started: Some(startup_started),
        }
    }

    fn switch_language(&mut self) {
        self.language = match self.language {
            Language::En => Language::Ja,
            Language::Ja => Language::En,
        };

        if self.current_file.is_none() {
            self.status_message = tr(self.language, TranslationKey::StatusNoFile).to_owned();
        }
    }

    fn switch_theme(&mut self) {
        self.theme_id = self.theme_id.next();
    }

    fn current_theme_label(&self) -> &'static str {
        match self.theme_id {
            ThemeId::WarmPaper => tr(self.language, TranslationKey::ThemeWarmPaper),
            ThemeId::Mist => tr(self.language, TranslationKey::ThemeMist),
            ThemeId::NightOwl => tr(self.language, TranslationKey::ThemeNightOwl),
        }
    }

    fn zoom_in(&mut self) {
        self.zoom_factor = (self.zoom_factor + ZOOM_STEP).clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);
    }

    fn zoom_out(&mut self) {
        self.zoom_factor = (self.zoom_factor - ZOOM_STEP).clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);
    }

    fn reset_zoom(&mut self) {
        self.zoom_factor = DEFAULT_ZOOM_FACTOR;
    }

    fn zoom_label(&self) -> String {
        format!(
            "{} {}%",
            tr(self.language, TranslationKey::LabelZoom),
            (self.zoom_factor * 100.0).round()
        )
    }

    fn zoom_percent(&self) -> f32 {
        self.zoom_factor * 100.0
    }

    fn open_markdown_file(&mut self) {
        let selected_file = FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file();

        if let Some(path) = selected_file {
            self.load_selected_file(path);
        }
    }

    fn current_file_label(&self) -> String {
        self.current_file
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| tr(self.language, TranslationKey::LabelNoFile).to_owned())
    }

    fn heading_nav_items(&self) -> Vec<HeadingNavItem> {
        self.document
            .as_ref()
            .map(|document| document.headings().to_vec())
            .unwrap_or_default()
    }

    fn load_selected_file(&mut self, path: PathBuf) {
        match load_markdown_document(&path) {
            Ok((document, timing)) => {
                self.current_file = Some(path.clone());
                self.document = Some(document);
                self.pending_reload_at = None;
                self.in_flight_reload_id = None;
                self.reload_status = ReloadStatus::Idle;
                self.start_watching_file(&path);
                metrics::log_initial_load(&path, &timing);
                self.status_message =
                    format!("{} {}", tr(self.language, TranslationKey::StatusLoaded), path.display());
            }
            Err(error) => {
                self.document = None;
                self.current_file = None;
                self.reload_status = ReloadStatus::Error;
                self.watcher = None;
                self.pending_reload_at = None;
                self.in_flight_reload_id = None;
                self.status_message =
                    format!("{} {}", tr(self.language, TranslationKey::StatusLoadFailed), error);
            }
        }
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
                    format!("{} {}", tr(self.language, TranslationKey::StatusWatchFailed), error);
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
            self.status_message = tr(self.language, TranslationKey::ReloadReloading).to_owned();
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
        }

        if let Some(error) = watch_error {
            self.reload_status = ReloadStatus::Error;
            self.status_message = format!(
                "{} {}",
                tr(self.language, TranslationKey::StatusWatchFailed),
                error
            );
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

        if self.in_flight_reload_id.is_some() {
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
            return;
        }

        self.pending_reload_at = None;
        self.enqueue_reload(path);
    }

    fn reload_status_label(&self) -> &'static str {
        match self.reload_status {
            ReloadStatus::Idle => tr(self.language, TranslationKey::ReloadIdle),
            ReloadStatus::Reloading => tr(self.language, TranslationKey::ReloadReloading),
            ReloadStatus::Error => tr(self.language, TranslationKey::ReloadError),
        }
    }

    fn request_manual_reload(&mut self) {
        let Some(path) = self.current_file.clone() else {
            self.status_message = tr(self.language, TranslationKey::StatusNoFile).to_owned();
            return;
        };

        if self.in_flight_reload_id.is_some() {
            return;
        }

        self.pending_reload_at = None;
        self.enqueue_reload(path);
    }

    fn enqueue_reload(&mut self, path: PathBuf) {
        self.queued_reload_id += 1;
        let reload_id = self.queued_reload_id;

        match self.reload_worker.request_reload(reload_id, path.clone()) {
            Ok(()) => {
                self.in_flight_reload_id = Some(reload_id);
                self.reload_status = ReloadStatus::Reloading;
                self.status_message = format!(
                    "{} {}",
                    tr(self.language, TranslationKey::StatusReloadStarted),
                    path.display()
                );
            }
            Err(error) => {
                self.reload_status = ReloadStatus::Error;
                self.status_message =
                    format!("{} {}", tr(self.language, TranslationKey::StatusWorkerFailed), error);
            }
        }
    }

    fn process_reload_results(&mut self) {
        while let Ok(result) = self.reload_worker.receiver.try_recv() {
            match result {
                ReloadResponse::Reloaded {
                    id,
                    path,
                    document,
                    timing,
                } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.in_flight_reload_id = None;
                    self.document = Some(document);
                    self.reload_status = ReloadStatus::Idle;
                    metrics::log_reload(&path, &timing);
                    self.status_message = format!(
                        "{} {}",
                        tr(self.language, TranslationKey::StatusReloaded),
                        path.display()
                    );
                }
                ReloadResponse::Error { id, path, error } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.in_flight_reload_id = None;
                    self.reload_status = ReloadStatus::Error;
                    self.status_message = format!(
                        "{} {} ({})",
                        tr(self.language, TranslationKey::StatusReloadFailed),
                        path.display(),
                        error
                    );
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let open_file = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))
        });
        let reload_file = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::R))
                || input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F5))
        });
        let switch_language = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::L))
        });
        let switch_theme = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::T))
        });
        let zoom_in = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Plus))
                || input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Equals))
        });
        let zoom_out = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus))
        });
        let reset_zoom = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0))
        });

        if open_file {
            self.open_markdown_file();
        }

        if reload_file {
            self.request_manual_reload();
        }

        if switch_language {
            self.switch_language();
        }

        if switch_theme {
            self.switch_theme();
        }

        if zoom_in {
            self.zoom_in();
        }

        if zoom_out {
            self.zoom_out();
        }

        if reset_zoom {
            self.reset_zoom();
        }
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let theme = theme(self.theme_id);

        TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(tr(self.language, TranslationKey::ActionOpen)).clicked() {
                    self.open_markdown_file();
                }

                if ui
                    .button(tr(self.language, TranslationKey::ActionSwitchLanguage))
                    .clicked()
                {
                    self.switch_language();
                }

                if ui
                    .button(format!(
                        "{} {}",
                        tr(self.language, TranslationKey::ActionSwitchTheme),
                        self.current_theme_label()
                    ))
                    .clicked()
                {
                    self.switch_theme();
                }

                ui.separator();
                ui.label(format!(
                    "{} {}",
                    tr(self.language, TranslationKey::LabelCurrentFile),
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
    }

    fn render_bottom_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.zoom_label());

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(tr(self.language, TranslationKey::ActionResetZoom))
                        .clicked()
                    {
                        self.reset_zoom();
                    }

                    let slider =
                        Slider::new(&mut self.zoom_factor, MIN_ZOOM_FACTOR..=MAX_ZOOM_FACTOR)
                            .show_value(false)
                            .step_by(ZOOM_STEP.into())
                            .smart_aim(false);
                    ui.add_sized([160.0, 0.0], slider);

                    ui.label(format!("{:.0}%", self.zoom_percent()));
                });
            });
        });
    }

    fn render_heading_panel(&mut self, ctx: &egui::Context) {
        let headings = self.heading_nav_items();

        if headings.is_empty() {
            return;
        }

        SidePanel::left("heading_navigation")
            .resizable(true)
            .default_width(220.0)
            .width_range(180.0..=320.0)
            .show(ctx, |ui| {
                ui.heading(tr(self.language, TranslationKey::NavSections));
                ui.add_space(8.0);

                ScrollArea::vertical().show(ui, |ui| {
                    for item in &headings {
                        let indent = match item.level {
                            pulldown_cmark::HeadingLevel::H1 => 0.0,
                            pulldown_cmark::HeadingLevel::H2 => 10.0,
                            pulldown_cmark::HeadingLevel::H3 => 20.0,
                            pulldown_cmark::HeadingLevel::H4 => 30.0,
                            pulldown_cmark::HeadingLevel::H5 => 40.0,
                            pulldown_cmark::HeadingLevel::H6 => 50.0,
                        };

                        ui.horizontal(|ui| {
                            ui.add_space(indent);

                            if ui
                                .selectable_label(false, &item.title)
                                .on_hover_text(tr(self.language, TranslationKey::NavJumpToHeading))
                                .clicked()
                            {
                                self.pending_heading_scroll = Some(item.block_index);
                            }
                        });
                    }
                });
            });
    }

    fn render_document_panel(&mut self, ctx: &egui::Context) {
        let theme = theme(self.theme_id);

        CentralPanel::default().show(ctx, |ui| {
            let Some(document) = self.document.as_ref() else {
                ui.vertical_centered(|ui| {
                    ui.add_space(48.0);
                    ui.label(
                        RichText::new(tr(self.language, TranslationKey::MessageEmpty)).heading(),
                    );
                    ui.add_space(12.0);
                    ui.label(tr(self.language, TranslationKey::MessageOpenPrompt));
                });
                return;
            };

            ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(18.0);
                ui.vertical_centered(|ui| {
                    ui.set_max_width(840.0);
                    Frame::new()
                        .fill(theme.content_background)
                        .stroke(egui::Stroke::new(1.0, theme.content_border))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 8],
                            blur: 28,
                            spread: 0,
                            color: theme.content_shadow,
                        })
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(Margin::symmetric(32, 28))
                        .show(ui, |ui| {
                            ui.set_max_width(760.0);
                            let did_scroll = render_markdown_document(
                                ui,
                                document,
                                self.language,
                                &theme,
                                self.zoom_factor,
                                self.pending_heading_scroll,
                            );

                            if did_scroll {
                                self.pending_heading_scroll = None;
                            }
                        });
                });
                ui.add_space(24.0);
            });
        });
    }
}

impl eframe::App for OxideMdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(startup_started) = self.startup_started.take() {
            metrics::log_startup(startup_started.elapsed());
        }

        self.handle_keyboard_shortcuts(ctx);
        self.process_watch_events();
        self.process_reload_results();
        self.reload_if_ready();

        let theme = theme(self.theme_id);
        apply_theme(ctx, &theme);
        self.render_top_bar(ctx);
        self.render_bottom_bar(ctx);
        self.render_heading_panel(ctx);
        self.render_document_panel(ctx);
    }
}
