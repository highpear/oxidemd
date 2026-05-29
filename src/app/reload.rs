use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::document_loader::{DocumentFingerprint, FileSnapshot};
use crate::document_session::{DocumentSession, RenderMeasurementReason, WatchEventSummary};
use crate::document_workspace::DocumentId;
use crate::i18n::{TranslationKey, tr};
use crate::metrics;
use crate::parser::MarkdownDocument;
use crate::reload_worker::ReloadResponse;

use super::{OxideMdApp, ReloadStatus, status_path_label};

impl OxideMdApp {
    pub(in crate::app) fn process_watch_events(&mut self) {
        let mut saw_change = false;
        let mut errors = Vec::new();

        for document_id in self.documents.document_ids() {
            let summary = self
                .documents
                .session_mut(document_id)
                .map(|session| {
                    let summary = session.drain_watch_events();
                    if summary.saw_change {
                        session.schedule_reload();
                    }
                    summary
                })
                .unwrap_or_else(empty_watch_event_summary);

            if summary.saw_change {
                saw_change = true;
            }

            if let Some(error) = summary.error {
                errors.push(error);
            }
        }

        if saw_change {
            self.set_reload_in_progress(TranslationKey::ReloadReloading, None);
            self.ui_context
                .request_repaint_after(Duration::from_millis(100));
        }

        for error in errors {
            self.set_reload_error(TranslationKey::StatusWatchFailed, error);
        }
    }

    pub(in crate::app) fn reload_if_ready(&mut self) {
        for document_id in self.documents.document_ids() {
            let Some(session) = self.documents.session(document_id) else {
                continue;
            };
            if session.pending_reload_at.is_none() {
                continue;
            };

            if !session.is_reload_due(Duration::from_millis(200)) {
                self.ui_context
                    .request_repaint_after(Duration::from_millis(100));
                continue;
            }

            if session.is_reload_in_flight() {
                self.ui_context
                    .request_repaint_after(Duration::from_millis(100));
                continue;
            }

            self.enqueue_reload(document_id);
        }
    }

