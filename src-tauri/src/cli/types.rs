use std::collections::HashMap;

use serde::Deserialize;

use crate::formatting_rules::types::FormattingSettings;
use crate::settings::types::PasteMethod;

#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    Import {
        file_path: String,
        strategy: ImportStrategy,
    },
    Transcribe {
        file_path: String,
    },
    /// Headless local transcription API (`plumed serve`); `None` keeps the
    /// port saved in the settings.
    Serve {
        port: Option<u16>,
    },
    Transcription,
    PasteLast,
    Cancel,
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportStrategy {
    Replace,
    Merge,
}

#[derive(Deserialize)]
pub struct MurmureExportData {
    pub version: u32,
    #[allow(dead_code)]
    pub app_version: String,
    #[allow(dead_code)]
    pub exported_at: String,
    pub categories: ExportedCategories,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ExportedCategories {
    pub settings: Option<SystemSettings>,
    pub shortcuts: Option<ShortcutSettings>,
    pub formatting_rules: Option<FormattingSettings>,
    pub dictionary: Option<DictionaryExport>,
    /// Plumed fork: the user's changes to the Corrections lists, one
    /// category each (`medical_phrases`, `medical_confusion_groups`,
    /// `medical_rules`, `grammar_rules`, `text_templates`), absent from older files.
    #[serde(flatten)]
    pub medical: crate::medical::MedicalChanges,
}

/// Current backups store the dictionary as a word list; older ones as a
/// `{ word: languages }` map whose values are ignored.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum DictionaryExport {
    Words(Vec<String>),
    Legacy(HashMap<String, Vec<String>>),
}

impl DictionaryExport {
    pub fn words(&self) -> Vec<String> {
        match self {
            DictionaryExport::Words(words) => words.clone(),
            DictionaryExport::Legacy(map) => map.keys().cloned().collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SystemSettings {
    pub record_mode: String,
    pub overlay_mode: String,
    pub overlay_position: String,
    pub copy_to_clipboard: bool,
    pub paste_method: PasteMethod,
    pub auto_insert: bool,
    pub persist_history: bool,
    pub language: String,
    pub sound_enabled: bool,
    pub sound_volume: u8,
    pub lower_output_while_recording: bool,
    pub output_volume_while_recording: u8,
    pub log_level: String,
    pub show_in_dock: bool,
    pub streaming_preview: bool,
    pub overlay_size: String,
    pub streaming_text_width: u32,
    pub streaming_font_size: u32,
    pub streaming_max_lines: u32,
    pub result_panel_mode: String,
    pub result_panel_duration_secs: u64,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            record_mode: "push_to_talk".to_string(),
            overlay_mode: "recording".to_string(),
            overlay_position: "bottom".to_string(),
            copy_to_clipboard: false,
            paste_method: PasteMethod::default(),
            auto_insert: true,
            persist_history: false,
            language: "default".to_string(),
            sound_enabled: true,
            sound_volume: 50,
            lower_output_while_recording: false,
            output_volume_while_recording: 50,
            log_level: "info".to_string(),
            show_in_dock: true,
            streaming_preview: false,
            overlay_size: "small".to_string(),
            streaming_text_width: 450,
            streaming_font_size: 11,
            streaming_max_lines: 5,
            result_panel_mode: "off".to_string(),
            result_panel_duration_secs: 5,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct ShortcutSettings {
    pub record_shortcut: String,
    pub last_transcript_shortcut: String,
    pub cancel_shortcut: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            record_shortcut: "ctrl+space".to_string(),
            last_transcript_shortcut: "ctrl+shift+space".to_string(),
            cancel_shortcut: "ctrl+backspace".to_string(),
        }
    }
}
