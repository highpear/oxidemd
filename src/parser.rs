use std::collections::HashMap;

use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd,
};

use crate::search::{SearchMatch, contains_normalized_query, normalized_query, preview_text};

#[derive(Clone)]
pub struct MarkdownDocument {
    pub blocks: Vec<Block>,
    headings: Vec<HeadingNavItem>,
}

#[derive(Clone)]
pub struct HeadingNavItem {
    pub block_index: usize,
    pub level: HeadingLevel,
    pub title: String,
    pub anchor: String,
}

#[derive(Clone)]
pub struct InlineContent {
    pub spans: Vec<InlineSpan>,
}

#[derive(Clone)]
pub enum InlineSpan {
    Text(String),
    Strong(String),
    Emphasis(String),
    Code(String),
    Math(String),
    Link { text: String, destination: String },
    Image { alt: String, destination: String },
    LineBreak,
}

#[derive(Clone)]
pub enum Block {
    Heading {
        level: HeadingLevel,
        content: InlineContent,
    },
    Paragraph(InlineContent),
    UnorderedList(Vec<InlineContent>),
    OrderedList {
        start: u64,
        items: Vec<InlineContent>,
    },
    BlockQuote(Vec<InlineContent>),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    MathBlock {
        expression: String,
    },
    Table {
        alignments: Vec<Alignment>,
        headers: Vec<InlineContent>,
        rows: Vec<Vec<InlineContent>>,
    },
}

pub fn parse_markdown(input: &str) -> MarkdownDocument {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_MATH);

    let mut parser = Parser::new_ext(input, options).peekable();
    let mut blocks = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let content = collect_inline_content(&mut parser, TagEnd::Heading(level));
                if !content.is_empty() {
                    blocks.push(Block::Heading { level, content });
                }
            }
            Event::Start(Tag::Paragraph) => {
                let content = collect_inline_content(&mut parser, TagEnd::Paragraph);
                if let Some(expression) = standalone_math_block(&content) {
                    blocks.push(Block::MathBlock { expression });
                } else if !content.is_empty() {
                    blocks.push(Block::Paragraph(content));
                }
            }
            Event::Start(Tag::List(start)) => {
                let ordered_start = start.unwrap_or(1);
                let items = collect_list_items(&mut parser);

                if start.is_some() {
                    blocks.push(Block::OrderedList {
                        start: ordered_start,
                        items,
                    });
                } else {
                    blocks.push(Block::UnorderedList(items));
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let lines = collect_blockquote_lines(&mut parser);
                if !lines.is_empty() {
                    blocks.push(Block::BlockQuote(lines));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(name) => {
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_owned())
                        }
                    }
                    CodeBlockKind::Indented => None,
                };

                let code = collect_text_until(&mut parser, TagEnd::CodeBlock);
                blocks.push(Block::CodeBlock { language, code });
            }
            Event::Start(Tag::Table(alignments)) => {
                let (headers, rows) = collect_table(&mut parser);
                if !headers.is_empty() || !rows.is_empty() {
                    blocks.push(Block::Table {
                        alignments,
                        headers,
                        rows,
                    });
                }
            }
            Event::DisplayMath(value) => {
                let expression = value.trim();
                if !expression.is_empty() {
                    blocks.push(Block::MathBlock {
                        expression: expression.to_owned(),
                    });
                }
            }
            _ => {}
        }
    }

    let headings = collect_heading_nav_items(&blocks);
    MarkdownDocument { blocks, headings }
}

fn collect_text_until<'a, I>(parser: &mut I, end_tag: TagEnd) -> String
where
    I: Iterator<Item = Event<'a>>,
{
    let mut text = String::new();

    for event in parser.by_ref() {
        match event {
            Event::End(tag) if tag == end_tag => break,
            Event::Text(value) | Event::Code(value) => text.push_str(value.as_ref()),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            _ => {}
        }
    }

    text.trim().to_owned()
}

fn collect_inline_content<'a, I>(parser: &mut I, end_tag: TagEnd) -> InlineContent
where
    I: Iterator<Item = Event<'a>>,
{
    let mut spans = Vec::new();

    while let Some(event) = parser.next() {
        if matches!(event, Event::End(tag) if tag == end_tag) {
            break;
        }

        push_inline_span(&mut spans, parser, event);
    }

    InlineContent { spans }
}

fn collect_list_items<'a, I>(parser: &mut std::iter::Peekable<I>) -> Vec<InlineContent>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut items = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Item) => {
                let content = collect_list_item(parser);
                if !content.is_empty() {
                    items.push(content);
                }
            }
            Event::End(TagEnd::List(_)) => break,
            _ => {}
        }
    }

    items
}

fn collect_list_item<'a, I>(parser: &mut std::iter::Peekable<I>) -> InlineContent
where
    I: Iterator<Item = Event<'a>>,
{
    let mut spans = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let content = collect_inline_content(parser, TagEnd::Paragraph);
                spans.extend(content.spans);
            }
            Event::End(TagEnd::Item) => break,
            _ => push_inline_span(&mut spans, parser, event),
        }
    }

    InlineContent { spans }
}

