use eframe::egui::{self, Align2, Vec2};

use crate::i18n::{tr, Language, TranslationKey};
use crate::session::ExternalLinkBehavior;

pub fn handle_external_link_click(
    ctx: &egui::Context,
    behavior: ExternalLinkBehavior,
    pending_external_link: &mut Option<String>,
    url: String,
) {
    match behavior {
        ExternalLinkBehavior::AskFirst => {
            *pending_external_link = Some(url);
        }
        ExternalLinkBehavior::OpenDirectly => {
            open_external_link(ctx, url);
        }
    }
}

pub fn render_external_link_confirmation(
    ctx: &egui::Context,
    language: Language,
    pending_external_link: &mut Option<String>,
) {
    let Some(url) = pending_external_link.clone() else {
        return;
    };

    let mut open_link = false;
    let mut cancel = false;

    egui::Window::new(tr(language, TranslationKey::MessageExternalLinkPrompt))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(url.as_str());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(tr(language, TranslationKey::ActionOpenExternalLink))
                    .clicked()
                {
                    open_link = true;
                }

                if ui
                    .button(tr(language, TranslationKey::ActionCancel))
                    .clicked()
                {
                    cancel = true;
                }
            });
        });

    if open_link {
        *pending_external_link = None;
        open_external_link(ctx, url);
    } else if cancel {
        *pending_external_link = None;
    }
}

fn open_external_link(ctx: &egui::Context, url: String) {
    ctx.open_url(egui::OpenUrl::new_tab(url));
}
