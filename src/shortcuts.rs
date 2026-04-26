use eframe::egui::{self, Align, Align2, Key, KeyboardShortcut, Layout, Modifiers, Vec2};

use crate::i18n::{tr, Language, TranslationKey};

pub struct ShortcutActions {
    pub open_file: bool,
    pub focus_search: bool,
    pub show_shortcuts_help: bool,
    pub reload_file: bool,
    pub next_search: bool,
    pub previous_search: bool,
    pub switch_language: bool,
    pub switch_theme: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub reset_zoom: bool,
}

pub fn consume_shortcuts(ctx: &egui::Context, has_search_matches: bool) -> ShortcutActions {
    let open_file = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))
    });
    let focus_search = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::F))
    });
    let show_shortcuts_help = ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F1));
    let reload_file = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::R))
            || input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F5))
    });
    let next_search = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F3))
    }) || (has_search_matches
        && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)));
    let previous_search = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::F3))
    });
    let switch_language = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::L))
    });
    let switch_theme = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::T))
    });
    let zoom_in = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Plus))
            || input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Equals))
    });
    let zoom_out = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus))
    });
    let reset_zoom = ctx.input_mut(|input| {
        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0))
    });

    ShortcutActions {
        open_file,
        focus_search,
        show_shortcuts_help,
        reload_file,
        next_search,
        previous_search,
        switch_language,
        switch_theme,
        zoom_in,
        zoom_out,
        reset_zoom,
    }
}

pub fn render_shortcuts_help(ctx: &egui::Context, language: Language, is_visible: &mut bool) {
    if !*is_visible {
        return;
    }

    let mut is_open = true;
    egui::Window::new(tr(language, TranslationKey::LabelShortcuts))
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .open(&mut is_open)
        .show(ctx, |ui| {
            egui::Grid::new("shortcuts_help_grid")
                .num_columns(2)
                .spacing(Vec2::new(24.0, 8.0))
                .striped(true)
                .show(ui, |ui| {
                    ui.strong(tr(language, TranslationKey::LabelShortcutAction));
                    ui.strong(tr(language, TranslationKey::LabelShortcut));
                    ui.end_row();

                    for (action, shortcut) in shortcuts_help_items(language) {
                        ui.label(action);
                        ui.monospace(shortcut);
                        ui.end_row();
                    }
                });

            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(tr(language, TranslationKey::ActionClose))
                    .clicked()
                {
                    *is_visible = false;
                }
            });
        });

    if !is_open || ctx.input(|input| input.key_pressed(Key::Escape)) {
        *is_visible = false;
    }
}

fn shortcuts_help_items(language: Language) -> [(&'static str, &'static str); 11] {
    [
        (tr(language, TranslationKey::ShortcutOpenFile), "Ctrl+O"),
        (tr(language, TranslationKey::ShortcutFocusSearch), "Ctrl+F"),
        (
            tr(language, TranslationKey::ShortcutSearchNext),
            "F3 / Enter",
        ),
        (
            tr(language, TranslationKey::ShortcutSearchPrevious),
            "Shift+F3",
        ),
        (
            tr(language, TranslationKey::ShortcutReloadFile),
            "Ctrl+R / F5",
        ),
        (tr(language, TranslationKey::ShortcutSwitchTheme), "Ctrl+T"),
        (
            tr(language, TranslationKey::ShortcutSwitchLanguage),
            "Ctrl+L",
        ),
        (
            tr(language, TranslationKey::ShortcutZoomIn),
            "Ctrl++ / Ctrl+=",
        ),
        (tr(language, TranslationKey::ShortcutZoomOut), "Ctrl+-"),
        (tr(language, TranslationKey::ShortcutResetZoom), "Ctrl+0"),
        (tr(language, TranslationKey::ShortcutShowHelp), "F1"),
    ]
}