fn collect_blockquote_lines<'a, I>(parser: &mut std::iter::Peekable<I>) -> Vec<InlineContent>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut lines = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let content = collect_inline_content(parser, TagEnd::Paragraph);
                if !content.is_empty() {
                    lines.push(content);
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => break,
            _ => {}
        }
    }

    lines
}

fn collect_table<'a, I>(
    parser: &mut std::iter::Peekable<I>,
) -> (Vec<InlineContent>, Vec<Vec<InlineContent>>)
where
    I: Iterator<Item = Event<'a>>,
{
    let mut headers = Vec::new();
    let mut rows = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::TableHead) => {
                headers = collect_table_cells(parser, TagEnd::TableHead);
            }
            Event::Start(Tag::TableRow) => {
                let row = collect_table_cells(parser, TagEnd::TableRow);
                if !row.is_empty() {
                    rows.push(row);
                }
            }
            Event::End(TagEnd::Table) => break,
            _ => {}
        }
    }

    (headers, rows)
}

fn collect_table_cells<'a, I>(
    parser: &mut std::iter::Peekable<I>,
    row_end: TagEnd,
) -> Vec<InlineContent>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut cells = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::TableCell) => {
                cells.push(collect_inline_content(parser, TagEnd::TableCell));
            }
            Event::End(tag) if tag == row_end => break,
            _ => {}
        }
    }

    cells
}

fn collect_link_span<'a, I>(
    parser: &mut I,
    _link_type: LinkType,
    destination: String,
) -> Option<InlineSpan>
where
    I: Iterator<Item = Event<'a>>,
{
    let text = collect_text_until(parser, TagEnd::Link);
    if text.is_empty() {
        None
    } else {
        Some(InlineSpan::Link { text, destination })
    }
}

fn push_inline_span<'a, I>(spans: &mut Vec<InlineSpan>, parser: &mut I, event: Event<'a>)
where
    I: Iterator<Item = Event<'a>>,
{
    match event {
        Event::Text(value) => spans.push(InlineSpan::Text(value.to_string())),
        Event::Code(value) => spans.push(InlineSpan::Code(value.to_string())),
        Event::InlineMath(value) | Event::DisplayMath(value) => {
            spans.push(InlineSpan::Math(value.to_string()))
        }
        Event::SoftBreak | Event::HardBreak => spans.push(InlineSpan::LineBreak),
        Event::Start(Tag::Strong) => {
            let text = collect_text_until(parser, TagEnd::Strong);
            if !text.is_empty() {
                spans.push(InlineSpan::Strong(text));
            }
        }
        Event::Start(Tag::Emphasis) => {
            let text = collect_text_until(parser, TagEnd::Emphasis);
            if !text.is_empty() {
                spans.push(InlineSpan::Emphasis(text));
            }
        }
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            ..
        }) => {
            let link = collect_link_span(parser, link_type, dest_url.to_string());
            if let Some(link) = link {
                spans.push(link);
            }
        }
        Event::Start(Tag::Image { dest_url, .. }) => {
            let alt = collect_text_until(parser, TagEnd::Image);
            spans.push(InlineSpan::Image {
                alt,
                destination: dest_url.to_string(),
            });
        }
        _ => {}
    }
}

fn collect_heading_nav_items(blocks: &[Block]) -> Vec<HeadingNavItem> {
    let mut used_anchors = HashMap::new();

    blocks
        .iter()
        .enumerate()
        .filter_map(|(block_index, block)| match block {
            Block::Heading { level, content } => {
                let title = content.plain_text();

                if title.is_empty() {
                    None
                } else {
                    let anchor = unique_anchor(&title, &mut used_anchors);
                    Some(HeadingNavItem {
                        block_index,
                        level: *level,
                        title,
                        anchor,
                    })
                }
            }
            _ => None,
        })
        .collect()
}

fn standalone_math_block(content: &InlineContent) -> Option<String> {
    let mut expression = None;

    for span in &content.spans {
        match span {
            InlineSpan::Math(value) if expression.is_none() => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return None;
                }
                expression = Some(trimmed.to_owned());
            }
            InlineSpan::Text(text) if text.trim().is_empty() => {}
            InlineSpan::LineBreak => {}
            _ => return None,
        }
    }

    expression
}

fn unique_anchor(title: &str, used_anchors: &mut HashMap<String, usize>) -> String {
    let base = anchor_slug(title);
    let count = used_anchors.entry(base.clone()).or_insert(0);
    let anchor = if *count == 0 {
        base
    } else {
        format!("{}-{}", base, count)
    };

    *count += 1;
    anchor
}

fn anchor_slug(title: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for character in title.trim().chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            slug.push(character);
            previous_was_separator = false;
        } else if character.is_whitespace() || character == '-' {
            if !slug.is_empty() && !previous_was_separator {
                slug.push('-');
                previous_was_separator = true;
            }
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "section".to_owned()
    } else {
        slug
    }
}

