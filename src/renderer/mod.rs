use eframe::egui::{self, Align, RichText, Ui};
use std::path::Path;

use crate::code_block::render_code_block;
use crate::diagram::DiagramRenderCache;
use crate::i18n::Language;
use crate::image_cache::ImageCache;
use crate::math::MathRenderCache;
use crate::parser::{Block, MarkdownDocument};
use crate::theme::Theme;

mod blocks;
mod embedded;
mod image;
mod inline;
mod sizing;

pub use sizing::estimate_document_block_heights;

use sizing::{estimate_block_height, scale_spacing};

use blocks::{render_blockquote, render_heading, render_list_item, render_table};
use embedded::{render_diagram_block, render_math_block};
use inline::{InlineStyle, render_inline};

pub(super) const BODY_TEXT_SIZE: f32 = 17.0;
pub(super) const QUOTE_TEXT_SIZE: f32 = 16.0;
pub(super) const INLINE_CODE_TEXT_SIZE: f32 = 15.0;
pub(super) const BLOCK_SPACING_PARAGRAPH: f32 = 18.0;
pub(super) const BLOCK_SPACING_SECTION: f32 = 24.0;
pub(super) const LIST_ITEM_SPACING: f32 = 8.0;
pub(super) const TABLE_CELL_MIN_WIDTH: f32 = 120.0;
pub(super) const MATH_BLOCK_PADDING_X: i8 = 18;
pub(super) const MATH_BLOCK_PADDING_Y: i8 = 16;
pub(super) const MATH_BLOCK_DISPLAY_SCALE: f32 = 1.35;
pub(super) const INLINE_MATH_TARGET_HEIGHT_MULTIPLIER: f32 = 1.3;
pub(super) const TALL_INLINE_MATH_TARGET_HEIGHT_MULTIPLIER: f32 = 2.15;
pub(super) const INLINE_MATH_LINE_HEIGHT_MULTIPLIER: f32 = 1.7;
pub(super) const INLINE_MATH_BASELINE_OFFSET_MULTIPLIER: f32 = 0.16;
pub(super) const INLINE_MATH_PLACEHOLDER_MIN_WIDTH: f32 = 28.0;
pub(super) const BLOCK_MATH_PLACEHOLDER_MIN_HEIGHT: f32 = 42.0;
const LARGE_DOCUMENT_BLOCK_THRESHOLD: usize = 2_000;
const VIRTUAL_RENDER_OVERSCAN: f32 = 1_200.0;
pub(super) const ESTIMATED_CHARS_PER_LINE: usize = 90;
pub(super) const COPY_FEEDBACK_DURATION_SECONDS: f64 = 1.2;
pub(super) const EMBEDDED_COPY_FEEDBACK_SLOT_WIDTH: f32 = 88.0;
pub(super) const DIAGRAM_BLOCK_MIN_SCALE: f32 = 0.9;

