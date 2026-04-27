mod app;
mod bottom_bar;
mod cli;
mod code_block;
mod diagram;
mod document_loader;
mod embedded_svg;
mod export;
mod external_links;
mod i18n;
mod image_cache;
mod math;
mod metrics;
mod parser;
mod reload_worker;
mod renderer;
mod search;
mod search_panel;
mod session;
mod shortcuts;
mod svg;
mod syntax;
mod theme;
mod top_bar;
mod watcher;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use app::OxideMdApp;
use cli::{parse_args, run_cli_action};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, Vec2};
use theme::{DEFAULT_THEME_ID, apply_theme, theme};

const MEIRYO_FONT_NAME: &str = "meiryo";
const INITIAL_WINDOW_WIDTH: f32 = 1180.0;
const INITIAL_WINDOW_HEIGHT: f32 = 760.0;

fn main() -> ExitCode {
    let action = match parse_args(env::args_os().skip(1)) {
        Ok(action) => action,
        Err(error) => {
            eprintln!("{}", error);
            return ExitCode::from(1);
        }
    };

    let initial_file = match run_cli_action(action) {
        Ok(initial_file) => initial_file,
        Err(code) => return ExitCode::from(code as u8),
    };

    match run_gui(initial_file) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Failed to start OxideMD: {}", error);
            ExitCode::from(1)
        }
    }
}

fn run_gui(initial_file: Option<PathBuf>) -> eframe::Result<()> {
    let startup_started = Instant::now();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("oxidemd")
            .with_inner_size(Vec2::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT)),
        ..Default::default()
    };

    eframe::run_native(
        "OxideMD",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            configure_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, &theme(DEFAULT_THEME_ID));
            Ok(Box::new(OxideMdApp::new(
                cc.egui_ctx.clone(),
                cc.storage,
                startup_started,
                initial_file,
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
