use eframe::egui::{self, Align, FontFamily, FontId, Frame, RichText, Stroke, Ui};
use pulldown_cmark::HeadingLevel;

use crate::code_block::render_code_block;
use crate::i18n::Language;
use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};
use crate::theme::Theme;

const BODY_TEXT_SIZE: f32 = 17.0;
const QUOTE_TEXT_SIZE: f32 = 16.0;
const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
const BLOCK_SPACING_PARAGRAPH: f32 = 18.0;
const BLOCK_SPACING_SECTION: f32 = 24.0;
const LIST_ITEM_SPACING: f32 = 8.0;

pub fn render_markdown_document(
    ui: &mut Ui,
    document: &MarkdownDocument,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
    scroll_to_block: Option<usize>,
    highlighted_block: Option<usize>,
) -> RenderOutcome {
    let mut did_scroll = false;
    let mut active_heading = None;
    let viewport_top = ui.clip_rect().top();

    for (block_index, block) in document.blocks.iter().enumerate() {
        let should_highlight = highlighted_block == Some(block_index);

        match block {
            Block::Heading { level, content } => {
                let heading_state = render_heading(
                    ui,
                    *level,
                    content,
                    theme,
                    zoom_factor,
                    scroll_to_block,
                    block_index,
                    viewport_top,
                    should_highlight,
                );
                did_scroll |= heading_state.did_scroll;

                if heading_state.is_active {
                    active_heading = Some(block_index);
                }
            }
            Block::Paragraph(text) => {
                render_highlighted_block(ui, should_highlight, theme, zoom_factor, |ui| {
                    render_inline(ui, text, InlineStyle::Body, theme, zoom_factor);
                    ui.add_space(scale_spacing(BLOCK_SPACING_PARAGRAPH, zoom_factor));
                });

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::UnorderedList(items) => {
                render_highlighted_block(ui, should_highlight, theme, zoom_factor, |ui| {
                    for item in items {
                        render_list_item(
                            ui,
                            RichText::new("- ").color(theme.text_secondary),
                            item,
                            theme,
                            zoom_factor,
                        );
                        ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                    }
                    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
                });

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::OrderedList { start, items } => {
                render_highlighted_block(ui, should_highlight, theme, zoom_factor, |ui| {
                    for (index, item) in items.iter().enumerate() {
                        render_list_item(
                            ui,
                            RichText::new(format!("{}. ", start + index as u64))
                                .color(theme.text_secondary),
                            item,
                            theme,
                            zoom_factor,
                        );
                        ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                    }
                    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
                });

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::BlockQuote(lines) => {
                render_highlighted_block(ui, should_highlight, theme, zoom_factor, |ui| {
                    render_blockquote(ui, lines, theme, zoom_factor);
                });

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::CodeBlock { language, code } => {
                render_highlighted_block(ui, should_highlight, theme, zoom_factor, |ui| {
                    render_code_block(
                        ui,
                        block_index,
                        ui_language,
                        language.as_deref(),
                        code,
                        theme,
                        zoom_factor,
                    );
                });

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
        }
    }

    RenderOutcome {
        did_scroll,
        active_heading,
    }
}

pub struct RenderOutcome {
    pub did_scroll: bool,
    pub active_heading: Option<usize>,
}

struct HeadingRenderState {
    did_scroll: bool,
    is_active: bool,
}

fn render_heading(
    ui: &mut Ui,
    level: HeadingLevel,
    content: &InlineContent,
    theme: &Theme,
    zoom_factor: f32,
    scroll_to_block: Option<usize>,
    block_index: usize,
    viewport_top: f32,
    should_highlight: bool,
) -> HeadingRenderState {
    let size = match level {
        HeadingLevel::H1 => 31.0,
        HeadingLevel::H2 => 26.0,
        HeadingLevel::H3 => 22.0,
        HeadingLevel::H4 => 19.0,
        HeadingLevel::H5 => 17.0,
        HeadingLevel::H6 => 16.0,
    } * zoom_factor;

    let anchor = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());

    if scroll_to_block == Some(block_index) {
        ui.scroll_to_rect(anchor.rect, Some(Align::TOP));
    }

    let heading_response = render_highlighted_block(ui, should_highlight, theme, zoom_factor, |ui| {
        render_inline(ui, content, InlineStyle::Heading(size), theme, zoom_factor);
        ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
    });

    let heading_rect = anchor.rect.union(heading_response.response.rect);
    let is_active = heading_rect.top() <= viewport_top + scale_spacing(8.0, zoom_factor);

    HeadingRenderState {
        did_scroll: scroll_to_block == Some(block_index),
        is_active,
    }
}

fn render_highlighted_block<R>(
    ui: &mut Ui,
    should_highlight: bool,
    theme: &Theme,
    zoom_factor: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    if should_highlight {
        Frame::new()
            .fill(theme.status_loading_background.gamma_multiply(if theme.is_dark {
                0.35
            } else {
                0.65
            }))
            .stroke(Stroke::new(1.0, theme.status_loading_text))
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(
                scale_margin(10, zoom_factor),
                scale_margin(8, zoom_factor),
            ))
            .show(ui, add_contents)
    } else {
        ui.scope(add_contents)
    }
}

fn render_list_item(
    ui: &mut Ui,
    marker: RichText,
    item: &InlineContent,
    theme: &Theme,
    zoom_factor: f32,
) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [scale_spacing(24.0, zoom_factor), 0.0],
            egui::Label::new(marker.size(BODY_TEXT_SIZE * zoom_factor)),
        );

        ui.vertical(|ui| {
            render_inline(ui, item, InlineStyle::Body, theme, zoom_factor);
        });
    });
}

