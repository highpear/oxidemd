use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{
    self, Align, Align2, CentralPanel, Frame, Layout, Margin, RichText, ScrollArea, SidePanel,
    UiBuilder, Vec2,
};

use crate::bottom_bar::{BottomBarState, render_bottom_bar};
use crate::external_links::handle_external_link_click;
use crate::i18n::{TranslationKey, tr};
use crate::metrics;
use crate::renderer::render_markdown_document;
use crate::search_panel::{render_search_controls, render_search_results};
use crate::theme::{Theme, ThemeId, theme};
use crate::top_bar::{TopBarState, render_top_bar};

use super::{
    DOCUMENT_FRAME_STROKE_WIDTH, HEADING_NAV_ITEM_INDENT, HEADING_PANEL_DEFAULT_WIDTH,
    HEADING_PANEL_MAX_WIDTH, HEADING_PANEL_MIN_WIDTH, HOME_PANEL_MAX_WIDTH, HOME_RECENT_FILE_LIMIT,
    MAX_ZOOM_FACTOR, MIN_ZOOM_FACTOR, OxideMdApp, ReloadStatus, ZOOM_STEP, heading_nav_indent,
    home_recent_file_label, scaled_document_body_max_width, scaled_document_frame_max_width,
    scaled_document_horizontal_padding, scaled_document_vertical_padding, scaled_margin,
};

impl OxideMdApp {
    pub(super) fn render_top_bar(&mut self, ctx: &egui::Context) {
        let theme = theme(self.theme_id);
        let (reload_status_background, reload_status_text) = match self.reload_status {
            ReloadStatus::Idle => (theme.status_idle_background, theme.status_idle_text),
            ReloadStatus::Reloading => (theme.status_loading_background, theme.status_loading_text),
            ReloadStatus::Error => (theme.status_error_background, theme.status_error_text),
        };
        let theme_options = [
            (
                ThemeId::WarmPaper,
                tr(self.language, TranslationKey::ThemeWarmPaper),
            ),
            (ThemeId::Mist, tr(self.language, TranslationKey::ThemeMist)),
            (
                ThemeId::NightOwl,
                tr(self.language, TranslationKey::ThemeNightOwl),
            ),
        ];

        let action = render_top_bar(
            ctx,
            TopBarState {
                language: self.language,
                current_theme_id: self.theme_id,
                theme_options: &theme_options,
                external_link_behavior: self.external_link_behavior,
                is_heading_panel_visible: self.is_heading_panel_visible,
                current_file: self.current_file(),
                recent_files: &self.recent_files,
                reload_status_label: self.reload_status_label(),
                reload_status_background,
                reload_status_text,
            },
        );

        if action.open_file {
            self.open_markdown_file();
        }

        if let Some(path) = action.open_recent_file {
            self.open_recent_file(path);
        }

        if action.clear_recent_files {
            self.clear_recent_files();
        }

        if action.export_html {
            self.export_current_file_as_html();
        }

        if action.switch_language {
            self.switch_language();
        }

        if let Some(theme_id) = action.select_theme {
            self.select_theme(theme_id);
        }

        if action.switch_external_links {
            self.switch_external_link_behavior();
        }

        if action.toggle_heading_panel {
            self.toggle_heading_panel();
        }

        if action.show_shortcuts_help {
            self.show_shortcuts_help = true;
        }

        if action.copy_path {
            self.copy_current_file_path(ctx);
        }
    }

    pub(super) fn render_bottom_bar(&mut self, ctx: &egui::Context) {
        let action = render_bottom_bar(
            ctx,
            BottomBarState {
                language: self.language,
                zoom_factor: self.zoom_factor,
                min_zoom_factor: MIN_ZOOM_FACTOR,
                max_zoom_factor: MAX_ZOOM_FACTOR,
                zoom_step: ZOOM_STEP,
                status_message: self.status_message.as_str(),
                status_hover_message: self.status_hover_message.as_deref(),
            },
            &mut self.zoom_factor,
        );

        if action.zoom_in {
            self.zoom_in();
        }

        if action.zoom_out {
            self.zoom_out();
        }

        if action.reset_zoom {
            self.reset_zoom();
        }
    }

