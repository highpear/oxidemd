use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Frame, Margin, RichText, TopBottomPanel};

use crate::i18n::{Language, TranslationKey, tr};
use crate::session::ExternalLinkBehavior;

const TOP_BAR_FILE_LABEL_MAX_WIDTH: f32 = 280.0;

#[derive(Default)]
pub struct TopBarAction {
    pub open_file: bool,
    pub open_recent_file: Option<PathBuf>,
    pub clear_recent_files: bool,
    pub export_html: bool,
    pub switch_language: bool,
    pub switch_theme: bool,
    pub switch_external_links: bool,
    pub show_shortcuts_help: bool,
    pub copy_path: bool,
}

pub struct TopBarState<'a> {
    pub language: Language,
    pub current_theme_label: &'a str,
    pub external_link_behavior: ExternalLinkBehavior,
    pub current_file: Option<&'a Path>,
    pub recent_files: &'a [PathBuf],
    pub reload_status_label: &'a str,
    pub reload_status_background: Color32,
    pub reload_status_text: Color32,
    pub status_message: &'a str,
    pub status_hover_message: Option<&'a str>,
}

pub fn render_top_bar(ctx: &egui::Context, state: TopBarState<'_>) -> TopBarAction {
    let mut action = TopBarAction::default();

    TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui
                .button(tr(state.language, TranslationKey::ActionOpen))
                .clicked()
            {
                action.open_file = true;
            }

            ui.menu_button(tr(state.language, TranslationKey::LabelRecentFiles), |ui| {
                if state.recent_files.is_empty() {
                    ui.add_enabled(
                        false,
                        egui::Button::new(tr(state.language, TranslationKey::MessageNoRecentFiles)),
                    );
                } else {
                    for path in state.recent_files.iter().cloned() {
                        let label = recent_file_label(&path);
                        if ui
                            .button(label)
                            .on_hover_text(path.display().to_string())
                            .clicked()
                        {
                            action.open_recent_file = Some(path);
                            ui.close();
                        }
                    }

                    ui.separator();
                    if ui
                        .button(tr(state.language, TranslationKey::ActionClearRecentFiles))
                        .clicked()
                    {
                        action.clear_recent_files = true;
                        ui.close();
                    }
                }
            });

            ui.add_enabled_ui(state.current_file.is_some(), |ui| {
                ui.menu_button(tr(state.language, TranslationKey::LabelExport), |ui| {
                    if ui
                        .button(tr(state.language, TranslationKey::ActionExportHtml))
                        .clicked()
                    {
                        action.export_html = true;
                        ui.close();
                    }
                });
            });

            if ui
                .button(tr(state.language, TranslationKey::ActionSwitchLanguage))
                .clicked()
            {
                action.switch_language = true;
            }

            if ui
                .button(format!(
                    "{} {}",
                    tr(state.language, TranslationKey::ActionSwitchTheme),
                    state.current_theme_label
                ))
                .clicked()
            {
                action.switch_theme = true;
            }

            if ui
                .button(format!(
                    "{} {}",
                    tr(state.language, TranslationKey::LabelExternalLinks),
                    state.external_link_behavior.label(state.language)
                ))
                .clicked()
            {
                action.switch_external_links = true;
            }

            if ui
                .button(tr(state.language, TranslationKey::LabelShortcuts))
                .clicked()
            {
                action.show_shortcuts_help = true;
            }

            ui.separator();
            let current_file_label = format!(
                "{} {}",
                tr(state.language, TranslationKey::LabelCurrentFile),
                current_file_label(state.language, state.current_file)
            );
            let file_label_response = ui.add_sized(
                [TOP_BAR_FILE_LABEL_MAX_WIDTH, ui.spacing().interact_size.y],
                egui::Label::new(current_file_label).truncate(),
            );

            if let Some(path) = state.current_file {
                file_label_response.on_hover_text(path.display().to_string());
            }

            if ui
                .add_enabled(
                    state.current_file.is_some(),
                    egui::Button::new(tr(state.language, TranslationKey::ActionCopyPath)),
                )
                .clicked()
            {
                action.copy_path = true;
            }

            ui.separator();
            Frame::new()
                .fill(state.reload_status_background)
                .corner_radius(egui::CornerRadius::same(255))
                .inner_margin(Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(state.reload_status_label)
                            .color(state.reload_status_text)
                            .strong(),
                    );
                });
        });

        let status_response = ui.add(egui::Label::new(state.status_message).truncate());
        if let Some(message) = state.status_hover_message {
            status_response.on_hover_text(message);
        }
    });

    action
}

fn current_file_label(language: Language, path: Option<&Path>) -> String {
    path.and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| tr(language, TranslationKey::LabelNoFile).to_owned())
}

fn recent_file_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}
