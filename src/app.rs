use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, CentralPanel, Frame, Key, KeyboardShortcut, Layout, Margin, Modifiers,
    RichText, ScrollArea, SidePanel, Slider, TextEdit, TopBottomPanel, UiBuilder, Vec2,
};
use rfd::FileDialog;

use crate::document_loader::{DocumentFingerprint, FileSnapshot, load_markdown_document};
use crate::i18n::{Language, TranslationKey, tr};
use crate::image_cache::ImageCache;
use crate::metrics;
use crate::parser::MarkdownDocument;
use crate::reload_worker::{ReloadResponse, ReloadWorkerHandle, spawn_reload_worker};
use crate::renderer::render_markdown_document;
use crate::search::SearchMatch;
use crate::theme::{DEFAULT_THEME_ID, ThemeId, apply_theme, available_themes, theme};
use crate::watcher::{FileWatchEvent, FileWatcherHandle, watch_file};

#[derive(Clone, Copy)]
enum ReloadStatus {
    Idle,
    Reloading,
    Error,
}

enum RenderMeasurementReason {
    Load,
    Reload,
}

#[derive(Clone, Copy)]
enum ExternalLinkBehavior {
    AskFirst,
    OpenDirectly,
}

struct PendingRenderMeasurement {
    reason: RenderMeasurementReason,
    path: PathBuf,
}

struct BlockHeightCache {
    fingerprint: Option<DocumentFingerprint>,
    zoom_factor_bits: u32,
    content_width_bits: u32,
    heights: Vec<Option<f32>>,
}

impl RenderMeasurementReason {
    fn as_log_label(&self) -> &'static str {
        match self {
            RenderMeasurementReason::Load => "load",
            RenderMeasurementReason::Reload => "reload",
        }
    }
}

impl ExternalLinkBehavior {
    fn next(self) -> Self {
        match self {
            Self::AskFirst => Self::OpenDirectly,
            Self::OpenDirectly => Self::AskFirst,
        }
    }

    fn label(self, language: Language) -> &'static str {
        match self {
            Self::AskFirst => tr(language, TranslationKey::ValueAskFirst),
            Self::OpenDirectly => tr(language, TranslationKey::ValueOpenDirectly),
        }
    }

    fn storage_value(self) -> &'static str {
        match self {
            Self::AskFirst => "ask",
            Self::OpenDirectly => "open",
        }
    }

    fn from_storage_value(value: &str) -> Option<Self> {
        match value {
            "ask" => Some(Self::AskFirst),
            "open" => Some(Self::OpenDirectly),
            _ => None,
        }
    }
}

