use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, CentralPanel, Frame, Key, KeyboardShortcut, Layout, Margin, Modifiers,
    RichText, ScrollArea, SidePanel, Slider, TextEdit, TopBottomPanel, Vec2,
};
use rfd::FileDialog;

use crate::document_loader::{DocumentFingerprint, load_markdown_document};
use crate::i18n::{Language, TranslationKey, tr};
use crate::image_cache::ImageCache;
use crate::metrics;
use crate::parser::{HeadingNavItem, MarkdownDocument, SearchMatch};
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
const SEARCH_INPUT_ID: &str = "document_search_input";

pub struct OxideMdApp {
    ui_context: egui::Context,
    language: Language,
    theme_id: ThemeId,
    zoom_factor: f32,
    current_file: Option<PathBuf>,
    document: Option<MarkdownDocument>,
    document_fingerprint: Option<DocumentFingerprint>,
    image_cache: ImageCache,
    status_message: String,
    reload_status: ReloadStatus,
    reload_worker: ReloadWorkerHandle,
    watcher: Option<FileWatcherHandle>,
    pending_reload_at: Option<Instant>,
    queued_reload_id: u64,
    in_flight_reload_id: Option<u64>,
    pending_block_scroll: Option<usize>,
    active_heading: Option<usize>,
    selected_heading: Option<usize>,
    search_query: String,
    search_matches: Vec<SearchMatch>,
    active_search_index: Option<usize>,
    focus_search_input: bool,
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
            document_fingerprint: None,
            image_cache: ImageCache::new(),
            status_message: tr(language, TranslationKey::StatusNoFile).to_owned(),
            reload_status: ReloadStatus::Idle,
            watcher: None,
            pending_reload_at: None,
            queued_reload_id: 0,
            in_flight_reload_id: None,
            pending_block_scroll: None,
            active_heading: None,
            selected_heading: None,
            search_query: String::new(),
            search_matches: Vec::new(),
            active_search_index: None,
            focus_search_input: false,
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

