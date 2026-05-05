use std::path::{Path, PathBuf};

use crate::i18n::{Language, TranslationKey, tr};
use crate::theme::ThemeId;

const MAX_RECENT_FILES: usize = 8;
const MAX_RESTORED_OPEN_FILES: usize = 32;
const STORAGE_KEY_LANGUAGE: &str = "oxidemd.language";
const STORAGE_KEY_THEME: &str = "oxidemd.theme";
const STORAGE_KEY_ZOOM: &str = "oxidemd.zoom";
const STORAGE_KEY_EXTERNAL_LINKS: &str = "oxidemd.external_links";
const STORAGE_KEY_HEADING_PANEL: &str = "oxidemd.heading_panel";
const STORAGE_KEY_CURRENT_FILE: &str = "oxidemd.current_file";
const STORAGE_KEY_OPEN_FILES: &str = "oxidemd.open_files";
const STORAGE_KEY_ACTIVE_FILE: &str = "oxidemd.active_file";
const STORAGE_KEY_RECENT_FILES: &str = "oxidemd.recent_files";

#[derive(Clone, Copy)]
pub enum ExternalLinkBehavior {
    AskFirst,
    OpenDirectly,
}

pub struct RestoredSession {
    pub language: Option<Language>,
    pub theme_id: Option<ThemeId>,
    pub zoom_factor: Option<f32>,
    pub external_link_behavior: Option<ExternalLinkBehavior>,
    pub is_heading_panel_visible: Option<bool>,
    pub recent_files: Option<Vec<PathBuf>>,
    pub unavailable_current_file: Option<PathBuf>,
    pub open_files: Vec<PathBuf>,
    pub active_file: Option<PathBuf>,
    pub unavailable_open_files: Vec<PathBuf>,
}

pub struct SessionSaveData<'a> {
    pub language: Language,
    pub theme_id: ThemeId,
    pub zoom_factor: f32,
    pub external_link_behavior: ExternalLinkBehavior,
    pub is_heading_panel_visible: bool,
    pub current_file: Option<&'a Path>,
    pub open_files: Vec<&'a Path>,
    pub recent_files: &'a [PathBuf],
}

impl ExternalLinkBehavior {
    pub fn next(self) -> Self {
        match self {
            Self::AskFirst => Self::OpenDirectly,
            Self::OpenDirectly => Self::AskFirst,
        }
    }

    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::AskFirst => tr(language, TranslationKey::ValueAskFirst),
            Self::OpenDirectly => tr(language, TranslationKey::ValueOpenDirectly),
        }
    }

    fn storage_value(self) -> &'static str {
        match self {
            Self::AskFirst => "ask",
            Self::OpenDirectly => "open",
        }
    }

    fn from_storage_value(value: &str) -> Option<Self> {
        match value {
            "ask" => Some(Self::AskFirst),
            "open" => Some(Self::OpenDirectly),
            _ => None,
        }
    }
}

pub fn restore_session(
    storage: Option<&dyn eframe::Storage>,
    min_zoom_factor: f32,
    max_zoom_factor: f32,
) -> RestoredSession {
    let Some(storage) = storage else {
        return RestoredSession::empty();
    };

    let language = storage
        .get_string(STORAGE_KEY_LANGUAGE)
        .and_then(|value| language_from_storage_value(&value));
    let theme_id = storage
        .get_string(STORAGE_KEY_THEME)
        .and_then(|value| theme_id_from_storage_value(&value));
    let zoom_factor = storage
        .get_string(STORAGE_KEY_ZOOM)
        .and_then(|value| value.parse::<f32>().ok())
        .map(|zoom_factor| zoom_factor.clamp(min_zoom_factor, max_zoom_factor));
    let external_link_behavior = storage
        .get_string(STORAGE_KEY_EXTERNAL_LINKS)
        .and_then(|value| ExternalLinkBehavior::from_storage_value(&value));
    let is_heading_panel_visible = storage
        .get_string(STORAGE_KEY_HEADING_PANEL)
        .and_then(|value| bool_from_storage_value(&value));
    let recent_files = storage
        .get_string(STORAGE_KEY_RECENT_FILES)
        .map(|value| recent_files_from_storage_value(&value));

    let (current_file, unavailable_current_file) =
        restored_current_file(storage.get_string(STORAGE_KEY_CURRENT_FILE));
    let (mut open_files, mut unavailable_open_files) = restored_file_list(
        storage.get_string(STORAGE_KEY_OPEN_FILES),
        MAX_RESTORED_OPEN_FILES,
    );

    if open_files.is_empty() {
        if let Some(path) = &current_file {
            open_files.push(path.clone());
        }
    } else if let Some(path) = &unavailable_current_file {
        unavailable_open_files.push(path.clone());
    }

    let active_file =
        restored_active_file(storage.get_string(STORAGE_KEY_ACTIVE_FILE), &open_files)
            .or_else(|| current_file.clone())
            .filter(|path| open_files.contains(path));

    RestoredSession {
        language,
        theme_id,
        zoom_factor,
        external_link_behavior,
        is_heading_panel_visible,
        recent_files,
        unavailable_current_file,
        open_files,
        active_file,
        unavailable_open_files,
    }
}

