use eframe::egui;

use crate::embedded_svg::{EmbeddedSvgRenderCache, EmbeddedSvgRenderResult};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DiagramCacheKey {
    language: String,
    text_color: [u8; 4],
}

pub type PreparedDiagram = EmbeddedSvgRenderResult;

pub struct DiagramRenderCache {
    cache: EmbeddedSvgRenderCache<DiagramCacheKey>,
}

impl DiagramRenderCache {
    pub fn new() -> Self {
        Self {
            cache: EmbeddedSvgRenderCache::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn prepare(
        &mut self,
        language: &str,
        source: &str,
        text_color: egui::Color32,
    ) -> PreparedDiagram {
        let key = DiagramCacheKey {
            language: language.to_owned(),
            text_color: text_color.to_array(),
        };

        self.cache.prepare_with(key, source, |source| {
            prepare_diagram(language, source, text_color)
        })
    }
}

fn prepare_diagram(_language: &str, _source: &str, _text_color: egui::Color32) -> PreparedDiagram {
    PreparedDiagram::Error("Mermaid SVG rendering is not enabled.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{DiagramRenderCache, PreparedDiagram};
    use eframe::egui::Color32;

    #[test]
    fn reuses_prepared_diagram_by_language_source_and_color() {
        let mut cache = DiagramRenderCache::new();
        let color = Color32::from_rgb(34, 34, 34);

        let first = cache.prepare("mermaid", "graph TD\n  A --> B", color);
        let second = cache.prepare("mermaid", "graph TD\n  A --> B", color);

        assert!(matches!(
            (first, second),
            (PreparedDiagram::Error(first), PreparedDiagram::Error(second))
                if first == second
        ));
    }

    #[test]
    fn keeps_source_available_after_cache_clear() {
        let mut cache = DiagramRenderCache::new();
        let color = Color32::from_rgb(34, 34, 34);

        let first = cache.prepare("mermaid", "graph TD\n  A --> B", color);
        cache.clear();
        let second = cache.prepare("mermaid", "graph TD\n  A --> B", color);

        assert!(matches!(
            (first, second),
            (PreparedDiagram::Error(first), PreparedDiagram::Error(second))
                if first == second
        ));
    }
}