impl InlineContent {
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|span| match span {
            InlineSpan::Text(text)
            | InlineSpan::Strong(text)
            | InlineSpan::Emphasis(text)
            | InlineSpan::Code(text)
            | InlineSpan::Math(text) => text.is_empty(),
            InlineSpan::Link { text, .. } => text.is_empty(),
            InlineSpan::Image {
                alt, destination, ..
            } => alt.is_empty() && destination.is_empty(),
            InlineSpan::LineBreak => false,
        })
    }

    pub fn plain_text(&self) -> String {
        let mut text = String::new();

        for span in &self.spans {
            match span {
                InlineSpan::Text(value)
                | InlineSpan::Strong(value)
                | InlineSpan::Emphasis(value)
                | InlineSpan::Code(value)
                | InlineSpan::Math(value) => append_normalized_text(&mut text, value),
                InlineSpan::Link { text: value, .. } => append_normalized_text(&mut text, value),
                InlineSpan::Image { alt, .. } => append_normalized_text(&mut text, alt),
                InlineSpan::LineBreak => push_normalized_space(&mut text),
            }
        }

        trim_trailing_normalized_space(&mut text);
        text
    }
}

impl Block {
    pub fn plain_text(&self) -> String {
        match self {
            Block::Heading { content, .. } | Block::Paragraph(content) => content.plain_text(),
            Block::UnorderedList(items) => join_inline_items(items),
            Block::OrderedList { items, .. } => join_inline_items(items),
            Block::BlockQuote(lines) => join_inline_items(lines),
            Block::CodeBlock { code, .. } => normalized_text(code),
            Block::MathBlock { expression } => normalized_text(expression),
            Block::Table { headers, rows, .. } => {
                let mut text = String::new();

                for header in headers {
                    append_joined_inline_content(&mut text, header);
                }

                for row in rows {
                    for cell in row {
                        append_joined_inline_content(&mut text, cell);
                    }
                }

                text
            }
        }
    }
}

impl MarkdownDocument {
    pub fn headings(&self) -> &[HeadingNavItem] {
        &self.headings
    }

    pub fn heading_block_for_anchor(&self, anchor: &str) -> Option<usize> {
        let normalized_anchor = normalize_anchor(anchor);

        self.headings
            .iter()
            .find(|heading| heading.anchor == normalized_anchor)
            .map(|heading| heading.block_index)
    }

    pub fn search_matches(&self, query: &str) -> Vec<SearchMatch> {
        let Some(normalized_query) = normalized_query(query) else {
            return Vec::new();
        };

        self.blocks
            .iter()
            .enumerate()
            .filter_map(|(block_index, block)| {
                let plain_text = block.plain_text();

                if contains_normalized_query(&plain_text, &normalized_query) {
                    Some(SearchMatch {
                        block_index,
                        preview: preview_text(&plain_text),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

fn normalize_anchor(anchor: &str) -> String {
    anchor
        .trim()
        .trim_start_matches('#')
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

fn join_inline_items(items: &[InlineContent]) -> String {
    let mut joined = String::new();

    for item in items {
        append_joined_inline_content(&mut joined, item);
    }

    joined
}

fn normalized_text(text: &str) -> String {
    let mut normalized = String::new();
    append_normalized_text(&mut normalized, text);
    trim_trailing_normalized_space(&mut normalized);
    normalized
}

fn append_joined_inline_content(buffer: &mut String, content: &InlineContent) {
    let plain_text = content.plain_text();
    if plain_text.is_empty() {
        return;
    }

    if !buffer.is_empty() {
        buffer.push(' ');
    }

    buffer.push_str(&plain_text);
}

fn append_normalized_text(buffer: &mut String, text: &str) {
    for character in text.chars() {
        if character.is_whitespace() {
            push_normalized_space(buffer);
        } else {
            buffer.push(character);
        }
    }
}

fn push_normalized_space(buffer: &mut String) {
    if !buffer.is_empty() && !buffer.ends_with(' ') {
        buffer.push(' ');
    }
}

fn trim_trailing_normalized_space(buffer: &mut String) {
    if buffer.ends_with(' ') {
        buffer.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, InlineSpan, parse_markdown};

    #[test]
    fn parses_inline_math_spans() {
        let document = parse_markdown("Euler wrote $e^{i\\pi} + 1 = 0$ in one line.");

        let Block::Paragraph(content) = &document.blocks[0] else {
            panic!("expected paragraph");
        };

        assert!(matches!(content.spans[0], InlineSpan::Text(_)));
        assert!(matches!(
            &content.spans[1],
            InlineSpan::Math(expression) if expression == "e^{i\\pi} + 1 = 0"
        ));
    }

    #[test]
    fn parses_display_math_blocks() {
        let document = parse_markdown("$$\na^2 + b^2 = c^2\n$$");

        assert!(matches!(
            &document.blocks[0],
            Block::MathBlock { expression } if expression == "a^2 + b^2 = c^2"
        ));
    }

    #[test]
    fn leaves_unclosed_inline_math_as_text() {
        let document = parse_markdown("This costs $5 today.");

        let Block::Paragraph(content) = &document.blocks[0] else {
            panic!("expected paragraph");
        };

        assert!(
            content
                .spans
                .iter()
                .all(|span| !matches!(span, InlineSpan::Math(_)))
        );
    }
}
