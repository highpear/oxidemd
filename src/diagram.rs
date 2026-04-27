use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use eframe::egui;

use crate::embedded_svg::{EmbeddedSvgContent, EmbeddedSvgContentKind, EmbeddedSvgRenderResult};
use crate::svg::{SvgAsset, apply_current_color};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DiagramCacheKey {
    language: String,
    text_color: [u8; 4],
}

#[derive(Clone)]
pub enum PreparedDiagram {
    Pending,
    Svg(EmbeddedSvgContent),
    Error(String),
}

enum DiagramRenderState {
    Pending,
    Ready(PreparedDiagram),
}

struct DiagramWorkerResult {
    key: DiagramCacheKey,
    source: String,
    result: PreparedDiagram,
}

pub struct DiagramRenderCache {
    entries: HashMap<(DiagramCacheKey, String), DiagramRenderState>,
    result_sender: Sender<DiagramWorkerResult>,
    result_receiver: Receiver<DiagramWorkerResult>,
}

impl DiagramRenderCache {
    pub fn new() -> Self {
        let (result_sender, result_receiver) = mpsc::channel();

        Self {
            entries: HashMap::new(),
            result_sender,
            result_receiver,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.drain_finished_jobs();
    }

    pub fn prepare(
        &mut self,
        ctx: egui::Context,
        language: &str,
        source: &str,
        text_color: egui::Color32,
    ) -> PreparedDiagram {
        self.drain_finished_jobs();

        let key = DiagramCacheKey {
            language: language.to_owned(),
            text_color: text_color.to_array(),
        };
        let entry_key = (key.clone(), source.to_owned());

        if let Some(state) = self.entries.get(&entry_key) {
            return match state {
                DiagramRenderState::Pending => PreparedDiagram::Pending,
                DiagramRenderState::Ready(result) => result.clone(),
            };
        }

        self.entries.insert(entry_key, DiagramRenderState::Pending);
        self.spawn_render_job(ctx, key, source.to_owned(), text_color);

        PreparedDiagram::Pending
    }

    fn spawn_render_job(
        &self,
        ctx: egui::Context,
        key: DiagramCacheKey,
        source: String,
        text_color: egui::Color32,
    ) {
        let sender = self.result_sender.clone();

        thread::spawn(move || {
            let result = prepare_diagram(&key.language, &source, text_color);
            let _ = sender.send(DiagramWorkerResult {
                key,
                source,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn drain_finished_jobs(&mut self) {
        while let Ok(result) = self.result_receiver.try_recv() {
            self.entries.insert(
                (result.key, result.source),
                DiagramRenderState::Ready(result.result),
            );
        }
    }
}

fn prepare_diagram(_language: &str, source: &str, text_color: egui::Color32) -> PreparedDiagram {
    let svg = match mermaid_rs_renderer::render(source) {
        Ok(svg) => apply_current_color(&svg, text_color),
        Err(error) => return PreparedDiagram::Error(error.to_string()),
    };
    let uri = format!(
        "bytes://diagram-{}-{}.svg",
        color_hash(text_color),
        svg_uri_hash(source)
    );

    match SvgAsset::from_source(uri, svg) {
        Ok(svg) => PreparedDiagram::Svg(EmbeddedSvgContent::new(
            EmbeddedSvgContentKind::Diagram,
            svg,
            source.to_owned(),
        )),
        Err(error) => PreparedDiagram::Error(error),
    }
}

impl From<EmbeddedSvgRenderResult> for PreparedDiagram {
    fn from(result: EmbeddedSvgRenderResult) -> Self {
        match result {
            EmbeddedSvgRenderResult::Svg(content) => Self::Svg(content),
            EmbeddedSvgRenderResult::Error(error) => Self::Error(error),
        }
    }
}

fn color_hash(color: egui::Color32) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        color.r(),
        color.g(),
        color.b(),
        color.a()
    )
}

fn svg_uri_hash(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{DiagramRenderCache, PreparedDiagram};
    use eframe::egui::{Color32, Context};

    #[test]
    fn starts_diagram_rendering_as_pending_work() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);

        let prepared = cache.prepare(ctx, "mermaid", "graph TD\n  A --> B", color);

        assert!(matches!(prepared, PreparedDiagram::Pending));
    }

    #[test]
    fn stores_finished_diagram_result() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let source = "graph TD\n  A --> B";

        let first = cache.prepare(ctx.clone(), "mermaid", source, color);
        assert!(matches!(first, PreparedDiagram::Pending));

        let finished = wait_for_finished_diagram(&mut cache, ctx, "mermaid", source, color);

        assert!(matches!(
            finished,
            PreparedDiagram::Svg(_) | PreparedDiagram::Error(_)
        ));
    }

    #[test]
    fn clears_pending_and_finished_diagram_results() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let source = "graph TD\n  A --> B";

        let _ = cache.prepare(ctx.clone(), "mermaid", source, color);
        cache.clear();
        let prepared = cache.prepare(ctx, "mermaid", source, color);

        assert!(matches!(prepared, PreparedDiagram::Pending));
    }

    fn wait_for_finished_diagram(
        cache: &mut DiagramRenderCache,
        ctx: Context,
        language: &str,
        source: &str,
        color: Color32,
    ) -> PreparedDiagram {
        let started = Instant::now();

        while started.elapsed() < Duration::from_secs(5) {
            let prepared = cache.prepare(ctx.clone(), language, source, color);
            if !matches!(prepared, PreparedDiagram::Pending) {
                return prepared;
            }
            thread::sleep(Duration::from_millis(10));
        }

        panic!("diagram render did not finish");
    }
}
