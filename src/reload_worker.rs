use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use crate::parser::{MarkdownDocument, parse_markdown};

enum ReloadRequest {
    Reload { id: u64, path: PathBuf },
}

pub enum ReloadResponse {
    Reloaded {
        id: u64,
        path: PathBuf,
        document: MarkdownDocument,
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
                ReloadRequest::Reload { id, path } => {
                    let response = match fs::read_to_string(&path) {
                        Ok(content) => ReloadResponse::Reloaded {
                            id,
                            path,
                            document: parse_markdown(&content),
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
    pub fn request_reload(&self, id: u64, path: PathBuf) -> Result<(), String> {
        self.sender
            .send(ReloadRequest::Reload { id, path })
            .map_err(|error| error.to_string())
    }
}