impl BlockHeightCache {
    fn new() -> Self {
        Self {
            fingerprint: None,
            zoom_factor_bits: 0,
            content_width_bits: 0,
            heights: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.fingerprint = None;
        self.zoom_factor_bits = 0;
        self.content_width_bits = 0;
        self.heights.clear();
    }

    fn heights_for(
        &mut self,
        fingerprint: Option<DocumentFingerprint>,
        zoom_factor: f32,
        content_width: f32,
        block_count: usize,
    ) -> &mut [Option<f32>] {
        let zoom_factor_bits = zoom_factor.to_bits();
        let content_width_bits = content_width.round().to_bits();

        if self.fingerprint != fingerprint
            || self.zoom_factor_bits != zoom_factor_bits
            || self.content_width_bits != content_width_bits
        {
            self.fingerprint = fingerprint;
            self.zoom_factor_bits = zoom_factor_bits;
            self.content_width_bits = content_width_bits;
            self.heights.clear();
        }

        if self.heights.len() != block_count {
            self.heights.resize(block_count, None);
        }

        &mut self.heights
    }
}

const DEFAULT_ZOOM_FACTOR: f32 = 1.0;
const MIN_ZOOM_FACTOR: f32 = 0.8;
const MAX_ZOOM_FACTOR: f32 = 1.8;
const ZOOM_STEP: f32 = 0.1;
const SEARCH_INPUT_ID: &str = "document_search_input";
const DOCUMENT_FRAME_MAX_WIDTH: f32 = 840.0;
const DOCUMENT_BODY_MAX_WIDTH: f32 = 760.0;
const DOCUMENT_HORIZONTAL_PADDING: f32 = 64.0;
const DOCUMENT_VERTICAL_PADDING: f32 = 56.0;
const DOCUMENT_FRAME_STROKE_WIDTH: f32 = 1.0;
const HEADING_PANEL_DEFAULT_WIDTH: f32 = 300.0;
const HEADING_PANEL_MIN_WIDTH: f32 = HEADING_PANEL_DEFAULT_WIDTH;
const HEADING_PANEL_MAX_WIDTH: f32 = 320.0;
const HEADING_NAV_ITEM_INDENT: f32 = 10.0;
const PREVIEW_WINDOW_SIDE_PADDING: f32 = 32.0;
const PREVIEW_WINDOW_FALLBACK_HEIGHT: f32 = 720.0;
const PREVIEW_WINDOW_MONITOR_MARGIN: f32 = 80.0;
const TOP_BAR_FILE_LABEL_MAX_WIDTH: f32 = 280.0;
const ZOOM_STEP_BUTTON_WIDTH: f32 = 28.0;
const STORAGE_KEY_LANGUAGE: &str = "oxidemd.language";
const STORAGE_KEY_THEME: &str = "oxidemd.theme";
const STORAGE_KEY_ZOOM: &str = "oxidemd.zoom";
const STORAGE_KEY_EXTERNAL_LINKS: &str = "oxidemd.external_links";
const STORAGE_KEY_CURRENT_FILE: &str = "oxidemd.current_file";

pub struct OxideMdApp {
    ui_context: egui::Context,
    language: Language,
    theme_id: ThemeId,
    zoom_factor: f32,
    current_file: Option<PathBuf>,
    document: Option<Arc<MarkdownDocument>>,
    document_fingerprint: Option<DocumentFingerprint>,
    document_file_snapshot: Option<FileSnapshot>,
    image_cache: ImageCache,
    status_message: String,
    status_hover_message: Option<String>,
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
    external_link_behavior: ExternalLinkBehavior,
    pending_external_link: Option<String>,
    pending_render_measurement: Option<PendingRenderMeasurement>,
    block_height_cache: BlockHeightCache,
    startup_started: Option<Instant>,
}

impl OxideMdApp {
    pub fn new(
        ui_context: egui::Context,
        storage: Option<&dyn eframe::Storage>,
        startup_started: Instant,
        initial_file: Option<PathBuf>,
    ) -> Self {
        let language = Language::En;
        debug_assert!(available_themes().contains(&DEFAULT_THEME_ID));

        let mut app = Self {
            reload_worker: spawn_reload_worker(ui_context.clone()),
            ui_context,
            language,
            theme_id: DEFAULT_THEME_ID,
            zoom_factor: DEFAULT_ZOOM_FACTOR,
            current_file: None,
            document: None,
            document_fingerprint: None,
            document_file_snapshot: None,
            image_cache: ImageCache::new(),
            status_message: tr(language, TranslationKey::StatusNoFile).to_owned(),
            status_hover_message: None,
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
            external_link_behavior: ExternalLinkBehavior::AskFirst,
            pending_external_link: None,
            pending_render_measurement: None,
            block_height_cache: BlockHeightCache::new(),
            startup_started: Some(startup_started),
        };

        let restored_file = app.restore_session(storage);
        apply_theme(&app.ui_context, &theme(app.theme_id));

        if let Some(path) = initial_file.or(restored_file) {
            app.load_initial_file(path);
        }

        app
    }

    fn restore_session(&mut self, storage: Option<&dyn eframe::Storage>) -> Option<PathBuf> {
        let storage = storage?;

        if let Some(language) = storage
            .get_string(STORAGE_KEY_LANGUAGE)
            .and_then(|value| language_from_storage_value(&value))
        {
            self.language = language;
        }

        if let Some(theme_id) = storage
            .get_string(STORAGE_KEY_THEME)
            .and_then(|value| theme_id_from_storage_value(&value))
        {
            self.theme_id = theme_id;
        }

        if let Some(zoom_factor) = storage
            .get_string(STORAGE_KEY_ZOOM)
            .and_then(|value| value.parse::<f32>().ok())
        {
            self.zoom_factor = zoom_factor.clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);
        }

        if let Some(external_link_behavior) = storage
            .get_string(STORAGE_KEY_EXTERNAL_LINKS)
            .and_then(|value| ExternalLinkBehavior::from_storage_value(&value))
        {
            self.external_link_behavior = external_link_behavior;
        }

