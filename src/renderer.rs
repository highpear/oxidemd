use std::path::{Path, PathBuf};

use eframe::egui::{self, Align, FontFamily, FontId, Frame, RichText, Stroke, Ui, WidgetText};
use pulldown_cmark::{Alignment, HeadingLevel};

use crate::code_block::render_code_block;
use crate::i18n::{Language, TranslationKey, tr};
use crate::image_cache::{ImageCache, ImageLoadState};
use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};
use crate::search::{split_highlighted_segments, text_matches_query};
use crate::theme::Theme;

const BODY_TEXT_SIZE: f32 = 17.0;
const QUOTE_TEXT_SIZE: f32 = 16.0;
const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
const BLOCK_SPACING_PARAGRAPH: f32 = 18.0;
const BLOCK_SPACING_SECTION: f32 = 24.0;
const LIST_ITEM_SPACING: f32 = 8.0;
const TABLE_CELL_MIN_WIDTH: f32 = 120.0;
const LARGE_DOCUMENT_BLOCK_THRESHOLD: usize = 2_000;
const VIRTUAL_RENDER_OVERSCAN: f32 = 1_200.0;
const ESTIMATED_CHARS_PER_LINE: usize = 90;

pub fn render_markdown_document(
    ui: &mut Ui,
    document: &MarkdownDocument,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
    document_base_dir: Option<&Path>,
    image_cache: &mut ImageCache,
    block_heights: &mut [Option<f32>],
    scroll_to_block: Option<usize>,
    search_query: Option<&str>,
    active_search_block: Option<usize>,
) -> RenderOutcome {
    let mut did_scroll = false;
    let mut needs_scroll_stabilization = false;
    let mut active_heading = None;
    let mut link_actions = LinkActions::default();
    let viewport_top = ui.clip_rect().top();
    let viewport_bottom = ui.clip_rect().bottom();
    let mut image_resources = ImageRenderResources {
        ui_language,
        document_base_dir,
        image_cache,
    };

    for (block_index, block) in document.blocks.iter().enumerate() {
        let search_highlight = SearchHighlight {
            query: search_query,
            is_active_block: active_search_block == Some(block_index),
        };
        let measured_block_height = block_heights.get(block_index).and_then(|height| *height);
        let block_height =
            measured_block_height.unwrap_or_else(|| estimate_block_height(block, zoom_factor));
        let block_top = ui.cursor().top();
        let block_bottom = block_top + block_height;

        if should_skip_block(
            document.blocks.len(),
            scroll_to_block,
            block_index,
            block_top,
            block_bottom,
            viewport_top,
            viewport_bottom,
        ) {
            if matches!(block, Block::Heading { .. })
                && block_top <= viewport_top + scale_spacing(8.0, zoom_factor)
            {
                active_heading = Some(block_index);
            }

            ui.add_space(block_height);
            continue;
        }

        let measured_top = ui.cursor().top();

        if scroll_to_block == Some(block_index) {
            let anchor = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());
            ui.scroll_to_rect(anchor.rect, Some(Align::TOP));
            did_scroll = true;
            needs_scroll_stabilization = measured_block_height.is_none();
        }

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
                    search_highlight,
                    &mut link_actions,
                    &mut image_resources,
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
                    search_highlight,
                    &mut link_actions,
                    &mut image_resources,
                );
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
                        search_highlight,
                        &mut link_actions,
                        &mut image_resources,
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
                        search_highlight,
                        &mut link_actions,
                        &mut image_resources,
                    );
                    ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                }
                ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
            }
            Block::BlockQuote(lines) => {
                render_blockquote(
                    ui,
                    lines,
                    theme,
                    zoom_factor,
                    search_highlight,
                    &mut link_actions,
                    &mut image_resources,
                );
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
                    search_highlight,
                    &mut link_actions,
                    &mut image_resources,
                );
            }
        }

        if let Some(height) = block_heights.get_mut(block_index) {
            let measured_height = (ui.cursor().top() - measured_top).max(0.0);
            *height = Some(measured_height);
        }
    }

    RenderOutcome {
        did_scroll,
        needs_scroll_stabilization,
        active_heading,
        clicked_anchor: link_actions.clicked_anchor,
        clicked_external_link: link_actions.clicked_external_link,
    }
}

