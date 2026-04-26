use std::collections::HashMap;
use std::hash::Hash;

use crate::svg::SvgAsset;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddedSvgContentKind {
    Math,
    #[allow(dead_code)]
    Diagram,
}

#[derive(Clone)]
pub struct EmbeddedSvgContent {
    kind: EmbeddedSvgContentKind,
    asset: SvgAsset,
    source_text: String,
}

impl EmbeddedSvgContent {
    pub fn new(kind: EmbeddedSvgContentKind, asset: SvgAsset, source_text: String) -> Self {
        Self {
            kind,
            asset,
            source_text,
        }
    }

    pub fn kind(&self) -> EmbeddedSvgContentKind {
        self.kind
    }

    pub fn asset(&self) -> &SvgAsset {
        &self.asset
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn source_action(&self) -> EmbeddedSourceAction<'_> {
        EmbeddedSourceAction::new(self.source_text())
    }
}

#[derive(Clone, Copy)]
pub struct EmbeddedSourceAction<'a> {
    source_text: &'a str,
}

impl<'a> EmbeddedSourceAction<'a> {
    pub fn new(source_text: &'a str) -> Self {
        Self { source_text }
    }

    pub fn source_text(&self) -> &'a str {
        self.source_text
    }
}

#[derive(Clone)]
pub enum EmbeddedSvgRenderResult {
    Svg(EmbeddedSvgContent),
    Error(String),
}

pub struct EmbeddedSvgRenderCache<Key>
where
    Key: Eq + Hash,
{
    entries: HashMap<(Key, String), EmbeddedSvgRenderResult>,
}

impl<Key> EmbeddedSvgRenderCache<Key>
where
    Key: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn prepare_with<F>(
        &mut self,
        key: Key,
        source_text: &str,
        prepare: F,
    ) -> EmbeddedSvgRenderResult
    where
        F: FnOnce(&str) -> EmbeddedSvgRenderResult,
    {
        self.entries
            .entry((key, source_text.to_owned()))
            .or_insert_with(|| prepare(source_text))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmbeddedSourceAction, EmbeddedSvgContent, EmbeddedSvgContentKind, EmbeddedSvgRenderCache,
        EmbeddedSvgRenderResult,
    };
    use crate::svg::SvgAsset;

    #[derive(Clone, Copy, Eq, Hash, PartialEq)]
    struct TestKey(u8);

    #[test]
    fn content_tracks_kind_and_source_action_separately_from_svg_asset() {
        let asset = SvgAsset::from_source(
            "bytes://test.svg".to_owned(),
            "<svg viewBox=\"0 0 1 1\"><g /></svg>".to_owned(),
        )
        .expect("valid test SVG");
        let content =
            EmbeddedSvgContent::new(EmbeddedSvgContentKind::Math, asset, "x^2".to_owned());

        assert_eq!(content.kind(), EmbeddedSvgContentKind::Math);
        assert_eq!(content.asset().uri(), "bytes://test.svg");
        assert_eq!(content.source_action().source_text(), "x^2");
    }

    #[test]
    fn source_action_can_wrap_fallback_source_text() {
        let action = EmbeddedSourceAction::new("fallback");

        assert_eq!(action.source_text(), "fallback");
    }

    #[test]
    fn reuses_render_result_by_key_and_source_text() {
        let mut cache = EmbeddedSvgRenderCache::new();
        let first = cache.prepare_with(TestKey(1), "x", |_| {
            EmbeddedSvgRenderResult::Error("first".to_owned())
        });
        let second = cache.prepare_with(TestKey(1), "x", |_| {
            EmbeddedSvgRenderResult::Error("second".to_owned())
        });

        assert!(matches!(
            (first, second),
            (
                EmbeddedSvgRenderResult::Error(first),
                EmbeddedSvgRenderResult::Error(second)
            ) if first == "first" && second == "first"
        ));
    }

    #[test]
    fn separates_render_result_by_source_text() {
        let mut cache = EmbeddedSvgRenderCache::new();
        let first = cache.prepare_with(TestKey(1), "x", |source| {
            EmbeddedSvgRenderResult::Error(source.to_owned())
        });
        let second = cache.prepare_with(TestKey(1), "y", |source| {
            EmbeddedSvgRenderResult::Error(source.to_owned())
        });

        assert!(matches!(
            (first, second),
            (
                EmbeddedSvgRenderResult::Error(first),
                EmbeddedSvgRenderResult::Error(second)
            ) if first == "x" && second == "y"
        ));
    }
}
