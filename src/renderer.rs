use eframe::egui::{self, Color32, Frame, RichText, Stroke, Ui};
use pulldown_cmark::HeadingLevel;

use crate::parser::{Block, MarkdownDocument};

pub fn render_markdown_document(ui: &mut Ui, document: &MarkdownDocument) {
    for block in &document.blocks {
        match block {
            Block::Heading { level, text } => render_heading(ui, *level, text),
            Block::Paragraph(text) => {
                ui.label(text);
                ui.add_space(12.0);
            }
            Block::UnorderedList(items) => {
                for item in items {
                    ui.label(format!("- {}", item));
                }
                ui.add_space(12.0);
            }
            Block::OrderedList { start, items } => {
                for (index, item) in items.iter().enumerate() {
                    ui.label(format!("{}. {}", start + index as u64, item));
                }
                ui.add_space(12.0);
            }
            Block::BlockQuote(lines) => render_blockquote(ui, lines),
            Block::CodeBlock { language, code } => render_code_block(ui, language.as_deref(), code),
        }
    }
}

fn render_heading(ui: &mut Ui, level: HeadingLevel, text: &str) {
    let size = match level {
        HeadingLevel::H1 => 28.0,
        HeadingLevel::H2 => 24.0,
        HeadingLevel::H3 => 20.0,
        HeadingLevel::H4 => 18.0,
        HeadingLevel::H5 => 16.0,
        HeadingLevel::H6 => 15.0,
    };

    ui.label(RichText::new(text).size(size).strong());
    ui.add_space(10.0);
}

fn render_blockquote(ui: &mut Ui, lines: &[String]) {
    Frame::new()
        .fill(Color32::from_rgb(245, 245, 245))
        .stroke(Stroke::new(1.0, Color32::from_rgb(200, 200, 200)))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            for line in lines {
                ui.label(RichText::new(line).italics());
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
