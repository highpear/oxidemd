use std::time::Duration;

use eframe::egui::{self, FontFamily, FontId, Frame, RichText, Stroke, Ui};

use crate::diagram::PreparedDiagram;
use crate::embedded_svg::{EmbeddedSourceAction, EmbeddedSvgContent, EmbeddedSvgContentKind};
use crate::i18n::{Language, TranslationKey, tr};
use crate::math::{MathRenderMode, PreparedMath};
use crate::theme::Theme;

use super::sizing::{scale_margin, scale_spacing};
use super::{
    BLOCK_MATH_PLACEHOLDER_MIN_HEIGHT, BLOCK_SPACING_SECTION, BODY_TEXT_SIZE,
    COPY_FEEDBACK_DURATION_SECONDS, DIAGRAM_BLOCK_MIN_SCALE, EMBEDDED_COPY_FEEDBACK_SLOT_WIDTH,
    INLINE_CODE_TEXT_SIZE, INLINE_MATH_BASELINE_OFFSET_MULTIPLIER,
    INLINE_MATH_LINE_HEIGHT_MULTIPLIER, INLINE_MATH_PLACEHOLDER_MIN_WIDTH,
    INLINE_MATH_TARGET_HEIGHT_MULTIPLIER, InlineStyle, MATH_BLOCK_DISPLAY_SCALE,
    MATH_BLOCK_PADDING_X, MATH_BLOCK_PADDING_Y, QUOTE_TEXT_SIZE, RenderResources, SpanKind,
    TALL_INLINE_MATH_TARGET_HEIGHT_MULTIPLIER, monospace_span_font_size, text_width,
};

#[derive(Clone, Copy)]
struct EmbeddedCopyFeedbackState {
    block_index: usize,
    copied_at: f64,
}

#[derive(Clone, Copy)]
struct EmbeddedSvgBlockLabels {
    title: TranslationKey,
    copy_action: TranslationKey,
}

pub(super) fn render_math_block(
    ui: &mut Ui,
    block_index: usize,
    expression: &str,
    theme: &Theme,
    zoom_factor: f32,
    render_resources: &mut RenderResources<'_>,
) {
    let prepared = render_resources.math_render_cache.prepare(
        ui.ctx(),
        expression,
        MathRenderMode::Block,
        theme.text_primary,
        zoom_factor,
    );

    Frame::new()
        .fill(theme.widget_inactive_background)
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(MATH_BLOCK_PADDING_X, zoom_factor),
            scale_margin(MATH_BLOCK_PADDING_Y, zoom_factor),
        ))
        .show(ui, |ui| {
            let source_action = match &prepared {
                PreparedMath::Svg(content) => content.source_action(),
                PreparedMath::Pending => EmbeddedSourceAction::new(expression),
                PreparedMath::Error(_) => EmbeddedSourceAction::new(expression),
            };

            render_math_block_header(
                ui,
                block_index,
                source_action,
                render_resources.ui_language,
                theme,
                zoom_factor,
            );
            ui.add_space(scale_spacing(6.0, zoom_factor));

            match prepared {
                PreparedMath::Svg(content) => {
                    debug_assert_eq!(content.kind(), EmbeddedSvgContentKind::Math);
                    render_embedded_svg_block_image(
                        ui,
                        &content,
                        render_resources.ui_language,
                        TranslationKey::ActionCopyTex,
                    );
                }
                PreparedMath::Pending => {
                    render_math_block_placeholder(ui, expression, theme, zoom_factor);
                }
                PreparedMath::Error(error) => {
                    ui.label(
                        RichText::new(error)
                            .size(QUOTE_TEXT_SIZE * zoom_factor)
                            .color(theme.status_error_text),
                    );
                    ui.add_space(scale_spacing(6.0, zoom_factor));
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(expression)
                                .size(BODY_TEXT_SIZE * zoom_factor)
                                .color(theme.text_primary)
                                .family(FontFamily::Monospace)
                                .font(FontId::new(
                                    BODY_TEXT_SIZE * zoom_factor,
                                    FontFamily::Monospace,
                                )),
                        );
                    });
                }
            }
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

