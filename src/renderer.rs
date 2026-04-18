use eframe::egui::{self, FontFamily, FontId, Frame, RichText, Stroke, Ui};
use pulldown_cmark::HeadingLevel;

use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};
use crate::theme::Theme;

const BODY_TEXT_SIZE: f32 = 17.0;
const QUOTE_TEXT_SIZE: f32 = 16.0;
const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
const BLOCK_SPACING_PARAGRAPH: f32 = 18.0;
const BLOCK_SPACING_SECTION: f32 = 24.0;
const LIST_ITEM_SPACING: f32 = 8.0;

pub fn render_markdown_document(ui: &mut Ui, document: &MarkdownDocument, theme: &Theme) {
    for block in &document.blocks {
        match block {
            Block::Heading { level, content } => render_heading(ui, *level, content, theme),
            Block::Paragraph(text) => {
                render_inline(ui, text, InlineStyle::Body, theme);
                ui.add_space(BLOCK_SPACING_PARAGRAPH);
            }
            Block::UnorderedList(items) => {
                for item in items {
                    render_list_item(
                        ui,
                        RichText::new("- ").color(theme.text_secondary),
                        item,
                        theme,
                    );
                    ui.add_space(LIST_ITEM_SPACING);
                }
                ui.add_space(BLOCK_SPACING_SECTION);
            }
            Block::OrderedList { start, items } => {
                for (index, item) in items.iter().enumerate() {
                    render_list_item(
                        ui,
                        RichText::new(format!("{}. ", start + index as u64))
                            .color(theme.text_secondary),
                        item,
                        theme,
                    );
                    ui.add_space(LIST_ITEM_SPACING);
                }
                ui.add_space(BLOCK_SPACING_SECTION);
            }
            Block::BlockQuote(lines) => render_blockquote(ui, lines, theme),
            Block::CodeBlock { language, code } => {
                render_code_block(ui, language.as_deref(), code, theme)
            }
        }
    }
}

fn render_heading(ui: &mut Ui, level: HeadingLevel, content: &InlineContent, theme: &Theme) {
    let size = match level {
        HeadingLevel::H1 => 31.0,
        HeadingLevel::H2 => 26.0,
        HeadingLevel::H3 => 22.0,
        HeadingLevel::H4 => 19.0,
        HeadingLevel::H5 => 17.0,
        HeadingLevel::H6 => 16.0,
    };

    render_inline(ui, content, InlineStyle::Heading(size), theme);
    ui.add_space(BLOCK_SPACING_SECTION);
}

fn render_list_item(ui: &mut Ui, marker: RichText, item: &InlineContent, theme: &Theme) {
    ui.horizontal_top(|ui| {
        ui.add_sized([24.0, 0.0], egui::Label::new(marker));

        ui.vertical(|ui| {
            render_inline(ui, item, InlineStyle::Body, theme);
        });
    });
}

fn render_blockquote(ui: &mut Ui, lines: &[InlineContent], theme: &Theme) {
    Frame::new()
        .fill(theme.quote_background)
        .stroke(Stroke::new(1.0, theme.quote_border))
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            for line in lines {
                render_inline(ui, line, InlineStyle::Quote, theme);
                ui.add_space(6.0);
            }
        });

    ui.add_space(BLOCK_SPACING_SECTION);
}

fn render_code_block(ui: &mut Ui, language: Option<&str>, code: &str, theme: &Theme) {
    Frame::new()
        .fill(theme.code_background)
        .stroke(Stroke::new(1.0, theme.content_border))
        .inner_margin(egui::Margin::symmetric(16, 14))
        .show(ui, |ui| {
            if let Some(language) = language {
                ui.label(
                    RichText::new(language)
                        .small()
                        .strong()
                        .color(theme.text_secondary),
                );
                ui.add_space(4.0);
            }

            ui.label(
                RichText::new(code)
                    .font(FontId::new(INLINE_CODE_TEXT_SIZE, FontFamily::Monospace))
                    .color(theme.text_primary),
            );
        });

    ui.add_space(BLOCK_SPACING_SECTION);
}

#[derive(Clone, Copy)]
enum InlineStyle {
    Body,
    Quote,
    Heading(f32),
}

fn render_inline(ui: &mut Ui, content: &InlineContent, style: InlineStyle, theme: &Theme) {
    let mut lines: Vec<Vec<&InlineSpan>> = vec![Vec::new()];

    for span in &content.spans {
        if matches!(span, InlineSpan::LineBreak) {
            lines.push(Vec::new());
        } else if let Some(current_line) = lines.last_mut() {
            current_line.push(span);
        }
    }

    for line in lines {
        ui.horizontal_wrapped(|ui| {
            for span in line {
                render_inline_span(ui, span, style, theme);
            }
        });
    }
}

enum SpanKind {
    Plain,
    Strong,
    Emphasis,
    Code,
    Link,
}

fn render_inline_span(ui: &mut Ui, span: &InlineSpan, style: InlineStyle, theme: &Theme) {
    match span {
        InlineSpan::Text(text) => render_text_label(ui, text, style, SpanKind::Plain, theme),
        InlineSpan::Strong(text) => render_text_label(ui, text, style, SpanKind::Strong, theme),
        InlineSpan::Emphasis(text) => render_text_label(ui, text, style, SpanKind::Emphasis, theme),
        InlineSpan::Code(text) => render_text_label(ui, text, style, SpanKind::Code, theme),
        InlineSpan::Link { text, destination } => {
            let rich_text = styled_text(text, style, SpanKind::Link, theme);
            ui.hyperlink_to(rich_text, destination);
        }
        InlineSpan::LineBreak => {}
    }
}

fn render_text_label(ui: &mut Ui, text: &str, style: InlineStyle, kind: SpanKind, theme: &Theme) {
    if text.is_empty() {
        return;
    }

    ui.label(styled_text(text, style, kind, theme));
}

fn styled_text(text: &str, style: InlineStyle, kind: SpanKind, theme: &Theme) -> RichText {
    let mut rich_text = match style {
        InlineStyle::Body => RichText::new(text)
            .size(BODY_TEXT_SIZE)
            .color(theme.text_primary),
        InlineStyle::Quote => RichText::new(text)
            .size(QUOTE_TEXT_SIZE)
            .color(theme.text_secondary)
            .italics(),
        InlineStyle::Heading(size) => RichText::new(text)
            .size(size)
            .color(theme.text_primary)
            .strong(),
    };

    match kind {
        SpanKind::Plain => rich_text,
        SpanKind::Strong => rich_text.strong(),
        SpanKind::Emphasis => rich_text.italics(),
        SpanKind::Code => {
            let font_size = match style {
                InlineStyle::Heading(size) => (size - 1.0).max(INLINE_CODE_TEXT_SIZE),
                _ => INLINE_CODE_TEXT_SIZE,
            };

            rich_text = rich_text
                .family(FontFamily::Monospace)
                .font(FontId::new(font_size, FontFamily::Monospace))
                .background_color(theme.code_background);

            rich_text
        }
        SpanKind::Link => {
            rich_text = rich_text.color(theme.link).underline();

            rich_text
        }
    }
}