    pub(super) fn render_heading_panel(&mut self, ctx: &egui::Context) {
        if !self.is_heading_panel_visible {
            return;
        }

        let language = self.language;
        let theme_id = self.theme_id;
        let Some(mut active_document) = self.documents.take_active_session() else {
            SidePanel::left("heading_navigation")
                .resizable(true)
                .default_width(HEADING_PANEL_DEFAULT_WIDTH)
                .width_range(HEADING_PANEL_MIN_WIDTH..=HEADING_PANEL_MAX_WIDTH)
                .show(ctx, |ui| {
                    ui.heading(tr(language, TranslationKey::NavSections));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(tr(language, TranslationKey::NavNoSections))
                            .color(theme(theme_id).text_secondary),
                    );
                });
            return;
        };
        let document_id = active_document.id();
        let session = &mut *active_document;
        let mut clicked_heading = None;

        SidePanel::left(egui::Id::new(("heading_navigation", document_id)))
            .resizable(true)
            .default_width(HEADING_PANEL_DEFAULT_WIDTH)
            .width_range(HEADING_PANEL_MIN_WIDTH..=HEADING_PANEL_MAX_WIDTH)
            .show(ctx, |ui| {
                let search_action =
                    render_search_controls(ui, document_id, language, &mut session.search);
                if search_action.query_changed {
                    session.refresh_search_matches();

                    if session.search.has_matches() {
                        session.select_search_match(0);
                    }
                }

                if search_action.select_previous {
                    session.select_previous_search_match();
                }

                if search_action.select_next {
                    session.select_next_search_match();
                }

                if let Some(index) =
                    render_search_results(ui, document_id, language, &session.search)
                {
                    session.select_search_match(index);
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.heading(tr(language, TranslationKey::NavSections));
                ui.add_space(8.0);

                let headings = session.document.headings();
                if headings.is_empty() {
                    ui.label(
                        RichText::new(tr(language, TranslationKey::NavNoSections))
                            .color(theme(theme_id).text_secondary),
                    );
                    return;
                }

                let highlighted_heading = session.selected_heading.or(session.active_heading);

                ScrollArea::vertical()
                    .id_salt(("heading_navigation_scroll", document_id))
                    .show_rows(
                        ui,
                        ui.spacing().interact_size.y,
                        headings.len(),
                        |ui, row_range| {
                            for row_index in row_range {
                                let item = &headings[row_index];
                                let is_active = highlighted_heading == Some(item.block_index);
                                let indent = heading_nav_indent(item.level);

                                ui.horizontal(|ui| {
                                    ui.add_space(indent);

                                    let available_width = (ui.available_width() - indent)
                                        .max(HEADING_NAV_ITEM_INDENT);
                                    let response = ui.add_sized(
                                        [available_width, ui.spacing().interact_size.y],
                                        egui::Button::selectable(is_active, &item.title).truncate(),
                                    );

                                    if response
                                        .on_hover_text(format!(
                                            "{}\n{}",
                                            tr(language, TranslationKey::NavJumpToHeading),
                                            item.title
                                        ))
                                        .clicked()
                                    {
                                        clicked_heading = Some(item.block_index);
                                    }
                                });
                            }
                        },
                    );
            });

        if let Some(block_index) = clicked_heading {
            session.jump_to_heading(block_index);
        }

        self.documents.restore_active_session(active_document);
    }