fn render_blockquote(ui: &mut Ui, lines: &[InlineContent], theme: &Theme, zoom_factor: f32) {
    Frame::new()
        .fill(theme.quote_background)
        .stroke(Stroke::new(1.0, theme.quote_border))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(16, zoom_factor),
            scale_margin(14, zoom_factor),
        ))
        .show(ui, |ui| {
            for line in lines {
                render_inline(ui, line, InlineStyle::Quote, theme, zoom_factor);
                ui.add_space(scale_spacing(6.0, zoom_factor));
            }
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

#[derive(Clone, Copy)]
enum InlineStyle {
    Body,
    Quote,
    Heading(f32),
}

fn render_inline(
    ui: &mut Ui,
    content: &InlineContent,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
) {
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
                render_inline_span(ui, span, style, theme, zoom_factor);
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

fn render_inline_span(
    ui: &mut Ui,
    span: &InlineSpan,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
) {
    match span {
        InlineSpan::Text(text) => {
            render_text_label(ui, text, style, SpanKind::Plain, theme, zoom_factor)
        }
        InlineSpan::Strong(text) => {
            render_text_label(ui, text, style, SpanKind::Strong, theme, zoom_factor)
        }
        InlineSpan::Emphasis(text) => {
            render_text_label(ui, text, style, SpanKind::Emphasis, theme, zoom_factor)
        }
        InlineSpan::Code(text) => {
            render_text_label(ui, text, style, SpanKind::Code, theme, zoom_factor)
        }
        InlineSpan::Link { text, destination } => {
            let rich_text = styled_text(text, style, SpanKind::Link, theme, zoom_factor);
            ui.hyperlink_to(rich_text, destination);
        }
        InlineSpan::LineBreak => {}
    }
}

fn render_text_label(
    ui: &mut Ui,
    text: &str,
    style: InlineStyle,
    kind: SpanKind,
    theme: &Theme,
    zoom_factor: f32,
) {
    if text.is_empty() {
        return;
    }

    ui.label(styled_text(text, style, kind, theme, zoom_factor));
}

fn styled_text(
    text: &str,
    style: InlineStyle,
    kind: SpanKind,
    theme: &Theme,
    zoom_factor: f32,
) -> RichText {
    let mut rich_text = match style {
        InlineStyle::Body => RichText::new(text)
            .size(BODY_TEXT_SIZE * zoom_factor)
            .color(theme.text_primary),
        InlineStyle::Quote => RichText::new(text)
            .size(QUOTE_TEXT_SIZE * zoom_factor)
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
                InlineStyle::Heading(size) => (size - zoom_factor).max(INLINE_CODE_TEXT_SIZE),
                _ => INLINE_CODE_TEXT_SIZE * zoom_factor,
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

fn scale_spacing(value: f32, zoom_factor: f32) -> f32 {
    value * zoom_factor
}

fn scale_margin(value: i8, zoom_factor: f32) -> i8 {
    ((value as f32) * zoom_factor)
        .round()
        .clamp(0.0, i8::MAX as f32) as i8
}
