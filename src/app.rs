use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui::{self, Vec2};
use rfd::FileDialog;

use crate::document_loader::load_markdown_document;
use crate::document_session::{DocumentSession, RenderMeasurementReason};
use crate::document_workspace::{DocumentId, DocumentWorkspace};
use crate::export::write_html_export;
use crate::external_links::render_external_link_confirmation;
use crate::i18n::{Language, TranslationKey, tr};
use crate::math::prewarm_math_renderer;
use crate::metrics;
use crate::reload_worker::{ReloadWorkerHandle, spawn_reload_worker};
use crate::session::{
    ExternalLinkBehavior, SessionSaveData, is_markdown_path, remember_recent_file,
    restore_session as restore_saved_session, save_session,
};
use crate::shortcuts::{consume_shortcuts, render_shortcuts_help};
use crate::theme::{DEFAULT_THEME_ID, ThemeId, apply_theme, available_themes, theme};

mod reload;
mod ui;

#[derive(Clone, Copy)]
enum ReloadStatus {
    Idle,
    Reloading,
    Error,
}

struct LoadedDocumentSession {
    session: DocumentSession,
    timing: metrics::DocumentTiming,
}

struct RestoredOpenFiles {
    files: Vec<PathBuf>,
    active_file: Option<PathBuf>,
}

struct AppSettings {
    language: Language,
    theme_id: ThemeId,
    zoom_factor: f32,
    is_heading_panel_visible: bool,
    external_link_behavior: ExternalLinkBehavior,
}

struct StatusState {
    message: String,
    hover_message: Option<String>,
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

impl AppSettings {
    fn new() -> Self {
        Self {
            language: Language::En,
            theme_id: DEFAULT_THEME_ID,
            zoom_factor: DEFAULT_ZOOM_FACTOR,
            is_heading_panel_visible: true,
            external_link_behavior: ExternalLinkBehavior::AskFirst,
        }
    }

    fn switch_language(&mut self) {
        self.language = match self.language {
            Language::En => Language::Ja,
            Language::Ja => Language::En,
        };
    }

    fn switch_theme(&mut self) {
        self.theme_id = self.theme_id.next();
    }

    fn switch_external_link_behavior(&mut self) {
        self.external_link_behavior = self.external_link_behavior.next();
    }

    fn set_zoom_factor(&mut self, zoom_factor: f32) {
        self.zoom_factor = zoom_factor.clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);
    }
}

impl StatusState {
    fn new(language: Language) -> Self {
        Self {
            message: tr(language, TranslationKey::StatusNoFile).to_owned(),
            hover_message: None,
        }
    }

    fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.hover_message = None;
    }

    fn set_with_hover(&mut self, message: String, hover_message: String) {
        self.message = message;
        self.hover_message = Some(hover_message);
    }
}

pub struct OxideMdApp {
    ui_context: egui::Context,
    settings: AppSettings,
    documents: DocumentWorkspace,
    recent_files: Vec<PathBuf>,
    status: StatusState,
    reload_status: ReloadStatus,
    reload_worker: ReloadWorkerHandle,
    queued_reload_id: u64,
    show_shortcuts_help: bool,
    pending_external_link: Option<String>,
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
        debug_assert!(available_themes().contains(&DEFAULT_THEME_ID));
        let settings = AppSettings::new();
        let status = StatusState::new(settings.language);

        let mut app = Self {
            reload_worker: spawn_reload_worker(ui_context.clone()),
            ui_context,
            settings,
            documents: DocumentWorkspace::new(),
            recent_files: Vec::new(),
            status,
            reload_status: ReloadStatus::Idle,
            queued_reload_id: 0,
            show_shortcuts_help: false,
            pending_external_link: None,
            startup_started: Some(startup_started),
        };

        prewarm_math_renderer(app.ui_context.clone());

        let restored_files = if reset_session {
            None
        } else {
            app.restore_session(storage, restore_file)
        };
        apply_theme(&app.ui_context, &theme(app.settings.theme_id));

        if let Some(path) = initial_file {
            app.load_initial_file(path);
        } else if let Some(restored_files) = restored_files {
            app.load_restored_files(restored_files);
        }