    pub(super) fn render_document_panel(&mut self, ctx: &egui::Context) {
        let theme = theme(self.theme_id);
        let Some(mut active_document) = self.documents.take_active_session() else {
            CentralPanel::default().show(ctx, |ui| {
                if let Some(path) = self.render_home_panel(ui, &theme) {
                    self.open_recent_file(path);
                }
            });
            return;
        };
        let document_id = active_document.id();
        let session = &mut *active_document;
        let document = Arc::clone(&session.document);
        let document_base_dir = session.base_dir().map(Path::to_path_buf);
        let active_search_block = session.search.active_block();
        let search_query = session.search.active_query().map(str::to_owned);
        let pending_block_scroll = session.pending_block_scroll;
        let language = self.language;
        let zoom_factor = self.zoom_factor;
        let external_link_behavior = self.external_link_behavior;

        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::both()
                .id_salt(("document_scroll", document_id))
                .show(ui, |ui| {
                    ui.add_space(18.0);
                    let content_rect = ui.max_rect();
                    let frame_width = scaled_document_frame_max_width(zoom_factor);
                    let frame_left = if content_rect.width() > frame_width {
                        content_rect.center().x - frame_width * 0.5
                    } else {
                        content_rect.left()
                    };
                    let frame_rect = egui::Rect::from_min_size(
                        egui::pos2(frame_left, ui.cursor().top()),
                        Vec2::new(frame_width, 0.0),
                    );
                    let horizontal_padding = scaled_document_horizontal_padding(zoom_factor);
                    let vertical_padding = scaled_document_vertical_padding(zoom_factor);

                    let document_frame = Frame::new()
                        .fill(theme.content_background)
                        .stroke(egui::Stroke::new(
                            DOCUMENT_FRAME_STROKE_WIDTH,
                            theme.content_border,
                        ))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 8],
                            blur: 28,
                            spread: 0,
                            color: theme.content_shadow,
                        })
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(Margin::symmetric(
                            scaled_margin(32, zoom_factor),
                            scaled_margin(28, zoom_factor),
                        ));
                    let background_shape = ui.painter().add(egui::Shape::Noop);
                    let content_width =
                        (frame_width - horizontal_padding - DOCUMENT_FRAME_STROKE_WIDTH * 2.0)
                            .max(0.0)
                            .min(scaled_document_body_max_width(zoom_factor));
                    let content_min = egui::pos2(
                        frame_rect.left() + horizontal_padding * 0.5 + DOCUMENT_FRAME_STROKE_WIDTH,
                        frame_rect.top() + vertical_padding * 0.5 + DOCUMENT_FRAME_STROKE_WIDTH,
                    );
                    let content_max_rect = egui::Rect::from_min_max(
                        content_min,
                        egui::pos2(content_min.x + content_width, content_rect.bottom()),
                    );

                    let mut document_ui = ui.new_child(
                        UiBuilder::new()
                            .max_rect(content_max_rect)
                            .layout(Layout::top_down(Align::Min)),
                    );
                    let mut document_clip_rect = document_ui.clip_rect();
                    document_clip_rect.min.x = content_max_rect.left();
                    document_clip_rect.max.x = content_max_rect.right();
                    document_ui.set_clip_rect(document_clip_rect);
                    document_ui.set_min_width(content_width);
                    document_ui.set_max_width(content_width);

                    let render_measurement = self.pending_render_measurement.take();
                    let render_started = render_measurement.as_ref().map(|_| Instant::now());
                    let block_count = document.blocks.len();
                    let heading_count = document.headings().len();
                    session.block_height_cache.prepare(
                        session.fingerprint,
                        &document,
                        zoom_factor,
                        content_width,
                    );
                    let block_height_cache = &mut session.block_height_cache;
                    let crate::document_session::BlockHeightCache {
                        heights: block_heights,
                        estimated_heights: estimated_block_heights,
                        ..
                    } = block_height_cache;
                    let render_outcome = render_markdown_document(
                        &mut document_ui,
                        &document,
                        language,
                        &theme,
                        zoom_factor,
                        document_base_dir.as_deref(),
                        &mut session.image_cache,
                        &mut session.math_render_cache,
                        &mut session.diagram_render_cache,
                        block_heights,
                        estimated_block_heights,
                        pending_block_scroll,
                        search_query.as_deref(),
                        active_search_block,
                    );

                    if let (Some(measurement), Some(started)) = (render_measurement, render_started)
                    {
                        metrics::log_document_render(
                            measurement.reason.as_log_label(),
                            &measurement.path,
                            started.elapsed(),
                            block_count,
                            heading_count,
                        );
                    }

                    if let Some(active_heading) = render_outcome.active_heading {
                        session.active_heading = Some(active_heading);
                    }

