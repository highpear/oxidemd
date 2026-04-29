use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::diagram::DiagramRenderCache;
use crate::document_loader::{DocumentFingerprint, FileSnapshot};
use crate::image_cache::ImageCache;
use crate::math::MathRenderCache;
use crate::parser::MarkdownDocument;
use crate::renderer::estimate_document_block_heights;

pub struct DocumentSession {
    pub path: PathBuf,
    pub document: Arc<MarkdownDocument>,
    pub fingerprint: DocumentFingerprint,
    pub file_snapshot: Option<FileSnapshot>,
    pub image_cache: ImageCache,
    pub math_render_cache: MathRenderCache,
    pub diagram_render_cache: DiagramRenderCache,
    pub block_height_cache: BlockHeightCache,
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
        Self {
            path,
            document,
            fingerprint,
            file_snapshot,
            image_cache: ImageCache::new(),
            math_render_cache: MathRenderCache::new(),
            diagram_render_cache: DiagramRenderCache::new(),
            block_height_cache: BlockHeightCache::new(),
        }
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