pub(super) fn render_diagram_block(
    ui: &mut Ui,
    block_index: usize,
    language: &str,
    source: &str,
    theme: &Theme,
    zoom_factor: f32,
    render_resources: &mut RenderResources<'_>,
) {
    let prepared = render_resources.diagram_render_cache.prepare(
        ui.ctx().clone(),
        language,
        source,
        theme.text_primary,
        theme.widget_inactive_background,
    );

    Frame::new()
        .fill(theme.widget_inactive_background)
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(MATH_BLOCK_PADDING_X, zoom_factor),
            scale_margin(MATH_BLOCK_PADDING_Y, zoom_factor),
        ))
        .show(ui, |ui| {
            let source_action = match &prepared {
                PreparedDiagram::Svg(content) => content.source_action(),
                PreparedDiagram::Pending => EmbeddedSourceAction::new(source),
                PreparedDiagram::Error(_) => EmbeddedSourceAction::new(source),
            };

            render_embedded_svg_block_header(
                ui,
                block_index,
                source_action,
                EmbeddedSvgBlockLabels {
                    title: TranslationKey::LabelMermaid,
                    copy_action: TranslationKey::ActionCopySource,
                },
                "diagram_block_copy_feedback",
                render_resources.ui_language,
                theme,
                zoom_factor,
            );
            ui.add_space(scale_spacing(6.0, zoom_factor));

            match prepared {
                PreparedDiagram::Svg(content) => {
                    debug_assert_eq!(content.kind(), EmbeddedSvgContentKind::Diagram);
                    render_embedded_svg_block_image(
                        ui,
                        &content,
                        render_resources.ui_language,
                        TranslationKey::ActionCopySource,
                    );
                }
                PreparedDiagram::Pending => {
                    ui.label(
                        RichText::new(tr(
                            render_resources.ui_language,
                            TranslationKey::MessageDiagramPreviewPending,
                        ))
                        .size(QUOTE_TEXT_SIZE * zoom_factor)
                        .color(theme.text_secondary),
                    );
                    ui.add_space(scale_spacing(8.0, zoom_factor));

                    render_embedded_source_fallback(ui, block_index, source, theme, zoom_factor);
                }
                PreparedDiagram::Error(error) => {
                    ui.label(
                        RichText::new(tr(
                            render_resources.ui_language,
                            TranslationKey::MessageDiagramPreviewUnavailable,
                        ))
                        .size(QUOTE_TEXT_SIZE * zoom_factor)
                        .color(theme.text_secondary),
                    )
                    .on_hover_text(error);
                    ui.add_space(scale_spacing(8.0, zoom_factor));

                    render_embedded_source_fallback(ui, block_index, source, theme, zoom_factor);
                }
            }
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

pub(super) fn fit_inline_math_size(
    expression: &str,
    style: InlineStyle,
    zoom_factor: f32,
    size: egui::Vec2,
) -> egui::Vec2 {
    if size.y <= 0.0 {
        return size;
    }

    let target_multiplier = if is_tall_inline_math(expression) {
        TALL_INLINE_MATH_TARGET_HEIGHT_MULTIPLIER
    } else {
        INLINE_MATH_TARGET_HEIGHT_MULTIPLIER
    };
    let target_height = monospace_span_font_size(style, zoom_factor) * target_multiplier;
    let scale = target_height / size.y;

    egui::vec2(size.x * scale, size.y * scale)
}

pub(super) fn render_inline_math_image(
    ui: &mut Ui,
    content: &EmbeddedSvgContent,
    style: InlineStyle,
    zoom_factor: f32,
    ui_language: Language,
    fitted_size: egui::Vec2,
) {
    debug_assert_eq!(content.kind(), EmbeddedSvgContentKind::Math);

    let line_height = inline_math_line_height(style, zoom_factor);
    let baseline_offset =
        monospace_span_font_size(style, zoom_factor) * INLINE_MATH_BASELINE_OFFSET_MULTIPLIER;
    let top_padding = (line_height - fitted_size.y - baseline_offset).max(0.0);
    let allocated_size = egui::vec2(
        fitted_size.x,
        (top_padding + fitted_size.y).max(fitted_size.y),
    );
    let (rect, response) = ui.allocate_exact_size(allocated_size, egui::Sense::click());
    let image_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.top() + top_padding),
        fitted_size,
    );

    if response.clicked() {
        copy_embedded_source(ui, content.source_action());
    }
    response.on_hover_text(tr(ui_language, TranslationKey::ActionCopyTex));

    ui.put(
        image_rect,
        egui::Image::from_bytes(content.asset().uri().to_owned(), content.asset().bytes())
            .fit_to_exact_size(fitted_size),
    );
}

