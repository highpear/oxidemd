use std::collections::HashMap;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions, Vec2};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MathRenderMode {
    Inline,
    Block,
}

#[derive(Clone)]
pub enum PreparedMath {
    FallbackText(String),
    Raster { texture: TextureHandle, size: Vec2 },
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
        ctx: &egui::Context,
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
            .or_insert_with(|| prepare_math(ctx, expression, mode, theme_is_dark, zoom_factor))
            .clone()
    }
}

fn prepare_math(
    ctx: &egui::Context,
    expression: &str,
    mode: MathRenderMode,
    theme_is_dark: bool,
    zoom_factor: f32,
) -> PreparedMath {
    match mode {
        MathRenderMode::Inline => PreparedMath::FallbackText(expression.to_owned()),
        MathRenderMode::Block => prepare_block_math(ctx, expression, theme_is_dark, zoom_factor),
    }
}

fn prepare_block_math(
    ctx: &egui::Context,
    expression: &str,
    theme_is_dark: bool,
    zoom_factor: f32,
) -> PreparedMath {
    let font_size = if theme_is_dark {
        18.0 * zoom_factor.max(1.0)
    } else {
        18.0 * zoom_factor
    };

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

    let image = match rasterize_svg(&svg) {
        Ok(image) => image,
        Err(error) => return PreparedMath::Error(error),
    };
    let image = recolor_math_image(image, math_text_rgb(theme_is_dark));

    let size = Vec2::new(image.size[0] as f32, image.size[1] as f32);
    let texture_name = format!(
        "math:{}:{}:{}:{}",
        if theme_is_dark { "dark" } else { "light" },
        zoom_bucket(zoom_factor),
        "block",
        expression
    );
    let texture = ctx.load_texture(texture_name, image, TextureOptions::LINEAR);

    PreparedMath::Raster { texture, size }
}

fn rasterize_svg(svg: &str) -> Result<ColorImage, String> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &options).map_err(|error| error.to_string())?;
    let size = tree.size();
    let width = size.width().ceil().max(1.0) as u32;
    let height = size.height().ceil().max(1.0) as u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).ok_or("Failed to allocate pixmap")?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    Ok(ColorImage::from_rgba_premultiplied(
        [width as usize, height as usize],
        pixmap.data(),
    ))
}

fn recolor_math_image(mut image: ColorImage, target_rgb: [u8; 3]) -> ColorImage {
    for pixel in &mut image.pixels {
        let alpha = pixel.a();
        if alpha == 0 {
            continue;
        }

        let premultiply =
            |channel: u8| -> u8 { ((channel as u16 * alpha as u16 + 127) / 255) as u8 };

        *pixel = egui::Color32::from_rgba_premultiplied(
            premultiply(target_rgb[0]),
            premultiply(target_rgb[1]),
            premultiply(target_rgb[2]),
            alpha,
        );
    }

    image
}

fn math_text_rgb(theme_is_dark: bool) -> [u8; 3] {
    if theme_is_dark {
        [224, 232, 242]
    } else {
        [34, 34, 34]
    }
}

fn zoom_bucket(zoom_factor: f32) -> u16 {
    (zoom_factor * 100.0).round().clamp(0.0, u16::MAX as f32) as u16
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

        assert!(matches!(first, PreparedMath::FallbackText(ref text) if text == "x^2"));
        assert!(matches!(second, PreparedMath::FallbackText(ref text) if text == "x^2"));
    }
}
