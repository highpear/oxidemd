use std::time::Duration;

use eframe::egui::{
    self, Align, FontFamily, FontId, Frame, Label, Layout, RichText, ScrollArea, Stroke,
    TextWrapMode, Ui,
};

use crate::i18n::{Language, tr};
use crate::syntax::highlight_code;
use crate::theme::Theme;

const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
const CODE_LANGUAGE_TEXT_SIZE: f32 = 11.0;
const COPY_FEEDBACK_DURATION_SECONDS: f64 = 1.2;
const COPY_FEEDBACK_SLOT_WIDTH: f32 = 88.0;
const BLOCK_SPACING_SECTION: f32 = 24.0;

#[derive(Clone, Copy)]
struct CopyFeedbackState {
    block_index: usize,
    copied_at: f64,
}

pub fn render_code_block(
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
    let available_width = ui.available_width();

    let response = ui
        .allocate_ui_with_layout(
            egui::vec2(available_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_min_width(available_width);
                ui.set_max_width(available_width);
                Frame::new()
                    .fill(theme.code_background)
                    .stroke(Stroke::new(1.0, theme.content_border))
                    .corner_radius(egui::CornerRadius::same(scale_margin(8, zoom_factor) as u8))
                    .inner_margin(egui::Margin::symmetric(
                        scale_margin(16, zoom_factor),
                        scale_margin(10, zoom_factor),
                    ))
                    .show(ui, |ui| {
                        let frame_width = ui.available_width();
                        ui.set_min_width(frame_width);
                        ui.set_max_width(frame_width);
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
                    .response
            },
        )
        .inner;

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

fn scale_spacing(value: f32, zoom_factor: f32) -> f32 {
    value * zoom_factor
}

fn scale_margin(value: i8, zoom_factor: f32) -> i8 {
    ((value as f32) * zoom_factor)
        .round()
        .clamp(0.0, i8::MAX as f32) as i8
}
