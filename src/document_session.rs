use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::diagram::DiagramRenderCache;
use crate::document_loader::{DocumentFingerprint, FileSnapshot};
use crate::image_cache::ImageCache;
use crate::math::MathRenderCache;
use crate::parser::MarkdownDocument;
use crate::renderer::estimate_document_block_heights;
use crate::search::SearchState;
use crate::watcher::{FileWatchEvent, FileWatcherHandle};

pub struct DocumentSession {
    pub path: PathBuf,
    pub document: Arc<MarkdownDocument>,
    pub fingerprint: DocumentFingerprint,
    pub file_snapshot: Option<FileSnapshot>,
    pub image_cache: ImageCache,
    pub math_render_cache: MathRenderCache,
    pub diagram_render_cache: DiagramRenderCache,
    pub block_height_cache: BlockHeightCache,
    pub pending_block_scroll: Option<usize>,
    pub active_heading: Option<usize>,
    pub selected_heading: Option<usize>,
    pub search: SearchState,
    pub watcher: Option<FileWatcherHandle>,
    pub pending_reload_at: Option<Instant>,
    pub in_flight_reload_id: Option<u64>,
}

pub struct WatchEventSummary {
    pub saw_change: bool,
    pub error: Option<String>,
}

pub struct BlockHeightCache {
    fingerprint: Option<DocumentFingerprint>,
    zoom_factor_bits: u32,
    content_width_bits: u32,
    estimated_zoom_factor_bits: u32,
    pub heights: Vec<Option<f32>>,
    pub estimated_heights: Vec<f32>,
}

impl DocumentSession {
    pub fn new(
        path: PathBuf,
        document: Arc<MarkdownDocument>,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) -> Self {
        let active_heading = document.headings().first().map(|item| item.block_index);
        let mut session = Self {
            path,
            document,
            fingerprint,
            file_snapshot,
            image_cache: ImageCache::new(),
            math_render_cache: MathRenderCache::new(),
            diagram_render_cache: DiagramRenderCache::new(),
            block_height_cache: BlockHeightCache::new(),
            pending_block_scroll: None,
            active_heading,
            selected_heading: None,
            search: SearchState::new(),
            watcher: None,
            pending_reload_at: None,
            in_flight_reload_id: None,
        };
        session.refresh_search_matches();

        session
    }

    pub fn replace_document(
        &mut self,
        document: Arc<MarkdownDocument>,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) {
        self.document = document;
        self.fingerprint = fingerprint;
        self.file_snapshot = file_snapshot;
        self.clear_render_caches();
        self.reset_navigation();
        self.refresh_search_matches();
    }

    pub fn update_unchanged_snapshot(
        &mut self,
        fingerprint: DocumentFingerprint,
        file_snapshot: Option<FileSnapshot>,
    ) {
        self.fingerprint = fingerprint;
        self.file_snapshot = file_snapshot;
    }

    pub fn clear_render_caches(&mut self) {
        self.image_cache.clear();
        self.math_render_cache.clear();
        self.diagram_render_cache.clear();
        self.block_height_cache.clear();
    }

    pub fn base_dir(&self) -> Option<&Path> {
        self.path.parent()
    }

    pub fn set_watcher(&mut self, watcher: FileWatcherHandle) {
        self.watcher = Some(watcher);
    }

    pub fn clear_watcher(&mut self) {
        self.watcher = None;
    }

    pub fn drain_watch_events(&self) -> WatchEventSummary {
        let mut summary = WatchEventSummary {
            saw_change: false,
            error: None,
        };

        if let Some(watcher) = &self.watcher {
            while let Ok(event) = watcher.receiver.try_recv() {
                match event {
                    FileWatchEvent::Changed => {
                        summary.saw_change = true;
                    }
                    FileWatchEvent::Error(error) => {
                        summary.error = Some(error);
                    }
                }
            }
        }

        summary
    }

    pub fn schedule_reload(&mut self) {
        self.pending_reload_at = Some(Instant::now());
    }

    pub fn clear_pending_reload(&mut self) {
        self.pending_reload_at = None;
    }

    pub fn start_reload(&mut self, id: u64) {
        self.pending_reload_at = None;
        self.in_flight_reload_id = Some(id);
    }

    pub fn finish_reload(&mut self) {
        self.in_flight_reload_id = None;
    }

    pub fn refresh_search_matches(&mut self) {
        self.search.refresh_matches(Some(&self.document));
    }

    pub fn select_search_match(&mut self, index: usize) {
        if let Some(block_index) = self.search.select_match(index) {
            self.pending_block_scroll = Some(block_index);
            self.selected_heading = None;
        }
    }

    pub fn select_next_search_match(&mut self) {
        if let Some(block_index) = self.search.select_next() {
            self.pending_block_scroll = Some(block_index);
            self.selected_heading = None;
        }
    }

    pub fn select_previous_search_match(&mut self) {
        if let Some(block_index) = self.search.select_previous() {
            self.pending_block_scroll = Some(block_index);
            self.selected_heading = None;
        }
    }

    pub fn jump_to_heading(&mut self, block_index: usize) {
        self.selected_heading = Some(block_index);
        self.active_heading = Some(block_index);
        self.pending_block_scroll = Some(block_index);
    }

    pub fn clear_selected_heading(&mut self) {
        self.selected_heading = None;
    }

    fn reset_navigation(&mut self) {
        self.pending_block_scroll = None;
        self.active_heading = self
            .document
            .headings()
            .first()
            .map(|item| item.block_index);
        self.selected_heading = None;
    }
}

impl BlockHeightCache {
    fn new() -> Self {
        Self {
            fingerprint: None,
            zoom_factor_bits: 0,
            content_width_bits: 0,
            estimated_zoom_factor_bits: 0,
            heights: Vec::new(),
            estimated_heights: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.fingerprint = None;
        self.zoom_factor_bits = 0;
        self.content_width_bits = 0;
        self.estimated_zoom_factor_bits = 0;
        self.heights.clear();
        self.estimated_heights.clear();
    }

    pub fn prepare(
        &mut self,
        fingerprint: DocumentFingerprint,
        document: &MarkdownDocument,
        zoom_factor: f32,
        content_width: f32,
    ) {
        let zoom_factor_bits = zoom_factor.to_bits();
        let content_width_bits = content_width.round().to_bits();
        let document_or_zoom_changed =
            self.fingerprint != Some(fingerprint) || self.zoom_factor_bits != zoom_factor_bits;
        let content_width_changed = self.content_width_bits != content_width_bits;

        if document_or_zoom_changed {
            self.fingerprint = Some(fingerprint);
            self.zoom_factor_bits = zoom_factor_bits;
            self.estimated_zoom_factor_bits = zoom_factor_bits;
            self.estimated_heights = estimate_document_block_heights(document, zoom_factor);
        }

        if document_or_zoom_changed || content_width_changed {
            self.content_width_bits = content_width_bits;
            self.heights.clear();
        }

        if self.heights.len() != document.blocks.len() {
            self.heights.resize(document.blocks.len(), None);
        }

        if self.estimated_zoom_factor_bits != zoom_factor_bits
            || self.estimated_heights.len() != document.blocks.len()
        {
            self.estimated_zoom_factor_bits = zoom_factor_bits;
            self.estimated_heights = estimate_document_block_heights(document, zoom_factor);
        }
    }
}
