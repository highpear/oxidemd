use eframe::egui::{self, Color32, CornerRadius, Stroke, Style, Visuals};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    WarmPaper,
    Mist,
}

impl ThemeId {
    pub fn next(self) -> Self {
        let themes = available_themes();
        let index = themes.iter().position(|theme| *theme == self).unwrap_or(0);
        themes[(index + 1) % themes.len()]
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub app_background: Color32,
    pub top_bar_background: Color32,
    pub content_background: Color32,
    pub content_border: Color32,
    pub content_shadow: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub link: Color32,
    pub quote_background: Color32,
    pub quote_border: Color32,
    pub code_background: Color32,
    pub status_idle_background: Color32,
    pub status_idle_text: Color32,
    pub status_loading_background: Color32,
    pub status_loading_text: Color32,
    pub status_error_background: Color32,
    pub status_error_text: Color32,
    pub widget_inactive_background: Color32,
    pub widget_hovered_background: Color32,
    pub widget_active_background: Color32,
}

pub const DEFAULT_THEME_ID: ThemeId = ThemeId::WarmPaper;

pub fn available_themes() -> &'static [ThemeId] {
    &[ThemeId::WarmPaper, ThemeId::Mist]
}

pub fn theme(theme_id: ThemeId) -> Theme {
    match theme_id {
        ThemeId::WarmPaper => warm_paper_theme(),
        ThemeId::Mist => mist_theme(),
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let mut style: Style = (*ctx.style()).clone();

    style.visuals = Visuals::light();
    style.visuals.panel_fill = theme.app_background;
    style.visuals.window_fill = theme.content_background;
    style.visuals.extreme_bg_color = theme.content_background;
    style.visuals.faint_bg_color = theme.top_bar_background;
    style.visuals.override_text_color = Some(theme.text_primary);
    style.visuals.widgets.noninteractive.bg_fill = theme.top_bar_background;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.inactive.bg_fill = theme.widget_inactive_background;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.hovered.bg_fill = theme.widget_hovered_background;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.active.bg_fill = theme.widget_active_background;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.open.bg_fill = theme.top_bar_background;
    style.visuals.hyperlink_color = theme.link;
    style.visuals.window_corner_radius = CornerRadius::same(10);
    style.visuals.menu_corner_radius = CornerRadius::same(10);

    ctx.set_style(style);
}

fn warm_paper_theme() -> Theme {
    Theme {
        app_background: Color32::from_rgb(241, 238, 232),
        top_bar_background: Color32::from_rgb(236, 232, 224),
        content_background: Color32::from_rgb(255, 255, 255),
        content_border: Color32::from_rgb(215, 208, 198),
        content_shadow: Color32::from_rgba_unmultiplied(60, 50, 35, 16),
        text_primary: Color32::from_rgb(34, 34, 34),
        text_secondary: Color32::from_rgb(98, 94, 87),
        link: Color32::from_rgb(0, 92, 197),
        quote_background: Color32::from_rgb(247, 245, 240),
        quote_border: Color32::from_rgb(214, 208, 198),
        code_background: Color32::from_rgb(244, 244, 244),
        status_idle_background: Color32::from_rgb(231, 239, 229),
        status_idle_text: Color32::from_rgb(58, 95, 60),
        status_loading_background: Color32::from_rgb(243, 236, 214),
        status_loading_text: Color32::from_rgb(122, 92, 25),
        status_error_background: Color32::from_rgb(245, 224, 224),
        status_error_text: Color32::from_rgb(150, 47, 47),
        widget_inactive_background: Color32::from_rgb(250, 248, 244),
        widget_hovered_background: Color32::from_rgb(245, 242, 235),
        widget_active_background: Color32::from_rgb(238, 234, 226),
    }
}

fn mist_theme() -> Theme {
    Theme {
        app_background: Color32::from_rgb(234, 240, 242),
        top_bar_background: Color32::from_rgb(226, 234, 237),
        content_background: Color32::from_rgb(252, 254, 255),
        content_border: Color32::from_rgb(198, 209, 214),
        content_shadow: Color32::from_rgba_unmultiplied(34, 57, 67, 18),
        text_primary: Color32::from_rgb(28, 40, 46),
        text_secondary: Color32::from_rgb(87, 103, 110),
        link: Color32::from_rgb(0, 98, 147),
        quote_background: Color32::from_rgb(241, 247, 249),
        quote_border: Color32::from_rgb(194, 209, 214),
        code_background: Color32::from_rgb(239, 244, 247),
        status_idle_background: Color32::from_rgb(223, 238, 232),
        status_idle_text: Color32::from_rgb(45, 97, 77),
        status_loading_background: Color32::from_rgb(242, 236, 214),
        status_loading_text: Color32::from_rgb(122, 92, 25),
        status_error_background: Color32::from_rgb(247, 228, 228),
        status_error_text: Color32::from_rgb(148, 53, 53),
        widget_inactive_background: Color32::from_rgb(244, 248, 250),
        widget_hovered_background: Color32::from_rgb(236, 243, 246),
        widget_active_background: Color32::from_rgb(228, 237, 241),
    }
}