fn should_skip_block(
    block_count: usize,
    scroll_to_block: Option<usize>,
    block_index: usize,
    block_top: f32,
    block_bottom: f32,
    viewport_top: f32,
    viewport_bottom: f32,
) -> bool {
    if block_count < LARGE_DOCUMENT_BLOCK_THRESHOLD || scroll_to_block == Some(block_index) {
        return false;
    }

    block_bottom < viewport_top - VIRTUAL_RENDER_OVERSCAN
        || block_top > viewport_bottom + VIRTUAL_RENDER_OVERSCAN
}

fn estimate_block_height(block: &Block, zoom_factor: f32) -> f32 {
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
            | InlineSpan::Code(text) => {
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

pub struct RenderOutcome {
    pub did_scroll: bool,
    pub needs_scroll_stabilization: bool,
    pub active_heading: Option<usize>,
    pub clicked_anchor: Option<String>,
    pub clicked_external_link: Option<String>,
}

#[derive(Clone, Copy)]
struct SearchHighlight<'a> {
    query: Option<&'a str>,
    is_active_block: bool,
}

struct ImageRenderResources<'a> {
    ui_language: Language,
    document_base_dir: Option<&'a Path>,
    image_cache: &'a mut ImageCache,
}

#[derive(Default)]
struct LinkActions {
    clicked_anchor: Option<String>,
    clicked_external_link: Option<String>,
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
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
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
            search_highlight,
            link_actions,
            image_resources,
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
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
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
                search_highlight,
                link_actions,
                image_resources,
            );
        });
    });
}

fn render_blockquote(
    ui: &mut Ui,
    lines: &[InlineContent],
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
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
                    search_highlight,
                    link_actions,
                    image_resources,
                );
                ui.add_space(scale_spacing(6.0, zoom_factor));
            }
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

