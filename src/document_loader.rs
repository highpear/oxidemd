use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::metrics::DocumentTiming;
use crate::parser::{MarkdownDocument, parse_markdown};

const MAX_PARSED_DOCUMENT_CACHE_ENTRIES: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentFingerprint {
    byte_len: usize,
    hash: u64,
}

#[derive(Clone)]
struct ParsedDocumentCacheEntry {
    fingerprint: DocumentFingerprint,
    document: MarkdownDocument,
}

pub struct LoadedMarkdownDocument {
    pub document: MarkdownDocument,
    pub timing: DocumentTiming,
    pub fingerprint: DocumentFingerprint,
}

pub enum ReloadDocumentOutcome {
    Reloaded(LoadedMarkdownDocument),
    Unchanged {
        fingerprint: DocumentFingerprint,
        timing: DocumentTiming,
    },
}

pub fn load_markdown_document(path: &Path) -> Result<LoadedMarkdownDocument, String> {
    let load_started = Instant::now();
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
        });
    }

    let parse_started = Instant::now();
    let document = parse_markdown(&content);
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
        byte_len: content.len(),
    };
    store_parsed_document(fingerprint, document.clone());

    Ok(LoadedMarkdownDocument {
        document,
        timing,
        fingerprint,
    })
}

pub fn reload_markdown_document(
    path: &Path,
    previous_fingerprint: Option<DocumentFingerprint>,
) -> Result<ReloadDocumentOutcome, String> {
    let load_started = Instant::now();
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let fingerprint = fingerprint_content(&content);

    if Some(fingerprint) == previous_fingerprint {
        return Ok(ReloadDocumentOutcome::Unchanged {
            fingerprint,
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
        }));
    }

    let parse_started = Instant::now();
    let document = parse_markdown(&content);
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
        byte_len: content.len(),
    };
    store_parsed_document(fingerprint, document.clone());

    Ok(ReloadDocumentOutcome::Reloaded(LoadedMarkdownDocument {
        document,
        timing,
        fingerprint,
    }))
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

fn cached_parsed_document(fingerprint: DocumentFingerprint) -> Option<MarkdownDocument> {
    let mut cache = parsed_document_cache().lock().ok()?;
    let index = cache
        .iter()
        .position(|entry| entry.fingerprint == fingerprint)?;
    let entry = cache.remove(index);
    let document = entry.document.clone();
    cache.push(entry);

    Some(document)
}

fn store_parsed_document(fingerprint: DocumentFingerprint, document: MarkdownDocument) {
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
