use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use eframe::egui;

use crate::embedded_svg::{EmbeddedSvgContent, EmbeddedSvgContentKind};
use crate::metrics;
use crate::svg::{SvgAsset, apply_current_color};

const MAX_ACTIVE_DIAGRAM_RENDER_JOBS: usize = 2;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct DiagramCacheKey {
    language: String,
    text_color: [u8; 4],
    background_color: [u8; 4],
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
    generation: u64,
    key: DiagramCacheKey,
    source: String,
    result: PreparedDiagram,
}

pub struct DiagramRenderCache {
    entries: HashMap<(DiagramCacheKey, String), DiagramRenderState>,
    queued_jobs: VecDeque<DiagramRenderJob>,
    active_job_count: usize,
    result_sender: Sender<DiagramWorkerResult>,
    result_receiver: Receiver<DiagramWorkerResult>,
    generation: u64,
}

struct DiagramRenderJob {
    generation: u64,
    key: DiagramCacheKey,
    source: String,
    text_color: egui::Color32,
    background_color: egui::Color32,
}

impl DiagramRenderCache {
    pub fn new() -> Self {
        let (result_sender, result_receiver) = mpsc::channel();

        Self {
            entries: HashMap::new(),
            queued_jobs: VecDeque::new(),
            active_job_count: 0,
            result_sender,
            result_receiver,
            generation: 0,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.queued_jobs.clear();
        self.active_job_count = 0;
        self.generation = self.generation.wrapping_add(1);
        self.drain_finished_jobs();
    }

    pub fn prepare(
        &mut self,
        ctx: egui::Context,
        language: &str,
        source: &str,
        text_color: egui::Color32,
        background_color: egui::Color32,
    ) -> PreparedDiagram {
        self.drain_finished_jobs();
        self.start_queued_jobs(ctx.clone());

        let key = DiagramCacheKey {
            language: language.to_owned(),
            text_color: text_color.to_array(),
            background_color: background_color.to_array(),
        };
        let entry_key = (key.clone(), source.to_owned());

        if let Some(state) = self.entries.get(&entry_key) {
            return match state {
                DiagramRenderState::Pending => PreparedDiagram::Pending,
                DiagramRenderState::Ready(result) => result.clone(),
            };
        }

        self.entries.insert(entry_key, DiagramRenderState::Pending);
        self.queued_jobs.push_back(DiagramRenderJob {
            generation: self.generation,
            key,
            source: source.to_owned(),
            text_color,
            background_color,
        });
        self.start_queued_jobs(ctx);

        PreparedDiagram::Pending
    }

    fn start_queued_jobs(&mut self, ctx: egui::Context) {
        while self.active_job_count < MAX_ACTIVE_DIAGRAM_RENDER_JOBS {
            let Some(job) = self.queued_jobs.pop_front() else {
                break;
            };

            self.active_job_count += 1;
            self.spawn_render_job(ctx.clone(), job);
        }
    }

    fn spawn_render_job(&self, ctx: egui::Context, job: DiagramRenderJob) {
        let sender = self.result_sender.clone();

        thread::spawn(move || {
            let started = Instant::now();
            let result = prepare_diagram(
                &job.key.language,
                &job.source,
                job.text_color,
                job.background_color,
            );
            let outcome = match &result {
                PreparedDiagram::Pending => "pending",
                PreparedDiagram::Svg(_) => "ok",
                PreparedDiagram::Error(_) => "error",
            };
            metrics::log_diagram_render(
                &job.key.language,
                job.source.len(),
                started.elapsed(),
                outcome,
            );
            let _ = sender.send(DiagramWorkerResult {
                generation: job.generation,
                key: job.key,
                source: job.source,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn drain_finished_jobs(&mut self) {
        let mut finished_current_jobs = 0usize;

        while let Ok(result) = self.result_receiver.try_recv() {
            if result.generation != self.generation {
                continue;
            }

            finished_current_jobs += 1;
            self.entries.insert(
                (result.key, result.source),
                DiagramRenderState::Ready(result.result),
            );
        }

        self.active_job_count = self.active_job_count.saturating_sub(finished_current_jobs);
    }
}

fn prepare_diagram(
    _language: &str,
    source: &str,
    text_color: egui::Color32,
    background_color: egui::Color32,
) -> PreparedDiagram {
    if let Err(error) = validate_diagram_source(source) {
        return PreparedDiagram::Error(error);
    }

    let svg = match mermaid_rs_renderer::render_with_options(
        source,
        mermaid_rs_renderer::RenderOptions {
            theme: mermaid_theme(text_color, background_color),
            layout: mermaid_rs_renderer::LayoutConfig::default(),
        },
    ) {
        Ok(svg) => apply_current_color(&svg, text_color),
        Err(error) => return PreparedDiagram::Error(error.to_string()),
    };
    let uri = format!(
        "bytes://diagram-{}-{}-{}.svg",
        color_hash(text_color),
        color_hash(background_color),
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

fn mermaid_theme(
    text_color: egui::Color32,
    background_color: egui::Color32,
) -> mermaid_rs_renderer::Theme {
    let text = color_to_hex(text_color);
    let background = color_to_hex(background_color);
    let mut theme = mermaid_rs_renderer::Theme::modern();

    theme.background = background.clone();
    theme.primary_color = background.clone();
    theme.secondary_color = background.clone();
    theme.tertiary_color = background.clone();
    theme.edge_label_background = background.clone();
    theme.cluster_background = background.clone();
    theme.sequence_actor_fill = background.clone();
    theme.sequence_note_fill = background.clone();
    theme.sequence_activation_fill = background.clone();

    theme.primary_text_color = text.clone();
    theme.text_color = text.clone();
    theme.pie_title_text_color = text.clone();
    theme.pie_section_text_color = text.clone();
    theme.pie_legend_text_color = text.clone();

    theme.primary_border_color = text.clone();
    theme.line_color = text.clone();
    theme.cluster_border = text.clone();
    theme.sequence_actor_border = text.clone();
    theme.sequence_actor_line = text.clone();
    theme.sequence_note_border = text.clone();
    theme.sequence_activation_border = text.clone();
    theme.pie_stroke_color = text.clone();
    theme.pie_outer_stroke_color = text;

    theme
}

fn color_to_hex(color: egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

fn validate_diagram_source(source: &str) -> Result<(), String> {
    for line in source.lines() {
        let trimmed = strip_mermaid_comment(line).trim();
        if trimmed.is_empty() || is_mermaid_header(trimmed) {
            continue;
        }

        if has_dangling_arrow_operator(trimmed) {
            return Err("Mermaid diagram has an incomplete arrow.".to_owned());
        }
    }

    Ok(())
}

fn strip_mermaid_comment(line: &str) -> &str {
    line.split_once("%%")
        .map(|(before_comment, _)| before_comment)
        .unwrap_or(line)
}

fn is_mermaid_header(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("flowchart ")
        || lower.starts_with("graph ")
        || lower == "sequencediagram"
        || lower == "classdiagram"
        || lower == "statediagram-v2"
}

fn has_dangling_arrow_operator(line: &str) -> bool {
    let Some(last_token) = line.split_whitespace().last() else {
        return false;
    };

    last_token.len() >= 2
        && last_token.chars().all(is_arrow_operator_char)
        && last_token
            .chars()
            .any(|character| matches!(character, '-' | '='))
}

fn is_arrow_operator_char(character: char) -> bool {
    matches!(character, '<' | '>' | '-' | '.' | '=' | 'o' | 'x')
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
    use std::fs;
    use std::path::PathBuf;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{DiagramRenderCache, PreparedDiagram};
    use eframe::egui::{Color32, Context};

    #[test]
    fn starts_diagram_rendering_as_pending_work() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let background = Color32::from_rgb(250, 250, 250);

        let prepared = cache.prepare(ctx, "mermaid", "graph TD\n  A --> B", color, background);

        assert!(matches!(prepared, PreparedDiagram::Pending));
    }

    #[test]
    fn limits_active_diagram_render_jobs() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let background = Color32::from_rgb(250, 250, 250);

        cache.active_job_count = super::MAX_ACTIVE_DIAGRAM_RENDER_JOBS;
        let prepared = cache.prepare(ctx, "mermaid", "graph TD\n  A --> B", color, background);

        assert!(matches!(prepared, PreparedDiagram::Pending));
        assert_eq!(
            cache.active_job_count,
            super::MAX_ACTIVE_DIAGRAM_RENDER_JOBS
        );
        assert_eq!(cache.queued_jobs.len(), 1);
    }

    #[test]
    fn stores_finished_diagram_result() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let background = Color32::from_rgb(250, 250, 250);
        let source = "graph TD\n  A --> B";

        let first = cache.prepare(ctx.clone(), "mermaid", source, color, background);
        assert!(matches!(first, PreparedDiagram::Pending));

        let finished =
            wait_for_finished_diagram(&mut cache, ctx, "mermaid", source, color, background);

        assert!(matches!(
            finished,
            PreparedDiagram::Svg(_) | PreparedDiagram::Error(_)
        ));
    }

    #[test]
    fn reuses_finished_diagram_result_from_cache() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let background = Color32::from_rgb(250, 250, 250);
        let source = "graph TD\n  A --> B";

        let finished = wait_for_finished_diagram(
            &mut cache,
            ctx.clone(),
            "mermaid",
            source,
            color,
            background,
        );
        assert!(matches!(
            finished,
            PreparedDiagram::Svg(_) | PreparedDiagram::Error(_)
        ));

        let cached = cache.prepare(ctx, "mermaid", source, color, background);
        assert!(matches!(
            cached,
            PreparedDiagram::Svg(_) | PreparedDiagram::Error(_)
        ));
    }

    #[test]
    fn clears_pending_and_finished_diagram_results() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let background = Color32::from_rgb(250, 250, 250);
        let source = "graph TD\n  A --> B";

        let _ = cache.prepare(ctx.clone(), "mermaid", source, color, background);
        cache.clear();
        let prepared = cache.prepare(ctx, "mermaid", source, color, background);

        assert!(matches!(prepared, PreparedDiagram::Pending));
    }

    #[test]
    fn ignores_finished_work_from_before_clear() {
        let mut cache = DiagramRenderCache::new();
        let ctx = Context::default();
        let color = Color32::from_rgb(34, 34, 34);
        let background = Color32::from_rgb(250, 250, 250);
        let source = "graph TD\n  A --> B";

        let _ = cache.prepare(ctx, "mermaid", source, color, background);
        cache.clear();

        thread::sleep(Duration::from_millis(50));
        cache.drain_finished_jobs();

        assert!(cache.entries.is_empty());
    }

    #[test]
    fn rejects_dangling_flowchart_arrow_before_rendering() {
        let prepared = super::prepare_diagram(
            "mermaid",
            "flowchart TD\n    Broken -->",
            Color32::from_rgb(34, 34, 34),
            Color32::from_rgb(250, 250, 250),
        );

        assert!(
            matches!(prepared, PreparedDiagram::Error(error) if error.contains("incomplete arrow"))
        );
    }

    #[test]
    fn renders_common_mermaid_evaluation_diagrams() {
        for (name, source) in mermaid_evaluation_cases() {
            let started = Instant::now();
            let prepared = super::prepare_diagram(
                "mermaid",
                source,
                Color32::from_rgb(34, 34, 34),
                Color32::from_rgb(250, 250, 250),
            );
            eprintln!(
                "[perf] diagram_eval: {} ms, {name}, {} source bytes",
                started.elapsed().as_millis(),
                source.len()
            );

            assert!(
                matches!(prepared, PreparedDiagram::Svg(_)),
                "{name} should render as SVG"
            );
        }

        let invalid_source = "flowchart TD\n    Broken -->";
        let started = Instant::now();
        let invalid = super::prepare_diagram(
            "mermaid",
            invalid_source,
            Color32::from_rgb(34, 34, 34),
            Color32::from_rgb(250, 250, 250),
        );
        eprintln!(
            "[perf] diagram_eval: {} ms, invalid, {} source bytes",
            started.elapsed().as_millis(),
            invalid_source.len()
        );
        assert!(matches!(invalid, PreparedDiagram::Error(_)));
    }

    #[test]
    #[ignore = "writes OxideMD Mermaid SVG comparison artifacts"]
    fn exports_mermaid_evaluation_svgs_for_cli_comparison() {
        let output_dir = std::env::var_os("OXIDEMD_MERMAID_OUTPUT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("oxidemd-mermaid-native-comparison"));

        if output_dir.exists() {
            fs::remove_dir_all(&output_dir).expect("remove old comparison output");
        }
        fs::create_dir_all(&output_dir).expect("create comparison output directory");

        for (index, (name, source)) in mermaid_evaluation_cases().into_iter().enumerate() {
            let prepared = super::prepare_diagram(
                "mermaid",
                source,
                Color32::from_rgb(34, 34, 34),
                Color32::from_rgb(250, 250, 250),
            );
            let output_index = if name == "larger-flowchart" {
                6
            } else {
                index + 1
            };
            let output_path = output_dir.join(format!("{output_index:02}-{name}.svg"));

            match prepared {
                PreparedDiagram::Svg(content) => {
                    fs::write(output_path, content.asset().bytes().as_ref())
                        .expect("write OxideMD SVG output");
                }
                PreparedDiagram::Error(error) => {
                    panic!("{name} should render as SVG, got error: {error}");
                }
                PreparedDiagram::Pending => {
                    panic!("{name} should render synchronously in the export test");
                }
            }
        }

        let invalid_source = "flowchart TD\n    Broken -->";
        let invalid = super::prepare_diagram(
            "mermaid",
            invalid_source,
            Color32::from_rgb(34, 34, 34),
            Color32::from_rgb(250, 250, 250),
        );
        let PreparedDiagram::Error(error) = invalid else {
            panic!("invalid diagram should produce an error");
        };
        fs::write(output_dir.join("05-invalid-error.txt"), error)
            .expect("write OxideMD invalid diagram error");

        eprintln!("Wrote OxideMD Mermaid SVG comparison files to {output_dir:?}");
    }

    #[test]
    fn rendered_flowchart_svg_keeps_text_nodes() {
        let prepared = super::prepare_diagram(
            "mermaid",
            "flowchart LR\n    Open[Open Markdown] --> Parse[Parse document]",
            Color32::from_rgb(34, 34, 34),
            Color32::from_rgb(250, 250, 250),
        );

        assert!(matches!(
            prepared,
            PreparedDiagram::Svg(content)
                if std::str::from_utf8(content.asset().bytes().as_ref())
                    .is_ok_and(|svg| svg.contains("<text") && svg.contains("Open Markdown"))
        ));
    }

    #[test]
    fn rendered_svg_uses_requested_theme_colors() {
        let prepared = super::prepare_diagram(
            "mermaid",
            "flowchart LR\n    A --> B",
            Color32::from_rgb(224, 232, 242),
            Color32::from_rgb(29, 39, 52),
        );

        assert!(matches!(
            prepared,
            PreparedDiagram::Svg(content)
                if std::str::from_utf8(content.asset().bytes().as_ref()).is_ok_and(|svg| {
                    svg.contains("#e0e8f2") && svg.contains("#1d2734")
                })
        ));
    }

    fn wait_for_finished_diagram(
        cache: &mut DiagramRenderCache,
        ctx: Context,
        language: &str,
        source: &str,
        color: Color32,
        background: Color32,
    ) -> PreparedDiagram {
        let started = Instant::now();

        while started.elapsed() < Duration::from_secs(5) {
            let prepared = cache.prepare(ctx.clone(), language, source, color, background);
            if !matches!(prepared, PreparedDiagram::Pending) {
                return prepared;
            }
            thread::sleep(Duration::from_millis(10));
        }

        panic!("diagram render did not finish");
    }

    fn mermaid_evaluation_cases() -> [(&'static str, &'static str); 5] {
        [
            (
                "flowchart",
                "flowchart TD\n    Open[Open Markdown] --> Parse[Parse document]\n    Parse --> Render[Render Mermaid SVG]\n    Render --> Cache{Cache hit?}\n    Cache -->|Yes| Show[Show cached SVG]\n    Cache -->|No| Worker[Background worker]\n    Worker --> Rendered[Show rendered SVG]",
            ),
            (
                "sequence",
                "sequenceDiagram\n    participant User\n    participant OxideMD\n    participant Worker\n    User->>OxideMD: Open document\n    OxideMD->>Worker: Queue diagram render\n    Worker-->>OxideMD: SVG result\n    OxideMD-->>User: Repaint diagram",
            ),
            (
                "class",
                "classDiagram\n    class DiagramRenderCache {\n        +prepare()\n        +clear()\n    }\n    class DiagramWorkerResult {\n        +source\n        +result\n    }\n    DiagramRenderCache --> DiagramWorkerResult",
            ),
            (
                "state",
                "stateDiagram-v2\n    [*] --> Pending\n    Pending --> Ready: done\n    Pending --> Failed: fail\n    Ready --> [*]\n    Failed --> [*]",
            ),
            (
                "larger-flowchart",
                "flowchart TD\n    N01[Node 01] --> N02[Node 02]\n    N02 --> N03[Node 03]\n    N03 --> N04[Node 04]\n    N04 --> N05[Node 05]\n    N05 --> N06[Node 06]\n    N06 --> N07[Node 07]\n    N07 --> N08[Node 08]\n    N08 --> N09[Node 09]\n    N09 --> N10[Node 10]\n    N10 --> N11[Node 11]\n    N11 --> N12[Node 12]\n    N12 --> N13[Node 13]\n    N13 --> N14[Node 14]\n    N14 --> N15[Node 15]\n    N15 --> N16[Node 16]\n    N16 --> N17[Node 17]\n    N17 --> N18[Node 18]\n    N18 --> N19[Node 19]\n    N19 --> N20[Node 20]",
            ),
        ]
    }
}