fn render_table(
    ui: &mut Ui,
    block_index: usize,
    alignments: &[Alignment],
    headers: &[InlineContent],
    rows: &[Vec<InlineContent>],
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
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
                                alignments,
                                InlineStyle::TableHeader,
                                theme,
                                zoom_factor,
                                search_highlight,
                                link_actions,
                                image_resources,
                            );

                            for row in rows {
                                render_table_row(
                                    ui,
                                    row,
                                    column_count,
                                    alignments,
                                    InlineStyle::TableCell,
                                    theme,
                                    zoom_factor,
                                    search_highlight,
                                    link_actions,
                                    image_resources,
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
    alignments: &[Alignment],
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
) {
    for column_index in 0..column_count {
        let cell = cells.get(column_index);
        let alignment = alignments
            .get(column_index)
            .copied()
            .unwrap_or(Alignment::None);
        let width = scale_spacing(TABLE_CELL_MIN_WIDTH, zoom_factor);

        let cell_frame = match style {
            InlineStyle::TableHeader => Frame::new()
                .fill(theme.widget_inactive_background)
                .inner_margin(egui::Margin::symmetric(8, 6)),
            InlineStyle::TableCell => Frame::new().inner_margin(egui::Margin::symmetric(8, 4)),
            _ => Frame::new(),
        };

        cell_frame.show(ui, |ui| {
            ui.set_min_width(width);
            render_aligned_cell(
                ui,
                cell,
                alignment,
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                image_resources,
            );
        });
    }

    ui.end_row();
}

fn render_aligned_cell(
    ui: &mut Ui,
    cell: Option<&InlineContent>,
    alignment: Alignment,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
) {
    let Some(cell) = cell else {
        ui.label("");
        return;
    };

    match alignment {
        Alignment::Center | Alignment::Right => render_inline_aligned(
            ui,
            cell,
            alignment,
            style,
            theme,
            zoom_factor,
            search_highlight,
            link_actions,
            image_resources,
        ),
        Alignment::None | Alignment::Left => {
            render_inline(
                ui,
                cell,
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                image_resources,
            );
        }
    }
}

fn render_inline_aligned(
    ui: &mut Ui,
    content: &InlineContent,
    alignment: Alignment,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
) {
    let available_width = ui.available_width();
    let line_width = inline_content_width(ui, content, style, theme, zoom_factor, search_highlight);
    let leading_space = match alignment {
        Alignment::Center => ((available_width - line_width) / 2.0).max(0.0),
        Alignment::Right => (available_width - line_width).max(0.0),
        Alignment::None | Alignment::Left => 0.0,
    };

    ui.horizontal(|ui| {
        ui.add_space(leading_space);
        for span in &content.spans {
            if matches!(span, InlineSpan::LineBreak) {
                continue;
            }

            render_inline_span(
                ui,
                span,
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                image_resources,
            );
        }
    });
}

fn inline_content_width(
    ui: &mut Ui,
    content: &InlineContent,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
) -> f32 {
    let mut width = 0.0;

    for span in &content.spans {
        if matches!(span, InlineSpan::LineBreak) {
            continue;
        }

        width += inline_span_width(ui, span, style, theme, zoom_factor, search_highlight);
        width += ui.spacing().item_spacing.x;
    }

    width
}

fn inline_span_width(
    ui: &mut Ui,
    span: &InlineSpan,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
) -> f32 {
    match span {
        InlineSpan::Text(text) => text_width(ui, text, style, SpanKind::Plain, theme, zoom_factor),
        InlineSpan::Strong(text) => {
            text_width(ui, text, style, SpanKind::Strong, theme, zoom_factor)
        }
        InlineSpan::Emphasis(text) => {
            text_width(ui, text, style, SpanKind::Emphasis, theme, zoom_factor)
        }
        InlineSpan::Code(text) => text_width(ui, text, style, SpanKind::Code, theme, zoom_factor),
        InlineSpan::Link { text, .. } => {
            text_width(ui, text, style, SpanKind::Link, theme, zoom_factor)
        }
        InlineSpan::Image { .. } => 0.0,
        InlineSpan::LineBreak => 0.0,
    }
    .max(highlighted_text_width(
        ui,
        span,
        style,
        theme,
        zoom_factor,
        search_highlight,
    ))
}

fn highlighted_text_width(
    ui: &mut Ui,
    span: &InlineSpan,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
) -> f32 {
    if search_highlight.query.is_none() {
        return 0.0;
    }

    let text = match span {
        InlineSpan::Text(text)
        | InlineSpan::Strong(text)
        | InlineSpan::Emphasis(text)
        | InlineSpan::Code(text) => text.as_str(),
        InlineSpan::Link { text, .. } => text.as_str(),
        InlineSpan::Image { alt, .. } => alt.as_str(),
        InlineSpan::LineBreak => return 0.0,
    };

    split_highlighted_segments(
        text,
        search_highlight.query,
        search_highlight.is_active_block,
    )
    .iter()
    .map(|segment| text_width(ui, segment.text, style, SpanKind::Plain, theme, zoom_factor))
    .sum()
}

fn text_width(
    ui: &mut Ui,
    text: &str,
    style: InlineStyle,
    kind: SpanKind,
    theme: &Theme,
    zoom_factor: f32,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    let rich_text = styled_text(text, style, kind, theme, zoom_factor);
    WidgetText::from(rich_text)
        .into_galley(
            ui,
            None,
            f32::INFINITY,
            FontId::proportional(BODY_TEXT_SIZE),
        )
        .size()
        .x
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
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
) {
    if !content
        .spans
        .iter()
        .any(|span| matches!(span, InlineSpan::LineBreak))
    {
        ui.horizontal_wrapped(|ui| {
            for span in &content.spans {
                render_inline_span(
                    ui,
                    span,
                    style,
                    theme,
                    zoom_factor,
                    search_highlight,
                    link_actions,
                    image_resources,
                );
            }
        });
        return;
    }

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
                    search_highlight,
                    link_actions,
                    image_resources,
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
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    image_resources: &mut ImageRenderResources<'_>,
) {
    match span {
        InlineSpan::Text(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Plain,
            theme,
            zoom_factor,
            search_highlight,
        ),
        InlineSpan::Strong(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Strong,
            theme,
            zoom_factor,
            search_highlight,
        ),
        InlineSpan::Emphasis(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Emphasis,
            theme,
            zoom_factor,
            search_highlight,
        ),
        InlineSpan::Code(text) => render_text_label(
            ui,
            text,
            style,
            SpanKind::Code,
            theme,
            zoom_factor,
            search_highlight,
        ),
        InlineSpan::Link { text, destination } => {
            let rich_text = styled_text(text, style, SpanKind::Link, theme, zoom_factor)
                .background_color(search_highlight_for_text(text, theme, search_highlight));
            if let Some(anchor) = internal_anchor(destination) {
                if ui.link(rich_text).clicked() {
                    link_actions.clicked_anchor = Some(anchor.to_owned());
                }
            } else if ui.link(rich_text).on_hover_text(destination).clicked() {
                link_actions.clicked_external_link = Some(destination.to_owned());
            }
        }
        InlineSpan::Image { alt, destination } => {
            render_image_span(ui, alt, destination, theme, zoom_factor, image_resources)
        }
        InlineSpan::LineBreak => {}
    }
}

fn render_image_span(
    ui: &mut Ui,
    alt: &str,
    destination: &str,
    theme: &Theme,
    zoom_factor: f32,
    image_resources: &mut ImageRenderResources<'_>,
) {
    let Some(path) = resolve_local_image_path(image_resources.document_base_dir, destination)
    else {
        render_image_message(
            ui,
            tr(
                image_resources.ui_language,
                TranslationKey::MessageImageUnsupported,
            ),
            destination,
            theme,
            zoom_factor,
        );
        return;
    };

    match image_resources.image_cache.load(ui.ctx(), &path) {
        ImageLoadState::Loaded(texture) => {
            let max_width = ui.available_width().max(120.0);
            ui.add(
                egui::Image::from_texture(texture)
                    .max_width(max_width)
                    .fit_to_original_size(zoom_factor)
                    .alt_text(alt),
            );
        }
        ImageLoadState::Failed(error) => {
            let detail = if alt.trim().is_empty() {
                error
            } else {
                alt.trim()
            };
            render_image_message(
                ui,
                tr(
                    image_resources.ui_language,
                    TranslationKey::MessageImageLoadFailed,
                ),
                detail,
                theme,
                zoom_factor,
            );
        }
    }
}

fn resolve_local_image_path(base_dir: Option<&Path>, destination: &str) -> Option<PathBuf> {
    let cleaned = destination.trim();
    if cleaned.is_empty() || is_remote_or_data_uri(cleaned) {
        return None;
    }

    let without_fragment = cleaned.split('#').next().unwrap_or(cleaned);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    let path = Path::new(without_query);

    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        base_dir.map(|base_dir| base_dir.join(path))
    }
}

