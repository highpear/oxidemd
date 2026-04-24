use crate::parser::MarkdownDocument;

const MAX_PREVIEW_CHARS: usize = 72;

#[derive(Clone)]
pub struct SearchMatch {
    pub block_index: usize,
    pub preview: String,
}

pub struct SearchState {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub active_index: Option<usize>,
    pub focus_input: bool,
}

pub struct HighlightSegment<'a> {
    pub text: &'a str,
    pub is_match: bool,
    pub is_active_match: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HighlightWalkSummary {
    pub segment_count: usize,
    pub match_count: usize,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            active_index: None,
            focus_input: false,
        }
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.active_index = None;
    }

    pub fn clear_matches(&mut self) {
        self.matches.clear();
        self.active_index = None;
    }

    pub fn refresh_matches(&mut self, document: Option<&MarkdownDocument>) {
        let Some(document) = document else {
            self.clear_matches();
            return;
        };

        let previous_block = self.active_block();
        self.matches = document.search_matches(&self.query);

        self.active_index = previous_block
            .and_then(|block_index| {
                self.matches
                    .iter()
                    .position(|entry| entry.block_index == block_index)
            })
            .or_else(|| (!self.matches.is_empty()).then_some(0));
    }

    pub fn select_match(&mut self, index: usize) -> Option<usize> {
        let search_match = self.matches.get(index)?;
        self.active_index = Some(index);
        Some(search_match.block_index)
    }

    pub fn select_next(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }

        let next_index = match self.active_index {
            Some(index) => (index + 1) % self.matches.len(),
            None => 0,
        };

        self.select_match(next_index)
    }

    pub fn select_previous(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }

        let previous_index = match self.active_index {
            Some(0) | None => self.matches.len() - 1,
            Some(index) => index - 1,
        };

        self.select_match(previous_index)
    }

    pub fn has_matches(&self) -> bool {
        !self.matches.is_empty()
    }

    pub fn active_block(&self) -> Option<usize> {
        self.active_index
            .and_then(|index| self.matches.get(index))
            .map(|search_match| search_match.block_index)
    }

    pub fn active_query(&self) -> Option<&str> {
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
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

    let mut preview = String::with_capacity(trimmed.len().min(MAX_PREVIEW_CHARS + 3));
    let mut preview_chars = 0usize;
    let mut truncated = false;

    for character in trimmed.chars() {
        if preview_chars == MAX_PREVIEW_CHARS {
            truncated = true;
            break;
        }

        preview.push(character);
        preview_chars += 1;
    }

    if truncated {
        preview.push_str("...");
    }

    preview
}

#[cfg(test)]
pub fn split_highlighted_segments<'a>(
    text: &'a str,
    search_query: Option<&str>,
    is_active_match: bool,
) -> Vec<HighlightSegment<'a>> {
    let mut segments = Vec::new();
    for_each_highlighted_segment(text, search_query, is_active_match, |segment| {
        segments.push(segment);
    });
    segments
}

pub fn for_each_highlighted_segment<'a, F>(
    text: &'a str,
    search_query: Option<&str>,
    is_active_match: bool,
    mut visit: F,
) -> HighlightWalkSummary
where
    F: FnMut(HighlightSegment<'a>),
{
    let Some(query) = search_query.and_then(normalized_query) else {
        visit_unmatched_segment(text, &mut visit);
        return HighlightWalkSummary {
            segment_count: 1,
            match_count: 0,
        };
    };

    for_each_segment_with_normalized_query(text, &query, is_active_match, visit)
}

pub fn text_matches_query(text: &str, search_query: Option<&str>) -> bool {
    let Some(query) = search_query.and_then(normalized_query) else {
        return false;
    };

    contains_normalized_query(text, &query)
}

fn for_each_segment_with_normalized_query<'a, F>(
    text: &'a str,
    normalized_query: &str,
    is_active_match: bool,
    mut visit: F,
) -> HighlightWalkSummary
where
    F: FnMut(HighlightSegment<'a>),
{
    if normalized_query.is_empty() {
        let mut summary = HighlightWalkSummary::default();
        visit_unmatched_segment(text, &mut |segment| {
            summary.segment_count += 1;
            visit(segment);
        });
        return summary;
    }

    let (folded_text, source_ranges) = folded_text_with_source_ranges(text);
    let mut summary = HighlightWalkSummary::default();
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
            visit(HighlightSegment {
                text: &text[current_start..match_start],
                is_match: false,
                is_active_match: false,
            });
            summary.segment_count += 1;
        }

        visit(HighlightSegment {
            text: &text[match_start..match_end],
            is_match: true,
            is_active_match,
        });
        summary.segment_count += 1;
        summary.match_count += 1;

        current_start = match_end;
        search_start = folded_match_end;
    }

    if current_start < text.len() {
        visit(HighlightSegment {
            text: &text[current_start..],
            is_match: false,
            is_active_match: false,
        });
        summary.segment_count += 1;
    }

    if summary.segment_count == 0 {
        visit_unmatched_segment(text, &mut visit);
        HighlightWalkSummary {
            segment_count: 1,
            match_count: 0,
        }
    } else {
        summary
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

fn visit_unmatched_segment<'a, F>(text: &'a str, visit: &mut F)
where
    F: FnMut(HighlightSegment<'a>),
{
    visit(HighlightSegment {
        text,
        is_match: false,
        is_active_match: false,
    });
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
        let segments = split_highlighted_segments("Fast Rust viewer", Some("rust"), false);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "Fast ");
        assert!(!segments[0].is_match);
        assert_eq!(segments[1].text, "Rust");
        assert!(segments[1].is_match);
        assert!(!segments[1].is_active_match);
        assert_eq!(segments[2].text, " viewer");
        assert!(!segments[2].is_match);
    }

    #[test]
    fn highlighted_segments_keep_source_boundaries_for_expanded_lowercase() {
        let segments = split_highlighted_segments("İstanbul", Some("i"), false);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "İ");
        assert!(segments[0].is_match);
        assert_eq!(segments[1].text, "stanbul");
        assert!(!segments[1].is_match);
    }

    #[test]
    fn highlighted_segments_mark_active_matches() {
        let segments = split_highlighted_segments("Rust", Some("rust"), true);

        assert_eq!(segments.len(), 1);
        assert!(segments[0].is_match);
        assert!(segments[0].is_active_match);
    }

    #[test]
    fn preview_text_truncates_by_characters() {
        let text = "あ".repeat(MAX_PREVIEW_CHARS + 1);
        let preview = preview_text(&text);

        assert_eq!(preview.chars().count(), MAX_PREVIEW_CHARS + 3);
        assert!(preview.ends_with("..."));
    }
}