                    if let Some(block_index) = render_outcome
                        .clicked_anchor
                        .and_then(|anchor| document.heading_block_for_anchor(&anchor))
                    {
                        session.jump_to_heading(block_index);
                        ctx.request_repaint();
                    }

                    if let Some(url) = render_outcome.clicked_external_link {
                        handle_external_link_click(
                            ctx,
                            external_link_behavior,
                            &mut self.pending_external_link,
                            url,
                        );
                    }

                    if render_outcome.needs_scroll_stabilization {
                        ctx.request_repaint();
                    } else if render_outcome.did_scroll {
                        session.pending_block_scroll = None;
                    }

                    let used_content_rect = document_ui.min_rect();
                    let fixed_content_rect = egui::Rect::from_min_size(
                        content_min,
                        Vec2::new(content_width, used_content_rect.height()),
                    );
                    let actual_frame_rect = document_frame.outer_rect(fixed_content_rect);
                    ui.painter()
                        .set(background_shape, document_frame.paint(fixed_content_rect));
                    ui.allocate_rect(actual_frame_rect, egui::Sense::hover());
                    ui.add_space(24.0);
                });
        });

        self.documents.restore_active_session(active_document);
    }

    pub(super) fn render_home_panel(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
    ) -> Option<PathBuf> {
        let mut selected_recent_file = None;

        ui.vertical_centered(|ui| {
            ui.add_space(48.0);

            let panel_width = ui.available_width().min(HOME_PANEL_MAX_WIDTH);
            Frame::new()
                .fill(theme.content_background)
                .stroke(egui::Stroke::new(1.0, theme.content_border))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(Margin::symmetric(24, 22))
                .show(ui, |ui| {
                    ui.set_width(panel_width);
                    ui.heading(tr(self.language, TranslationKey::LabelStart));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(tr(self.language, TranslationKey::MessageRecentFilesPrompt))
                            .color(theme.text_secondary),
                    );

                    ui.add_space(16.0);
                    if ui
                        .button(tr(self.language, TranslationKey::ActionOpen))
                        .clicked()
                    {
                        self.open_markdown_file();
                    }

                    ui.add_space(18.0);
                    ui.separator();
                    ui.add_space(12.0);
                    ui.strong(tr(self.language, TranslationKey::LabelRecentFiles));
                    ui.add_space(8.0);

                    if self.recent_files.is_empty() {
                        ui.label(
                            RichText::new(tr(self.language, TranslationKey::MessageNoRecentFiles))
                                .color(theme.text_secondary),
                        );
                    } else {
                        for path in self.recent_files.iter().take(HOME_RECENT_FILE_LIMIT) {
                            let label = home_recent_file_label(path);
                            let response = ui.add_sized(
                                [ui.available_width(), ui.spacing().interact_size.y],
                                egui::Button::new(label).truncate(),
                            );

                            if response.on_hover_text(path.display().to_string()).clicked() {
                                selected_recent_file = Some(path.clone());
                            }
                        }
                    }

                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(tr(self.language, TranslationKey::MessageDropMarkdown))
                            .color(theme.text_secondary),
                    );
                });
        });

        selected_recent_file
    }

    pub(super) fn render_drop_overlay(&self, ctx: &egui::Context) {
        let is_dragging_file = ctx.input(|input| !input.raw.hovered_files.is_empty());
        if !is_dragging_file {
            return;
        }

        let theme = theme(self.theme_id);
        let viewport_rect = ctx.content_rect();
        let overlay_color = if theme.is_dark {
            egui::Color32::from_rgba_unmultiplied(8, 12, 18, 150)
        } else {
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 150)
        };

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop_markdown_overlay_background"),
        ));
        painter.rect_filled(viewport_rect, 0.0, overlay_color);

        egui::Area::new(egui::Id::new("drop_markdown_overlay"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme.status_loading_background)
                    .stroke(egui::Stroke::new(1.0, theme.content_border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(18, 12))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr(self.language, TranslationKey::MessageDropMarkdown))
                                .color(theme.status_loading_text)
                                .strong(),
                        );
                    });
            });
    }
}
