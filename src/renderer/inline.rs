use eframe::egui::{self, FontFamily, FontId, RichText, Ui, WidgetText};

use crate::math::{MathRenderMode, PreparedMath};
use crate::parser::{InlineContent, InlineSpan};
use crate::search::{for_each_highlighted_segment, text_matches_query};
use crate::theme::Theme;

use super::embedded::{
    fit_inline_math_size, render_inline_math_image, render_inline_math_placeholder,
};
use super::image::render_image_span;
use super::{
    BODY_TEXT_SIZE, INLINE_CODE_TEXT_SIZE, LinkActions, QUOTE_TEXT_SIZE, RenderResources,
    SearchHighlight,
};

#[derive(Clone, Copy)]
pub(super) enum InlineStyle {
    Body,
    Quote,
    Heading(f32),
    TableHeader,
    TableCell,
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

pub(super) fn monospace_span_font_size(style: InlineStyle, zoom_factor: f32) -> f32 {
    match style {
        InlineStyle::Heading(size) => (size - zoom_factor).max(INLINE_CODE_TEXT_SIZE),
        _ => INLINE_CODE_TEXT_SIZE * zoom_factor,
    }
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
