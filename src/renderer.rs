use eframe::egui::{self, Align, FontFamily, FontId, Frame, RichText, Stroke, Ui};
use pulldown_cmark::{Alignment, HeadingLevel};

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
const TABLE_CELL_MIN_WIDTH: f32 = 120.0;

pub fn render_markdown_document(
    ui: &mut Ui,
    document: &MarkdownDocument,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
    scroll_to_block: Option<usize>,
    search_query: Option<&str>,
) -> RenderOutcome {
    let mut did_scroll = false;
    let mut active_heading = None;
    let mut clicked_anchor = None;
    let viewport_top = ui.clip_rect().top();

    for (block_index, block) in document.blocks.iter().enumerate() {
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
                    search_query,
                    &mut clicked_anchor,
                );
                did_scroll |= heading_state.did_scroll;

                if heading_state.is_active {
                    active_heading = Some(block_index);
                }
            }
            Block::Paragraph(text) => {
                render_inline(
                    ui,
                    text,
                    InlineStyle::Body,
                    theme,
                    zoom_factor,
                    search_query,
                    &mut clicked_anchor,
                );
                ui.add_space(scale_spacing(BLOCK_SPACING_PARAGRAPH, zoom_factor));

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::UnorderedList(items) => {
                for item in items {
                    render_list_item(
                        ui,
                        RichText::new("- ").color(theme.text_secondary),
                        item,
                        theme,
                        zoom_factor,
                        search_query,
                        &mut clicked_anchor,
                    );
                    ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                }
                ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
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
                        search_query,
                        &mut clicked_anchor,
                    );
                    ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                }
                ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::BlockQuote(lines) => {
                render_blockquote(
                    ui,
                    lines,
                    theme,
                    zoom_factor,
                    search_query,
                    &mut clicked_anchor,
                );
                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::CodeBlock { language, code } => {
                render_code_block(
                    ui,
                    block_index,
                    ui_language,
                    language.as_deref(),
                    code,
                    theme,
                    zoom_factor,
                );

                if scroll_to_block == Some(block_index) {
                    ui.scroll_to_cursor(Some(Align::TOP));
                    did_scroll = true;
                }
            }
            Block::Table {
                alignments,
                headers,
                rows,
            } => {
                render_table(
                    ui,
                    block_index,
                    alignments,
                    headers,
                    rows,
                    theme,
                    zoom_factor,
                    search_query,
                    &mut clicked_anchor,
                );

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
        clicked_anchor,
    }
}

pub struct RenderOutcome {
    pub did_scroll: bool,
    pub active_heading: Option<usize>,
    pub clicked_anchor: Option<String>,
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
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
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

    let heading_response = ui.scope(|ui| {
        render_inline(
            ui,
            content,
            InlineStyle::Heading(size),
            theme,
            zoom_factor,
            search_query,
            clicked_anchor,
        );
        ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
    });

    let heading_rect = anchor.rect.union(heading_response.response.rect);
    let is_active = heading_rect.top() <= viewport_top + scale_spacing(8.0, zoom_factor);

    HeadingRenderState {
        did_scroll: scroll_to_block == Some(block_index),
        is_active,
    }
}

fn render_list_item(
    ui: &mut Ui,
    marker: RichText,
    item: &InlineContent,
    theme: &Theme,
    zoom_factor: f32,
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [scale_spacing(24.0, zoom_factor), 0.0],
            egui::Label::new(marker.size(BODY_TEXT_SIZE * zoom_factor)),
        );

        ui.vertical(|ui| {
            render_inline(
                ui,
                item,
                InlineStyle::Body,
                theme,
                zoom_factor,
                search_query,
                clicked_anchor,
            );
        });
    });
}

fn render_blockquote(
    ui: &mut Ui,
    lines: &[InlineContent],
    theme: &Theme,
    zoom_factor: f32,
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
) {
    Frame::new()
        .fill(theme.quote_background)
        .stroke(Stroke::new(1.0, theme.quote_border))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(16, zoom_factor),
            scale_margin(14, zoom_factor),
        ))
        .show(ui, |ui| {
            for line in lines {
                render_inline(
                    ui,
                    line,
                    InlineStyle::Quote,
                    theme,
                    zoom_factor,
                    search_query,
                    clicked_anchor,
                );
                ui.add_space(scale_spacing(6.0, zoom_factor));
            }
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

fn render_table(
    ui: &mut Ui,
    block_index: usize,
    _alignments: &[Alignment],
    headers: &[InlineContent],
    rows: &[Vec<InlineContent>],
    theme: &Theme,
    zoom_factor: f32,
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
) {
    let column_count = table_column_count(headers, rows);
    if column_count == 0 {
        return;
    }

    Frame::new()
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .id_salt(("table_scroll", block_index))
                .show(ui, |ui| {
                    egui::Grid::new(("markdown_table", block_index))
                        .num_columns(column_count)
                        .spacing(egui::vec2(16.0, 10.0))
                        .striped(true)
                        .show(ui, |ui| {
                            render_table_row(
                                ui,
                                headers,
                                column_count,
                                InlineStyle::TableHeader,
                                theme,
                                zoom_factor,
                                search_query,
                                clicked_anchor,
                            );

                            for row in rows {
                                render_table_row(
                                    ui,
                                    row,
                                    column_count,
                                    InlineStyle::TableCell,
                                    theme,
                                    zoom_factor,
                                    search_query,
                                    clicked_anchor,
                                );
                            }
                        });
                });
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

fn render_table_row(
    ui: &mut Ui,
    cells: &[InlineContent],
    column_count: usize,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
) {
    for column_index in 0..column_count {
        let cell = cells.get(column_index);
        let width = scale_spacing(TABLE_CELL_MIN_WIDTH, zoom_factor);

        ui.vertical(|ui| {
            ui.set_min_width(width);
            if let Some(cell) = cell {
                render_inline(
                    ui,
                    cell,
                    style,
                    theme,
                    zoom_factor,
                    search_query,
                    clicked_anchor,
                );
            }
        });
    }

    ui.end_row();
}

fn table_column_count(headers: &[InlineContent], rows: &[Vec<InlineContent>]) -> usize {
    rows.iter()
        .map(Vec::len)
        .chain(std::iter::once(headers.len()))
        .max()
        .unwrap_or(0)
}

#[derive(Clone, Copy)]
enum InlineStyle {
    Body,
    Quote,
    Heading(f32),
    TableHeader,
    TableCell,
}

fn render_inline(
    ui: &mut Ui,
    content: &InlineContent,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
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
                render_inline_span(
                    ui,
                    span,
                    style,
                    theme,
                    zoom_factor,
                    search_query,
                    clicked_anchor,
                );
            }
        });
    }
}

