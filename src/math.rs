use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use eframe::egui::{self};

use crate::embedded_svg::{EmbeddedSvgContent, EmbeddedSvgContentKind};
use crate::metrics;
use crate::svg::{SvgAsset, apply_current_color};

const INLINE_MATH_BASE_FONT_SIZE: f32 = 16.0;
const BLOCK_MATH_BASE_FONT_SIZE: f32 = 24.0;
const MAX_ACTIVE_MATH_RENDER_JOBS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MathRenderMode {
    Inline,
    Block,
}

#[derive(Clone)]
pub enum PreparedMath {
    Pending,
    Svg(EmbeddedSvgContent),
    Error(String),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MathCacheKey {
    mode: MathRenderMode,
    zoom_bucket: u16,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ColoredMathCacheKey {
    base: MathCacheKey,
    text_color: [u8; 4],
}

pub struct MathRenderCache {
    entries: HashMap<(MathCacheKey, String), MathRenderState>,
    colored_entries: HashMap<(ColoredMathCacheKey, String), PreparedMath>,
    queued_jobs: VecDeque<MathRenderJob>,
    active_job_count: usize,
    result_sender: Sender<MathWorkerResult>,
    result_receiver: Receiver<MathWorkerResult>,
    generation: u64,
}

enum MathRenderState {
    Pending,
    Ready(RawMathResult),
}

struct MathWorkerResult {
    generation: u64,
    key: MathCacheKey,
    expression: String,
    result: RawMathResult,
}

struct MathRenderJob {
    generation: u64,
    key: MathCacheKey,
    expression: String,
    mode: MathRenderMode,
    zoom_factor: f32,
}

#[derive(Clone)]
enum RawMathResult {
    Svg(RawMathSvg),
    Error(String),
}

#[derive(Clone)]
struct RawMathSvg {
    source: String,
    display_style: MathDisplayStyle,
    font_size_bucket: u16,
}

#[derive(Clone, Copy)]
enum MathDisplayStyle {
    Text,
    Display,
}

impl MathRenderCache {
    pub fn new() -> Self {
        let (result_sender, result_receiver) = mpsc::channel();

        Self {
            entries: HashMap::new(),
            colored_entries: HashMap::new(),
            queued_jobs: VecDeque::new(),
            active_job_count: 0,
            result_sender,
            result_receiver,
            generation: 0,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.colored_entries.clear();
        self.queued_jobs.clear();
        self.active_job_count = 0;
        self.generation = self.generation.wrapping_add(1);
        self.drain_finished_jobs();
    }

    pub fn prepare(
        &mut self,
        ctx: &egui::Context,
        expression: &str,
        mode: MathRenderMode,
        text_color: egui::Color32,
        zoom_factor: f32,
    ) -> PreparedMath {
        self.drain_finished_jobs();
        self.start_queued_jobs(ctx.clone());

        let key = MathCacheKey {
            mode,
            zoom_bucket: zoom_bucket(zoom_factor),
        };
        let entry_key = (key, expression.to_owned());
        let colored_key = (
            ColoredMathCacheKey {
                base: key,
                text_color: text_color.to_array(),
            },
            expression.to_owned(),
        );

        if let Some(result) = self.colored_entries.get(&colored_key) {
            return result.clone();
        }

        if let Some(state) = self.entries.get(&entry_key) {
            return match state {
                MathRenderState::Pending => PreparedMath::Pending,
                MathRenderState::Ready(result) => {
                    let prepared = prepare_colored_math(expression, key, result, text_color);
                    self.colored_entries.insert(colored_key, prepared.clone());
                    prepared
                }
            };
        }

        self.entries.insert(entry_key, MathRenderState::Pending);
        self.queued_jobs.push_back(MathRenderJob {
            generation: self.generation,
            key,
            expression: expression.to_owned(),
            mode,
            zoom_factor,
        });
        self.start_queued_jobs(ctx.clone());

        PreparedMath::Pending
    }

    fn start_queued_jobs(&mut self, ctx: egui::Context) {
        while self.active_job_count < MAX_ACTIVE_MATH_RENDER_JOBS {
            let Some(job) = self.queued_jobs.pop_front() else {
                break;
            };

            self.active_job_count += 1;
            self.spawn_render_job(ctx.clone(), job);
        }
    }

    fn spawn_render_job(&self, ctx: egui::Context, job: MathRenderJob) {
        let sender = self.result_sender.clone();

        thread::spawn(move || {
            let started = Instant::now();
            let result = prepare_math(&job.expression, job.mode, job.zoom_factor);
            let outcome = match &result {
                RawMathResult::Svg(_) => "ok",
                RawMathResult::Error(_) => "error",
            };
            metrics::log_math_render(job.expression.len(), started.elapsed(), outcome);
            let _ = sender.send(MathWorkerResult {
                generation: job.generation,
                key: job.key,
                expression: job.expression,
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
                (result.key, result.expression),
                MathRenderState::Ready(result.result),
            );
        }

        self.active_job_count = self.active_job_count.saturating_sub(finished_current_jobs);
    }
}

fn prepare_math(expression: &str, mode: MathRenderMode, zoom_factor: f32) -> RawMathResult {
    match mode {
        MathRenderMode::Inline => {
            prepare_svg_math(expression, zoom_factor, INLINE_MATH_BASE_FONT_SIZE, mode)
        }
        MathRenderMode::Block => {
            prepare_svg_math(expression, zoom_factor, BLOCK_MATH_BASE_FONT_SIZE, mode)
        }
    }
}

fn prepare_svg_math(
    expression: &str,
    zoom_factor: f32,
    base_font_size: f32,
    mode: MathRenderMode,
) -> RawMathResult {
    let font_size = base_font_size * zoom_factor;
    let use_display_style = mode == MathRenderMode::Inline && is_fraction_inline_math(expression);
    let render_expression = if use_display_style {
        format!("\\displaystyle {expression}")
    } else {
        expression.to_owned()
    };

    let svg = match mathjax_svg_rs::render_tex(
        &render_expression,
        &mathjax_svg_rs::Options {
            font_size: font_size.into(),
            ..Default::default()
        },
    ) {
        Ok(svg) => svg,
        Err(error) => return RawMathResult::Error(error.to_string()),
    };

    RawMathResult::Svg(RawMathSvg {
        source: svg,
        display_style: if use_display_style {
            MathDisplayStyle::Display
        } else {
            MathDisplayStyle::Text
        },
        font_size_bucket: font_size_bucket(font_size),
    })
}

fn prepare_colored_math(
    expression: &str,
    key: MathCacheKey,
    result: &RawMathResult,
    text_color: egui::Color32,
) -> PreparedMath {
    let raw = match result {
        RawMathResult::Svg(raw) => raw,
        RawMathResult::Error(error) => return PreparedMath::Error(error.clone()),
    };

    let svg = apply_current_color(&raw.source, text_color);
    let uri = format!(
        "bytes://math-{}-{}-{}-{}-{}-{}.svg",
        color_hash(text_color),
        key.zoom_bucket,
        match key.mode {
            MathRenderMode::Inline => "inline",
            MathRenderMode::Block => "block",
        },
        match raw.display_style {
            MathDisplayStyle::Text => "text",
            MathDisplayStyle::Display => "display",
        },
        raw.font_size_bucket,
        svg_uri_hash(expression)
    );

    match SvgAsset::from_source(uri, svg) {
        Ok(svg) => PreparedMath::Svg(EmbeddedSvgContent::new(
            EmbeddedSvgContentKind::Math,
            svg,
            expression.to_owned(),
        )),
        Err(error) => PreparedMath::Error(error),
    }
}

fn zoom_bucket(zoom_factor: f32) -> u16 {
    (zoom_factor * 100.0).round().clamp(0.0, u16::MAX as f32) as u16
}

fn font_size_bucket(font_size: f32) -> u16 {
    (font_size * 10.0).round().clamp(0.0, u16::MAX as f32) as u16
}

fn is_fraction_inline_math(expression: &str) -> bool {
    expression.contains("\\frac")
        || expression.contains("\\dfrac")
        || expression.contains("\\tfrac")
        || expression.contains("\\genfrac")
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

fn svg_uri_hash(expression: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;

    for byte in expression.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::{MathRenderCache, MathRenderMode, PreparedMath};
    use eframe::egui::{Color32, Context};
    use std::time::{Duration, Instant};

    #[test]
    fn reuses_prepared_math_by_key() {
        let mut cache = MathRenderCache::new();
        let ctx = Context::default();

        let text_color = Color32::from_rgb(34, 34, 34);
        assert!(matches!(
            cache.prepare(&ctx, "x^2", MathRenderMode::Inline, text_color, 1.0),
            PreparedMath::Pending
        ));
        let first = wait_for_prepared_math(
            &mut cache,
            &ctx,
            "x^2",
            MathRenderMode::Inline,
            text_color,
            1.0,
        );
        let second = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, text_color, 1.0);

        assert!(
            matches!((&first, &second), (PreparedMath::Svg(first_svg), PreparedMath::Svg(second_svg))
                if first_svg.asset().size() == second_svg.asset().size()
                    && first_svg.asset().uri() == second_svg.asset().uri()
                    && first_svg.kind() == super::EmbeddedSvgContentKind::Math
                    && first_svg.source_text() == "x^2"
                    && second_svg.source_text() == "x^2")
                || matches!((&first, &second), (PreparedMath::Error(first_error), PreparedMath::Error(second_error))
                    if first_error == second_error)
        );
    }

    fn wait_for_prepared_math(
        cache: &mut MathRenderCache,
        ctx: &Context,
        expression: &str,
        mode: MathRenderMode,
        text_color: Color32,
        zoom_factor: f32,
    ) -> PreparedMath {
        let started = Instant::now();

        loop {
            match cache.prepare(ctx, expression, mode, text_color, zoom_factor) {
                PreparedMath::Pending if started.elapsed() < Duration::from_secs(5) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                result => return result,
            }
        }
    }

    #[test]
    fn generated_svg_uri_does_not_embed_tex_source() {
        let raw = super::prepare_math(r"\frac{1}{x+y}", MathRenderMode::Inline, 1.0);
        let prepared = super::prepare_colored_math(
            r"\frac{1}{x+y}",
            super::MathCacheKey {
                mode: MathRenderMode::Inline,
                zoom_bucket: super::zoom_bucket(1.0),
            },
            &raw,
            Color32::from_rgb(34, 34, 34),
        );

        if let PreparedMath::Svg(svg) = prepared {
            assert!(!svg.asset().uri().contains(r"\frac"));
            assert!(!svg.asset().uri().contains('{'));
            assert!(
                svg.asset()
                    .uri()
                    .starts_with("bytes://math-222222ff-100-inline-display-160-")
            );
            assert_eq!(svg.source_text(), r"\frac{1}{x+y}");
        }
    }

    #[test]
    fn color_is_part_of_prepared_math_uri() {
        let raw = super::prepare_math("x^2", MathRenderMode::Inline, 1.0);
        let key = super::MathCacheKey {
            mode: MathRenderMode::Inline,
            zoom_bucket: super::zoom_bucket(1.0),
        };
        let dark = super::prepare_colored_math("x^2", key, &raw, Color32::from_rgb(224, 232, 242));
        let mist = super::prepare_colored_math("x^2", key, &raw, Color32::from_rgb(28, 40, 46));

        if let (PreparedMath::Svg(dark), PreparedMath::Svg(mist)) = (dark, mist) {
            assert_ne!(dark.asset().uri(), mist.asset().uri());
        }
    }

    #[test]
    fn reuses_raw_math_when_color_changes() {
        let mut cache = MathRenderCache::new();
        let ctx = Context::default();

        let first_color = Color32::from_rgb(34, 34, 34);
        let second_color = Color32::from_rgb(224, 232, 242);
        assert!(matches!(
            cache.prepare(&ctx, "x^2", MathRenderMode::Inline, first_color, 1.0),
            PreparedMath::Pending
        ));
        let first = wait_for_prepared_math(
            &mut cache,
            &ctx,
            "x^2",
            MathRenderMode::Inline,
            first_color,
            1.0,
        );
        let second = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, second_color, 1.0);

        assert!(matches!(
            (&first, &second),
            (PreparedMath::Svg(first_svg), PreparedMath::Svg(second_svg))
                if first_svg.asset().size() == second_svg.asset().size()
                && first_svg.asset().uri() != second_svg.asset().uri()
        ));
    }

    #[test]
    fn limits_active_math_render_jobs() {
        let mut cache = MathRenderCache::new();
        let ctx = Context::default();
        let text_color = Color32::from_rgb(34, 34, 34);

        cache.active_job_count = super::MAX_ACTIVE_MATH_RENDER_JOBS;
        let prepared = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, text_color, 1.0);

        assert!(matches!(prepared, PreparedMath::Pending));
        assert_eq!(cache.active_job_count, super::MAX_ACTIVE_MATH_RENDER_JOBS);
        assert_eq!(cache.queued_jobs.len(), 1);
    }
}
