#[derive(Clone, Copy)]
pub enum Language {
    En,
    Ja,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TranslationKey {
    ActionCancel,
    ActionClose,
    ActionCopy,
    ActionOpenExternalLink,
    ActionOpen,
    ActionSearchNext,
    ActionSearchPrevious,
    ActionSearchClear,
    ActionResetZoom,
    ActionSwitchLanguage,
    ActionSwitchTheme,
    LabelCurrentFile,
    LabelExternalLinks,
    LabelNoFile,
    LabelRecentFiles,
    LabelSearch,
    LabelSearchResults,
    LabelShortcut,
    LabelShortcutAction,
    LabelShortcuts,
    LabelZoom,
    MessageCopied,
    MessageDropMarkdown,
    MessageEmpty,
    MessageExternalLinkPrompt,
    MessageImageLoadFailed,
    MessageImageUnsupported,
    MessageOpenPrompt,
    MessageRecentFileUnavailable,
    MessageSearchNoResults,
    NavJumpToHeading,
    NavNoSections,
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
    ShortcutFocusSearch,
    ShortcutOpenFile,
    ShortcutReloadFile,
    ShortcutResetZoom,
    ShortcutSearchNext,
    ShortcutSearchPrevious,
    ShortcutShowHelp,
    ShortcutSwitchLanguage,
    ShortcutSwitchTheme,
    ShortcutZoomIn,
    ShortcutZoomOut,
    ThemeMist,
    ThemeNightOwl,
    ThemeWarmPaper,
    ValueAskFirst,
    ValueOpenDirectly,
}

struct TranslationEntry {
    key: TranslationKey,
    en: &'static str,
    ja: &'static str,
}

const TRANSLATIONS: &[TranslationEntry] = &[
    entry(TranslationKey::ActionCancel, "Cancel", "キャンセル"),
    entry(TranslationKey::ActionClose, "Close", "閉じる"),
    entry(TranslationKey::ActionCopy, "Copy", "コピー"),
    entry(
        TranslationKey::ActionOpenExternalLink,
        "Open link",
        "リンクを開く",
    ),
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
    entry(TranslationKey::LabelExternalLinks, "Links:", "リンク:"),
    entry(
        TranslationKey::LabelNoFile,
        "No file selected",
        "ファイル未選択",
    ),
    entry(
        TranslationKey::LabelRecentFiles,
        "Recent",
        "最近使ったファイル",
    ),
    entry(TranslationKey::LabelSearch, "Search:", "検索:"),
    entry(TranslationKey::LabelSearchResults, "Matches:", "一致:"),
    entry(TranslationKey::LabelShortcut, "Shortcut", "ショートカット"),
    entry(TranslationKey::LabelShortcutAction, "Action", "操作"),
    entry(
        TranslationKey::LabelShortcuts,
        "Shortcuts",
        "ショートカット",
    ),
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
        TranslationKey::MessageExternalLinkPrompt,
        "Open this external link?",
        "この外部リンクを開きますか？",
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
        TranslationKey::MessageRecentFileUnavailable,
        "Recent file is unavailable:",
        "最近使ったファイルを開けません:",
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
    entry(
        TranslationKey::NavNoSections,
        "No sections",
        "見出しはありません",
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
    entry(
        TranslationKey::ShortcutFocusSearch,
        "Focus search",
        "検索へ移動",
    ),
    entry(
        TranslationKey::ShortcutOpenFile,
        "Open Markdown",
        "Markdownを開く",
    ),
    entry(
        TranslationKey::ShortcutReloadFile,
        "Reload file",
        "再読み込み",
    ),
    entry(
        TranslationKey::ShortcutResetZoom,
        "Reset zoom",
        "ズームをリセット",
    ),
    entry(TranslationKey::ShortcutSearchNext, "Next match", "次の一致"),
    entry(
        TranslationKey::ShortcutSearchPrevious,
        "Previous match",
        "前の一致",
    ),
    entry(
        TranslationKey::ShortcutShowHelp,
        "Show shortcuts",
        "ショートカットを表示",
    ),
    entry(
        TranslationKey::ShortcutSwitchLanguage,
        "Switch language",
        "言語を切り替え",
    ),
    entry(
        TranslationKey::ShortcutSwitchTheme,
        "Switch theme",
        "テーマを切り替え",
    ),
    entry(TranslationKey::ShortcutZoomIn, "Zoom in", "ズームイン"),
    entry(TranslationKey::ShortcutZoomOut, "Zoom out", "ズームアウト"),
    entry(TranslationKey::ThemeMist, "Mist", "ミスト"),
    entry(TranslationKey::ThemeNightOwl, "Night Owl", "ナイトアウル"),
    entry(
        TranslationKey::ThemeWarmPaper,
        "Warm Paper",
        "ウォームペーパー",
    ),
    entry(TranslationKey::ValueAskFirst, "Ask", "確認"),
    entry(TranslationKey::ValueOpenDirectly, "Open", "直接開く"),
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
