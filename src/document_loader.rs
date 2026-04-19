use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::metrics::DocumentTiming;
use crate::parser::{MarkdownDocument, parse_markdown};

pub fn load_markdown_document(path: &Path) -> Result<(MarkdownDocument, DocumentTiming), String> {
    let load_started = Instant::now();
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let parse_started = Instant::now();
    let document = parse_markdown(&content);
    let timing = DocumentTiming {
        total: load_started.elapsed(),
        parse: parse_started.elapsed(),
    };

    Ok((document, timing))
}