pub fn save_session(storage: &mut dyn eframe::Storage, data: SessionSaveData<'_>) {
    storage.set_string(
        STORAGE_KEY_LANGUAGE,
        language_storage_value(data.language).to_owned(),
    );
    storage.set_string(
        STORAGE_KEY_THEME,
        theme_id_storage_value(data.theme_id).to_owned(),
    );
    storage.set_string(STORAGE_KEY_ZOOM, data.zoom_factor.to_string());
    storage.set_string(
        STORAGE_KEY_EXTERNAL_LINKS,
        data.external_link_behavior.storage_value().to_owned(),
    );
    storage.set_string(
        STORAGE_KEY_HEADING_PANEL,
        bool_storage_value(data.is_heading_panel_visible).to_owned(),
    );

    if let Some(path) = data.current_file {
        storage.set_string(STORAGE_KEY_CURRENT_FILE, path.display().to_string());
        storage.set_string(STORAGE_KEY_ACTIVE_FILE, path.display().to_string());
    } else {
        storage.set_string(STORAGE_KEY_CURRENT_FILE, String::new());
        storage.set_string(STORAGE_KEY_ACTIVE_FILE, String::new());
    }

    storage.set_string(
        STORAGE_KEY_OPEN_FILES,
        file_list_storage_value(&data.open_files, MAX_RESTORED_OPEN_FILES),
    );
    storage.set_string(
        STORAGE_KEY_RECENT_FILES,
        recent_files_storage_value(data.recent_files),
    );
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
        .unwrap_or(false)
}

pub fn remember_recent_file(recent_files: &mut Vec<PathBuf>, path: &Path) {
    let path = path.to_path_buf();
    recent_files.retain(|recent_path| recent_path != &path && recent_path.is_file());
    recent_files.insert(0, path);
    recent_files.truncate(MAX_RECENT_FILES);
}

impl RestoredSession {
    fn empty() -> Self {
        Self {
            language: None,
            theme_id: None,
            zoom_factor: None,
            external_link_behavior: None,
            is_heading_panel_visible: None,
            recent_files: None,
            unavailable_current_file: None,
            open_files: Vec::new(),
            active_file: None,
            unavailable_open_files: Vec::new(),
        }
    }
}

fn bool_storage_value(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn bool_from_storage_value(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn restored_current_file(value: Option<String>) -> (Option<PathBuf>, Option<PathBuf>) {
    let Some(current_file) = value else {
        return (None, None);
    };

    if current_file.is_empty() {
        return (None, None);
    }

    let path = PathBuf::from(current_file);
    if path.is_file() && is_markdown_path(&path) {
        (Some(path), None)
    } else {
        (None, Some(path))
    }
}

fn recent_files_from_storage_value(value: &str) -> Vec<PathBuf> {
    restored_file_list(Some(value.to_owned()), MAX_RECENT_FILES).0
}

fn restored_file_list(value: Option<String>, max_count: usize) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut recent_files = Vec::new();
    let mut unavailable_files = Vec::new();

    let Some(value) = value else {
        return (recent_files, unavailable_files);
    };

    for path in value.lines().map(PathBuf::from) {
        if recent_files.len() >= max_count {
            break;
        }

        if path.is_file() && is_markdown_path(&path) && !recent_files.contains(&path) {
            recent_files.push(path);
        } else if !path.as_os_str().is_empty() && !unavailable_files.contains(&path) {
            unavailable_files.push(path);
        }
    }

    (recent_files, unavailable_files)
}

fn restored_active_file(value: Option<String>, open_files: &[PathBuf]) -> Option<PathBuf> {
    let path = PathBuf::from(value?);
    if open_files.contains(&path) {
        Some(path)
    } else {
        None
    }
}

fn recent_files_storage_value(recent_files: &[PathBuf]) -> String {
    file_list_storage_value(recent_files, MAX_RECENT_FILES)
}

fn file_list_storage_value(paths: &[impl AsRef<Path>], max_count: usize) -> String {
    paths
        .iter()
        .take(max_count)
        .map(|path| path.as_ref().display().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn language_storage_value(language: Language) -> &'static str {
    match language {
        Language::En => "en",
        Language::Ja => "ja",
    }
}

fn language_from_storage_value(value: &str) -> Option<Language> {
    match value {
        "en" => Some(Language::En),
        "ja" => Some(Language::Ja),
        _ => None,
    }
}

fn theme_id_storage_value(theme_id: ThemeId) -> &'static str {
    match theme_id {
        ThemeId::WarmPaper => "warm_paper",
        ThemeId::Mist => "mist",
        ThemeId::NightOwl => "night_owl",
    }
}

fn theme_id_from_storage_value(value: &str) -> Option<ThemeId> {
    match value {
        "warm_paper" => Some(ThemeId::WarmPaper),
        "mist" => Some(ThemeId::Mist),
        "night_owl" => Some(ThemeId::NightOwl),
        _ => None,
    }
}
