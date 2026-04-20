use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

pub struct ImageCache {
    entries: HashMap<PathBuf, ImageCacheEntry>,
}

enum ImageCacheEntry {
    Loaded(TextureHandle),
    Failed(String),
}

pub enum ImageLoadState<'a> {
    Loaded(&'a TextureHandle),
    Failed(&'a str),
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn load(&mut self, ctx: &egui::Context, path: &Path) -> ImageLoadState<'_> {
        let key = normalize_path(path);

        if !self.entries.contains_key(&key) {
            let max_texture_side = ctx.input(|input| input.max_texture_side);
            let entry = match load_color_image(&key, max_texture_side) {
                Ok(image) => {
                    let texture_name = format!("markdown-image:{}", key.display());
                    let texture = ctx.load_texture(texture_name, image, TextureOptions::LINEAR);
                    ImageCacheEntry::Loaded(texture)
                }
                Err(error) => ImageCacheEntry::Failed(error),
            };

            self.entries.insert(key.clone(), entry);
        }

        match self.entries.get(&key).expect("image cache entry exists") {
            ImageCacheEntry::Loaded(texture) => ImageLoadState::Loaded(texture),
            ImageCacheEntry::Failed(error) => ImageLoadState::Failed(error),
        }
    }
}

fn load_color_image(path: &Path, max_texture_side: usize) -> Result<ColorImage, String> {
    let mut image = image::open(path).map_err(|error| error.to_string())?;
    let max_side = max_texture_side as u32;

    if image.width() > max_side || image.height() > max_side {
        image = image.resize(max_side, max_side, image::imageops::FilterType::Triangle);
    }

    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    Ok(ColorImage::from_rgba_unmultiplied(size, &pixels))
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
