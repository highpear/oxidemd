use eframe::egui::{self};

use crate::embedded_svg::{EmbeddedSvgContent, EmbeddedSvgRenderCache, EmbeddedSvgRenderResult};
use crate::svg::{apply_current_color, SvgAsset};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MathRenderMode {
    Inline,
    Block,
}

pub type PreparedMath = EmbeddedSvgRenderResult;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MathCacheKey {
    mode: MathRenderMode,
    text_color: [u8; 4],
    zoom_bucket: u16,
}

pub struct MathRenderCache {
    cache: EmbeddedSvgRenderCache<MathCacheKey>,
}

impl MathRenderCache {
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
        _ctx: &egui::Context,
        expression: &str,
        mode: MathRenderMode,
        text_color: egui::Color32,
        zoom_factor: f32,
    ) -> PreparedMath {
        let key = MathCacheKey {
            mode,
            text_color: text_color.to_array(),
            zoom_bucket: zoom_bucket(zoom_factor),
        };

        self.cache.prepare_with(key, expression, |expression| {
            prepare_math(expression, mode, text_color, zoom_factor)
        })
    }
}

fn prepare_math(
    expression: &str,
    mode: MathRenderMode,
    text_color: egui::Color32,
    zoom_factor: f32,
) -> PreparedMath {
    match mode {
        MathRenderMode::Inline => prepare_svg_math(expression, text_color, zoom_factor, 15.0, mode),
        MathRenderMode::Block => prepare_svg_math(expression, text_color, zoom_factor, 18.0, mode),
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

    let svg = match mathjax_svg_rs::render_tex(
        expression,
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
        "bytes://math-{}-{}-{}-{}.svg",
        color_hash(text_color),
        zoom_bucket(zoom_factor),
        match mode {
            MathRenderMode::Inline => "inline",
            MathRenderMode::Block => "block",
        },
        svg_uri_hash(expression)
    );

    match SvgAsset::from_source(uri, svg) {
        Ok(svg) => PreparedMath::Svg(EmbeddedSvgContent::new(svg, expression.to_owned())),
        Err(error) => PreparedMath::Error(error),
    }
}

fn zoom_bucket(zoom_factor: f32) -> u16 {
    (zoom_factor * 100.0).round().clamp(0.0, u16::MAX as f32) as u16
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

    #[test]
    fn reuses_prepared_math_by_key() {
        let mut cache = MathRenderCache::new();
        let ctx = Context::default();

        let text_color = Color32::from_rgb(34, 34, 34);
        let first = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, text_color, 1.0);
        let second = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, text_color, 1.0);

        assert!(
            matches!(
                (&first, &second),
                (PreparedMath::Svg(first_svg), PreparedMath::Svg(second_svg))
                    if first_svg.asset().size() == second_svg.asset().size()
                    && first_svg.asset().uri() == second_svg.asset().uri()
                    && first_svg.source_text() == "x^2"
                    && second_svg.source_text() == "x^2"
            ) || matches!(
                (&first, &second),
                (PreparedMath::Error(first_error), PreparedMath::Error(second_error))
                    if first_error == second_error
            )
        );
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
            assert!(svg
                .asset()
                .uri()
                .starts_with("bytes://math-222222ff-100-inline-"));
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
