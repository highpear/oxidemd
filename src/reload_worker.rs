use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use crate::document_loader::{
    DocumentFingerprint, ReloadDocumentOutcome, reload_markdown_document,
};
use crate::metrics::DocumentTiming;
use crate::parser::MarkdownDocument;

enum ReloadRequest {
    Reload {
        id: u64,
        path: PathBuf,
        previous_fingerprint: Option<DocumentFingerprint>,
    },
}

pub enum ReloadResponse {
    Reloaded {
        id: u64,
        path: PathBuf,
        document: MarkdownDocument,
        timing: DocumentTiming,
        fingerprint: DocumentFingerprint,
    },
    Unchanged {
        id: u64,
        path: PathBuf,
        timing: DocumentTiming,
        fingerprint: DocumentFingerprint,
    },
    Error {
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
                    id,
                    path,
                    previous_fingerprint,
                } => {
                    let response = match reload_markdown_document(&path, previous_fingerprint) {
                        Ok(ReloadDocumentOutcome::Reloaded(loaded)) => ReloadResponse::Reloaded {
                            id,
                            path,
                            document: loaded.document,
                            timing: loaded.timing,
                            fingerprint: loaded.fingerprint,
                        },
                        Ok(ReloadDocumentOutcome::Unchanged {
                            fingerprint,
                            timing,
                        }) => ReloadResponse::Unchanged {
                            id,
                            path,
                            timing,
                            fingerprint,
                        },
                        Err(error) => ReloadResponse::Error {
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
        id: u64,
        path: PathBuf,
        previous_fingerprint: Option<DocumentFingerprint>,
    ) -> Result<(), String> {
        self.sender
            .send(ReloadRequest::Reload {
                id,
                path,
                previous_fingerprint,
            })
            .map_err(|error| error.to_string())
    }
}