pub(super) fn render_inline_math_placeholder(
    ui: &mut Ui,
    expression: &str,
    style: InlineStyle,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
) {
    let width = text_width(ui, expression, style, SpanKind::Math, theme, zoom_factor).max(
        scale_spacing(INLINE_MATH_PLACEHOLDER_MIN_WIDTH, zoom_factor),
    );
    let line_height = inline_math_line_height(style, zoom_factor);
    let bar_height = (monospace_span_font_size(style, zoom_factor) * 0.34).max(3.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, line_height), egui::Sense::click());
    let bar_rect = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(width, bar_height).min(rect.size()),
    );

    if response.clicked() {
        ui.ctx().copy_text(expression.to_owned());
    }
    response.on_hover_text(tr(ui_language, TranslationKey::ActionCopyTex));

    ui.painter().rect_filled(
        bar_rect,
        egui::CornerRadius::same(3),
        theme.widget_active_background,
    );
}

fn is_tall_inline_math(expression: &str) -> bool {
    expression.contains("\\frac")
        || expression.contains("\\dfrac")
        || expression.contains("\\tfrac")
        || expression.contains("\\genfrac")
}

fn fit_math_block_svg_size(size: egui::Vec2, max_width: f32) -> egui::Vec2 {
    if size.x <= 0.0 || size.y <= 0.0 {
        return size;
    }

    let scale = MATH_BLOCK_DISPLAY_SCALE.min(max_width / size.x);
    egui::vec2(size.x * scale, size.y * scale)
}

fn fit_diagram_block_svg_size(size: egui::Vec2, max_width: f32) -> egui::Vec2 {
    if size.x <= 0.0 || size.x <= max_width {
        return size;
    }

    let scale = (max_width / size.x).max(DIAGRAM_BLOCK_MIN_SCALE);
    egui::vec2(size.x * scale, size.y * scale)
}

fn render_embedded_svg_block_image(
    ui: &mut Ui,
    content: &EmbeddedSvgContent,
    ui_language: Language,
    copy_action_key: TranslationKey,
) {
    let max_width = ui.available_width().max(120.0);
    let fitted_size = match content.kind() {
        EmbeddedSvgContentKind::Diagram => {
            fit_diagram_block_svg_size(content.asset().size(), max_width)
        }
        EmbeddedSvgContentKind::Math => fit_math_block_svg_size(content.asset().size(), max_width),
    };

    if content.kind() == EmbeddedSvgContentKind::Diagram && fitted_size.x > max_width {
        egui::ScrollArea::horizontal()
            .id_salt(("embedded_svg_block_image", content.asset().uri()))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                render_embedded_svg_image(ui, content, ui_language, copy_action_key, fitted_size);
            });
        return;
    }

    ui.vertical_centered(|ui| {
        render_embedded_svg_image(ui, content, ui_language, copy_action_key, fitted_size);
    });
}

fn render_embedded_svg_image(
    ui: &mut Ui,
    content: &EmbeddedSvgContent,
    ui_language: Language,
    copy_action_key: TranslationKey,
    fitted_size: egui::Vec2,
) {
    let response = ui.add(
        egui::Image::from_bytes(content.asset().uri().to_owned(), content.asset().bytes())
            .fit_to_exact_size(fitted_size)
            .sense(egui::Sense::click()),
    );
    if response.clicked() {
        copy_embedded_source(ui, content.source_action());
    }
    response.on_hover_text(tr(ui_language, copy_action_key));
}

