use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub enum FileWatchEvent {
    Changed,
    Error(String),
}

pub struct FileWatcherHandle {
    _watcher: RecommendedWatcher,
    pub receiver: Receiver<FileWatchEvent>,
}

pub fn watch_file(path: &Path, ctx: egui::Context) -> Result<FileWatcherHandle, String> {
    let watched_file = path.to_path_buf();
    let watch_root = watched_file
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Missing parent directory".to_owned())?;

    let (sender, receiver) = mpsc::channel();
    let callback_context = ctx.clone();
    let callback_file = watched_file.clone();

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<Event>| match result {
            Ok(event) => {
                if should_reload(&event, &callback_file) {
                    let _ = sender.send(FileWatchEvent::Changed);
                    callback_context.request_repaint();
                }
            }
            Err(error) => {
                let _ = sender.send(FileWatchEvent::Error(error.to_string()));
                callback_context.request_repaint();
            }
        })
        .map_err(|error| error.to_string())?;

    watcher
        .configure(Config::default())
        .map_err(|error| error.to_string())?;
    watcher
        .watch(&watch_root, RecursiveMode::NonRecursive)
        .map_err(|error| error.to_string())?;

    Ok(FileWatcherHandle {
        _watcher: watcher,
        receiver,
    })
}

fn should_reload(event: &Event, watched_file: &Path) -> bool {
    if !matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    ) {
        return false;
    }

    event
        .paths
        .iter()
        .any(|path| matches_watched_file(path, watched_file))
}

fn matches_watched_file(path: &Path, watched_file: &Path) -> bool {
    if path == watched_file {
        return true;
    }

    path.parent() == watched_file.parent() && path.file_name() == watched_file.file_name()
}