fn is_remote_or_data_uri(destination: &str) -> bool {
    let normalized = destination.trim().to_ascii_lowercase();
    normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("data:")
}

fn render_image_message(ui: &mut Ui, prefix: &str, detail: &str, theme: &Theme, zoom_factor: f32) {
    Frame::new()
        .fill(theme.widget_inactive_background)
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("{} {}", prefix, detail))
                    .size(QUOTE_TEXT_SIZE * zoom_factor)
                    .color(theme.text_secondary),
            );
        });
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
    search_highlight: SearchHighlight<'_>,
) {
    if text.is_empty() {
        return;
    }

    if search_highlight.query.is_none() {
        ui.label(styled_text(text, style, kind, theme, zoom_factor));
        return;
    }

    let segments = split_highlighted_segments(
        text,
        search_highlight.query,
        search_highlight.is_active_block,
    );

    if segments.len() == 1 && !segments[0].is_match {
        ui.label(styled_text(text, style, kind, theme, zoom_factor));
        return;
    }

    for segment in segments {
        let mut rich_text = styled_text(segment.text, style, kind, theme, zoom_factor);

        if segment.is_match {
            rich_text =
                rich_text.background_color(search_highlight_color(theme, segment.is_active_match));
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

fn search_highlight_for_text(
    text: &str,
    theme: &Theme,
    search_highlight: SearchHighlight<'_>,
) -> egui::Color32 {
    if text_matches_query(text, search_highlight.query) {
        search_highlight_color(theme, search_highlight.is_active_block)
    } else {
        egui::Color32::TRANSPARENT
    }
}

fn search_highlight_color(theme: &Theme, is_active_match: bool) -> egui::Color32 {
    if is_active_match {
        theme.search_active_match_background
    } else {
        theme.search_match_background
    }
}