fn render_math_block_placeholder(ui: &mut Ui, expression: &str, theme: &Theme, zoom_factor: f32) {
    let line_count = expression.lines().count().max(1).min(3);
    let placeholder_height = (line_count as f32 * scale_spacing(22.0, zoom_factor)).max(
        scale_spacing(BLOCK_MATH_PLACEHOLDER_MIN_HEIGHT, zoom_factor),
    );
    let placeholder_width = ui
        .available_width()
        .min(scale_spacing(360.0, zoom_factor))
        .max(scale_spacing(96.0, zoom_factor));

    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(placeholder_width, placeholder_height),
            egui::Sense::hover(),
        );
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(5),
            theme.widget_inactive_background,
            Stroke::new(1.0, theme.content_border),
            egui::StrokeKind::Inside,
        );

        let bar_height = scale_spacing(5.0, zoom_factor).max(3.0);
        let gap = scale_spacing(8.0, zoom_factor);
        let total_bar_height = line_count as f32 * bar_height + (line_count - 1) as f32 * gap;
        let mut y = rect.center().y - total_bar_height / 2.0;
        let width_factors = [0.64, 0.48, 0.56];

        for factor in width_factors.iter().take(line_count) {
            let bar_width = placeholder_width * factor;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.center().x - bar_width / 2.0, y),
                egui::vec2(bar_width, bar_height),
            );
            ui.painter().rect_filled(
                bar_rect,
                egui::CornerRadius::same(3),
                theme.widget_active_background,
            );
            y += bar_height + gap;
        }
    });
}

fn render_embedded_source_fallback(
    ui: &mut Ui,
    block_index: usize,
    source: &str,
    theme: &Theme,
    zoom_factor: f32,
) {
    egui::ScrollArea::horizontal()
        .id_salt(("embedded_source_fallback", block_index))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.add(
                egui::Label::new(
                    RichText::new(source)
                        .size(INLINE_CODE_TEXT_SIZE * zoom_factor)
                        .color(theme.text_primary)
                        .family(FontFamily::Monospace)
                        .font(FontId::new(
                            INLINE_CODE_TEXT_SIZE * zoom_factor,
                            FontFamily::Monospace,
                        )),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

fn render_math_block_header(
    ui: &mut Ui,
    block_index: usize,
    source_action: EmbeddedSourceAction<'_>,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
) {
    render_embedded_svg_block_header(
        ui,
        block_index,
        source_action,
        EmbeddedSvgBlockLabels {
            title: TranslationKey::LabelMath,
            copy_action: TranslationKey::ActionCopyTex,
        },
        "math_block_copy_feedback",
        ui_language,
        theme,
        zoom_factor,
    );
}

fn render_embedded_svg_block_header(
    ui: &mut Ui,
    block_index: usize,
    source_action: EmbeddedSourceAction<'_>,
    labels: EmbeddedSvgBlockLabels,
    feedback_id_salt: &'static str,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
) {
    let feedback_id = ui.make_persistent_id(feedback_id_salt);
    let copied = ui
        .ctx()
        .data(|data| data.get_temp::<EmbeddedCopyFeedbackState>(feedback_id));
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
        ui.label(
            RichText::new(tr(ui_language, labels.title))
                .size(QUOTE_TEXT_SIZE * zoom_factor)
                .strong()
                .color(theme.text_secondary),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr(ui_language, labels.copy_action)).clicked() {
                let copied_at = ui.ctx().input(|input| input.time);
                copy_embedded_source(ui, source_action);
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        feedback_id,
                        EmbeddedCopyFeedbackState {
                            block_index,
                            copied_at,
                        },
                    );
                });
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64(COPY_FEEDBACK_DURATION_SECONDS));
            }

            ui.add_space(scale_spacing(8.0, zoom_factor));

            ui.allocate_ui_with_layout(
                egui::vec2(EMBEDDED_COPY_FEEDBACK_SLOT_WIDTH * zoom_factor, 0.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if show_copied {
                        ui.label(
                            RichText::new(tr(ui_language, TranslationKey::MessageCopied))
                                .size(QUOTE_TEXT_SIZE * zoom_factor)
                                .color(theme.text_secondary),
                        );
                    }
                },
            );
        });
    });
}

fn copy_embedded_source(ui: &mut Ui, source_action: EmbeddedSourceAction<'_>) {
    ui.ctx().copy_text(source_action.source_text().to_owned());
}

fn inline_math_line_height(style: InlineStyle, zoom_factor: f32) -> f32 {
    match style {
        InlineStyle::Heading(size) => size * INLINE_MATH_LINE_HEIGHT_MULTIPLIER,
        InlineStyle::Quote => QUOTE_TEXT_SIZE * zoom_factor * INLINE_MATH_LINE_HEIGHT_MULTIPLIER,
        InlineStyle::Body | InlineStyle::TableHeader | InlineStyle::TableCell => {
            BODY_TEXT_SIZE * zoom_factor * INLINE_MATH_LINE_HEIGHT_MULTIPLIER
        }
    }
}
