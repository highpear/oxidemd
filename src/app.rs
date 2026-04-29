use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Vec2};
use rfd::FileDialog;

use crate::document_loader::{DocumentFingerprint, FileSnapshot, load_markdown_document};
use crate::document_session::DocumentSession;
use crate::export::write_html_export;
use crate::external_links::render_external_link_confirmation;
use crate::i18n::{Language, TranslationKey, tr};
use crate::metrics;
use crate::parser::MarkdownDocument;
use crate::reload_worker::{ReloadResponse, ReloadWorkerHandle, spawn_reload_worker};
use crate::session::{
    ExternalLinkBehavior, SessionSaveData, is_markdown_path, remember_recent_file,
    restore_session as restore_saved_session, save_session,
};
use crate::shortcuts::{consume_shortcuts, render_shortcuts_help};
use crate::theme::{DEFAULT_THEME_ID, ThemeId, apply_theme, available_themes, theme};

mod ui;

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

struct PendingRenderMeasurement {
    reason: RenderMeasurementReason,
    path: PathBuf,
}

impl RenderMeasurementReason {
    fn as_log_label(&self) -> &'static str {
        match self {
            RenderMeasurementReason::Load => "load",
            RenderMeasurementReason::Reload => "reload",
        }
    }
}

const DEFAULT_ZOOM_FACTOR: f32 = 1.0;
const MIN_ZOOM_FACTOR: f32 = 0.8;
const MAX_ZOOM_FACTOR: f32 = 1.8;
const ZOOM_STEP: f32 = 0.1;
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
const HOME_PANEL_MAX_WIDTH: f32 = 520.0;
const HOME_RECENT_FILE_LIMIT: usize = 6;
pub struct OxideMdApp {
    ui_context: egui::Context,
    language: Language,
    theme_id: ThemeId,
    zoom_factor: f32,
    document_session: Option<DocumentSession>,
    recent_files: Vec<PathBuf>,
    status_message: String,
    status_hover_message: Option<String>,
    reload_status: ReloadStatus,
    reload_worker: ReloadWorkerHandle,
    queued_reload_id: u64,
    is_heading_panel_visible: bool,
    show_shortcuts_help: bool,
    external_link_behavior: ExternalLinkBehavior,
    pending_external_link: Option<String>,
    pending_render_measurement: Option<PendingRenderMeasurement>,
    startup_started: Option<Instant>,
}

impl OxideMdApp {
    pub fn new(
        ui_context: egui::Context,
        storage: Option<&dyn eframe::Storage>,
        startup_started: Instant,
        initial_file: Option<PathBuf>,
        restore_file: bool,
        reset_session: bool,
    ) -> Self {
        let language = Language::En;
        debug_assert!(available_themes().contains(&DEFAULT_THEME_ID));

        let mut app = Self {
            reload_worker: spawn_reload_worker(ui_context.clone()),
            ui_context,
            language,
            theme_id: DEFAULT_THEME_ID,
            zoom_factor: DEFAULT_ZOOM_FACTOR,
            document_session: None,
            recent_files: Vec::new(),
            status_message: tr(language, TranslationKey::StatusNoFile).to_owned(),
            status_hover_message: None,
            reload_status: ReloadStatus::Idle,
            queued_reload_id: 0,
            is_heading_panel_visible: true,
            show_shortcuts_help: false,
            external_link_behavior: ExternalLinkBehavior::AskFirst,
            pending_external_link: None,
            pending_render_measurement: None,
            startup_started: Some(startup_started),
        };

        let restored_file = if reset_session {
            None
        } else {
            app.restore_session(storage, restore_file)
        };
        apply_theme(&app.ui_context, &theme(app.theme_id));

        if let Some(path) = initial_file.or(restored_file) {
            app.load_initial_file(path);
        }

        app
    }

    fn restore_session(
        &mut self,
        storage: Option<&dyn eframe::Storage>,
        restore_file: bool,
    ) -> Option<PathBuf> {
        let restored = restore_saved_session(storage, MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);

        if let Some(language) = restored.language {
            self.language = language;
        }

        if let Some(theme_id) = restored.theme_id {
            self.theme_id = theme_id;
        }

        if let Some(zoom_factor) = restored.zoom_factor {
            self.zoom_factor = zoom_factor;
        }

        if let Some(external_link_behavior) = restored.external_link_behavior {
            self.external_link_behavior = external_link_behavior;
        }

        if let Some(is_heading_panel_visible) = restored.is_heading_panel_visible {
            self.is_heading_panel_visible = is_heading_panel_visible;
        }

        if let Some(recent_files) = restored.recent_files {
            self.recent_files = recent_files;
        }

        if restore_file && let Some(path) = restored.unavailable_current_file {
            self.set_reload_error(
                TranslationKey::StatusLastFileUnavailable,
                path.display().to_string(),
            );
        }

        restore_file.then_some(restored.current_file).flatten()
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

        if self.document_session.is_none() {
            self.set_status_message(tr(self.language, TranslationKey::StatusNoFile));
        }
    }

