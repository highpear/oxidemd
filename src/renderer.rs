use std::time::Duration;

use eframe::egui::{
    self, Align, FontFamily, FontId, Frame, Label, Layout, RichText, ScrollArea, Stroke,
    TextWrapMode, Ui,
};
use pulldown_cmark::HeadingLevel;

use crate::i18n::{Language, tr};
use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};
use crate::syntax::highlight_code;
use crate::theme::Theme;

const BODY_TEXT_SIZE: f32 = 17.0;
const QUOTE_TEXT_SIZE: f32 = 16.0;
const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
const CODE_LANGUAGE_TEXT_SIZE: f32 = 11.0;
const COPY_FEEDBACK_DURATION_SECONDS: f64 = 1.2;
const COPY_FEEDBACK_SLOT_WIDTH: f32 = 88.0;
const BLOCK_SPACING_PARAGRAPH: f32 = 18.0;
const BLOCK_SPACING_SECTION: f32 = 24.0;
const LIST_ITEM_SPACING: f32 = 8.0;

#[derive(Clone, Copy)]
struct CopyFeedbackState {
    block_index: usize,
    copied_at: f64,
}

pub fn render_markdown_document(
    ui: &mut Ui,
    document: &MarkdownDocument,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
    scroll_to_heading: Option<usize>,
) -> bool {
    let mut did_scroll = false;

    for (block_index, block) in document.blocks.iter().enumerate() {
        match block {
            Block::Heading { level, content } => {
                did_scroll |= render_heading(
                    ui,
                    *level,
                    content,
                    theme,
                    zoom_factor,
                    scroll_to_heading,
                    block_index,
                );
            }
            Block::Paragraph(text) => {
                render_inline(ui, text, InlineStyle::Body, theme, zoom_factor);
                ui.add_space(scale_spacing(BLOCK_SPACING_PARAGRAPH, zoom_factor));
            }
            Block::UnorderedList(items) => {
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
            }
            Block::OrderedList { start, items } => {
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
            }
            Block::BlockQuote(lines) => render_blockquote(ui, lines, theme, zoom_factor),
            Block::CodeBlock { language, code } => {
                render_code_block(
                    ui,
                    block_index,
                    ui_language,
                    language.as_deref(),
                    code,
                    theme,
                    zoom_factor,
                )
            }
        }
    }

    did_scroll
}

fn render_heading(
    ui: &mut Ui,
    level: HeadingLevel,
    content: &InlineContent,
    theme: &Theme,
    zoom_factor: f32,
    scroll_to_heading: Option<usize>,
    block_index: usize,
) -> bool {
    let size = match level {
        HeadingLevel::H1 => 31.0,
        HeadingLevel::H2 => 26.0,
        HeadingLevel::H3 => 22.0,
        HeadingLevel::H4 => 19.0,
        HeadingLevel::H5 => 17.0,
        HeadingLevel::H6 => 16.0,
    } * zoom_factor;

    let anchor = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());

    if scroll_to_heading == Some(block_index) {
        ui.scroll_to_rect(anchor.rect, Some(Align::TOP));
    }

    render_inline(ui, content, InlineStyle::Heading(size), theme, zoom_factor);
    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
    scroll_to_heading == Some(block_index)
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

fn render_code_block(
    ui: &mut Ui,
    block_index: usize,
    ui_language: Language,
    language: Option<&str>,
    code: &str,
    theme: &Theme,
    zoom_factor: f32,
) {
    let highlighted = highlight_code(
        language,
        code,
        theme.is_dark,
        INLINE_CODE_TEXT_SIZE * zoom_factor,
    );

    let response = Frame::new()
        .fill(theme.code_background)
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(scale_margin(8, zoom_factor) as u8))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(16, zoom_factor),
            scale_margin(10, zoom_factor),
        ))
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                render_code_block_header(
                    ui,
                    block_index,
                    ui_language,
                    language,
                    code,
                    theme,
                    zoom_factor,
                );
                ui.add_space(scale_spacing(6.0, zoom_factor));

                ScrollArea::horizontal()
                    .id_salt(("code_block_scroll", block_index))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

                        if let Some(job) = highlighted {
                            ui.add(Label::new(job).wrap_mode(TextWrapMode::Extend));
                        } else {
                            ui.add(
                                Label::new(
                                    RichText::new(code)
                                        .font(FontId::new(
                                            INLINE_CODE_TEXT_SIZE * zoom_factor,
                                            FontFamily::Monospace,
                                        ))
                                        .color(theme.text_primary),
                                )
                                .wrap_mode(TextWrapMode::Extend),
                            );
                        }
                    });
            });
        })
        .response;

    if let Some(language) = language {
        response.on_hover_text(language);
    }

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

fn render_code_block_header(
    ui: &mut Ui,
    block_index: usize,
    ui_language: Language,
    language: Option<&str>,
    code: &str,
    theme: &Theme,
    zoom_factor: f32,
) {
    let feedback_id = ui.make_persistent_id("code_block_copy_feedback");
    let copied = ui
        .ctx()
        .data(|data| data.get_temp::<CopyFeedbackState>(feedback_id));
    let now = ui.ctx().input(|input| input.time);
    let show_copied = copied
        .filter(|copied| copied.block_index == block_index)
        .map(|copied| now - copied.copied_at < COPY_FEEDBACK_DURATION_SECONDS)
        .unwrap_or(false);

    if show_copied {
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(COPY_FEEDBACK_DURATION_SECONDS));
    }

    ui.horizontal(|ui| {
        if let Some(language) = language.filter(|language| !language.trim().is_empty()) {
            render_code_language_label(ui, language, theme, zoom_factor);
        }

        ui.add_space(ui.available_width());

        ui.allocate_ui_with_layout(
            egui::vec2(COPY_FEEDBACK_SLOT_WIDTH * zoom_factor, 0.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                if show_copied {
                    ui.label(
                        RichText::new(tr(ui_language, "message.copied"))
                            .size(CODE_LANGUAGE_TEXT_SIZE * zoom_factor)
                            .color(theme.text_secondary),
                    );
                }
            },
        );

        ui.add_space(scale_spacing(8.0, zoom_factor));

        if ui.button(tr(ui_language, "action.copy")).clicked() {
            let copied_at = ui.ctx().input(|input| input.time);
            ui.ctx().copy_text(code.to_owned());
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    feedback_id,
                    CopyFeedbackState {
                        block_index,
                        copied_at,
                    },
                );
            });
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(COPY_FEEDBACK_DURATION_SECONDS));
        }
    });
}

fn render_code_language_label(ui: &mut Ui, language: &str, theme: &Theme, zoom_factor: f32) {
    Frame::new()
        .fill(theme.top_bar_background)
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(scale_margin(6, zoom_factor) as u8))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(8, zoom_factor),
            scale_margin(4, zoom_factor),
        ))
        .show(ui, |ui| {
            ui.label(
                RichText::new(language.trim())
                    .size(CODE_LANGUAGE_TEXT_SIZE * zoom_factor)
                    .strong()
                    .color(theme.text_secondary),
            );
        });
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
