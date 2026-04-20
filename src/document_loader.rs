use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::metrics::DocumentTiming;
use crate::parser::{MarkdownDocument, parse_markdown};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentFingerprint {
    byte_len: usize,
    hash: u64,
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
    let parse_started = Instant::now();
    let document = parse_markdown(&content);
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
    };

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
            },
        });
    }

    let parse_started = Instant::now();
    let document = parse_markdown(&content);
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
    };

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
