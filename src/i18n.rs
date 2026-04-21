#[derive(Clone, Copy)]
pub enum Language {
    En,
    Ja,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TranslationKey {
    ActionCopy,
    ActionOpen,
    ActionSearchNext,
    ActionSearchPrevious,
    ActionSearchClear,
    ActionResetZoom,
    ActionSwitchLanguage,
    ActionSwitchTheme,
    LabelCurrentFile,
    LabelNoFile,
    LabelSearch,
    LabelSearchResults,
    LabelZoom,
    MessageCopied,
    MessageDropMarkdown,
    MessageEmpty,
    MessageImageLoadFailed,
    MessageImageUnsupported,
    MessageOpenPrompt,
    MessageSearchNoResults,
    NavJumpToHeading,
    NavSections,
    ReloadError,
    ReloadIdle,
    ReloadReloading,
    StatusLoadFailed,
    StatusLoaded,
    StatusNoFile,
    StatusReloadFailed,
    StatusReloadSkipped,
    StatusReloadStarted,
    StatusReloaded,
    StatusUnsupportedFile,
    StatusWatchFailed,
    StatusWorkerFailed,
    ThemeMist,
    ThemeNightOwl,
    ThemeWarmPaper,
}

struct TranslationEntry {
    key: TranslationKey,
    en: &'static str,
    ja: &'static str,
}

const TRANSLATIONS: &[TranslationEntry] = &[
    entry(TranslationKey::ActionCopy, "Copy", "コピー"),
    entry(
        TranslationKey::ActionOpen,
        "Open Markdown",
        "Markdownを開く",
    ),
    entry(TranslationKey::ActionSearchNext, "Next", "次へ"),
    entry(TranslationKey::ActionSearchPrevious, "Previous", "前へ"),
    entry(TranslationKey::ActionSearchClear, "Clear", "クリア"),
    entry(TranslationKey::ActionResetZoom, "Reset", "リセット"),
    entry(
        TranslationKey::ActionSwitchLanguage,
        "日本語 / English",
        "日本語 / English",
    ),
    entry(TranslationKey::ActionSwitchTheme, "Theme:", "テーマ:"),
    entry(
        TranslationKey::LabelCurrentFile,
        "Current file:",
        "現在のファイル:",
    ),
    entry(
        TranslationKey::LabelNoFile,
        "No file selected",
        "ファイル未選択",
    ),
    entry(TranslationKey::LabelSearch, "Search:", "検索:"),
    entry(TranslationKey::LabelSearchResults, "Matches:", "一致:"),
    entry(TranslationKey::LabelZoom, "Zoom:", "ズーム:"),
    entry(TranslationKey::MessageCopied, "Copied", "コピーしました"),
    entry(
        TranslationKey::MessageDropMarkdown,
        "Drop Markdown file to open",
        "Markdownファイルをドロップして開く",
    ),
    entry(
        TranslationKey::MessageEmpty,
        "No markdown file is open",
        "Markdownファイルはまだ開かれていません",
    ),
    entry(
        TranslationKey::MessageImageLoadFailed,
        "Failed to load image:",
        "画像の読み込みに失敗しました:",
    ),
    entry(
        TranslationKey::MessageImageUnsupported,
        "Only local image paths are supported:",
        "ローカル画像パスのみ対応しています:",
    ),
    entry(
        TranslationKey::MessageOpenPrompt,
        "Choose or drop a Markdown file to start reading.",
        "Markdownファイルを選択またはドロップして読み込みます。",
    ),
    entry(
        TranslationKey::MessageSearchNoResults,
        "No matches found",
        "一致する内容はありません",
    ),
    entry(
        TranslationKey::NavJumpToHeading,
        "Jump to this section",
        "この見出しへ移動",
    ),
    entry(TranslationKey::NavSections, "Sections", "見出し"),
    entry(TranslationKey::ReloadError, "Error", "エラー"),
    entry(TranslationKey::ReloadIdle, "Ready", "待機中"),
    entry(TranslationKey::ReloadReloading, "Reloading", "再読み込み中"),
    entry(
        TranslationKey::StatusLoadFailed,
        "Failed to load file:",
        "ファイルの読み込みに失敗しました:",
    ),
    entry(TranslationKey::StatusLoaded, "Loaded:", "読み込み完了:"),
    entry(
        TranslationKey::StatusNoFile,
        "No file selected.",
        "ファイルが選択されていません。",
    ),
    entry(
        TranslationKey::StatusReloadFailed,
        "Failed to reload file:",
        "ファイルの再読み込みに失敗しました:",
    ),
    entry(
        TranslationKey::StatusReloadSkipped,
        "No changes detected:",
        "変更はありません:",
    ),
    entry(
        TranslationKey::StatusReloadStarted,
        "Reloading file:",
        "ファイルを再読み込み中:",
    ),
    entry(
        TranslationKey::StatusReloaded,
        "Reloaded:",
        "再読み込み完了:",
    ),
    entry(
        TranslationKey::StatusUnsupportedFile,
        "Unsupported file type:",
        "対応していないファイル形式です:",
    ),
    entry(
        TranslationKey::StatusWatchFailed,
        "Failed to watch file:",
        "ファイル監視の開始に失敗しました:",
    ),
    entry(
        TranslationKey::StatusWorkerFailed,
        "Failed to queue reload:",
        "再読み込み要求の送信に失敗しました:",
    ),
    entry(TranslationKey::ThemeMist, "Mist", "ミスト"),
    entry(TranslationKey::ThemeNightOwl, "Night Owl", "ナイトアウル"),
    entry(
        TranslationKey::ThemeWarmPaper,
        "Warm Paper",
        "ウォームペーパー",
    ),
];

const fn entry(key: TranslationKey, en: &'static str, ja: &'static str) -> TranslationEntry {
    TranslationEntry { key, en, ja }
}

pub fn tr(language: Language, key: TranslationKey) -> &'static str {
    let Some(entry) = TRANSLATIONS.iter().find(|entry| entry.key == key) else {
        debug_assert!(false, "missing translation key");
        return "";
    };

    match language {
        Language::En => entry.en,
        Language::Ja => entry.ja,
    }
}