pub fn render_markdown_document(
    ui: &mut Ui,
    document: &MarkdownDocument,
    ui_language: Language,
    theme: &Theme,
    zoom_factor: f32,
    document_base_dir: Option<&Path>,
    image_cache: &mut ImageCache,
    math_render_cache: &mut MathRenderCache,
    diagram_render_cache: &mut DiagramRenderCache,
    block_heights: &mut [Option<f32>],
    estimated_block_heights: &[f32],
    scroll_to_block: Option<usize>,
    search_query: Option<&str>,
    active_search_block: Option<usize>,
) -> RenderOutcome {
    let mut did_scroll = false;
    let mut needs_scroll_stabilization = false;
    let mut active_heading = None;
    let mut link_actions = LinkActions::default();
    let viewport_top = ui.clip_rect().top();
    let viewport_bottom = ui.clip_rect().bottom();
    let mut render_resources = RenderResources {
        ui_language,
        document_base_dir,
        image_cache,
        math_render_cache,
        diagram_render_cache,
    };

    for (block_index, block) in document.blocks.iter().enumerate() {
        let search_highlight = SearchHighlight {
            query: search_query,
            is_active_block: active_search_block == Some(block_index),
        };
        let measured_block_height = block_heights.get(block_index).and_then(|height| *height);
        let block_height = measured_block_height
            .or_else(|| estimated_block_heights.get(block_index).copied())
            .unwrap_or_else(|| estimate_block_height(block, zoom_factor));
        let block_top = ui.cursor().top();
        let block_bottom = block_top + block_height;

        if should_skip_block(
            document.blocks.len(),
            scroll_to_block,
            block_index,
            block_top,
            block_bottom,
            viewport_top,
            viewport_bottom,
        ) {
            if matches!(block, Block::Heading { .. })
                && block_top <= viewport_top + scale_spacing(8.0, zoom_factor)
            {
                active_heading = Some(block_index);
            }

            ui.add_space(block_height);
            continue;
        }

        let measured_top = ui.cursor().top();

        if scroll_to_block == Some(block_index) {
            let anchor = ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover());
            ui.scroll_to_rect(anchor.rect, Some(Align::TOP));
            did_scroll = true;
            needs_scroll_stabilization = measured_block_height.is_none();
        }

        match block {
            Block::Heading { level, content } => {
                let heading_state = render_heading(
                    ui,
                    *level,
                    content,
                    theme,
                    zoom_factor,
                    scroll_to_block,
                    block_index,
                    viewport_top,
                    search_highlight,
                    &mut link_actions,
                    &mut render_resources,
                );
                did_scroll |= heading_state.did_scroll;

                if heading_state.is_active {
                    active_heading = Some(block_index);
                }
            }
            Block::Paragraph(text) => {
                render_inline(
                    ui,
                    text,
                    InlineStyle::Body,
                    theme,
                    zoom_factor,
                    search_highlight,
                    &mut link_actions,
                    &mut render_resources,
                );
                ui.add_space(scale_spacing(BLOCK_SPACING_PARAGRAPH, zoom_factor));
            }
            Block::UnorderedList(items) => {
                for item in items {
                    render_list_item(
                        ui,
                        RichText::new("- ").color(theme.text_secondary),
                        item,
                        theme,
                        zoom_factor,
                        search_highlight,
                        &mut link_actions,
                        &mut render_resources,
                    );
                    ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                }
                ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
            }
            Block::OrderedList { start, items } => {
                for (index, item) in items.iter().enumerate() {
                    render_list_item(
                        ui,
                        RichText::new(format!("{}. ", start + index as u64))
                            .color(theme.text_secondary),
                        item,
                        theme,
                        zoom_factor,
                        search_highlight,
                        &mut link_actions,
                        &mut render_resources,
                    );
                    ui.add_space(scale_spacing(LIST_ITEM_SPACING, zoom_factor));
                }
                ui.add_space(scale_spacing(BLOCK_SPACING_SECTION, zoom_factor));
            }
            Block::BlockQuote(lines) => {
                render_blockquote(
                    ui,
                    lines,
                    theme,
                    zoom_factor,
                    search_highlight,
                    &mut link_actions,
                    &mut render_resources,
                );
            }
            Block::CodeBlock { language, code } => {
                render_code_block(
                    ui,
                    block_index,
                    ui_language,
                    language.as_deref(),
                    code,
                    theme,
                    zoom_factor,
                );
            }
            Block::DiagramBlock { language, source } => {
                render_diagram_block(
                    ui,
                    block_index,
                    language,
                    source,
                    theme,
                    zoom_factor,
                    &mut render_resources,
                );
            }
            Block::MathBlock { expression } => {
                render_math_block(
                    ui,
                    block_index,
                    expression,
                    theme,
                    zoom_factor,
                    &mut render_resources,
                );
            }
            Block::Table {
                alignments,
                headers,
                rows,
            } => {
                render_table(
                    ui,
                    block_index,
                    alignments,
                    headers,
                    rows,
                    theme,
                    zoom_factor,
                    search_highlight,
                    &mut link_actions,
                    &mut render_resources,
                );
            }
        }

        if let Some(height) = block_heights.get_mut(block_index) {
            let measured_height = (ui.cursor().top() - measured_top).max(0.0);
            *height = Some(measured_height);
        }
    }

    RenderOutcome {
        did_scroll,
        needs_scroll_stabilization,
        active_heading,
        clicked_anchor: link_actions.clicked_anchor,
        clicked_external_link: link_actions.clicked_external_link,
    }
}

fn should_skip_block(
    block_count: usize,
    scroll_to_block: Option<usize>,
    block_index: usize,
    block_top: f32,
    block_bottom: f32,
    viewport_top: f32,
    viewport_bottom: f32,
) -> bool {
    if block_count < LARGE_DOCUMENT_BLOCK_THRESHOLD || scroll_to_block == Some(block_index) {
        return false;
    }

    block_bottom < viewport_top - VIRTUAL_RENDER_OVERSCAN
        || block_top > viewport_bottom + VIRTUAL_RENDER_OVERSCAN
}

pub struct RenderOutcome {
    pub did_scroll: bool,
    pub needs_scroll_stabilization: bool,
    pub active_heading: Option<usize>,
    pub clicked_anchor: Option<String>,
    pub clicked_external_link: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) struct SearchHighlight<'a> {
    pub(super) query: Option<&'a str>,
    pub(super) is_active_block: bool,
}

pub(super) struct RenderResources<'a> {
    pub(super) ui_language: Language,
    pub(super) document_base_dir: Option<&'a Path>,
    pub(super) image_cache: &'a mut ImageCache,
    pub(super) math_render_cache: &'a mut MathRenderCache,
    pub(super) diagram_render_cache: &'a mut DiagramRenderCache,
}

#[derive(Default)]
pub(super) struct LinkActions {
    pub(super) clicked_anchor: Option<String>,
    pub(super) clicked_external_link: Option<String>,
}

pub(super) struct HeadingRenderState {
    pub(super) did_scroll: bool,
    pub(super) is_active: bool,
}
