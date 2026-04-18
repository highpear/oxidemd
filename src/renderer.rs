use eframe::egui::{self, Color32, FontFamily, FontId, Frame, RichText, Stroke, Ui};
use pulldown_cmark::HeadingLevel;

use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};

pub fn render_markdown_document(ui: &mut Ui, document: &MarkdownDocument) {
    for block in &document.blocks {
        match block {
            Block::Heading { level, content } => render_heading(ui, *level, content),
            Block::Paragraph(text) => {
                render_inline(ui, text, InlineStyle::Body);
                ui.add_space(12.0);
            }
            Block::UnorderedList(items) => {
                for item in items {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("- ");
                        render_inline(ui, item, InlineStyle::Body);
                    });
                }
                ui.add_space(12.0);
            }
            Block::OrderedList { start, items } => {
                for (index, item) in items.iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{}. ", start + index as u64));
                        render_inline(ui, item, InlineStyle::Body);
                    });
                }
                ui.add_space(12.0);
            }
            Block::BlockQuote(lines) => render_blockquote(ui, lines),
            Block::CodeBlock { language, code } => render_code_block(ui, language.as_deref(), code),
        }
    }
}

fn render_heading(ui: &mut Ui, level: HeadingLevel, content: &InlineContent) {
    let size = match level {
        HeadingLevel::H1 => 28.0,
        HeadingLevel::H2 => 24.0,
        HeadingLevel::H3 => 20.0,
        HeadingLevel::H4 => 18.0,
        HeadingLevel::H5 => 16.0,
        HeadingLevel::H6 => 15.0,
    };

    render_inline(ui, content, InlineStyle::Heading(size));
    ui.add_space(10.0);
}

fn render_blockquote(ui: &mut Ui, lines: &[InlineContent]) {
    Frame::new()
        .fill(Color32::from_rgb(245, 245, 245))
        .stroke(Stroke::new(1.0, Color32::from_rgb(200, 200, 200)))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            for line in lines {
                render_inline(ui, line, InlineStyle::Quote);
            }
        });

    ui.add_space(12.0);
}

fn render_code_block(ui: &mut Ui, language: Option<&str>, code: &str) {
    Frame::new()
        .fill(Color32::from_rgb(248, 248, 248))
        .stroke(Stroke::new(1.0, Color32::from_rgb(220, 220, 220)))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            if let Some(language) = language {
                ui.label(RichText::new(language).small().strong());
                ui.add_space(4.0);
            }

            ui.label(RichText::new(code).monospace());
        });

    ui.add_space(12.0);
}

#[derive(Clone, Copy)]
enum InlineStyle {
    Body,
    Quote,
    Heading(f32),
}

fn render_inline(ui: &mut Ui, content: &InlineContent, style: InlineStyle) {
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
                render_inline_span(ui, span, style);
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

fn render_inline_span(ui: &mut Ui, span: &InlineSpan, style: InlineStyle) {
    match span {
        InlineSpan::Text(text) => render_text_label(ui, text, style, SpanKind::Plain),
        InlineSpan::Strong(text) => render_text_label(ui, text, style, SpanKind::Strong),
        InlineSpan::Emphasis(text) => render_text_label(ui, text, style, SpanKind::Emphasis),
        InlineSpan::Code(text) => render_text_label(ui, text, style, SpanKind::Code),
        InlineSpan::Link { text, destination } => {
            let rich_text = styled_text(text, style, SpanKind::Link);
            ui.hyperlink_to(rich_text, destination);
        }
        InlineSpan::LineBreak => {}
    }
}

fn render_text_label(ui: &mut Ui, text: &str, style: InlineStyle, kind: SpanKind) {
    if text.is_empty() {
        return;
    }

    ui.label(styled_text(text, style, kind));
}

fn styled_text(text: &str, style: InlineStyle, kind: SpanKind) -> RichText {
    let mut rich_text = match style {
        InlineStyle::Body => RichText::new(text)
            .size(16.0)
            .color(Color32::from_rgb(30, 30, 30)),
        InlineStyle::Quote => RichText::new(text)
            .size(16.0)
            .color(Color32::from_rgb(70, 70, 70))
            .italics(),
        InlineStyle::Heading(size) => RichText::new(text)
            .size(size)
            .color(Color32::from_rgb(20, 20, 20))
            .strong(),
    };

    match kind {
        SpanKind::Plain => rich_text,
        SpanKind::Strong => rich_text.strong(),
        SpanKind::Emphasis => rich_text.italics(),
        SpanKind::Code => {
            let font_size = match style {
                InlineStyle::Heading(size) => (size - 1.0).max(15.0),
                _ => 15.0,
            };

            rich_text = rich_text
                .family(FontFamily::Monospace)
                .font(FontId::new(font_size, FontFamily::Monospace))
                .background_color(Color32::from_rgb(240, 240, 240));

            rich_text
        }
        SpanKind::Link => {
            rich_text = rich_text.color(Color32::from_rgb(0, 92, 197)).underline();

            rich_text
        }
    }
}
