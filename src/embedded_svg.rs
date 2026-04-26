use std::collections::HashMap;
use std::hash::Hash;

use crate::svg::SvgAsset;

#[derive(Clone)]
pub struct EmbeddedSvgContent {
    asset: SvgAsset,
    source_text: String,
}

impl EmbeddedSvgContent {
    pub fn new(asset: SvgAsset, source_text: String) -> Self {
        Self { asset, source_text }
    }

    pub fn asset(&self) -> &SvgAsset {
        &self.asset
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
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
    use super::{EmbeddedSvgRenderCache, EmbeddedSvgRenderResult};

    #[derive(Clone, Copy, Eq, Hash, PartialEq)]
    struct TestKey(u8);

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