    fn switch_theme(&mut self) {
        self.theme_id = self.theme_id.next();
    }

    fn select_theme(&mut self, theme_id: ThemeId) {
        self.theme_id = theme_id;
    }

    fn switch_external_link_behavior(&mut self) {
        self.external_link_behavior = self.external_link_behavior.next();
    }

    fn toggle_heading_panel(&mut self) {
        self.is_heading_panel_visible = !self.is_heading_panel_visible;
    }

    fn zoom_in(&mut self) {
        self.set_zoom_factor(self.zoom_factor + ZOOM_STEP);
    }

    fn zoom_out(&mut self) {
        self.set_zoom_factor(self.zoom_factor - ZOOM_STEP);
    }

    fn reset_zoom(&mut self) {
        self.set_zoom_factor(DEFAULT_ZOOM_FACTOR);
    }

    fn set_zoom_factor(&mut self, zoom_factor: f32) {
        self.zoom_factor = zoom_factor.clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);
    }

    fn handle_pointer_zoom(&mut self, ctx: &egui::Context) {
        let zoom_delta = ctx.input(|input| input.zoom_delta());

        if (zoom_delta - 1.0).abs() <= f32::EPSILON {
            return;
        }

        self.set_zoom_factor(self.zoom_factor * zoom_delta);
    }

    fn open_markdown_file(&mut self) {
        let selected_file = FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file();

        if let Some(path) = selected_file {
            self.load_selected_file(path);
        }
    }

    fn export_current_file_as_html(&mut self) {
        let Some(source_path) = self.current_file().map(Path::to_path_buf) else {
            self.set_status_message(tr(self.language, TranslationKey::StatusNoFile));
            return;
        };

        let default_name = export_file_name(&source_path);
        let Some(output_path) = FileDialog::new()
            .add_filter("HTML", &["html", "htm"])
            .set_file_name(&default_name)
            .save_file()
        else {
            return;
        };

        match write_html_export(&source_path, &output_path) {
            Ok(()) => {
                self.reload_status = ReloadStatus::Idle;
                self.set_status_with_path(TranslationKey::StatusExported, &output_path);
            }
            Err(error) => {
                self.set_reload_error(TranslationKey::StatusExportFailed, error);
            }
        }
    }

    fn copy_current_file_path(&mut self, ctx: &egui::Context) {
        let Some(path) = self.current_file().map(Path::to_path_buf) else {
            self.set_status_message(tr(self.language, TranslationKey::StatusNoFile));
            return;
        };

        ctx.copy_text(path.display().to_string());
        self.reload_status = ReloadStatus::Idle;
        self.set_status_with_path(TranslationKey::StatusPathCopied, &path);
    }

    fn open_recent_file(&mut self, path: PathBuf) {
        if path.is_file() && is_markdown_path(&path) {
            self.load_selected_file(path);
            return;
        }

        self.recent_files.retain(|recent_path| recent_path != &path);
        self.set_reload_error(
            TranslationKey::MessageRecentFileUnavailable,
            path.display().to_string(),
        );
    }

    fn clear_recent_files(&mut self) {
        self.recent_files.clear();
        self.reload_status = ReloadStatus::Idle;
        self.set_status_message(tr(self.language, TranslationKey::StatusRecentFilesCleared));
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

    fn load_selected_file(&mut self, path: PathBuf) {
        match load_markdown_document(&path) {
            Ok(loaded) => {
                remember_recent_file(&mut self.recent_files, &path);
                self.document_session = Some(DocumentSession::new(
                    path.clone(),
                    loaded.document,
                    loaded.fingerprint,
                    loaded.file_snapshot,
                ));
                self.reload_status = ReloadStatus::Idle;
                self.start_watching_file();
                self.pending_render_measurement = Some(PendingRenderMeasurement {
                    reason: RenderMeasurementReason::Load,
                    path: path.clone(),
                });
                self.request_window_expansion_for_preview();
                metrics::log_initial_load(&path, &loaded.timing);
                self.set_status_with_path(TranslationKey::StatusLoaded, &path);
            }
            Err(error) => {
                self.document_session = None;
                self.pending_render_measurement = None;
                self.set_reload_error(TranslationKey::StatusLoadFailed, error);
            }
        }
    }

    fn start_watching_file(&mut self) {
        if let Some(session) = self.document_session.as_mut() {
            if let Err(error) = session.start_watching(self.ui_context.clone()) {
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

        let target_width = HEADING_PANEL_MAX_WIDTH
            + scaled_document_frame_max_width(self.zoom_factor)
            + PREVIEW_WINDOW_SIDE_PADDING;
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
        let Some(session) = self.document_session.as_ref() else {
            return;
        };
        let summary = session.drain_watch_events();

        if summary.saw_change {
            self.schedule_reload();
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
        }

        if let Some(error) = summary.error {
            self.set_reload_error(TranslationKey::StatusWatchFailed, error);
        }
    }

    fn reload_if_ready(&mut self) {
        let Some(session) = self.document_session.as_ref() else {
            return;
        };
        if session.pending_reload_at.is_none() {
            return;
        };

        if !session.is_reload_due(Duration::from_millis(200)) {
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
            return;
        }

        if session.is_reload_in_flight() {
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
            return;
        }

        self.enqueue_reload();
    }

    fn reload_status_label(&self) -> &'static str {
        match self.reload_status {
            ReloadStatus::Idle => tr(self.language, TranslationKey::ReloadIdle),
            ReloadStatus::Reloading => tr(self.language, TranslationKey::ReloadReloading),
            ReloadStatus::Error => tr(self.language, TranslationKey::ReloadError),
        }
    }

    fn request_manual_reload(&mut self) {
        if self.current_file().is_none() {
            self.set_status_message(tr(self.language, TranslationKey::StatusNoFile));
            return;
        };

        if self
            .document_session
            .as_ref()
            .map(DocumentSession::is_reload_in_flight)
            .unwrap_or(false)
        {
            return;
        }

        self.enqueue_reload();
    }

    fn enqueue_reload(&mut self) {
        self.queued_reload_id += 1;
        let reload_id = self.queued_reload_id;

        if let Some(session) = self.document_session.as_mut() {
            session.clear_pending_reload();
        }

        let Some(request_data) = self
            .document_session
            .as_ref()
            .map(DocumentSession::reload_request_data)
        else {
            return;
        };

        match self.reload_worker.request_reload(
            reload_id,
            request_data.path.clone(),
            request_data.previous_fingerprint,
            request_data.previous_file_snapshot,
        ) {
            Ok(()) => {
                if let Some(session) = self.document_session.as_mut() {
                    session.start_reload(reload_id);
                }
                self.set_reload_in_progress(
                    TranslationKey::StatusReloadStarted,
                    Some(&request_data.path),
                );
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
                    if !self.is_current_reload(id) {
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
                    if !self.is_current_reload(id) {
                        continue;
                    }

                    self.finish_reload_unchanged(path, timing, fingerprint, file_snapshot);
                }
                ReloadResponse::Error { id, path, error } => {
                    if !self.is_current_reload(id) {
                        continue;
                    }

                    self.finish_reload_error(path, error);
                }
            }
        }
    }

    fn schedule_reload(&mut self) {
        if let Some(session) = self.document_session.as_mut() {
            session.schedule_reload();
        }
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
        if let Some(session) = self.document_session.as_mut() {
            session.replace_reloaded_document(path.clone(), document, fingerprint, file_snapshot);
        } else {
            self.document_session = Some(DocumentSession::new(
                path.clone(),
                document,
                fingerprint,
                file_snapshot,
            ));
        }
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
        if let Some(session) = self.document_session.as_mut() {
            session.finish_unchanged_reload(fingerprint, file_snapshot);
        }
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload_skipped(&path, &timing);
        self.set_status_with_path(TranslationKey::StatusReloadSkipped, &path);
    }

    fn finish_reload_error(&mut self, path: PathBuf, error: String) {
        if let Some(session) = self.document_session.as_mut() {
            session.finish_reload();
        }
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

    fn is_current_reload(&self, id: u64) -> bool {
        self.document_session
            .as_ref()
            .map(|session| session.is_current_reload(id))
            .unwrap_or(false)
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let has_search_matches = self
            .document_session
            .as_ref()
            .map(|session| session.search.has_matches())
            .unwrap_or(false);
        let shortcuts = consume_shortcuts(ctx, has_search_matches);

        if shortcuts.open_file {
            self.open_markdown_file();
        }

        if shortcuts.focus_search {
            self.is_heading_panel_visible = true;
            if let Some(session) = self.document_session.as_mut() {
                session.search.focus_input = true;
            }
        }

        if shortcuts.show_shortcuts_help {
            self.show_shortcuts_help = true;
        }

        if shortcuts.reload_file {
            self.request_manual_reload();
        }

        if shortcuts.previous_search {
            self.select_previous_search_match();
        }

        if shortcuts.next_search {
            self.select_next_search_match();
        }

        if shortcuts.switch_language {
            self.switch_language();
        }

        if shortcuts.switch_theme {
            self.switch_theme();
        }

        if shortcuts.zoom_in {
            self.zoom_in();
        }

        if shortcuts.zoom_out {
            self.zoom_out();
        }

        if shortcuts.reset_zoom {
            self.reset_zoom();
        }
    }

    fn select_next_search_match(&mut self) {
        if let Some(session) = self.document_session.as_mut() {
            session.select_next_search_match();
        }
    }

    fn select_previous_search_match(&mut self) {
        if let Some(session) = self.document_session.as_mut() {
            session.select_previous_search_match();
        }
    }

    fn clear_selected_heading_on_manual_scroll(&mut self, ctx: &egui::Context) {
        let (scroll_delta_y, is_zoom_scroll) = ctx.input(|input| {
            (
                input.raw_scroll_delta.y,
                input.modifiers.matches_any(egui::Modifiers::COMMAND),
            )
        });

        if scroll_delta_y.abs() > f32::EPSILON && !is_zoom_scroll {
            if let Some(session) = self.document_session.as_mut() {
                session.clear_selected_heading();
            }
        }
    }

    fn current_file(&self) -> Option<&Path> {
        self.document_session
            .as_ref()
            .map(|session| session.path.as_path())
    }
}

impl eframe::App for OxideMdApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        save_session(
            storage,
            SessionSaveData {
                language: self.language,
                theme_id: self.theme_id,
                zoom_factor: self.zoom_factor,
                external_link_behavior: self.external_link_behavior,
                is_heading_panel_visible: self.is_heading_panel_visible,
                current_file: self.current_file(),
                recent_files: &self.recent_files,
            },
        );
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(startup_started) = self.startup_started.take() {
            metrics::log_startup(startup_started.elapsed());
        }

        let previous_zoom_factor = self.zoom_factor;

        self.handle_keyboard_shortcuts(ctx);
        self.handle_pointer_zoom(ctx);
        self.handle_file_drops(ctx);
        self.clear_selected_heading_on_manual_scroll(ctx);
        self.process_watch_events();
        self.process_reload_results();
        self.reload_if_ready();

        let theme = theme(self.theme_id);
        apply_theme(ctx, &theme);
        self.render_top_bar(ctx);
        self.render_bottom_bar(ctx);
        if self.document_session.is_some()
            && self.zoom_factor.to_bits() != previous_zoom_factor.to_bits()
        {
            self.request_window_expansion_for_preview();
        }
        self.render_heading_panel(ctx);
        self.render_document_panel(ctx);
        render_external_link_confirmation(ctx, self.language, &mut self.pending_external_link);
        render_shortcuts_help(ctx, self.language, &mut self.show_shortcuts_help);
        self.render_drop_overlay(ctx);
    }
}

fn status_path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn home_recent_file_label(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string());
    let Some(parent) = path.parent().and_then(|parent| parent.to_str()) else {
        return file_name;
    };

    format!("{file_name}  {parent}")
}

fn export_file_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("export");

    format!("{}.html", stem)
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

fn scaled_document_frame_max_width(zoom_factor: f32) -> f32 {
    DOCUMENT_FRAME_MAX_WIDTH * zoom_factor
}

fn scaled_document_body_max_width(zoom_factor: f32) -> f32 {
    DOCUMENT_BODY_MAX_WIDTH * zoom_factor
}

fn scaled_document_horizontal_padding(zoom_factor: f32) -> f32 {
    DOCUMENT_HORIZONTAL_PADDING * zoom_factor
}

fn scaled_document_vertical_padding(zoom_factor: f32) -> f32 {
    DOCUMENT_VERTICAL_PADDING * zoom_factor
}

fn scaled_margin(value: i8, zoom_factor: f32) -> i8 {
    ((value as f32) * zoom_factor)
        .round()
        .clamp(0.0, i8::MAX as f32) as i8
}
