use eframe::egui::{self, Align, Frame, RichText, Stroke, Ui};
use pulldown_cmark::{Alignment, HeadingLevel};

use crate::parser::{InlineContent, InlineSpan};
use crate::theme::Theme;

use super::sizing::{scale_margin, scale_spacing};
use super::{
    BLOCK_SPACING_SECTION, BODY_TEXT_SIZE, HeadingRenderState, InlineStyle, LinkActions,
    RenderResources, SearchHighlight, TABLE_CELL_MIN_WIDTH, inline_content_width, render_inline,
    render_inline_span,
};

pub(super) fn render_heading(
    ui: &mut Ui,
    level: HeadingLevel,
    content: &InlineContent,
    theme: &Theme,
    zoom_factor: f32,
    scroll_to_block: Option<usize>,
    block_index: usize,
    viewport_top: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) -> HeadingRenderState {
    let size = match level {
        HeadingLevel::H1 => 31.0,
        HeadingLevel::H2 => 26.0,
        HeadingLevel::H3 => 22.0,
        HeadingLevel::H4 => 19.0,
        HeadingLevel::H5 => 17.0,
        HeadingLevel::H6 => 16.0,
    } * zoom_factor;

    let anchor = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());

    if scroll_to_block == Some(block_index) {
        ui.scroll_to_rect(anchor.rect, Some(Align::TOP));
    }

    let heading_response = ui.scope(|ui| {
        render_inline(
            ui,
            content,
            InlineStyle::Heading(size),
            theme,
            zoom_factor,
            search_highlight,
            link_actions,
            render_resources,
        );
        ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
    });

    let heading_rect = anchor.rect.union(heading_response.response.rect);
    let is_active = heading_rect.top() <= viewport_top + scale_spacing(8.0, zoom_factor);

    HeadingRenderState {
        did_scroll: scroll_to_block == Some(block_index),
        is_active,
    }
}

pub(super) fn render_list_item(
    ui: &mut Ui,
    marker: RichText,
    item: &InlineContent,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [scale_spacing(24.0, zoom_factor), 0.0],
            egui::Label::new(marker.size(BODY_TEXT_SIZE * zoom_factor)),
        );

        ui.vertical(|ui| {
            render_inline(
                ui,
                item,
                InlineStyle::Body,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                render_resources,
            );
        });
    });
}

pub(super) fn render_blockquote(
    ui: &mut Ui,
    lines: &[InlineContent],
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    Frame::new()
        .fill(theme.quote_background)
        .stroke(Stroke::new(1.0, theme.quote_border))
        .inner_margin(egui::Margin::symmetric(
            scale_margin(16, zoom_factor),
            scale_margin(14, zoom_factor),
        ))
        .show(ui, |ui| {
            for line in lines {
                render_inline(
                    ui,
                    line,
                    InlineStyle::Quote,
                    theme,
                    zoom_factor,
                    search_highlight,
                    link_actions,
                    render_resources,
                );
                ui.add_space(scale_spacing(6.0, zoom_factor));
            }
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

pub(super) fn render_table(
    ui: &mut Ui,
    block_index: usize,
    alignments: &[Alignment],
    headers: &[InlineContent],
    rows: &[Vec<InlineContent>],
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    let column_count = table_column_count(headers, rows);
    if column_count == 0 {
        return;
    }

    Frame::new()
        .stroke(Stroke::new(1.0, theme.content_border))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .id_salt(("table_scroll", block_index))
                .show(ui, |ui| {
                    egui::Grid::new(("markdown_table", block_index))
                        .num_columns(column_count)
                        .spacing(egui::vec2(16.0, 10.0))
                        .striped(true)
                        .show(ui, |ui| {
                            render_table_row(
                                ui,
                                headers,
                                column_count,
                                alignments,
                                InlineStyle::TableHeader,
                                theme,
                                zoom_factor,
                                search_highlight,
                                link_actions,
                                render_resources,
                            );

                            for row in rows {
                                render_table_row(
                                    ui,
                                    row,
                                    column_count,
                                    alignments,
                                    InlineStyle::TableCell,
                                    theme,
                                    zoom_factor,
                                    search_highlight,
                                    link_actions,
                                    render_resources,
                                );
                            }
                        });
                });
        });

    ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
}

fn render_table_row(
    ui: &mut Ui,
    cells: &[InlineContent],
    column_count: usize,
    alignments: &[Alignment],
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    for column_index in 0..column_count {
        let cell = cells.get(column_index);
        let alignment = alignments
            .get(column_index)
            .copied()
            .unwrap_or(Alignment::None);
        let width = scale_spacing(TABLE_CELL_MIN_WIDTH, zoom_factor);

        let cell_frame = match style {
            InlineStyle::TableHeader => Frame::new()
                .fill(theme.widget_inactive_background)
                .inner_margin(egui::Margin::symmetric(8, 6)),
            InlineStyle::TableCell => Frame::new().inner_margin(egui::Margin::symmetric(8, 4)),
            _ => Frame::new(),
        };

        cell_frame.show(ui, |ui| {
            ui.set_min_width(width);
            render_aligned_cell(
                ui,
                cell,
                alignment,
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                render_resources,
            );
        });
    }

    ui.end_row();
}

fn render_aligned_cell(
    ui: &mut Ui,
    cell: Option<&InlineContent>,
    alignment: Alignment,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    let Some(cell) = cell else {
        ui.label("");
        return;
    };

    match alignment {
        Alignment::Center | Alignment::Right => render_inline_aligned(
            ui,
            cell,
            alignment,
            style,
            theme,
            zoom_factor,
            search_highlight,
            link_actions,
            render_resources,
        ),
        Alignment::None | Alignment::Left => {
            render_inline(
                ui,
                cell,
                style,
                theme,
                zoom_factor,
                search_highlight,
                link_actions,
                render_resources,
            );
        }
    }
}

fn render_inline_aligned(
    ui: &mut Ui,
    content: &InlineContent,
    alignment: Alignment,
    style: InlineStyle,
    theme: &Theme,
    zoom_factor: f32,
    search_highlight: SearchHighlight<'_>,
    link_actions: &mut LinkActions,
    render_resources: &mut RenderResources<'_>,
) {
    let available_width = ui.available_width();
    let line_width = inline_content_width(
        ui,
        content,
        style,
        theme,
        zoom_factor,
        search_highlight,
        render_resources,
    );
    let leading_space = match alignment {
        Alignment::Center => ((available_width - line_width) / 2.0).max(0.0),
        Alignment::Right => (available_width - line_width).max(0.0),
        Alignment::None | Alignment::Left => 0.0,
    };

    ui.horizontal(|ui| {
        ui.add_space(leading_space);
        for span in &content.spans {
            if matches!(span, InlineSpan::LineBreak) {
                continue;
            }

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

fn table_column_count(headers: &[InlineContent], rows: &[Vec<InlineContent>]) -> usize {
    rows.iter()
        .map(Vec::len)
        .chain(std::iter::once(headers.len()))
        .max()
        .unwrap_or(0)
}