    pub(in crate::app) fn reload_status_label(&self) -> &'static str {
        match self.reload_status {
            ReloadStatus::Idle => tr(self.settings.language, TranslationKey::ReloadIdle),
            ReloadStatus::Reloading => tr(self.settings.language, TranslationKey::ReloadReloading),
            ReloadStatus::Error => tr(self.settings.language, TranslationKey::ReloadError),
        }
    }

    pub(in crate::app) fn request_manual_reload(&mut self) {
        if self.current_file().is_none() {
            self.set_status_message(tr(self.settings.language, TranslationKey::StatusNoFile));
            return;
        };

        if self
            .documents
            .active_session()
            .map(DocumentSession::is_reload_in_flight)
            .unwrap_or(false)
        {
            return;
        }

        let Some(document_id) = self.documents.active_document_id() else {
            return;
        };

        self.enqueue_reload(document_id);
    }

    pub(in crate::app) fn process_reload_results(&mut self) {
        while let Ok(result) = self.reload_worker.receiver.try_recv() {
            match result {
                ReloadResponse::Reloaded {
                    document_id,
                    id,
                    path,
                    document,
                    timing,
                    fingerprint,
                    file_snapshot,
                } => {
                    if !self.is_current_reload(document_id, id) {
                        continue;
                    }

                    self.finish_reload_success(
                        document_id,
                        path,
                        document,
                        timing,
                        fingerprint,
                        file_snapshot,
                    );
                }
                ReloadResponse::Unchanged {
                    document_id,
                    id,
                    path,
                    timing,
                    fingerprint,
                    file_snapshot,
                } => {
                    if !self.is_current_reload(document_id, id) {
                        continue;
                    }

                    self.finish_reload_unchanged(
                        document_id,
                        path,
                        timing,
                        fingerprint,
                        file_snapshot,
                    );
                }
                ReloadResponse::Error {
                    document_id,
                    id,
                    path,
                    error,
                } => {
                    if !self.is_current_reload(document_id, id) {
                        continue;
                    }

                    self.finish_reload_error(document_id, path, error);
                }
            }
        }
    }

    pub(in crate::app) fn set_reload_in_progress(
        &mut self,
        key: TranslationKey,
        path: Option<&Path>,
    ) {
        self.reload_status = ReloadStatus::Reloading;
        match path {
            Some(path) => self.set_status_with_path(key, path),
            None => self.set_status_message(tr(self.settings.language, key)),
        };
    }

    pub(in crate::app) fn set_reload_error(&mut self, key: TranslationKey, error: String) {
        self.reload_status = ReloadStatus::Error;
        self.set_status_message(format!("{} {}", tr(self.settings.language, key), error));
    }

    pub(in crate::app) fn set_status_message(&mut self, message: impl Into<String>) {
        self.status.set_message(message);
    }

    pub(in crate::app) fn set_status_with_path(&mut self, key: TranslationKey, path: &Path) {
        self.status.set_with_hover(
            format!(
                "{} {}",
                tr(self.settings.language, key),
                status_path_label(path)
            ),
            format!("{} {}", tr(self.settings.language, key), path.display()),
        );
    }

    fn enqueue_reload(&mut self, document_id: DocumentId) {
        self.queued_reload_id += 1;
        let reload_id = self.queued_reload_id;
        let Some(request_data) = self.documents.session_mut(document_id).map(|session| {
            session.clear_pending_reload();
            session.reload_request_data()
        }) else {
            return;
        };

        match self.reload_worker.request_reload(
            document_id,
            reload_id,
            request_data.path.clone(),
            request_data.previous_fingerprint,
            request_data.previous_file_snapshot,
        ) {
            Ok(()) => {
                if let Some(session) = self.documents.session_mut(document_id) {
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

    fn finish_reload_success(
        &mut self,
        document_id: DocumentId,
        path: PathBuf,
        document: Arc<MarkdownDocument>,
        timing: metrics::DocumentTiming,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) {
        if let Some(session) = self.documents.active_session_mut_for_id(document_id) {
            session.replace_reloaded_document(path.clone(), document, fingerprint, file_snapshot);
            session.request_render_measurement(RenderMeasurementReason::Reload, path.clone());
        } else {
            return;
        }
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload(&path, &timing);
        self.set_status_with_path(TranslationKey::StatusReloaded, &path);
    }

    fn finish_reload_unchanged(
        &mut self,
        document_id: DocumentId,
        path: PathBuf,
        timing: metrics::DocumentTiming,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) {
        if let Some(session) = self.documents.active_session_mut_for_id(document_id) {
            session.finish_unchanged_reload(fingerprint, file_snapshot);
        }
        self.reload_status = ReloadStatus::Idle;
        metrics::log_reload_skipped(&path, &timing);
        self.set_status_with_path(TranslationKey::StatusReloadSkipped, &path);
    }

    fn finish_reload_error(&mut self, document_id: DocumentId, path: PathBuf, error: String) {
        if let Some(session) = self.documents.active_session_mut_for_id(document_id) {
            session.finish_reload();
        }
        self.reload_status = ReloadStatus::Error;
        let display_path = status_path_label(&path);
        self.status.set_with_hover(
            format!(
                "{} {} ({})",
                tr(self.settings.language, TranslationKey::StatusReloadFailed),
                display_path,
                error
            ),
            format!(
                "{} {} ({})",
                tr(self.settings.language, TranslationKey::StatusReloadFailed),
                path.display(),
                error
            ),
        );
    }

    fn is_current_reload(&self, document_id: DocumentId, id: u64) -> bool {
        self.documents
            .session(document_id)
            .map(|session| session.is_current_reload(id))
            .unwrap_or(false)
    }
}

fn empty_watch_event_summary() -> WatchEventSummary {
    WatchEventSummary {
        saw_change: false,
        error: None,
    }
}