        storage
            .get_string(STORAGE_KEY_CURRENT_FILE)
            .map(PathBuf::from)
            .filter(|path| path.is_file() && is_markdown_path(path))
    }

    fn load_initial_file(&mut self, path: PathBuf) {
        if is_markdown_path(&path) {
            self.load_selected_file(path);
            return;
        }

        self.set_reload_error(
            TranslationKey::StatusUnsupportedFile,
            path.display().to_string(),
        );
    }

    fn switch_language(&mut self) {
        self.language = match self.language {
            Language::En => Language::Ja,
            Language::Ja => Language::En,
        };

        if self.current_file.is_none() {
            self.set_status_message(tr(self.language, TranslationKey::StatusNoFile));
        }
    }

    fn switch_theme(&mut self) {
        self.theme_id = self.theme_id.next();
    }

    fn switch_external_link_behavior(&mut self) {
        self.external_link_behavior = self.external_link_behavior.next();
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

    fn load_selected_file(&mut self, path: PathBuf) {
        match load_markdown_document(&path) {
            Ok(loaded) => {
                let document = loaded.document;
                let active_heading = document.headings().first().map(|item| item.block_index);
                self.current_file = Some(path.clone());
                self.document = Some(document);
                self.document_fingerprint = Some(loaded.fingerprint);
                self.document_file_snapshot = loaded.file_snapshot;
                self.image_cache.clear();
                self.block_height_cache.clear();
                self.pending_reload_at = None;
                self.in_flight_reload_id = None;
                self.reload_status = ReloadStatus::Idle;
                self.pending_block_scroll = None;
                self.active_heading = active_heading;
                self.selected_heading = None;
                self.refresh_search_matches();
                self.start_watching_file(&path);
                self.pending_render_measurement = Some(PendingRenderMeasurement {
                    reason: RenderMeasurementReason::Load,
                    path: path.clone(),
                });
                self.request_window_expansion_for_preview();
                metrics::log_initial_load(&path, &loaded.timing);
                self.set_status_with_path(TranslationKey::StatusLoaded, &path);
            }
            Err(error) => {
                self.document = None;
                self.document_fingerprint = None;
                self.document_file_snapshot = None;
                self.image_cache.clear();
                self.block_height_cache.clear();
                self.current_file = None;
                self.watcher = None;
                self.pending_reload_at = None;
                self.in_flight_reload_id = None;
                self.pending_block_scroll = None;
                self.active_heading = None;
                self.selected_heading = None;
                self.search_matches.clear();
                self.active_search_index = None;
                self.pending_render_measurement = None;
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

    fn request_window_expansion_for_preview(&self) {
        let (current_size, monitor_size, is_maximized, is_fullscreen) =
            self.ui_context.input(|input| {
                let viewport = input.viewport();
                (
                    viewport.inner_rect.map(|rect| rect.size()),
                    viewport.monitor_size,
                    viewport.maximized.unwrap_or(false),
                    viewport.fullscreen.unwrap_or(false),
                )
            });

        if is_maximized || is_fullscreen {
            return;
        }

        let target_width =
            HEADING_PANEL_MAX_WIDTH + DOCUMENT_FRAME_MAX_WIDTH + PREVIEW_WINDOW_SIDE_PADDING;
        let current_height = current_size
            .map(|size| size.y)
            .unwrap_or(PREVIEW_WINDOW_FALLBACK_HEIGHT);
        let target_size = Vec2::new(
            capped_preview_window_width(target_width, monitor_size),
            current_height,
        );

        let Some(current_size) = current_size else {
            self.ui_context
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size));
            return;
        };

        if current_size.x + 1.0 < target_size.x {
            self.ui_context
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                    target_size.x,
                    current_size.y,
                )));
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
            self.set_status_message(tr(self.language, TranslationKey::StatusNoFile));
            return;
        };

        if self.in_flight_reload_id.is_some() {
            return;
        }

