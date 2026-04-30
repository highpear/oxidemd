use eframe::egui::{self, Align, Key, Layout, ScrollArea, TextEdit};

use crate::document_workspace::DocumentId;
use crate::i18n::{Language, TranslationKey, tr};
use crate::search::SearchState;

const SEARCH_INPUT_ID: &str = "document_search_input";

#[derive(Default)]
pub struct SearchPanelAction {
    pub query_changed: bool,
    pub select_next: bool,
    pub select_previous: bool,
}

pub fn render_search_controls(
    ui: &mut egui::Ui,
    document_id: DocumentId,
    language: Language,
    search: &mut SearchState,
) -> SearchPanelAction {
    let mut action = SearchPanelAction::default();

    ui.label(tr(language, TranslationKey::LabelSearch));

    let search_input_id = egui::Id::new((SEARCH_INPUT_ID, document_id));
    let response = ui.add(
        TextEdit::singleline(&mut search.query)
            .id(search_input_id)
            .desired_width(f32::INFINITY),
    );

    if search.focus_input {
        response.request_focus();
        search.focus_input = false;
    }

    action.query_changed = response.changed();

    if response.has_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
        action.select_next = true;
    }

    ui.horizontal(|ui| {
        let result_label = if search.matches.is_empty() {
            tr(language, TranslationKey::MessageSearchNoResults).to_owned()
        } else {
            let position = search.active_index.map(|index| index + 1).unwrap_or(0);
            format!(
                "{} {}/{}",
                tr(language, TranslationKey::LabelSearchResults),
                position,
                search.matches.len()
            )
        };
        ui.label(result_label);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_enabled(
                    !search.query.is_empty(),
                    egui::Button::new(tr(language, TranslationKey::ActionSearchClear)),
                )
                .clicked()
            {
                search.clear();
            }

            if ui
                .add_enabled(
                    search.has_matches(),
                    egui::Button::new(tr(language, TranslationKey::ActionSearchNext)),
                )
                .clicked()
            {
                action.select_next = true;
            }

            if ui
                .add_enabled(
                    search.has_matches(),
                    egui::Button::new(tr(language, TranslationKey::ActionSearchPrevious)),
                )
                .clicked()
            {
                action.select_previous = true;
            }
        });
    });

    action
}

pub fn render_search_results(
    ui: &mut egui::Ui,
    document_id: DocumentId,
    language: Language,
    search: &SearchState,
) -> Option<usize> {
    search.active_query()?;

    let mut clicked_match = None;

    ui.add_space(8.0);
    ScrollArea::vertical()
        .id_salt(("search_results_scroll", document_id))
        .max_height(180.0)
        .show(ui, |ui| {
            if search.matches.is_empty() {
                ui.label(tr(language, TranslationKey::MessageSearchNoResults));
                return;
            }

            for (index, search_match) in search.matches.iter().enumerate() {
                let is_active = search.active_index == Some(index);

                let clicked = if search_match.preview.is_empty() {
                    ui.selectable_label(is_active, format!("#{}", search_match.block_index + 1))
                        .clicked()
                } else {
                    ui.selectable_label(is_active, search_match.preview.as_str())
                        .clicked()
                };

                if clicked {
                    clicked_match = Some(index);
                }
            }
        });

    clicked_match
}
