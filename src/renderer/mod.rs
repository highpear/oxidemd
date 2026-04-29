use eframe::egui::{self, Align, FontFamily, FontId, RichText, Ui, WidgetText};
use std::path::Path;

use crate::code_block::render_code_block;
use crate::diagram::DiagramRenderCache;
use crate::i18n::Language;
use crate::image_cache::ImageCache;
use crate::math::{MathRenderCache, MathRenderMode, PreparedMath};
use crate::parser::{Block, InlineContent, InlineSpan, MarkdownDocument};
use crate::search::{for_each_highlighted_segment, text_matches_query};
use crate::theme::Theme;

mod blocks;
mod embedded;
mod image;
mod sizing;

pub use sizing::estimate_document_block_heights;

use sizing::{estimate_block_height, scale_spacing};

use blocks::{render_blockquote, render_heading, render_list_item, render_table};
use embedded::{
    fit_inline_math_size, render_diagram_block, render_inline_math_image,
    render_inline_math_placeholder, render_math_block,
};
use image::render_image_span;

pub(super) const BODY_TEXT_SIZE: f32 = 17.0;
pub(super) const QUOTE_TEXT_SIZE: f32 = 16.0;
pub(super) const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
pub(super) const BLOCK_SPACING_PARAGRAPH: f32 = 18.0;
pub(super) const BLOCK_SPACING_SECTION: f32 = 24.0;
pub(super) const LIST_ITEM_SPACING: f32 = 8.0;
pub(super) const TABLE_CELL_MIN_WIDTH: f32 = 120.0;
pub(super) const MATH_BLOCK_PADDING_X: i8 = 18;
pub(super) const MATH_BLOCK_PADDING_Y: i8 = 16;
pub(super) const MATH_BLOCK_DISPLAY_SCALE: f32 = 1.35;
pub(super) const INLINE_MATH_TARGET_HEIGHT_MULTIPLIER: f32 = 1.3;
pub(super) const TALL_INLINE_MATH_TARGET_HEIGHT_MULTIPLIER: f32 = 2.15;
pub(super) const INLINE_MATH_LINE_HEIGHT_MULTIPLIER: f32 = 1.7;
pub(super) const INLINE_MATH_BASELINE_OFFSET_MULTIPLIER: f32 = 0.16;
pub(super) const INLINE_MATH_PLACEHOLDER_MIN_WIDTH: f32 = 28.0;
pub(super) const BLOCK_MATH_PLACEHOLDER_MIN_HEIGHT: f32 = 42.0;
const LARGE_DOCUMENT_BLOCK_THRESHOLD: usize = 2_000;
const VIRTUAL_RENDER_OVERSCAN: f32 = 1_200.0;
pub(super) const ESTIMATED_CHARS_PER_LINE: usize = 90;
pub(super) const COPY_FEEDBACK_DURATION_SECONDS: f64 = 1.2;
pub(super) const EMBEDDED_COPY_FEEDBACK_SLOT_WIDTH: f32 = 88.0;
pub(super) const DIAGRAM_BLOCK_MIN_SCALE: f32 = 0.9;

