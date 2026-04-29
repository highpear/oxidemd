use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use eframe::egui::{self};

use crate::embedded_svg::{EmbeddedSvgContent, EmbeddedSvgContentKind};
use crate::metrics;
use crate::svg::{SvgAsset, apply_current_color};

const INLINE_MATH_BASE_FONT_SIZE: f32 = 16.0;
const BLOCK_MATH_BASE_FONT_SIZE: f32 = 24.0;

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
    text_color: [u8; 4],
    zoom_bucket: u16,
}

pub struct MathRenderCache {
    entries: HashMap<(MathCacheKey, String), MathRenderState>,
    result_sender: Sender<MathWorkerResult>,
    result_receiver: Receiver<MathWorkerResult>,
    generation: u64,
}

enum MathRenderState {
    Pending,
    Ready(PreparedMath),
}

struct MathWorkerResult {
    generation: u64,
    key: MathCacheKey,
    expression: String,
    result: PreparedMath,
}

impl MathRenderCache {
    pub fn new() -> Self {
        let (result_sender, result_receiver) = mpsc::channel();

        Self {
            entries: HashMap::new(),
            result_sender,
            result_receiver,
            generation: 0,
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
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

        let key = MathCacheKey {
            mode,
            text_color: text_color.to_array(),
            zoom_bucket: zoom_bucket(zoom_factor),
        };
        let entry_key = (key, expression.to_owned());

        if let Some(state) = self.entries.get(&entry_key) {
            return match state {
                MathRenderState::Pending => PreparedMath::Pending,
                MathRenderState::Ready(result) => result.clone(),
            };
        }

        self.entries.insert(entry_key, MathRenderState::Pending);
        self.spawn_render_job(
            ctx.clone(),
            self.generation,
            key,
            expression.to_owned(),
            mode,
            text_color,
            zoom_factor,
        );

        PreparedMath::Pending
    }

    fn spawn_render_job(
        &self,
        ctx: egui::Context,
        generation: u64,
        key: MathCacheKey,
        expression: String,
        mode: MathRenderMode,
        text_color: egui::Color32,
        zoom_factor: f32,
    ) {
        let sender = self.result_sender.clone();

        thread::spawn(move || {
            let started = Instant::now();
            let result = prepare_math(&expression, mode, text_color, zoom_factor);
            let outcome = match &result {
                PreparedMath::Pending => "pending",
                PreparedMath::Svg(_) => "ok",
                PreparedMath::Error(_) => "error",
            };
            metrics::log_math_render(expression.len(), started.elapsed(), outcome);
            let _ = sender.send(MathWorkerResult {
                generation,
                key,
                expression,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn drain_finished_jobs(&mut self) {
        while let Ok(result) = self.result_receiver.try_recv() {
            if result.generation != self.generation {
                continue;
            }

            self.entries.insert(
                (result.key, result.expression),
                MathRenderState::Ready(result.result),
            );
        }
    }
}

fn prepare_math(
    expression: &str,
    mode: MathRenderMode,
    text_color: egui::Color32,
    zoom_factor: f32,
) -> PreparedMath {
    match mode {
        MathRenderMode::Inline => prepare_svg_math(
            expression,
            text_color,
            zoom_factor,
            INLINE_MATH_BASE_FONT_SIZE,
            mode,
        ),
        MathRenderMode::Block => prepare_svg_math(
            expression,
            text_color,
            zoom_factor,
            BLOCK_MATH_BASE_FONT_SIZE,
            mode,
        ),
    }
}

fn prepare_svg_math(
    expression: &str,
    text_color: egui::Color32,
    zoom_factor: f32,
    base_font_size: f32,
    mode: MathRenderMode,
) -> PreparedMath {
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
        Err(error) => return PreparedMath::Error(error.to_string()),
    };

    let svg = apply_current_color(&svg, text_color);
    let uri = format!(
        "bytes://math-{}-{}-{}-{}-{}-{}.svg",
        color_hash(text_color),
        zoom_bucket(zoom_factor),
        match mode {
            MathRenderMode::Inline => "inline",
            MathRenderMode::Block => "block",
        },
        if use_display_style { "display" } else { "text" },
        font_size_bucket(font_size),
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
            matches!(
                (&first, &second),
                (PreparedMath::Svg(first_svg), PreparedMath::Svg(second_svg))
                    if first_svg.asset().size() == second_svg.asset().size()
                    && first_svg.asset().uri() == second_svg.asset().uri()
                    && first_svg.kind() == super::EmbeddedSvgContentKind::Math
                    && first_svg.source_text() == "x^2"
                    && second_svg.source_text() == "x^2"
            ) || matches!(
                (&first, &second),
                (PreparedMath::Error(first_error), PreparedMath::Error(second_error))
                    if first_error == second_error
            )
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
        let prepared = super::prepare_math(
            r"\frac{1}{x+y}",
            MathRenderMode::Inline,
            Color32::from_rgb(34, 34, 34),
            1.0,
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
        let dark = super::prepare_math(
            "x^2",
            MathRenderMode::Inline,
            Color32::from_rgb(224, 232, 242),
            1.0,
        );
        let mist = super::prepare_math(
            "x^2",
            MathRenderMode::Inline,
            Color32::from_rgb(28, 40, 46),
            1.0,
        );

        if let (PreparedMath::Svg(dark), PreparedMath::Svg(mist)) = (dark, mist) {
            assert_ne!(dark.asset().uri(), mist.asset().uri());
        }
    }
}
