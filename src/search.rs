const MAX_PREVIEW_CHARS: usize = 72;

#[derive(Clone)]
pub struct SearchMatch {
    pub block_index: usize,
    pub preview: String,
}

pub struct HighlightSegment<'a> {
    pub text: &'a str,
    pub is_match: bool,
}

pub fn normalized_query(query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_lowercase())
    }
}

pub fn contains_normalized_query(text: &str, normalized_query: &str) -> bool {
    text.to_lowercase().contains(normalized_query)
}

pub fn preview_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut preview = String::new();
    for character in trimmed.chars().take(MAX_PREVIEW_CHARS) {
        preview.push(character);
    }

    if trimmed.chars().count() > MAX_PREVIEW_CHARS {
        preview.push_str("...");
    }

    preview
}

pub fn split_highlighted_segments<'a>(
    text: &'a str,
    search_query: Option<&str>,
) -> Vec<HighlightSegment<'a>> {
    let Some(query) = search_query.and_then(normalized_query) else {
        return unmatched_segment(text);
    };

    split_segments_with_normalized_query(text, &query)
}

pub fn text_matches_query(text: &str, search_query: Option<&str>) -> bool {
    let Some(query) = search_query.and_then(normalized_query) else {
        return false;
    };

    contains_normalized_query(text, &query)
}

fn split_segments_with_normalized_query<'a>(
    text: &'a str,
    normalized_query: &str,
) -> Vec<HighlightSegment<'a>> {
    if normalized_query.is_empty() {
        return unmatched_segment(text);
    }

    let (folded_text, source_ranges) = folded_text_with_source_ranges(text);
    let mut segments = Vec::new();
    let mut current_start = 0usize;
    let mut search_start = 0usize;

    while let Some(relative_match_start) = folded_text[search_start..].find(normalized_query) {
        let folded_match_start = search_start + relative_match_start;
        let folded_match_end = folded_match_start + normalized_query.len();
        let Some((match_start, match_end)) =
            source_range_for_folded_match(&source_ranges, folded_match_start, folded_match_end)
        else {
            break;
        };

        if current_start < match_start {
            segments.push(HighlightSegment {
                text: &text[current_start..match_start],
                is_match: false,
            });
        }

        segments.push(HighlightSegment {
            text: &text[match_start..match_end],
            is_match: true,
        });

        current_start = match_end;
        search_start = folded_match_end;
    }

    if current_start < text.len() {
        segments.push(HighlightSegment {
            text: &text[current_start..],
            is_match: false,
        });
    }

    if segments.is_empty() {
        unmatched_segment(text)
    } else {
        segments
    }
}

#[derive(Clone, Copy)]
struct SourceRange {
    folded_start: usize,
    folded_end: usize,
    source_start: usize,
    source_end: usize,
}

fn folded_text_with_source_ranges(text: &str) -> (String, Vec<SourceRange>) {
    let mut folded_text = String::new();
    let mut source_ranges = Vec::new();

    for (source_start, character) in text.char_indices() {
        let source_end = source_start + character.len_utf8();
        let folded_start = folded_text.len();

        for folded_character in character.to_lowercase() {
            folded_text.push(folded_character);
        }

        source_ranges.push(SourceRange {
            folded_start,
            folded_end: folded_text.len(),
            source_start,
            source_end,
        });
    }

    (folded_text, source_ranges)
}

fn source_range_for_folded_match(
    source_ranges: &[SourceRange],
    folded_match_start: usize,
    folded_match_end: usize,
) -> Option<(usize, usize)> {
    let start_range = source_ranges.iter().find(|range| {
        range.folded_start <= folded_match_start && folded_match_start < range.folded_end
    })?;
    let end_range = source_ranges.iter().find(|range| {
        range.folded_start < folded_match_end && folded_match_end <= range.folded_end
    })?;

    Some((start_range.source_start, end_range.source_end))
}

fn unmatched_segment(text: &str) -> Vec<HighlightSegment<'_>> {
    vec![HighlightSegment {
        text,
        is_match: false,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_matching_is_case_insensitive() {
        let query = normalized_query("rust").expect("query should not be empty");

        assert!(contains_normalized_query("Rust Markdown viewer", &query));
    }

    #[test]
    fn highlighted_segments_match_case_insensitively() {
        let segments = split_highlighted_segments("Fast Rust viewer", Some("rust"));

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "Fast ");
        assert!(!segments[0].is_match);
        assert_eq!(segments[1].text, "Rust");
        assert!(segments[1].is_match);
        assert_eq!(segments[2].text, " viewer");
        assert!(!segments[2].is_match);
    }

    #[test]
    fn highlighted_segments_keep_source_boundaries_for_expanded_lowercase() {
        let segments = split_highlighted_segments("İstanbul", Some("i"));

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "İ");
        assert!(segments[0].is_match);
        assert_eq!(segments[1].text, "stanbul");
        assert!(!segments[1].is_match);
    }

    #[test]
    fn preview_text_truncates_by_characters() {
        let text = "あ".repeat(MAX_PREVIEW_CHARS + 1);
        let preview = preview_text(&text);

        assert_eq!(preview.chars().count(), MAX_PREVIEW_CHARS + 3);
        assert!(preview.ends_with("..."));
    }
}
