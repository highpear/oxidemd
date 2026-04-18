use eframe::egui::{self, Color32, CornerRadius, Stroke, Style, Visuals};

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
}

pub fn default_theme() -> Theme {
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
    }
}

pub fn apply_theme(ctx: &egui::Context) {
    let theme = default_theme();
    let mut style: Style = (*ctx.style()).clone();

    style.visuals = Visuals::light();
    style.visuals.panel_fill = theme.app_background;
    style.visuals.window_fill = theme.content_background;
    style.visuals.extreme_bg_color = theme.content_background;
    style.visuals.faint_bg_color = theme.top_bar_background;
    style.visuals.override_text_color = Some(theme.text_primary);
    style.visuals.widgets.noninteractive.bg_fill = theme.top_bar_background;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(250, 248, 244);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(245, 242, 235);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(238, 234, 226);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, theme.content_border);
    style.visuals.widgets.open.bg_fill = theme.top_bar_background;
    style.visuals.hyperlink_color = theme.link;
    style.visuals.window_corner_radius = CornerRadius::same(10);
    style.visuals.menu_corner_radius = CornerRadius::same(10);

    ctx.set_style(style);
}
