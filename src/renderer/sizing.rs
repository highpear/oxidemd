use pulldown_cmark::HeadingLevel;

use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};

use super::{
    BLOCK_SPACING_PARAGRAPH, BLOCK_SPACING_SECTION, BODY_TEXT_SIZE, ESTIMATED_CHARS_PER_LINE,
    LIST_ITEM_SPACING,
};

pub fn estimate_document_block_heights(document: &MarkdownDocument, zoom_factor: f32) -> Vec<f32> {
    document
        .blocks
        .iter()
        .map(|block| estimate_block_height(block, zoom_factor))
        .collect()
}

pub(super) fn estimate_block_height(block: &Block, zoom_factor: f32) -> f32 {
    match block {
        Block::Heading { level, content } => {
            let size = match level {
                HeadingLevel::H1 => 31.0,
                HeadingLevel::H2 => 26.0,
                HeadingLevel::H3 => 22.0,
                HeadingLevel::H4 => 19.0,
                HeadingLevel::H5 => 17.0,
                HeadingLevel::H6 => 16.0,
            };
            estimate_inline_height(content, size, zoom_factor)
                + scale_spacing(BLOCK_SPACING_SECTION, zoom_factor)
        }
        Block::Paragraph(content) => {
            estimate_inline_height(content, BODY_TEXT_SIZE, zoom_factor)
                + scale_spacing(BLOCK_SPACING_PARAGRAPH, zoom_factor)
        }
        Block::UnorderedList(items) | Block::BlockQuote(items) => {
            estimate_inline_items_height(items, BODY_TEXT_SIZE, zoom_factor)
                + scale_spacing(BLOCK_SPACING_SECTION, zoom_factor)
        }
        Block::OrderedList { items, .. } => {
            estimate_inline_items_height(items, BODY_TEXT_SIZE, zoom_factor)
                + scale_spacing(BLOCK_SPACING_SECTION, zoom_factor)
        }
        Block::CodeBlock { code, .. } => {
            let line_count = code.lines().count().max(1) as f32;
            line_count * scale_spacing(20.0, zoom_factor) + scale_spacing(42.0, zoom_factor)
        }
        Block::DiagramBlock { source, .. } => {
            let line_count = source.lines().count().max(1) as f32;
            line_count * scale_spacing(20.0, zoom_factor) + scale_spacing(72.0, zoom_factor)
        }
        Block::MathBlock { expression } => {
            let line_count = expression.lines().count().max(1) as f32;
            line_count * scale_spacing(22.0, zoom_factor) + scale_spacing(58.0, zoom_factor)
        }
        Block::Table { headers, rows, .. } => {
            let row_count = rows.len() + usize::from(!headers.is_empty());
            row_count.max(1) as f32 * scale_spacing(34.0, zoom_factor)
                + scale_spacing(BLOCK_SPACING_SECTION + 18.0, zoom_factor)
        }
    }
}

fn estimate_inline_items_height(items: &[InlineContent], text_size: f32, zoom_factor: f32) -> f32 {
    items
        .iter()
        .map(|item| {
            estimate_inline_height(item, text_size, zoom_factor)
                + scale_spacing(LIST_ITEM_SPACING, zoom_factor)
        })
        .sum()
}

fn estimate_inline_height(content: &InlineContent, text_size: f32, zoom_factor: f32) -> f32 {
    let lines = estimate_inline_line_count(content) as f32;
    lines * text_size * zoom_factor * 1.45
}

fn estimate_inline_line_count(content: &InlineContent) -> usize {
    let mut line_count = 1usize;
    let mut line_len = 0usize;

    for span in &content.spans {
        match span {
            InlineSpan::Text(text)
            | InlineSpan::Strong(text)
            | InlineSpan::Emphasis(text)
            | InlineSpan::Code(text)
            | InlineSpan::Math(text) => {
                line_len += text.len();
            }
            InlineSpan::Link { text, .. } | InlineSpan::Image { alt: text, .. } => {
                line_len += text.len();
            }
            InlineSpan::LineBreak => {
                line_count += 1;
                line_len = 0;
            }
        }

        if line_len >= ESTIMATED_CHARS_PER_LINE {
            line_count += line_len / ESTIMATED_CHARS_PER_LINE;
            line_len %= ESTIMATED_CHARS_PER_LINE;
        }
    }

    line_count
}

pub(super) fn scale_spacing(value: f32, zoom_factor: f32) -> f32 {
    value * zoom_factor
}

pub(super) fn scale_margin(value: i8, zoom_factor: f32) -> i8 {
    ((value as f32) * zoom_factor)
        .round()
        .clamp(0.0, i8::MAX as f32) as i8
}
