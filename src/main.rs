mod app;
mod bottom_bar;
mod cli;
mod code_block;
mod diagram;
mod document_loader;
mod document_session;
mod document_workspace;
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
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use app::OxideMdApp;
use cli::{parse_args, run_cli_action};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, Vec2};
use theme::{DEFAULT_THEME_ID, apply_theme, theme};

const CJK_FONT_NAME: &str = "cjk_fallback";
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

    let launch_options = match run_cli_action(action) {
        Ok(launch_options) => launch_options,
        Err(code) => return ExitCode::from(code as u8),
    };

    match run_gui(launch_options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Failed to start OxideMD: {}", error);
            ExitCode::from(1)
        }
    }
}

fn run_gui(launch_options: cli::GuiLaunchOptions) -> eframe::Result<()> {
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
                launch_options.initial_file,
                launch_options.restore_file,
                launch_options.reset_session,
            )))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let Some(font_data) = load_cjk_font() else {
        return;
    };

    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        CJK_FONT_NAME.to_owned(),
        FontData::from_owned(font_data).into(),
    );

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, CJK_FONT_NAME.to_owned());
    }

    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.insert(0, CJK_FONT_NAME.to_owned());
    }

    ctx.set_fonts(fonts);
}

fn load_cjk_font() -> Option<Vec<u8>> {
    let windows_candidates = [
        PathBuf::from(r"C:\Windows\Fonts\meiryo.ttc"),
        PathBuf::from(r"C:\Windows\Fonts\meiryo.ttf"),
    ];

    for path in windows_candidates {
        if let Ok(data) = fs::read(path) {
            return Some(data);
        }
    }

    if let Some(path) = find_japanese_system_font(Path::new("/System/Library/Fonts")) {
        if let Ok(data) = fs::read(path) {
            return Some(data);
        }
    }

    let fallback_candidates = [
        PathBuf::from("/System/Library/Fonts/Hiragino Sans GB.ttc"),
        PathBuf::from("/System/Library/Fonts/AppleSDGothicNeo.ttc"),
        PathBuf::from("/System/Library/Fonts/CJKSymbolsFallback.ttc"),
    ];

    for path in fallback_candidates {
        if let Ok(data) = fs::read(path) {
            return Some(data);
        }
    }

    None
}

fn find_japanese_system_font(font_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(font_dir).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                return false;
            };

            file_name.contains("ヒラ") && file_name.contains("角") && file_name.ends_with(".ttc")
        })
        .collect::<Vec<_>>();

    candidates.sort();
    candidates.into_iter().next()
}
