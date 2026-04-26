use eframe::egui::{self, Align, Layout, Slider, TopBottomPanel, Vec2};

use crate::i18n::{Language, TranslationKey, tr};

const ZOOM_STEP_BUTTON_WIDTH: f32 = 28.0;

#[derive(Default)]
pub struct BottomBarAction {
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub reset_zoom: bool,
}

pub struct BottomBarState {
    pub language: Language,
    pub zoom_factor: f32,
    pub min_zoom_factor: f32,
    pub max_zoom_factor: f32,
    pub zoom_step: f32,
}

pub fn render_bottom_bar(
    ctx: &egui::Context,
    state: BottomBarState,
    zoom_factor: &mut f32,
) -> BottomBarAction {
    let mut action = BottomBarAction::default();

    TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(zoom_label(state.language, state.zoom_factor));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(tr(state.language, TranslationKey::ActionResetZoom))
                    .clicked()
                {
                    action.reset_zoom = true;
                }

                if ui
                    .add_enabled(
                        state.zoom_factor < state.max_zoom_factor,
                        egui::Button::new("+").min_size(Vec2::splat(ZOOM_STEP_BUTTON_WIDTH)),
                    )
                    .clicked()
                {
                    action.zoom_in = true;
                }

                let slider =
                    Slider::new(zoom_factor, state.min_zoom_factor..=state.max_zoom_factor)
                        .show_value(false)
                        .step_by(state.zoom_step.into())
                        .smart_aim(false);
                ui.add_sized([160.0, 0.0], slider);

                if ui
                    .add_enabled(
                        state.zoom_factor > state.min_zoom_factor,
                        egui::Button::new("-").min_size(Vec2::splat(ZOOM_STEP_BUTTON_WIDTH)),
                    )
                    .clicked()
                {
                    action.zoom_out = true;
                }

                ui.label(format!("{:.0}%", state.zoom_factor * 100.0));
            });
        });
    });

    action
}

fn zoom_label(language: Language, zoom_factor: f32) -> String {
    format!(
        "{} {}%",
        tr(language, TranslationKey::LabelZoom),
        (zoom_factor * 100.0).round()
    )
}
