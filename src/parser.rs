use std::collections::HashMap;

use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd,
};

#[derive(Clone)]
pub struct MarkdownDocument {
    pub blocks: Vec<Block>,
    headings: Vec<HeadingNavItem>,
}

#[derive(Clone)]
pub struct SearchMatch {
    pub block_index: usize,
    pub preview: String,
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
    Table {
        alignments: Vec<Alignment>,
        headers: Vec<InlineContent>,
        rows: Vec<Vec<InlineContent>>,
    },
}

pub fn parse_markdown(input: &str) -> MarkdownDocument {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);

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
                if !content.is_empty() {
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
        Event::Text(value) => push_text_span(spans, value.as_ref()),
        Event::Code(value) => spans.push(InlineSpan::Code(value.to_string())),
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

fn push_text_span(spans: &mut Vec<InlineSpan>, text: &str) {
    if text.is_empty() {
        return;
    }

    spans.push(InlineSpan::Text(text.to_owned()));
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
            | InlineSpan::Code(text) => text.is_empty(),
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
                | InlineSpan::Code(value) => text.push_str(value),
                InlineSpan::Link { text: value, .. } => text.push_str(value),
                InlineSpan::Image { alt, .. } => text.push_str(alt),
                InlineSpan::LineBreak => text.push(' '),
            }
        }

        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

impl Block {
    pub fn plain_text(&self) -> String {
        match self {
            Block::Heading { content, .. } | Block::Paragraph(content) => content.plain_text(),
            Block::UnorderedList(items) => join_inline_items(items),
            Block::OrderedList { items, .. } => join_inline_items(items),
            Block::BlockQuote(lines) => join_inline_items(lines),
            Block::CodeBlock { code, .. } => code.split_whitespace().collect::<Vec<_>>().join(" "),
            Block::Table { headers, rows, .. } => {
                let mut items = headers.to_vec();
                items.extend(rows.iter().flatten().cloned());
                join_inline_items(&items)
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
        let normalized_query = query.trim().to_lowercase();
        if normalized_query.is_empty() {
            return Vec::new();
        }

        self.blocks
            .iter()
            .enumerate()
            .filter_map(|(block_index, block)| {
                let plain_text = block.plain_text();
                let normalized_block = plain_text.to_lowercase();

                if normalized_block.contains(&normalized_query) {
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
    items
        .iter()
        .map(InlineContent::plain_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn preview_text(text: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 72;

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