        self.pending_reload_at = None;
        self.enqueue_reload(path);
    }

    fn handle_external_link_click(&mut self, ctx: &egui::Context, url: String) {
        match self.external_link_behavior {
            ExternalLinkBehavior::AskFirst => {
                self.pending_external_link = Some(url);
            }
            ExternalLinkBehavior::OpenDirectly => {
                open_external_link(ctx, url);
            }
        }
    }

    fn render_external_link_confirmation(&mut self, ctx: &egui::Context) {
        let Some(url) = self.pending_external_link.clone() else {
            return;
        };

        let mut open_link = false;
        let mut cancel = false;

        egui::Window::new(tr(self.language, TranslationKey::MessageExternalLinkPrompt))
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(url.as_str());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(tr(self.language, TranslationKey::ActionOpenExternalLink))
                        .clicked()
                    {
                        open_link = true;
                    }

                    if ui
                        .button(tr(self.language, TranslationKey::ActionCancel))
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });

        if open_link {
            self.pending_external_link = None;
            open_external_link(ctx, url);
        } else if cancel {
            self.pending_external_link = None;
        }
    }

    fn enqueue_reload(&mut self, path: PathBuf) {
        self.queued_reload_id += 1;
        let reload_id = self.queued_reload_id;

        match self.reload_worker.request_reload(
            reload_id,
            path.clone(),
            self.document_fingerprint,
            self.document_file_snapshot,
        ) {
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
                    file_snapshot,
                } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.finish_reload_success(path, document, timing, fingerprint, file_snapshot);
                }
                ReloadResponse::Unchanged {
                    id,
                    path,
                    timing,
                    fingerprint,
                    file_snapshot,
                } => {
                    if self.in_flight_reload_id != Some(id) {
                        continue;
                    }

                    self.finish_reload_unchanged(path, timing, fingerprint, file_snapshot);
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
        document: Arc<MarkdownDocument>,
        timing: metrics::DocumentTiming,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) {
        let active_heading = document.headings().first().map(|item| item.block_index);
        self.in_flight_reload_id = None;
        self.pending_block_scroll = None;
        self.active_heading = active_heading;
        self.selected_heading = None;
        self.document = Some(document);
        self.document_fingerprint = Some(fingerprint);
        self.document_file_snapshot = file_snapshot;
        self.image_cache.clear();
        self.block_height_cache.clear();
        self.refresh_search_matches();
        self.pending_render_measurement = Some(PendingRenderMeasurement {
            reason: RenderMeasurementReason::Reload,
            path: path.clone(),
        });
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload(&path, &timing);
        self.set_status_with_path(TranslationKey::StatusReloaded, &path);
    }

    fn finish_reload_unchanged(
        &mut self,
        path: PathBuf,
        timing: metrics::DocumentTiming,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) {
        self.in_flight_reload_id = None;
        self.document_fingerprint = Some(fingerprint);
        self.document_file_snapshot = file_snapshot;
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload_skipped(&path, &timing);
        self.set_status_with_path(TranslationKey::StatusReloadSkipped, &path);
    }

    fn finish_reload_error(&mut self, path: PathBuf, error: String) {
        self.in_flight_reload_id = None;
        self.reload_status = ReloadStatus::Error;
        let display_path = status_path_label(&path);
        self.status_message = format!(
            "{} {} ({})",
            tr(self.language, TranslationKey::StatusReloadFailed),
            display_path,
            error
        );
        self.status_hover_message = Some(format!(
            "{} {} ({})",
            tr(self.language, TranslationKey::StatusReloadFailed),
            path.display(),
            error
        ));
    }

    fn set_reload_in_progress(&mut self, key: TranslationKey, path: Option<&Path>) {
        self.reload_status = ReloadStatus::Reloading;
        match path {
            Some(path) => self.set_status_with_path(key, path),
            None => self.set_status_message(tr(self.language, key)),
        };
    }

    fn set_reload_error(&mut self, key: TranslationKey, error: String) {
        self.reload_status = ReloadStatus::Error;
        self.set_status_message(format!("{} {}", tr(self.language, key), error));
    }

    fn set_status_message(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
        self.status_hover_message = None;
    }

    fn set_status_with_path(&mut self, key: TranslationKey, path: &Path) {
        self.status_message = format!("{} {}", tr(self.language, key), status_path_label(path));
        self.status_hover_message = Some(format!("{} {}", tr(self.language, key), path.display()));
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
        let next_search_from_enter = !self.search_matches.is_empty()
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
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

        if next_search || next_search_from_enter {
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

        let mut clicked_match = None;

        ui.add_space(8.0);
        ScrollArea::vertical()
            .id_salt("search_results_scroll")
            .max_height(180.0)
            .show(ui, |ui| {
                if self.search_matches.is_empty() {
                    ui.label(tr(self.language, TranslationKey::MessageSearchNoResults));
                    return;
                }

                for (index, search_match) in self.search_matches.iter().enumerate() {
                    let is_active = self.active_search_index == Some(index);

                    let clicked = if search_match.preview.is_empty() {
                        ui.selectable_label(is_active, format!("#{}", search_match.block_index + 1))
                            .clicked()
                    } else {
                        ui.selectable_label(is_active, search_match.preview.as_str())
                            .clicked()
                    };

                    if clicked {
                        clicked_match = Some(index);
                    }
                }
            });

        if let Some(index) = clicked_match {
            self.select_search_match(index);
        }
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

                if ui
                    .button(format!(
                        "{} {}",
                        tr(self.language, TranslationKey::LabelExternalLinks),
                        self.external_link_behavior.label(self.language)
                    ))
                    .clicked()
                {
                    self.switch_external_link_behavior();
                }

                ui.separator();
                let current_file_label = format!(
                    "{} {}",
                    tr(self.language, TranslationKey::LabelCurrentFile),
                    self.current_file_label()
                );
                let file_label_response = ui.add_sized(
                    [TOP_BAR_FILE_LABEL_MAX_WIDTH, ui.spacing().interact_size.y],
                    egui::Label::new(current_file_label).truncate(),
                );

                if let Some(path) = &self.current_file {
                    file_label_response.on_hover_text(path.display().to_string());
                }

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

            let status_response = ui.add(egui::Label::new(self.status_message.as_str()).truncate());
            if let Some(message) = &self.status_hover_message {
                status_response.on_hover_text(message);
            }
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

                    if ui
                        .add_enabled(
                            self.zoom_factor < MAX_ZOOM_FACTOR,
                            egui::Button::new("+").min_size(Vec2::splat(ZOOM_STEP_BUTTON_WIDTH)),
                        )
                        .clicked()
                    {
                        self.zoom_in();
                    }

                    let slider =
                        Slider::new(&mut self.zoom_factor, MIN_ZOOM_FACTOR..=MAX_ZOOM_FACTOR)
                            .show_value(false)
                            .step_by(ZOOM_STEP.into())
                            .smart_aim(false);
                    ui.add_sized([160.0, 0.0], slider);

                    if ui
                        .add_enabled(
                            self.zoom_factor > MIN_ZOOM_FACTOR,
                            egui::Button::new("-").min_size(Vec2::splat(ZOOM_STEP_BUTTON_WIDTH)),
                        )
                        .clicked()
                    {
                        self.zoom_out();
                    }

                    ui.label(format!("{:.0}%", self.zoom_percent()));
                });
            });
        });
    }

    fn render_heading_panel(&mut self, ctx: &egui::Context) {
        let mut clicked_heading = None;

        SidePanel::left("heading_navigation")
            .resizable(true)
            .default_width(HEADING_PANEL_DEFAULT_WIDTH)
            .width_range(HEADING_PANEL_MIN_WIDTH..=HEADING_PANEL_MAX_WIDTH)
            .show(ctx, |ui| {
                self.render_search_controls(ui);
                self.render_search_results(ui);
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading(tr(self.language, TranslationKey::NavSections));
                ui.add_space(8.0);

                let Some(document) = self.document.as_ref() else {
                    return;
                };

                let headings = document.headings();
                if headings.is_empty() {
                    ui.label(
                        RichText::new(tr(self.language, TranslationKey::NavNoSections))
                            .color(theme(self.theme_id).text_secondary),
                    );
                    return;
                }

                let highlighted_heading = self.selected_heading.or(self.active_heading);

                ScrollArea::vertical()
                    .id_salt("heading_navigation_scroll")
                    .show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        headings.len(),
                        |ui, row_range| {
                            for row_index in row_range {
                                let item = &headings[row_index];
                                let is_active = highlighted_heading == Some(item.block_index);
                                let indent = heading_nav_indent(item.level);

                                ui.horizontal(|ui| {
                                    ui.add_space(indent);

                                    let available_width = (ui.available_width() - indent)
                                        .max(HEADING_NAV_ITEM_INDENT);
                                    let response = ui.add_sized(
                                        [available_width, ui.spacing().interact_size.y],
                                        egui::Button::selectable(is_active, &item.title).truncate(),
                                    );

                                    if response
                                        .on_hover_text(format!(
                                            "{}\n{}",
                                            tr(self.language, TranslationKey::NavJumpToHeading),
                                            item.title
                                        ))
                                        .clicked()
                                    {
                                        clicked_heading = Some(item.block_index);
                                    }
                                });
                            }
                        },
                    );
            });

        if let Some(block_index) = clicked_heading {
            self.selected_heading = Some(block_index);
            self.active_heading = Some(block_index);
            self.pending_block_scroll = Some(block_index);
        }
    }

    fn render_document_panel(&mut self, ctx: &egui::Context) {
        let theme = theme(self.theme_id);

        CentralPanel::default().show(ctx, |ui| {
            let Some(document) = self.document.clone() else {
                ui.vertical_centered(|ui| {
                    ui.add_space(48.0);
                    ui.label(
                        RichText::new(tr(self.language, TranslationKey::MessageEmpty)).heading(),
                    );
                    ui.add_space(12.0);
                    ui.label(tr(self.language, TranslationKey::MessageOpenPrompt));
                    ui.add_space(16.0);
                    if ui
                        .button(tr(self.language, TranslationKey::ActionOpen))
                        .clicked()
                    {
                        self.open_markdown_file();
                    }
                });
                return;
            };
            let active_search_block = self.active_search_block();

            ScrollArea::vertical().show(ui, |ui| {
                let document_base_dir = self.current_file.as_ref().and_then(|path| path.parent());

                ui.add_space(18.0);
                let content_rect = ui.max_rect();
                let frame_width = content_rect.width().min(DOCUMENT_FRAME_MAX_WIDTH);
                let frame_left = content_rect.center().x - frame_width * 0.5;
                let frame_rect = egui::Rect::from_min_size(
                    egui::pos2(frame_left, ui.cursor().top()),
                    Vec2::new(frame_width, 0.0),
                );

                let document_frame = Frame::new()
                    .fill(theme.content_background)
                    .stroke(egui::Stroke::new(
                        DOCUMENT_FRAME_STROKE_WIDTH,
                        theme.content_border,
                    ))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 28,
                        spread: 0,
                        color: theme.content_shadow,
                    })
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(32, 28));
                let background_shape = ui.painter().add(egui::Shape::Noop);
                let content_width =
                    (frame_width - DOCUMENT_HORIZONTAL_PADDING - DOCUMENT_FRAME_STROKE_WIDTH * 2.0)
                        .max(0.0)
                        .min(DOCUMENT_BODY_MAX_WIDTH);
                let content_min = egui::pos2(
                    frame_rect.left()
                        + DOCUMENT_HORIZONTAL_PADDING * 0.5
                        + DOCUMENT_FRAME_STROKE_WIDTH,
                    frame_rect.top()
                        + DOCUMENT_VERTICAL_PADDING * 0.5
                        + DOCUMENT_FRAME_STROKE_WIDTH,
                );
                let content_max_rect = egui::Rect::from_min_max(
                    content_min,
                    egui::pos2(content_min.x + content_width, content_rect.bottom()),
                );

                let mut document_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(content_max_rect)
                        .layout(Layout::top_down(Align::Min)),
                );
                let mut document_clip_rect = document_ui.clip_rect();
                document_clip_rect.min.x = content_max_rect.left();
                document_clip_rect.max.x = content_max_rect.right();
                document_ui.set_clip_rect(document_clip_rect);
                document_ui.set_min_width(content_width);
                document_ui.set_max_width(content_width);

                let render_measurement = self.pending_render_measurement.take();
                let render_started = render_measurement.as_ref().map(|_| Instant::now());
                let block_count = document.blocks.len();
                let heading_count = document.headings().len();
                let block_heights = self.block_height_cache.heights_for(
                    self.document_fingerprint,
                    self.zoom_factor,
                    content_width,
                    block_count,
                );
                let render_outcome = render_markdown_document(
                    &mut document_ui,
                    &document,
                    self.language,
                    &theme,
                    self.zoom_factor,
                    document_base_dir,
                    &mut self.image_cache,
                    block_heights,
                    self.pending_block_scroll,
                    active_search_query(&self.search_query),
                    active_search_block,
                );

                if let (Some(measurement), Some(started)) = (render_measurement, render_started) {
                    metrics::log_document_render(
                        measurement.reason.as_log_label(),
                        &measurement.path,
                        started.elapsed(),
                        block_count,
                        heading_count,
                    );
                }

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

                if let Some(url) = render_outcome.clicked_external_link {
                    self.handle_external_link_click(ctx, url);
                }

                if render_outcome.needs_scroll_stabilization {
                    ctx.request_repaint();
                } else if render_outcome.did_scroll {
                    self.pending_block_scroll = None;
                }

                let used_content_rect = document_ui.min_rect();
                let fixed_content_rect = egui::Rect::from_min_size(
                    content_min,
                    Vec2::new(content_width, used_content_rect.height()),
                );
                let actual_frame_rect = document_frame.outer_rect(fixed_content_rect);
                ui.painter()
                    .set(background_shape, document_frame.paint(fixed_content_rect));
                ui.allocate_rect(actual_frame_rect, egui::Sense::hover());
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
        let viewport_rect = ctx.content_rect();
        let overlay_color = if theme.is_dark {
            egui::Color32::from_rgba_unmultiplied(8, 12, 18, 150)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 150)
        };

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop_markdown_overlay_background"),
        ));
        painter.rect_filled(viewport_rect, 0.0, overlay_color);

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

    fn active_search_block(&self) -> Option<usize> {
        self.active_search_index
            .and_then(|index| self.search_matches.get(index))
            .map(|search_match| search_match.block_index)
    }
}

