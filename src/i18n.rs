#[derive(Clone, Copy)]
pub enum Language {
    En,
    Ja,
}

pub fn tr(language: Language, key: &str) -> &'static str {
    match language {
        Language::En => en(key),
        Language::Ja => ja(key),
    }
}

fn en(key: &str) -> &'static str {
    match key {
        "action.open" => "Open Markdown",
        "action.switch_language" => "日本語 / English",
        "action.switch_theme" => "Theme:",
        "label.current_file" => "Current file:",
        "label.no_file" => "No file selected",
        "message.empty" => "No markdown file is open",
        "message.open_prompt" => "Choose a Markdown file to start reading.",
        "status.no_file" => "No file selected.",
        "status.loaded" => "Loaded:",
        "status.load_failed" => "Failed to load file:",
        "status.reloaded" => "Reloaded:",
        "status.reload_failed" => "Failed to reload file:",
        "status.reload_started" => "Reloading file:",
        "status.watch_failed" => "Failed to watch file:",
        "status.worker_failed" => "Failed to queue reload:",
        "reload.idle" => "Ready",
        "reload.reloading" => "Reloading",
        "reload.error" => "Error",
        "theme.warm_paper" => "Warm Paper",
        "theme.mist" => "Mist",
        _ => "missing translation",
    }
}

fn ja(key: &str) -> &'static str {
    match key {
        "action.open" => "Markdownを開く",
        "action.switch_language" => "日本語 / English",
        "action.switch_theme" => "テーマ:",
        "label.current_file" => "現在のファイル:",
        "label.no_file" => "ファイル未選択",
        "message.empty" => "Markdownファイルはまだ開かれていません",
        "message.open_prompt" => "Markdownファイルを選択して読み込みます。",
        "status.no_file" => "ファイルが選択されていません。",
        "status.loaded" => "読み込み完了:",
        "status.load_failed" => "ファイルの読み込みに失敗しました:",
        "status.reloaded" => "再読み込み完了:",
        "status.reload_failed" => "ファイルの再読み込みに失敗しました:",
        "status.reload_started" => "ファイルを再読み込み中:",
        "status.watch_failed" => "ファイル監視の開始に失敗しました:",
        "status.worker_failed" => "再読み込み要求の送信に失敗しました:",
        "reload.idle" => "待機中",
        "reload.reloading" => "再読み込み中",
        "reload.error" => "エラー",
        "theme.warm_paper" => "ウォームペーパー",
        "theme.mist" => "ミスト",
        _ => "missing translation",
    }
}
