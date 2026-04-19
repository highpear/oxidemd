use std::sync::OnceLock;

use eframe::egui::{Color32, FontFamily, FontId, text::LayoutJob, text::TextFormat};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

pub fn highlight_code(
    language: Option<&str>,
    code: &str,
    is_dark: bool,
    font_size: f32,
) -> Option<LayoutJob> {
    let language = language?;
    let syntax = find_syntax(language)?;
    let theme = find_theme(is_dark)?;
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut job = LayoutJob::default();

    for line in LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, syntax_set()).ok()?;

        for (style, text) in ranges {
            job.append(text, 0.0, text_format(style, font_size));
        }
    }

    Some(job)
}

fn find_syntax(language: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let syntax_set = syntax_set();
    let language = language.trim();

    syntax_set
        .find_syntax_by_token(language)
        .or_else(|| syntax_set.find_syntax_by_extension(language))
        .or_else(|| syntax_set.find_syntax_by_name(language))
}

fn find_theme(is_dark: bool) -> Option<&'static Theme> {
    let name = if is_dark {
        "base16-ocean.dark"
    } else {
        "InspiredGitHub"
    };

    theme_set().themes.get(name)
}

fn text_format(style: Style, font_size: f32) -> TextFormat {
    TextFormat {
        font_id: FontId::new(font_size, FontFamily::Monospace),
        color: color32(style.foreground),
        background: color32(style.background),
        ..Default::default()
    }
}

fn color32(color: syntect::highlighting::Color) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
}