impl eframe::App for OxideMdApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            STORAGE_KEY_LANGUAGE,
            language_storage_value(self.language).to_owned(),
        );
        storage.set_string(
            STORAGE_KEY_THEME,
            theme_id_storage_value(self.theme_id).to_owned(),
        );
        storage.set_string(STORAGE_KEY_ZOOM, self.zoom_factor.to_string());
        storage.set_string(
            STORAGE_KEY_EXTERNAL_LINKS,
            self.external_link_behavior.storage_value().to_owned(),
        );

        if let Some(path) = &self.current_file {
            storage.set_string(STORAGE_KEY_CURRENT_FILE, path.display().to_string());
        } else {
            storage.set_string(STORAGE_KEY_CURRENT_FILE, String::new());
        }
    }

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
        self.render_external_link_confirmation(ctx);
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

fn status_path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn active_search_query(search_query: &str) -> Option<&str> {
    let trimmed = search_query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn open_external_link(ctx: &egui::Context, url: String) {
    ctx.open_url(egui::OpenUrl::new_tab(url));
}

fn language_storage_value(language: Language) -> &'static str {
    match language {
        Language::En => "en",
        Language::Ja => "ja",
    }
}

fn language_from_storage_value(value: &str) -> Option<Language> {
    match value {
        "en" => Some(Language::En),
        "ja" => Some(Language::Ja),
        _ => None,
    }
}

