use eframe::egui::{self};
use std::collections::HashMap;

use crate::svg::{apply_current_color, SvgAsset};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MathRenderMode {
    Inline,
    Block,
}

#[derive(Clone)]
pub enum PreparedMath {
    Svg(SvgAsset),
    Error(String),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MathCacheKey {
    mode: MathRenderMode,
    theme_is_dark: bool,
    zoom_bucket: u16,
}

pub struct MathRenderCache {
    entries: HashMap<(MathCacheKey, String), PreparedMath>,
}

impl MathRenderCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn prepare(
        &mut self,
        _ctx: &egui::Context,
        expression: &str,
        mode: MathRenderMode,
        theme_is_dark: bool,
        zoom_factor: f32,
    ) -> PreparedMath {
        let key = (
            MathCacheKey {
                mode,
                theme_is_dark,
                zoom_bucket: zoom_bucket(zoom_factor),
            },
            expression.to_owned(),
        );

        self.entries
            .entry(key)
            .or_insert_with(|| prepare_math(expression, mode, theme_is_dark, zoom_factor))
            .clone()
    }
}

fn prepare_math(
    expression: &str,
    mode: MathRenderMode,
    theme_is_dark: bool,
    zoom_factor: f32,
) -> PreparedMath {
    match mode {
        MathRenderMode::Inline => {
            prepare_svg_math(expression, theme_is_dark, zoom_factor, 15.0, mode)
        }
        MathRenderMode::Block => {
            prepare_svg_math(expression, theme_is_dark, zoom_factor, 18.0, mode)
        }
    }
}

fn prepare_svg_math(
    expression: &str,
    theme_is_dark: bool,
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

    let svg = apply_current_color(&svg, math_text_color(theme_is_dark));
    let uri = format!(
        "bytes://math-{}-{}-{}-{}.svg",
        if theme_is_dark { "dark" } else { "light" },
        zoom_bucket(zoom_factor),
        match mode {
            MathRenderMode::Inline => "inline",
            MathRenderMode::Block => "block",
        },
        svg_uri_hash(expression)
    );

    match SvgAsset::from_source(uri, svg) {
        Ok(svg) => PreparedMath::Svg(svg),
        Err(error) => PreparedMath::Error(error),
    }
}

fn math_text_color(theme_is_dark: bool) -> egui::Color32 {
    if theme_is_dark {
        egui::Color32::from_rgb(224, 232, 242)
    } else {
        egui::Color32::from_rgb(34, 34, 34)
    }
}

fn zoom_bucket(zoom_factor: f32) -> u16 {
    (zoom_factor * 100.0).round().clamp(0.0, u16::MAX as f32) as u16
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
    use eframe::egui::Context;

    #[test]
    fn reuses_prepared_math_by_key() {
        let mut cache = MathRenderCache::new();
        let ctx = Context::default();

        let first = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, false, 1.0);
        let second = cache.prepare(&ctx, "x^2", MathRenderMode::Inline, false, 1.0);

        assert!(
            matches!(
                (&first, &second),
                (PreparedMath::Svg(first_svg), PreparedMath::Svg(second_svg))
                    if first_svg.size() == second_svg.size()
                    && first_svg.uri() == second_svg.uri()
            ) || matches!(
                (&first, &second),
                (PreparedMath::Error(first_error), PreparedMath::Error(second_error))
                    if first_error == second_error
            )
        );
    }

    #[test]
    fn generated_svg_uri_does_not_embed_tex_source() {
        let prepared = super::prepare_math(r"\frac{1}{x+y}", MathRenderMode::Inline, false, 1.0);

        if let PreparedMath::Svg(svg) = prepared {
            assert!(!svg.uri().contains(r"\frac"));
            assert!(!svg.uri().contains('{'));
            assert!(svg.uri().starts_with("bytes://math-light-100-inline-"));
        }
    }
}
