use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use crate::metrics::DocumentTiming;
use crate::parser::{parse_markdown, MarkdownDocument};

const MAX_PARSED_DOCUMENT_CACHE_ENTRIES: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentFingerprint {
    byte_len: usize,
    hash: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileSnapshot {
    byte_len: usize,
    modified: SystemTime,
}

#[derive(Clone)]
struct ParsedDocumentCacheEntry {
    fingerprint: DocumentFingerprint,
    document: Arc<MarkdownDocument>,
}

pub struct LoadedMarkdownDocument {
    pub document: Arc<MarkdownDocument>,
    pub timing: DocumentTiming,
    pub fingerprint: DocumentFingerprint,
    pub file_snapshot: Option<FileSnapshot>,
}

pub enum ReloadDocumentOutcome {
    Reloaded(LoadedMarkdownDocument),
    Unchanged {
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
        timing: DocumentTiming,
    },
}

pub fn load_markdown_document(path: &Path) -> Result<LoadedMarkdownDocument, String> {
    let load_started = Instant::now();
    let file_snapshot = file_snapshot(path);
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let fingerprint = fingerprint_content(&content);

    if let Some(document) = cached_parsed_document(fingerprint) {
        return Ok(LoadedMarkdownDocument {
            document,
            timing: DocumentTiming {
                total: load_started.elapsed(),
                parse: Duration::ZERO,
                byte_len: content.len(),
            },
            fingerprint,
            file_snapshot,
        });
    }

    let parse_started = Instant::now();
    let document = Arc::new(parse_markdown(&content));
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
        byte_len: content.len(),
    };
    store_parsed_document(fingerprint, Arc::clone(&document));

    Ok(LoadedMarkdownDocument {
        document,
        timing,
        fingerprint,
        file_snapshot,
    })
}

pub fn reload_markdown_document(
    path: &Path,
    previous_fingerprint: Option<DocumentFingerprint>,
    previous_file_snapshot: Option<FileSnapshot>,
) -> Result<ReloadDocumentOutcome, String> {
    let load_started = Instant::now();
    let file_snapshot = file_snapshot(path);

    if let (Some(current_snapshot), Some(previous_snapshot), Some(fingerprint)) =
        (file_snapshot, previous_file_snapshot, previous_fingerprint)
    {
        if current_snapshot == previous_snapshot {
            return Ok(ReloadDocumentOutcome::Unchanged {
                fingerprint,
                file_snapshot,
                timing: DocumentTiming {
                    total: load_started.elapsed(),
                    parse: Duration::ZERO,
                    byte_len: current_snapshot.byte_len,
                },
            });
        }
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let fingerprint = fingerprint_content(&content);

    if Some(fingerprint) == previous_fingerprint {
        return Ok(ReloadDocumentOutcome::Unchanged {
            fingerprint,
            file_snapshot,
            timing: DocumentTiming {
                total: load_started.elapsed(),
                parse: Duration::ZERO,
                byte_len: content.len(),
            },
        });
    }

    if let Some(document) = cached_parsed_document(fingerprint) {
        return Ok(ReloadDocumentOutcome::Reloaded(LoadedMarkdownDocument {
            document,
            timing: DocumentTiming {
                total: load_started.elapsed(),
                parse: Duration::ZERO,
                byte_len: content.len(),
            },
            fingerprint,
            file_snapshot,
        }));
    }

    let parse_started = Instant::now();
    let document = Arc::new(parse_markdown(&content));
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
        byte_len: content.len(),
    };
    store_parsed_document(fingerprint, Arc::clone(&document));

    Ok(ReloadDocumentOutcome::Reloaded(LoadedMarkdownDocument {
        document,
        timing,
        fingerprint,
        file_snapshot,
    }))
}

fn file_snapshot(path: &Path) -> Option<FileSnapshot> {
    let metadata = fs::metadata(path).ok()?;
    let byte_len = usize::try_from(metadata.len()).ok()?;
    let modified = metadata.modified().ok()?;

    Some(FileSnapshot { byte_len, modified })
}

fn fingerprint_content(content: &str) -> DocumentFingerprint {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);

    DocumentFingerprint {
        byte_len: content.len(),
        hash: hasher.finish(),
    }
}

fn parsed_document_cache() -> &'static Mutex<Vec<ParsedDocumentCacheEntry>> {
    static PARSED_DOCUMENT_CACHE: OnceLock<Mutex<Vec<ParsedDocumentCacheEntry>>> = OnceLock::new();
    PARSED_DOCUMENT_CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

fn cached_parsed_document(fingerprint: DocumentFingerprint) -> Option<Arc<MarkdownDocument>> {
    let mut cache = parsed_document_cache().lock().ok()?;
    let index = cache
        .iter()
        .position(|entry| entry.fingerprint == fingerprint)?;
    let entry = cache.remove(index);
    let document = Arc::clone(&entry.document);
    cache.push(entry);

    Some(document)
}

fn store_parsed_document(fingerprint: DocumentFingerprint, document: Arc<MarkdownDocument>) {
    let Ok(mut cache) = parsed_document_cache().lock() else {
        return;
    };

    if let Some(index) = cache
        .iter()
        .position(|entry| entry.fingerprint == fingerprint)
    {
        cache.remove(index);
    }

    if cache.len() >= MAX_PARSED_DOCUMENT_CACHE_ENTRIES {
        cache.remove(0);
    }

    cache.push(ParsedDocumentCacheEntry {
        fingerprint,
        document,
    });
}