fn theme_id_storage_value(theme_id: ThemeId) -> &'static str {
    match theme_id {
        ThemeId::WarmPaper => "warm_paper",
        ThemeId::Mist => "mist",
        ThemeId::NightOwl => "night_owl",
    }
}

fn theme_id_from_storage_value(value: &str) -> Option<ThemeId> {
    match value {
        "warm_paper" => Some(ThemeId::WarmPaper),
        "mist" => Some(ThemeId::Mist),
        "night_owl" => Some(ThemeId::NightOwl),
        _ => None,
    }
}

fn heading_nav_indent(level: pulldown_cmark::HeadingLevel) -> f32 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 0.0,
        pulldown_cmark::HeadingLevel::H2 => HEADING_NAV_ITEM_INDENT,
        pulldown_cmark::HeadingLevel::H3 => HEADING_NAV_ITEM_INDENT * 2.0,
        pulldown_cmark::HeadingLevel::H4 => HEADING_NAV_ITEM_INDENT * 3.0,
        pulldown_cmark::HeadingLevel::H5 => HEADING_NAV_ITEM_INDENT * 4.0,
        pulldown_cmark::HeadingLevel::H6 => HEADING_NAV_ITEM_INDENT * 5.0,
    }
}

fn capped_preview_window_width(target_width: f32, monitor_size: Option<Vec2>) -> f32 {
    monitor_size
        .map(|size| (size.x - PREVIEW_WINDOW_MONITOR_MARGIN).max(DOCUMENT_FRAME_MAX_WIDTH))
        .map(|max_width| target_width.min(max_width))
        .unwrap_or(target_width)
}
