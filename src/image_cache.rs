use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use crate::metrics;

const MAX_ACTIVE_IMAGE_LOAD_JOBS: usize = 2;

enum ImageCacheEntry {
    Pending,
    Loaded(TextureHandle),
    Failed(String),
}

pub enum ImageLoadState<'a> {
    Pending,
    Loaded(&'a TextureHandle),
    Failed(&'a str),
}

struct ImageLoadJob {
    generation: u64,
    key: PathBuf,
    max_texture_side: usize,
}

struct ImageWorkerResult {
    generation: u64,
    key: PathBuf,
    result: Result<ColorImage, String>,
}

pub struct ImageCache {
    entries: HashMap<PathBuf, ImageCacheEntry>,
    queued_jobs: VecDeque<ImageLoadJob>,
    active_job_count: usize,
    result_sender: Sender<ImageWorkerResult>,
    result_receiver: Receiver<ImageWorkerResult>,
    generation: u64,
}

impl ImageCache {
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
        while self.result_receiver.try_recv().is_ok() {}
    }

    pub fn prepare(&mut self, ctx: &egui::Context, path: &Path) -> ImageLoadState<'_> {
        self.drain_finished_jobs(ctx);
        self.start_queued_jobs(ctx);

        let key = normalize_path(path);

        if !self.entries.contains_key(&key) {
            let max_texture_side = ctx.input(|input| input.max_texture_side);
            self.entries.insert(key.clone(), ImageCacheEntry::Pending);
            self.queued_jobs.push_back(ImageLoadJob {
                generation: self.generation,
                key: key.clone(),
                max_texture_side,
            });
            self.start_queued_jobs(ctx);
        }

        match self.entries.get(&key).expect("image cache entry exists") {
            ImageCacheEntry::Pending => ImageLoadState::Pending,
            ImageCacheEntry::Loaded(texture) => ImageLoadState::Loaded(texture),
            ImageCacheEntry::Failed(error) => ImageLoadState::Failed(error),
        }
    }

    fn start_queued_jobs(&mut self, ctx: &egui::Context) {
        while self.active_job_count < MAX_ACTIVE_IMAGE_LOAD_JOBS {
            let Some(job) = self.queued_jobs.pop_front() else {
                break;
            };

            self.active_job_count += 1;
            self.spawn_load_job(ctx.clone(), job);
        }
    }

    fn spawn_load_job(&self, ctx: egui::Context, job: ImageLoadJob) {
        let sender = self.result_sender.clone();

        thread::spawn(move || {
            let started = Instant::now();
            let result = load_color_image(&job.key, job.max_texture_side);
            let outcome = if result.is_ok() { "ok" } else { "error" };
            metrics::log_image_load(&job.key, started.elapsed(), outcome);
            let _ = sender.send(ImageWorkerResult {
                generation: job.generation,
                key: job.key,
                result,
            });
            ctx.request_repaint();
        });
    }

    fn drain_finished_jobs(&mut self, ctx: &egui::Context) {
        let mut finished_current_jobs = 0usize;

        while let Ok(result) = self.result_receiver.try_recv() {
            if result.generation != self.generation {
                continue;
            }

            finished_current_jobs += 1;
            let entry = match result.result {
                Ok(image) => {
                    let texture_name = format!("markdown-image:{}", result.key.display());
                    let texture = ctx.load_texture(texture_name, image, TextureOptions::LINEAR);
                    ImageCacheEntry::Loaded(texture)
                }
                Err(error) => ImageCacheEntry::Failed(error),
            };

            self.entries.insert(result.key, entry);
        }

        self.active_job_count = self.active_job_count.saturating_sub(finished_current_jobs);
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{ImageCache, ImageLoadState};
    use eframe::egui::Context;

    #[test]
    fn loads_valid_image_after_pending() {
        let dir =
            std::env::temp_dir().join(format!("oxidemd-image-cache-valid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("pixel.png");
        write_test_png(&path);

        let mut cache = ImageCache::new();
        let ctx = Context::default();

        assert!(matches!(
            cache.prepare(&ctx, &path),
            ImageLoadState::Pending
        ));

        let started = Instant::now();
        loop {
            match cache.prepare(&ctx, &path) {
                ImageLoadState::Loaded(_) => break,
                ImageLoadState::Failed(error) => panic!("unexpected image failure: {error}"),
                ImageLoadState::Pending => {}
            }
            if started.elapsed() > Duration::from_secs(5) {
                panic!("image load did not finish");
            }
            thread::sleep(Duration::from_millis(10));
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn keeps_missing_image_failed() {
        let mut cache = ImageCache::new();
        let ctx = Context::default();
        let path = std::env::temp_dir().join(format!(
            "oxidemd-image-cache-missing-{}.png",
            std::process::id()
        ));
        std::fs::remove_file(&path).ok();

        assert!(matches!(
            cache.prepare(&ctx, &path),
            ImageLoadState::Pending
        ));

        let started = Instant::now();
        loop {
            match cache.prepare(&ctx, &path) {
                ImageLoadState::Failed(_) => break,
                ImageLoadState::Loaded(_) => panic!("missing image should not load"),
                ImageLoadState::Pending => {}
            }
            if started.elapsed() > Duration::from_secs(5) {
                panic!("missing image load did not finish");
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(matches!(
            cache.prepare(&ctx, &path),
            ImageLoadState::Failed(_)
        ));
        assert!(cache.queued_jobs.is_empty());
    }

    fn write_test_png(path: &Path) {
        let mut image = image::RgbaImage::new(2, 2);
        for pixel in image.pixels_mut() {
            *pixel = image::Rgba([120, 180, 240, 255]);
        }
        image.save(path).expect("write test png");
    }
}
