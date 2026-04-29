use std::path::{Path, PathBuf};

use eframe::egui::{self, Frame, RichText, Stroke, Ui};

use crate::i18n::{TranslationKey, tr};
use crate::image_cache::ImageLoadState;
use crate::theme::Theme;

use super::{QUOTE_TEXT_SIZE, RenderResources};

pub(super) fn render_image_span(
    ui: &mut Ui,
    alt: &str,
    destination: &str,
    theme: &Theme,
    zoom_factor: f32,
    render_resources: &mut RenderResources<'_>,
) {
    let Some(path) = resolve_local_image_path(render_resources.document_base_dir, destination)
    else {
        render_image_message(
            ui,
            tr(
                render_resources.ui_language,
                TranslationKey::MessageImageUnsupported,
            ),
            destination,
            theme,
            zoom_factor,
        );
        return;
    };

    match render_resources.image_cache.load(ui.ctx(), &path) {
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
                    render_resources.ui_language,
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
