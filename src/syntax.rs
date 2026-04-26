use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use eframe::egui::{Color32, FontFamily, FontId, text::LayoutJob, text::TextFormat};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const MAX_HIGHLIGHT_CACHE_ENTRIES: usize = 128;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum HighlightTheme {
    Light,
    Dark,
}

#[derive(Hash, PartialEq, Eq)]
struct HighlightCacheKey {
    language: String,
    code: String,
    theme: HighlightTheme,
    font_size_bits: u32,
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

fn highlight_cache() -> &'static Mutex<HashMap<HighlightCacheKey, LayoutJob>> {
    static HIGHLIGHT_CACHE: OnceLock<Mutex<HashMap<HighlightCacheKey, LayoutJob>>> =
        OnceLock::new();
    HIGHLIGHT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn highlight_code(
    language: Option<&str>,
    code: &str,
    is_dark: bool,
    font_size: f32,
) -> Option<LayoutJob> {
    let language = language?.trim();
    if language.is_empty() {
        return None;
    }

    let theme_kind = highlight_theme(is_dark);
    let cache_key = HighlightCacheKey {
        language: language.to_owned(),
        code: code.to_owned(),
        theme: theme_kind,
        font_size_bits: font_size.to_bits(),
    };

    if let Some(job) = cached_highlight(&cache_key) {
        return Some(job);
    }

    let syntax = find_syntax(language)?;
    let theme = find_theme(theme_kind)?;
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut job = LayoutJob::default();

    for line in LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, syntax_set()).ok()?;

        for (style, text) in ranges {
            job.append(text, 0.0, text_format(style, font_size));
        }
    }

    store_highlight(cache_key, job.clone());
    Some(job)
}

fn find_syntax(language: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let syntax_set = syntax_set();

    syntax_set
        .find_syntax_by_token(language)
        .or_else(|| syntax_set.find_syntax_by_extension(language))
        .or_else(|| syntax_set.find_syntax_by_name(language))
}

fn highlight_theme(is_dark: bool) -> HighlightTheme {
    if is_dark {
        HighlightTheme::Dark
    } else {
        HighlightTheme::Light
    }
}

fn find_theme(theme_kind: HighlightTheme) -> Option<&'static Theme> {
    let name = match theme_kind {
        HighlightTheme::Dark => "base16-ocean.dark",
        HighlightTheme::Light => "InspiredGitHub",
    };

    theme_set().themes.get(name)
}

fn cached_highlight(cache_key: &HighlightCacheKey) -> Option<LayoutJob> {
    let cache = highlight_cache().lock().ok()?;
    cache.get(cache_key).cloned()
}

fn store_highlight(cache_key: HighlightCacheKey, job: LayoutJob) {
    let Ok(mut cache) = highlight_cache().lock() else {
        return;
    };

    if cache.len() >= MAX_HIGHLIGHT_CACHE_ENTRIES {
        cache.clear();
    }

    cache.insert(cache_key, job);
}

fn text_format(style: Style, font_size: f32) -> TextFormat {
    TextFormat {
        font_id: FontId::new(font_size, FontFamily::Monospace),
        color: color32(style.foreground),
        background: Color32::TRANSPARENT,
        ..Default::default()
    }
}

fn color32(color: syntect::highlighting::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}
