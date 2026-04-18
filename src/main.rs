mod app;
mod i18n;
mod metrics;
mod parser;
mod reload_worker;
mod renderer;
mod theme;
mod watcher;

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use app::OxideMdApp;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use theme::{DEFAULT_THEME_ID, apply_theme, theme};

const MEIRYO_FONT_NAME: &str = "meiryo";

fn main() -> eframe::Result<()> {
    let startup_started = Instant::now();
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "OxideMD",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, &theme(DEFAULT_THEME_ID));
            Ok(Box::new(OxideMdApp::new(
                cc.egui_ctx.clone(),
                startup_started,
            )))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let Some(font_data) = load_meiryo_font() else {
        return;
    };

    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        MEIRYO_FONT_NAME.to_owned(),
        FontData::from_owned(font_data).into(),
    );

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, MEIRYO_FONT_NAME.to_owned());
    }

    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.insert(0, MEIRYO_FONT_NAME.to_owned());
    }

    ctx.set_fonts(fonts);
}

fn load_meiryo_font() -> Option<Vec<u8>> {
    let candidates = [
        PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
        PathBuf::from(r"C:\Windows\Fonts\meiryo.ttf"),
    ];

    for path in candidates {
        if let Ok(data) = fs::read(path) {
            return Some(data);
        }
    }

    None
}