#[derive(Clone, Copy)]
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
    search_query: Option<&str>,
    clicked_anchor: &mut Option<String>,
) {
    match span {
        InlineSpan::Text(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Plain,
            theme,
            zoom_factor,
            search_query,
        ),
        InlineSpan::Strong(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Strong,
            theme,
            zoom_factor,
            search_query,
        ),
        InlineSpan::Emphasis(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Emphasis,
            theme,
            zoom_factor,
            search_query,
        ),
        InlineSpan::Code(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Code,
            theme,
            zoom_factor,
            search_query,
        ),
        InlineSpan::Link { text, destination } => {
            let rich_text = styled_text(text, style, SpanKind::Link, theme, zoom_factor)
                .background_color(search_highlight_for_text(text, theme, search_query));
            if let Some(anchor) = internal_anchor(destination) {
                if ui.link(rich_text).clicked() {
                    *clicked_anchor = Some(anchor.to_owned());
                }
            } else {
                ui.hyperlink_to(rich_text, destination);
            }
        }
        InlineSpan::LineBreak => {}
    }
}

fn internal_anchor(destination: &str) -> Option<&str> {
    let trimmed = destination.trim();
    trimmed
        .strip_prefix('#')
        .filter(|anchor| !anchor.trim().is_empty())
}

fn render_text_label(
    ui: &mut Ui,
    text: &str,
    style: InlineStyle,
    kind: SpanKind,
    theme: &Theme,
    zoom_factor: f32,
    search_query: Option<&str>,
) {
    if text.is_empty() {
        return;
    }

    let segments = split_highlighted_segments(text, search_query);

    if segments.len() == 1 && !segments[0].is_match {
        ui.label(styled_text(text, style, kind, theme, zoom_factor));
        return;
    }

    for segment in segments {
        let mut rich_text = styled_text(segment.text, style, kind, theme, zoom_factor);

        if segment.is_match {
            rich_text = rich_text.background_color(search_highlight_color(theme));
        }

        ui.label(rich_text);
    }
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
        InlineStyle::TableHeader => RichText::new(text)
            .size(BODY_TEXT_SIZE * zoom_factor)
            .color(theme.text_primary)
            .strong(),
        InlineStyle::TableCell => RichText::new(text)
            .size(BODY_TEXT_SIZE * zoom_factor)
            .color(theme.text_primary),
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

struct HighlightSegment<'a> {
    text: &'a str,
    is_match: bool,
}

fn split_highlighted_segments<'a>(
    text: &'a str,
    search_query: Option<&str>,
) -> Vec<HighlightSegment<'a>> {
    let Some(query) = normalized_search_query(search_query) else {
        return vec![HighlightSegment {
            text,
            is_match: false,
        }];
    };

    let normalized_text = text.to_lowercase();
    let mut segments = Vec::new();
    let mut current_start = 0usize;
    let mut search_start = 0usize;

    while let Some(relative_match_start) = normalized_text[search_start..].find(query) {
        let match_start = search_start + relative_match_start;
        let match_end = match_start + query.len();

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
        search_start = match_end;
    }

    if current_start < text.len() {
        segments.push(HighlightSegment {
            text: &text[current_start..],
            is_match: false,
        });
    }

    if segments.is_empty() {
        vec![HighlightSegment {
            text,
            is_match: false,
        }]
    } else {
        segments
    }
}

fn normalized_search_query(search_query: Option<&str>) -> Option<&str> {
    search_query
        .map(str::trim)
        .filter(|query| !query.is_empty())
}

fn search_highlight_for_text(
    text: &str,
    theme: &Theme,
    search_query: Option<&str>,
) -> egui::Color32 {
    match normalized_search_query(search_query) {
        Some(query) if text.to_lowercase().contains(query) => search_highlight_color(theme),
        _ => egui::Color32::TRANSPARENT,
    }
}

fn search_highlight_color(theme: &Theme) -> egui::Color32 {
    theme
        .status_loading_background
        .gamma_multiply(if theme.is_dark { 0.65 } else { 0.9 })
}