pub fn render_markdown_document(
    ui: &mut Ui,
    document: &MarkdownDocument,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
    document_base_dir: Option<&Path>,
    image_cache: &mut ImageCache,
    math_render_cache: &mut MathRenderCache,
    diagram_render_cache: &mut DiagramRenderCache,
    block_heights: &mut [Option<f32>],
    estimated_block_heights: &[f32],
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
    let mut render_resources = RenderResources {
        ui_language,
        document_base_dir,
        image_cache,
        math_render_cache,
        diagram_render_cache,
    };

    for (block_index, block) in document.blocks.iter().enumerate() {
        let search_highlight = SearchHighlight {
            query: search_query,
            is_active_block: active_search_block == Some(block_index),
        };
        let measured_block_height = block_heights.get(block_index).and_then(|height| *height);
        let block_height = measured_block_height
            .or_else(|| estimated_block_heights.get(block_index).copied())
            .unwrap_or_else(|| estimate_block_height(block, zoom_factor));
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
                    &mut render_resources,
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
                    &mut render_resources,
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
                        &mut render_resources,
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
                        &mut render_resources,
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
                    &mut render_resources,
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
            Block::DiagramBlock { language, source } => {
                render_diagram_block(
                    ui,
                    block_index,
                    language,
                    source,
                    theme,
                    zoom_factor,
                    &mut render_resources,
                );
            }
            Block::MathBlock { expression } => {
                render_math_block(
                    ui,
                    block_index,
                    expression,
                    theme,
                    zoom_factor,
                    &mut render_resources,
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
                    &mut render_resources,
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

pub struct RenderOutcome {
    pub did_scroll: bool,
    pub needs_scroll_stabilization: bool,
    pub active_heading: Option<usize>,
    pub clicked_anchor: Option<String>,
    pub clicked_external_link: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) struct SearchHighlight<'a> {
    pub(super) query: Option<&'a str>,
    pub(super) is_active_block: bool,
}

pub(super) struct RenderResources<'a> {
    pub(super) ui_language: Language,
    pub(super) document_base_dir: Option<&'a Path>,
    pub(super) image_cache: &'a mut ImageCache,
    pub(super) math_render_cache: &'a mut MathRenderCache,
    pub(super) diagram_render_cache: &'a mut DiagramRenderCache,
}

#[derive(Default)]
pub(super) struct LinkActions {
    pub(super) clicked_anchor: Option<String>,
    pub(super) clicked_external_link: Option<String>,
}

pub(super) struct HeadingRenderState {
    pub(super) did_scroll: bool,
    pub(super) is_active: bool,
}

pub(super) fn inline_content_width(
    ui: &mut Ui,
    content: &InlineContent,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    render_resources: &mut RenderResources<'_>,
) -> f32 {
    let mut width = 0.0;

    for span in &content.spans {
        if matches!(span, InlineSpan::LineBreak) {
            continue;
        }

        width += inline_span_width(
            ui,
            span,
            style,
            theme,
            zoom_factor,
            search_highlight,
            render_resources,
        );
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
    render_resources: &mut RenderResources<'_>,
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
        InlineSpan::Math(text) => {
            inline_math_width(ui, text, style, theme, zoom_factor, render_resources)
        }
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
        | InlineSpan::Code(text)
        | InlineSpan::Math(text) => text.as_str(),
        InlineSpan::Link { text, .. } => text.as_str(),
        InlineSpan::Image { alt, .. } => alt.as_str(),
        InlineSpan::LineBreak => return 0.0,
    };

    let mut width = 0.0;
    for_each_highlighted_segment(
        text,
        search_highlight.query,
        search_highlight.is_active_block,
        |segment| {
            width += text_width(ui, segment.text, style, SpanKind::Plain, theme, zoom_factor);
        },
    );
    width
}

fn inline_math_width(
    ui: &mut Ui,
    text: &str,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    render_resources: &mut RenderResources<'_>,
) -> f32 {
    let prepared = render_resources.math_render_cache.prepare(
        ui.ctx(),
        text,
        MathRenderMode::Inline,
        theme.text_primary,
        zoom_factor,
    );

    match prepared {
        PreparedMath::Svg(content) => {
            fit_inline_math_size(text, style, zoom_factor, content.asset().size()).x
        }
        PreparedMath::Pending => text_width(ui, text, style, SpanKind::Math, theme, zoom_factor),
        PreparedMath::Error(_) => text_width(ui, text, style, SpanKind::Math, theme, zoom_factor),
    }
}

pub(super) fn text_width(
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

#[derive(Clone, Copy)]
pub(super) enum InlineStyle {
    Body,
    Quote,
    Heading(f32),
    TableHeader,
    TableCell,
}

pub(super) fn render_inline(
    ui: &mut Ui,
    content: &InlineContent,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    let mut line_start = 0usize;
    let mut has_line_break = false;

    for (index, span) in content.spans.iter().enumerate() {
        if matches!(span, InlineSpan::LineBreak) {
            has_line_break = true;
            render_inline_line(
                ui,
                &content.spans[line_start..index],
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                render_resources,
            );
            line_start = index + 1;
        }
    }

    if has_line_break {
        render_inline_line(
            ui,
            &content.spans[line_start..],
            style,
            theme,
            zoom_factor,
            search_highlight,
            link_actions,
            render_resources,
        );
        return;
    }

    render_inline_line(
        ui,
        &content.spans,
        style,
        theme,
        zoom_factor,
        search_highlight,
        link_actions,
        render_resources,
    );
}

fn render_inline_line(
    ui: &mut Ui,
    spans: &[InlineSpan],
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    ui.horizontal_wrapped(|ui| {
        for span in spans {
            render_inline_span(
                ui,
                span,
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                render_resources,
            );
        }
    });
}

#[derive(Clone, Copy)]
pub(super) enum SpanKind {
    Plain,
    Strong,
    Emphasis,
    Code,
    Math,
    Link,
}

pub(super) fn render_inline_span(
    ui: &mut Ui,
    span: &InlineSpan,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
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
        InlineSpan::Math(text) => {
            let prepared = render_resources.math_render_cache.prepare(
                ui.ctx(),
                text,
                MathRenderMode::Inline,
                theme.text_primary,
                zoom_factor,
            );

            match prepared {
                PreparedMath::Svg(content) => {
                    let fitted_size =
                        fit_inline_math_size(text, style, zoom_factor, content.asset().size());
                    render_inline_math_image(
                        ui,
                        &content,
                        style,
                        zoom_factor,
                        render_resources.ui_language,
                        fitted_size,
                    );
                }
                PreparedMath::Pending => {
                    render_inline_math_placeholder(
                        ui,
                        text,
                        style,
                        render_resources.ui_language,
                        theme,
                        zoom_factor,
                    );
                }
                PreparedMath::Error(_) => render_text_label(
                    ui,
                    text,
                    style,
                    SpanKind::Math,
                    theme,
                    zoom_factor,
                    search_highlight,
                ),
            }
        }
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
            render_image_span(ui, alt, destination, theme, zoom_factor, render_resources)
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
    search_highlight: SearchHighlight<'_>,
) {
    if text.is_empty() {
        return;
    }

    if search_highlight.query.is_none() {
        ui.label(styled_text(text, style, kind, theme, zoom_factor));
        return;
    }

    let summary = for_each_highlighted_segment(
        text,
        search_highlight.query,
        search_highlight.is_active_block,
        |_| {},
    );

    if summary.match_count == 0 {
        ui.label(styled_text(text, style, kind, theme, zoom_factor));
        return;
    }

    for_each_highlighted_segment(
        text,
        search_highlight.query,
        search_highlight.is_active_block,
        |segment| {
            let mut rich_text = styled_text(segment.text, style, kind, theme, zoom_factor);

            if segment.is_match {
                rich_text = rich_text
                    .background_color(search_highlight_color(theme, segment.is_active_match));
            }

            ui.label(rich_text);
        },
    );
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
            let font_size = monospace_span_font_size(style, zoom_factor);

            rich_text = rich_text
                .family(FontFamily::Monospace)
                .font(FontId::new(font_size, FontFamily::Monospace))
                .background_color(theme.code_background);

            rich_text
        }
        SpanKind::Math => {
            let font_size = monospace_span_font_size(style, zoom_factor);

            rich_text = rich_text
                .family(FontFamily::Monospace)
                .font(FontId::new(font_size, FontFamily::Monospace))
                .color(theme.text_primary)
                .italics();

            rich_text
        }
        SpanKind::Link => {
            rich_text = rich_text.color(theme.link).underline();

            rich_text
        }
    }
}

pub(super) fn monospace_span_font_size(style: InlineStyle, zoom_factor: f32) -> f32 {
    match style {
        InlineStyle::Heading(size) => (size - zoom_factor).max(INLINE_CODE_TEXT_SIZE),
        _ => INLINE_CODE_TEXT_SIZE * zoom_factor,
    }
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