    fn handle_file_drops(&mut self, ctx: &egui::Context) {
        let dropped_paths: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect()
        });

        if dropped_paths.is_empty() {
            return;
        }

        if let Some(path) = dropped_paths.iter().find(|path| is_markdown_path(path)) {
            self.load_selected_file(path.clone());
            return;
        }

        if let Some(path) = dropped_paths.first() {
            self.set_reload_error(
                TranslationKey::StatusUnsupportedFile,
                path.display().to_string(),
            );
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
            Ok(loaded) => {
                let document = loaded.document;
                let active_heading = document.headings().first().map(|item| item.block_index);
                self.current_file = Some(path.clone());
                self.document = Some(document);
                self.document_fingerprint = Some(loaded.fingerprint);
                self.image_cache.clear();
                self.pending_reload_at = None;
                self.in_flight_reload_id = None;
                self.reload_status = ReloadStatus::Idle;
                self.pending_block_scroll = None;
                self.active_heading = active_heading;
                self.selected_heading = None;
                self.refresh_search_matches();
                self.start_watching_file(&path);
                metrics::log_initial_load(&path, &loaded.timing);
                self.status_message = self.status_with_path(TranslationKey::StatusLoaded, &path);
            }
            Err(error) => {
                self.document = None;
                self.document_fingerprint = None;
                self.image_cache.clear();
                self.current_file = None;
                self.watcher = None;
                self.pending_reload_at = None;
                self.in_flight_reload_id = None;
                self.pending_block_scroll = None;
                self.active_heading = None;
                self.selected_heading = None;
                self.search_matches.clear();
                self.active_search_index = None;
                self.set_reload_error(TranslationKey::StatusLoadFailed, error);
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
                self.set_reload_error(TranslationKey::StatusWatchFailed, error);
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
            self.schedule_reload();
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
        }

        if let Some(error) = watch_error {
            self.set_reload_error(TranslationKey::StatusWatchFailed, error);
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

        match self
            .reload_worker
            .request_reload(reload_id, path.clone(), self.document_fingerprint)
        {
            Ok(()) => {
                self.in_flight_reload_id = Some(reload_id);
                self.set_reload_in_progress(TranslationKey::StatusReloadStarted, Some(&path));
            }
            Err(error) => {
                self.set_reload_error(TranslationKey::StatusWorkerFailed, error);
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
                    fingerprint,
                } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.finish_reload_success(path, document, timing, fingerprint);
                }
                ReloadResponse::Unchanged {
                    id,
                    path,
                    timing,
                    fingerprint,
                } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.finish_reload_unchanged(path, timing, fingerprint);
                }
                ReloadResponse::Error { id, path, error } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.finish_reload_error(path, error);
                }
            }
        }
    }

    fn schedule_reload(&mut self) {
        self.pending_reload_at = Some(Instant::now());
        self.set_reload_in_progress(TranslationKey::ReloadReloading, None);
    }

    fn finish_reload_success(
        &mut self,
        path: PathBuf,
        document: MarkdownDocument,
        timing: metrics::DocumentTiming,
        fingerprint: DocumentFingerprint,
    ) {
        let active_heading = document.headings().first().map(|item| item.block_index);
        self.in_flight_reload_id = None;
        self.pending_block_scroll = None;
        self.active_heading = active_heading;
        self.selected_heading = None;
        self.document = Some(document);
        self.document_fingerprint = Some(fingerprint);
        self.image_cache.clear();
        self.refresh_search_matches();
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload(&path, &timing);
        self.status_message = self.status_with_path(TranslationKey::StatusReloaded, &path);
    }

    fn finish_reload_unchanged(
        &mut self,
        path: PathBuf,
        timing: metrics::DocumentTiming,
        fingerprint: DocumentFingerprint,
    ) {
        self.in_flight_reload_id = None;
        self.document_fingerprint = Some(fingerprint);
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload_skipped(&path, &timing);
        self.status_message = self.status_with_path(TranslationKey::StatusReloadSkipped, &path);
    }

    fn finish_reload_error(&mut self, path: PathBuf, error: String) {
        self.in_flight_reload_id = None;
        self.reload_status = ReloadStatus::Error;
        self.status_message = format!(
            "{} {} ({})",
            tr(self.language, TranslationKey::StatusReloadFailed),
            path.display(),
            error
        );
    }

    fn set_reload_in_progress(&mut self, key: TranslationKey, path: Option<&Path>) {
        self.reload_status = ReloadStatus::Reloading;
        self.status_message = match path {
            Some(path) => self.status_with_path(key, path),
            None => tr(self.language, key).to_owned(),
        };
    }

    fn set_reload_error(&mut self, key: TranslationKey, error: String) {
        self.reload_status = ReloadStatus::Error;
        self.status_message = format!("{} {}", tr(self.language, key), error);
    }

    fn status_with_path(&self, key: TranslationKey, path: &Path) -> String {
        format!("{} {}", tr(self.language, key), path.display())
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let open_file = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))
        });
        let focus_search = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::F))
        });
        let reload_file = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::R))
                || input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F5))
        });
        let next_search = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F3))
        });
        let previous_search = ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::F3))
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

        if focus_search {
            self.focus_search_input = true;
        }

        if reload_file {
            self.request_manual_reload();
        }

        if previous_search {
            self.select_previous_search_match();
        }

        if next_search {
            self.select_next_search_match();
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

    fn refresh_search_matches(&mut self) {
        let Some(document) = self.document.as_ref() else {
            self.search_matches.clear();
            self.active_search_index = None;
            return;
        };

        let previous_block = self
            .active_search_index
            .and_then(|index| self.search_matches.get(index))
            .map(|entry| entry.block_index);

        self.search_matches = document.search_matches(&self.search_query);

        self.active_search_index = previous_block
            .and_then(|block_index| {
                self.search_matches
                    .iter()
                    .position(|entry| entry.block_index == block_index)
            })
            .or_else(|| (!self.search_matches.is_empty()).then_some(0));
    }

    fn select_search_match(&mut self, index: usize) {
        let Some(search_match) = self.search_matches.get(index) else {
            return;
        };

        self.active_search_index = Some(index);
        self.pending_block_scroll = Some(search_match.block_index);
        self.selected_heading = None;
    }

    fn select_next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        let next_index = match self.active_search_index {
            Some(index) => (index + 1) % self.search_matches.len(),
            None => 0,
        };

        self.select_search_match(next_index);
    }

    fn select_previous_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        let previous_index = match self.active_search_index {
            Some(0) | None => self.search_matches.len() - 1,
            Some(index) => index - 1,
        };

        self.select_search_match(previous_index);
    }

    fn clear_selected_heading_on_manual_scroll(&mut self, ctx: &egui::Context) {
        let scroll_delta_y = ctx.input(|input| input.raw_scroll_delta.y);

        if scroll_delta_y.abs() > f32::EPSILON {
            self.selected_heading = None;
        }
    }

    fn render_search_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(tr(self.language, TranslationKey::LabelSearch));

        let search_input_id = egui::Id::new(SEARCH_INPUT_ID);
        let response = ui.add(
            TextEdit::singleline(&mut self.search_query)
                .id(search_input_id)
                .desired_width(f32::INFINITY),
        );

        if self.focus_search_input {
            response.request_focus();
            self.focus_search_input = false;
        }

        if response.changed() {
            self.refresh_search_matches();

            if !self.search_matches.is_empty() {
                self.select_search_match(0);
            }
        }

        if response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
            self.select_next_search_match();
        }

        ui.horizontal(|ui| {
            let result_label = if self.search_matches.is_empty() {
                tr(self.language, TranslationKey::MessageSearchNoResults).to_owned()
            } else {
                let position = self.active_search_index.map(|index| index + 1).unwrap_or(0);
                format!(
                    "{} {}/{}",
                    tr(self.language, TranslationKey::LabelSearchResults),
                    position,
                    self.search_matches.len()
                )
            };
            ui.label(result_label);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.search_query.is_empty(),
                        egui::Button::new(tr(self.language, TranslationKey::ActionSearchClear)),
                    )
                    .clicked()
                {
                    self.search_query.clear();
                    self.search_matches.clear();
                    self.active_search_index = None;
                }

                if ui
                    .add_enabled(
                        !self.search_matches.is_empty(),
                        egui::Button::new(tr(self.language, TranslationKey::ActionSearchNext)),
                    )
                    .clicked()
                {
                    self.select_next_search_match();
                }

                if ui
                    .add_enabled(
                        !self.search_matches.is_empty(),
                        egui::Button::new(tr(self.language, TranslationKey::ActionSearchPrevious)),
                    )
                    .clicked()
                {
                    self.select_previous_search_match();
                }
            });
        });
    }

    fn render_search_results(&mut self, ui: &mut egui::Ui) {
        if self.search_query.trim().is_empty() {
            return;
        }

        ui.add_space(8.0);
        ScrollArea::vertical()
            .id_salt("search_results_scroll")
            .max_height(180.0)
            .show(ui, |ui| {
                if self.search_matches.is_empty() {
                    ui.label(tr(self.language, TranslationKey::MessageSearchNoResults));
                    return;
                }

                let items: Vec<(usize, SearchMatch)> =
                    self.search_matches.iter().cloned().enumerate().collect();

                for (index, search_match) in items {
                    let is_active = self.active_search_index == Some(index);
                    let label = if search_match.preview.is_empty() {
                        format!("#{}", search_match.block_index + 1)
                    } else {
                        search_match.preview
                    };

                    if ui.selectable_label(is_active, label).clicked() {
                        self.select_search_match(index);
                    }
                }
            });
    }

    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let theme = theme(self.theme_id);

        TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(tr(self.language, TranslationKey::ActionOpen))
                    .clicked()
                {
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

        SidePanel::left("heading_navigation")
            .resizable(true)
            .default_width(220.0)
            .width_range(180.0..=320.0)
            .show(ctx, |ui| {
                self.render_search_controls(ui);
                self.render_search_results(ui);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading(tr(self.language, TranslationKey::NavSections));
                ui.add_space(8.0);

                if headings.is_empty() {
                    return;
                }

                ScrollArea::vertical()
                    .id_salt("heading_navigation_scroll")
                    .show(ui, |ui| {
                        for item in &headings {
                            let highlighted_heading = self.selected_heading.or(self.active_heading);
                            let is_active = highlighted_heading == Some(item.block_index);
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
                                    .selectable_label(is_active, &item.title)
                                    .on_hover_text(tr(
                                        self.language,
                                        TranslationKey::NavJumpToHeading,
                                    ))
                                    .clicked()
                                {
                                    self.selected_heading = Some(item.block_index);
                                    self.active_heading = Some(item.block_index);
                                    self.pending_block_scroll = Some(item.block_index);
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
                let document_base_dir = self.current_file.as_ref().and_then(|path| path.parent());

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
                            let render_outcome = render_markdown_document(
                                ui,
                                document,
                                self.language,
                                &theme,
                                self.zoom_factor,
                                document_base_dir,
                                &mut self.image_cache,
                                self.pending_block_scroll,
                                Some(self.search_query.as_str()),
                            );

                            if let Some(active_heading) = render_outcome.active_heading {
                                self.active_heading = Some(active_heading);
                            }

                            if let Some(block_index) = render_outcome
                                .clicked_anchor
                                .and_then(|anchor| document.heading_block_for_anchor(&anchor))
                            {
                                self.selected_heading = Some(block_index);
                                self.active_heading = Some(block_index);
                                self.pending_block_scroll = Some(block_index);
                                ctx.request_repaint();
                            }

                            if render_outcome.did_scroll {
                                self.pending_block_scroll = None;
                            }
                        });
                });
                ui.add_space(24.0);
            });
        });
    }

    fn render_drop_overlay(&self, ctx: &egui::Context) {
        let is_dragging_file = ctx.input(|input| !input.raw.hovered_files.is_empty());
        if !is_dragging_file {
            return;
        }

        let theme = theme(self.theme_id);
        egui::Area::new(egui::Id::new("drop_markdown_overlay"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme.status_loading_background)
                    .stroke(egui::Stroke::new(1.0, theme.content_border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(18, 12))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr(self.language, TranslationKey::MessageDropMarkdown))
                                .color(theme.status_loading_text)
                                .strong(),
                        );
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
        self.handle_file_drops(ctx);
        self.clear_selected_heading_on_manual_scroll(ctx);
        self.process_watch_events();
        self.process_reload_results();
        self.reload_if_ready();

        let theme = theme(self.theme_id);
        apply_theme(ctx, &theme);
        self.render_top_bar(ctx);
        self.render_bottom_bar(ctx);
        self.render_heading_panel(ctx);
        self.render_document_panel(ctx);
        self.render_drop_overlay(ctx);
    }
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
        .unwrap_or(false)
}