        app
    }

    fn restore_session(
        &mut self,
        storage: Option<&dyn eframe::Storage>,
        restore_file: bool,
    ) -> Option<RestoredOpenFiles> {
        let restored = restore_saved_session(storage, MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR);

        if let Some(language) = restored.language {
            self.settings.language = language;
        }

        if let Some(theme_id) = restored.theme_id {
            self.settings.theme_id = theme_id;
        }

        if let Some(zoom_factor) = restored.zoom_factor {
            self.settings.zoom_factor = zoom_factor;
        }

        if let Some(external_link_behavior) = restored.external_link_behavior {
            self.settings.external_link_behavior = external_link_behavior;
        }

        if let Some(is_heading_panel_visible) = restored.is_heading_panel_visible {
            self.settings.is_heading_panel_visible = is_heading_panel_visible;
        }

        if let Some(recent_files) = restored.recent_files {
            self.recent_files = recent_files;
        }

        if restore_file
            && let Some(path) = restored
                .unavailable_open_files
                .first()
                .or(restored.unavailable_current_file.as_ref())
        {
            self.set_reload_error(
                TranslationKey::StatusLastFileUnavailable,
                path.display().to_string(),
            );
        }

        restore_file
            .then_some(RestoredOpenFiles {
                files: restored.open_files,
                active_file: restored.active_file,
            })
            .filter(|restored_files| !restored_files.files.is_empty())
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

    fn load_restored_files(&mut self, restored_files: RestoredOpenFiles) {
        let active_file = restored_files.active_file;

        for path in restored_files.files {
            self.load_file_as_tab(path, false);
        }

        if let Some(active_file) = active_file
            && let Some(document_id) = self.documents.document_id_for_path(&active_file)
        {
            self.switch_to_document(document_id);
        } else {
            self.documents.clear_active_document();
        }
    }

    fn switch_language(&mut self) {
        self.settings.switch_language();

        if self.documents.is_empty() {
            self.set_status_message(tr(self.settings.language, TranslationKey::StatusNoFile));
        }
    }

    fn switch_theme(&mut self) {
        self.settings.switch_theme();
    }

    fn select_theme(&mut self, theme_id: ThemeId) {
        self.settings.theme_id = theme_id;
    }

    fn switch_external_link_behavior(&mut self) {
        self.settings.switch_external_link_behavior();
    }

    fn toggle_heading_panel(&mut self) {
        self.settings.is_heading_panel_visible = !self.settings.is_heading_panel_visible;
    }

    fn zoom_in(&mut self) {
        self.set_zoom_factor(self.settings.zoom_factor + ZOOM_STEP);
    }

    fn zoom_out(&mut self) {
        self.set_zoom_factor(self.settings.zoom_factor - ZOOM_STEP);
    }

    fn reset_zoom(&mut self) {
        self.set_zoom_factor(DEFAULT_ZOOM_FACTOR);
    }

    fn set_zoom_factor(&mut self, zoom_factor: f32) {
        self.settings.set_zoom_factor(zoom_factor);
    }

    fn handle_pointer_zoom(&mut self, ctx: &egui::Context) {
        let zoom_delta = ctx.input(|input| input.zoom_delta());

        if (zoom_delta - 1.0).abs() <= f32::EPSILON {
            return;
        }

        self.set_zoom_factor(self.settings.zoom_factor * zoom_delta);
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
            self.set_status_message(tr(self.settings.language, TranslationKey::StatusNoFile));
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
            self.set_status_message(tr(self.settings.language, TranslationKey::StatusNoFile));
            return;
        };

        self.copy_file_path(ctx, &path);
    }

    fn copy_file_path(&mut self, ctx: &egui::Context, path: &Path) {
        ctx.copy_text(path.display().to_string());
        self.reload_status = ReloadStatus::Idle;
        self.set_status_with_path(TranslationKey::StatusPathCopied, path);
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
        self.set_status_message(tr(
            self.settings.language,
            TranslationKey::StatusRecentFilesCleared,
        ));
    }

    fn show_home_tab(&mut self) {
        self.documents.clear_active_document();
        self.reload_status = ReloadStatus::Idle;
        self.set_status_message(tr(self.settings.language, TranslationKey::StatusNoFile));
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

        let markdown_paths = dropped_paths
            .iter()
            .filter(|path| is_markdown_path(path))
            .cloned()
            .collect::<Vec<_>>();

        if !markdown_paths.is_empty() {
            for path in markdown_paths {
                self.load_selected_file(path);
            }
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
        self.load_file_as_tab(path, true);
    }

    fn load_file_as_tab(&mut self, path: PathBuf, update_recent_files: bool) {
        let path = user_visible_file_path(path);

        if let Some(document_id) = self.documents.document_id_for_path(&path) {
            self.documents.switch_to(document_id);
            if update_recent_files {
                remember_recent_file(&mut self.recent_files, &path);
            }
            self.reload_status = ReloadStatus::Idle;
            self.request_window_expansion_for_preview();
            self.set_status_with_path(TranslationKey::StatusLoaded, &path);
            return;
        }

        match self.load_document_session(&path) {
            Ok(loaded) => {
                if update_recent_files {
                    remember_recent_file(&mut self.recent_files, &path);
                }
                self.documents.open_document(loaded.session);
                self.reload_status = ReloadStatus::Idle;
                self.start_watching_file();
                if let Some(session) = self.documents.active_session_mut() {
                    session.request_render_measurement(RenderMeasurementReason::Load, path.clone());
                }
                self.request_window_expansion_for_preview();
                metrics::log_initial_load(&path, &loaded.timing);
                self.set_status_with_path(TranslationKey::StatusLoaded, &path);
            }
            Err(error) => {
                self.set_reload_error(TranslationKey::StatusLoadFailed, error);
            }
        }
    }

    fn load_document_session(&self, path: &Path) -> Result<LoadedDocumentSession, String> {
        let loaded = load_markdown_document(path)?;
        let session = DocumentSession::new(
            path.to_path_buf(),
            loaded.document,
            loaded.fingerprint,
            loaded.file_snapshot,
        );

        Ok(LoadedDocumentSession {
            session,
            timing: loaded.timing,
        })
    }

    fn start_watching_file(&mut self) {
        if let Some(session) = self.documents.active_session_mut() {
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
            + scaled_document_frame_max_width(self.settings.zoom_factor)
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

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let has_search_matches = self
            .documents
            .active_session()
            .map(|session| session.search.has_matches())
            .unwrap_or(false);
        let shortcuts = consume_shortcuts(ctx, has_search_matches);

        if shortcuts.open_file {
            self.open_markdown_file();
        }

        if shortcuts.focus_search {
            self.settings.is_heading_panel_visible = true;
            if let Some(session) = self.documents.active_session_mut() {
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
        if let Some(session) = self.documents.active_session_mut() {
            session.select_next_search_match();
        }
    }

    fn select_previous_search_match(&mut self) {
        if let Some(session) = self.documents.active_session_mut() {
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
            if let Some(session) = self.documents.active_session_mut() {
                session.clear_selected_heading();
            }
        }
    }

    fn current_file(&self) -> Option<&Path> {
        self.documents.current_file()
    }

    fn switch_to_document(&mut self, document_id: DocumentId) {
        if !self.documents.switch_to(document_id) {
            return;
        }

        self.reload_status = ReloadStatus::Idle;
        if let Some(path) = self.current_file().map(Path::to_path_buf) {
            self.set_status_with_path(TranslationKey::StatusLoaded, &path);
        }
    }

    fn close_document(&mut self, document_id: DocumentId) {
        if self.documents.close(document_id).is_none() {
            return;
        }

        self.update_status_after_tab_change();
    }

    fn close_other_documents(&mut self, document_id: DocumentId) {
        if !self.documents.close_other_documents(document_id) {
            return;
        }

        self.update_status_after_tab_change();
    }

    fn close_documents_to_right(&mut self, document_id: DocumentId) {
        if !self.documents.close_documents_to_right(document_id) {
            return;
        }

        self.update_status_after_tab_change();
    }

    fn update_status_after_tab_change(&mut self) {
        self.reload_status = ReloadStatus::Idle;
        if let Some(path) = self.current_file().map(Path::to_path_buf) {
            self.set_status_with_path(TranslationKey::StatusLoaded, &path);
        } else {
            self.set_status_message(tr(self.settings.language, TranslationKey::StatusNoFile));
        }
    }
}

impl eframe::App for OxideMdApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        save_session(
            storage,
            SessionSaveData {
                language: self.settings.language,
                theme_id: self.settings.theme_id,
                zoom_factor: self.settings.zoom_factor,
                external_link_behavior: self.settings.external_link_behavior,
                is_heading_panel_visible: self.settings.is_heading_panel_visible,
                current_file: self.current_file(),
                open_files: self.documents.open_files(),
                recent_files: &self.recent_files,
            },
        );
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(startup_started) = self.startup_started.take() {
            metrics::log_startup(startup_started.elapsed());
        }

        let previous_zoom_factor = self.settings.zoom_factor;

        self.handle_keyboard_shortcuts(ctx);
        self.handle_pointer_zoom(ctx);
        self.handle_file_drops(ctx);
        self.clear_selected_heading_on_manual_scroll(ctx);
        self.process_watch_events();
        self.process_reload_results();
        self.reload_if_ready();

        let theme = theme(self.settings.theme_id);
        apply_theme(ctx, &theme);
        self.render_top_bar(ctx);
        self.render_bottom_bar(ctx);
        if !self.documents.is_empty()
            && self.settings.zoom_factor.to_bits() != previous_zoom_factor.to_bits()
        {
            self.request_window_expansion_for_preview();
        }
        self.render_heading_panel(ctx);
        self.render_document_panel(ctx);
        render_external_link_confirmation(
            ctx,
            self.settings.language,
            &mut self.pending_external_link,
        );
        render_shortcuts_help(ctx, self.settings.language, &mut self.show_shortcuts_help);
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

fn user_visible_file_path(path: PathBuf) -> PathBuf {
    let path_text = path.as_os_str().to_string_lossy();

    if let Some(stripped) = path_text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{stripped}"));
    }

    if let Some(stripped) = path_text.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }

    path
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::user_visible_file_path;

    #[test]
    fn user_visible_file_path_removes_windows_verbatim_disk_prefix() {
        let path = user_visible_file_path(PathBuf::from(r"\\?\C:\Users\example\doc.md"));

        assert_eq!(path, PathBuf::from(r"C:\Users\example\doc.md"));
    }

    #[test]
    fn user_visible_file_path_removes_windows_verbatim_unc_prefix() {
        let path = user_visible_file_path(PathBuf::from(r"\\?\UNC\server\share\doc.md"));

        assert_eq!(path, PathBuf::from(r"\\server\share\doc.md"));
    }
}
