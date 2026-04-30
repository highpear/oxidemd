use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use crate::document_loader::{
    DocumentFingerprint, FileSnapshot, ReloadDocumentOutcome, reload_markdown_document,
};
use crate::document_workspace::DocumentId;
use crate::metrics::DocumentTiming;
use crate::parser::MarkdownDocument;

enum ReloadRequest {
    Reload {
        document_id: DocumentId,
        id: u64,
        path: PathBuf,
        previous_fingerprint: Option<DocumentFingerprint>,
        previous_file_snapshot: Option<FileSnapshot>,
    },
}

pub enum ReloadResponse {
    Reloaded {
        document_id: DocumentId,
        id: u64,
        path: PathBuf,
        document: Arc<MarkdownDocument>,
        timing: DocumentTiming,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    },
    Unchanged {
        document_id: DocumentId,
        id: u64,
        path: PathBuf,
        timing: DocumentTiming,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    },
    Error {
        document_id: DocumentId,
        id: u64,
        path: PathBuf,
        error: String,
    },
}

pub struct ReloadWorkerHandle {
    sender: Sender<ReloadRequest>,
    pub receiver: Receiver<ReloadResponse>,
    _thread: thread::JoinHandle<()>,
}

pub fn spawn_reload_worker(ctx: egui::Context) -> ReloadWorkerHandle {
    let (request_sender, request_receiver) = mpsc::channel();
    let (response_sender, response_receiver) = mpsc::channel();

    let worker_thread = thread::spawn(move || {
        while let Ok(request) = request_receiver.recv() {
            match request {
                ReloadRequest::Reload {
                    document_id,
                    id,
                    path,
                    previous_fingerprint,
                    previous_file_snapshot,
                } => {
                    let response = match reload_markdown_document(
                        &path,
                        previous_fingerprint,
                        previous_file_snapshot,
                    ) {
                        Ok(ReloadDocumentOutcome::Reloaded(loaded)) => ReloadResponse::Reloaded {
                            document_id,
                            id,
                            path,
                            document: loaded.document,
                            timing: loaded.timing,
                            fingerprint: loaded.fingerprint,
                            file_snapshot: loaded.file_snapshot,
                        },
                        Ok(ReloadDocumentOutcome::Unchanged {
                            fingerprint,
                            file_snapshot,
                            timing,
                        }) => ReloadResponse::Unchanged {
                            document_id,
                            id,
                            path,
                            timing,
                            fingerprint,
                            file_snapshot,
                        },
                        Err(error) => ReloadResponse::Error {
                            document_id,
                            id,
                            path,
                            error: error.to_string(),
                        },
                    };

                    if response_sender.send(response).is_err() {
                        break;
                    }

                    ctx.request_repaint();
                }
            }
        }
    });

    ReloadWorkerHandle {
        sender: request_sender,
        receiver: response_receiver,
        _thread: worker_thread,
    }
}

impl ReloadWorkerHandle {
    pub fn request_reload(
        &self,
        document_id: DocumentId,
        id: u64,
        path: PathBuf,
        previous_fingerprint: Option<DocumentFingerprint>,
        previous_file_snapshot: Option<FileSnapshot>,
    ) -> Result<(), String> {
        self.sender
            .send(ReloadRequest::Reload {
                document_id,
                id,
                path,
                previous_fingerprint,
                previous_file_snapshot,
            })
            .map_err(|error| error.to_string())
    }
}
