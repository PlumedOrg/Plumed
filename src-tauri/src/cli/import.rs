use std::path::Path;

use log::info;
use tauri::{AppHandle, Emitter, Manager};

use super::types::{ImportStrategy, MurmureExportData};
use crate::dictionary::Dictionary;
use crate::formatting_rules::types::FormattingSettings;

const MAX_SUPPORTED_VERSION: u32 = 1;

fn validate_extension(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("murmure") => Ok(()),
        _ => Err("Error: File must have a .murmure extension.".to_string()),
    }
}

fn ensure_import_allowed(block_cli_import: bool) -> Result<(), String> {
    match block_cli_import {
        true => Err(
            "Error: Importing settings from the command line is blocked. To allow it, open Plumed, \
             Settings > System > Advanced, and turn off \"Block command-line import\" (in French: \
             Paramètres > Système > Avancé, \"Bloquer l'import en ligne de commande\"). You can \
             also import the file from the app window: Import / Export."
                .to_string(),
        ),
        false => Ok(()),
    }
}

fn category_display_name(key: &str) -> &str {
    match key {
        "settings" => "System Settings",
        "shortcuts" => "Shortcuts",
        "formatting_rules" => "Formatting Rules",
        "dictionary" => "Dictionary",
        "medical_phrases" => "Medical vocabulary (your changes)",
        "medical_confusion_groups" => "Sound-alike words (your changes)",
        "medical_rules" => "Residual errors (your changes)",
        "grammar_rules" => "Grammar (your changes)",
        "text_templates" => "Text templates (your changes)",
        _ => key,
    }
}

pub fn execute_import(
    app: &AppHandle,
    file_path: &str,
    strategy: &ImportStrategy,
) -> Result<String, String> {
    ensure_import_allowed(crate::settings::load_settings(app).block_cli_import)?;
    validate_extension(file_path)?;

    let content = std::fs::read_to_string(file_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => format!("Error: File not found: {}", file_path),
        std::io::ErrorKind::PermissionDenied => {
            format!("Error: Permission denied: {}", file_path)
        }
        _ => format!("Error: Failed to read file: {}", e),
    })?;

    let data = parse_export(&content)?;

    if data.version > MAX_SUPPORTED_VERSION {
        return Err(format!(
            "Error: Unsupported file version ({}). Maximum supported: {}.",
            data.version, MAX_SUPPORTED_VERSION
        ));
    }

    let mut imported_categories: Vec<&str> = Vec::new();

    let has_settings = data.categories.settings.is_some();
    let has_shortcuts = data.categories.shortcuts.is_some();

    if has_settings || has_shortcuts {
        let mut current = crate::settings::load_settings(app);

        if let Some(ref s) = data.categories.settings {
            current.record_mode = s.record_mode.clone();
            current.overlay_mode = s.overlay_mode.clone();
            current.overlay_position = s.overlay_position.clone();
            current.copy_to_clipboard = s.copy_to_clipboard;
            current.paste_method = s.paste_method.clone();
            current.auto_insert = s.auto_insert;
            current.persist_history = s.persist_history;
            current.language = s.language.clone();
            current.sound_enabled = s.sound_enabled;
            current.sound_volume = s.sound_volume;
            current.lower_output_while_recording = s.lower_output_while_recording;
            current.output_volume_while_recording = s.output_volume_while_recording;
            current.log_level = s.log_level.clone();
            current.show_in_dock = s.show_in_dock;
            current.streaming_preview = s.streaming_preview;
            current.overlay_size = s.overlay_size.clone();
            current.streaming_text_width = s.streaming_text_width;
            current.streaming_font_size = s.streaming_font_size;
            current.streaming_max_lines = s.streaming_max_lines;
            current.result_panel_mode =
                crate::settings::normalize_result_panel_mode(&s.result_panel_mode).to_string();
            current.result_panel_duration_secs = s.result_panel_duration_secs;
            imported_categories.push("settings");
        }

        if let Some(ref sc) = data.categories.shortcuts {
            current.record_shortcut = sc.record_shortcut.clone();
            current.last_transcript_shortcut = sc.last_transcript_shortcut.clone();
            current.cancel_shortcut = sc.cancel_shortcut.clone();
            imported_categories.push("shortcuts");
        }

        crate::settings::save_settings(app, &current)?;
    }

    if let Some(ref imported) = data.categories.formatting_rules {
        apply_formatting_rules(app, imported, strategy)?;
        imported_categories.push("formatting_rules");
    }

    if let Some(ref imported) = data.categories.dictionary {
        apply_dictionary(app, &imported.words(), strategy)?;
        imported_categories.push("dictionary");
    }

    // Plumed fork: the Corrections lists, through the same validation and
    // saving as the settings import of the app.
    let medical_categories = data.categories.medical.categories();
    if !medical_categories.is_empty() {
        // The cold CLI path runs before `medical::init`: locate the medical
        // data in use first (idempotent), so the edits activate on it.
        crate::medical::data_set::init(app);
        let merge = matches!(strategy, ImportStrategy::Merge);
        crate::medical::changes::import(app, data.categories.medical, merge)?;
        imported_categories.extend(medical_categories);
    }

    // Wayland onboarding flags are environment-specific (not part of the config payload),
    // so importing must always re-show those cards.
    if !imported_categories.is_empty() {
        let mut s = crate::settings::load_settings(app);
        let mut dirty = false;

        if s.wayland_notice_dismissed {
            s.wayland_notice_dismissed = false;
            dirty = true;
        }
        if s.wayland_clipboard_fallback_dismissed {
            s.wayland_clipboard_fallback_dismissed = false;
            dirty = true;
        }

        if dirty {
            let _ = crate::settings::save_settings(app, &s);
        }
    }

    if imported_categories.is_empty() {
        return Ok(
            "Configuration imported successfully.\nNo categories found in file.".to_string(),
        );
    }

    let display_names: Vec<&str> = imported_categories
        .iter()
        .map(|k| category_display_name(k))
        .collect();

    Ok(format!(
        "Configuration imported successfully.\nUpdated: {}.",
        display_names.join(", ")
    ))
}

fn parse_export(content: &str) -> Result<MurmureExportData, String> {
    let invalid_format = |_| "Error: Invalid file format.".to_string();
    let mut raw: serde_json::Value = serde_json::from_str(content).map_err(invalid_format)?;
    if let Some(settings) = raw.pointer_mut("/categories/settings") {
        crate::settings::migrate_legacy_paste_method(settings);
    }
    serde_json::from_value(raw).map_err(invalid_format)
}

fn apply_formatting_rules(
    app: &AppHandle,
    imported: &FormattingSettings,
    strategy: &ImportStrategy,
) -> Result<(), String> {
    match strategy {
        ImportStrategy::Replace => {
            crate::formatting_rules::save(app, imported)?;
        }
        ImportStrategy::Merge => {
            let current = crate::formatting_rules::load(app)?;

            let mut merged_rules = current.rules;
            for imported_rule in &imported.rules {
                if let Some(idx) = merged_rules.iter().position(|r| r.id == imported_rule.id) {
                    merged_rules[idx] = imported_rule.clone();
                } else {
                    merged_rules.push(imported_rule.clone());
                }
            }

            let merged = FormattingSettings {
                built_in: imported.built_in.clone(),
                rules: merged_rules,
            };
            crate::formatting_rules::save(app, &merged)?;
        }
    }
    Ok(())
}

fn apply_dictionary(
    app: &AppHandle,
    imported: &[String],
    strategy: &ImportStrategy,
) -> Result<(), String> {
    match strategy {
        ImportStrategy::Replace => {
            crate::dictionary::store::save(app, imported)?;
        }
        ImportStrategy::Merge => {
            let current = crate::dictionary::store::load(app)?;
            let merged = merge_dictionaries(&current, imported);
            crate::dictionary::store::save(app, &merged)?;
        }
    }
    Ok(())
}

fn merge_dictionaries(current: &[String], imported: &[String]) -> Vec<String> {
    let mut merged = current.to_vec();
    for word in imported {
        if !merged.iter().any(|k| k.eq_ignore_ascii_case(word)) {
            merged.push(word.clone());
        }
    }
    merged
}

pub fn apply_hot_reload_side_effects(app: &AppHandle) {
    crate::shortcuts::init_shortcuts(app.clone());

    let settings = crate::settings::load_settings(app);

    if settings.overlay_mode.as_str() == "always" {
        crate::overlay::overlay::show_recording_overlay(app);
    } else {
        crate::overlay::overlay::hide_recording_overlay(app);
    }

    crate::overlay::overlay::update_overlay_position(app);

    if let Ok(level) = settings.log_level.parse::<log::LevelFilter>() {
        log::set_max_level(level);
    }

    match crate::dictionary::store::load(app) {
        Ok(dict) => {
            let dictionary_state = app.state::<Dictionary>();
            dictionary_state.set(dict);
            let _ = app.emit("dictionary:updated", ());
        }
        Err(e) => {
            log::error!("Failed to reload dictionary after CLI import: {}", e);
        }
    }

    let _ = app.emit("config-imported", ());

    info!("Hot-reload side effects applied after CLI import");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::PasteMethod;

    #[test]
    fn test_import_refused_when_blocked() {
        assert!(ensure_import_allowed(true)
            .unwrap_err()
            .contains("turn off \"Block command-line import\""));
        assert!(ensure_import_allowed(false).is_ok());
    }

    #[test]
    fn test_validate_extension_valid() {
        assert!(validate_extension("/tmp/config.murmure").is_ok());
        assert!(validate_extension("C:\\config\\standard.murmure").is_ok());
    }

    #[test]
    fn test_validate_extension_invalid() {
        let result = validate_extension("/tmp/config.json");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("File must have a .murmure extension"));
    }

    #[test]
    fn test_validate_extension_no_extension() {
        let result = validate_extension("/tmp/config");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_dictionaries_new_word() {
        let current = vec!["Kubernetes".to_string()];
        let imported = vec!["Docker".to_string()];

        let merged = merge_dictionaries(&current, &imported);
        assert_eq!(merged.len(), 2);
        assert!(merged.contains(&"Kubernetes".to_string()));
        assert!(merged.contains(&"Docker".to_string()));
    }

    #[test]
    fn test_merge_dictionaries_case_insensitive() {
        let current = vec!["Kubernetes".to_string()];
        let imported = vec!["kubernetes".to_string()];

        let merged = merge_dictionaries(&current, &imported);
        // The existing casing wins, the case-variant duplicate is dropped.
        assert_eq!(merged, vec!["Kubernetes".to_string()]);
    }

    #[test]
    fn test_merge_dictionaries_empty_current() {
        let current: Vec<String> = Vec::new();
        let imported = vec!["Docker".to_string()];

        let merged = merge_dictionaries(&current, &imported);
        assert_eq!(merged.len(), 1);
        assert!(merged.contains(&"Docker".to_string()));
    }

    #[test]
    fn test_merge_dictionaries_empty_imported() {
        let current = vec!["Kubernetes".to_string()];
        let imported: Vec<String> = Vec::new();

        let merged = merge_dictionaries(&current, &imported);
        assert_eq!(merged, vec!["Kubernetes".to_string()]);
    }

    #[test]
    fn test_parse_export_migrates_legacy_none_paste_method() {
        let content = r#"{
            "version": 1,
            "app_version": "1.0.0",
            "exported_at": "2026-01-01T00:00:00Z",
            "categories": { "settings": { "paste_method": "none", "copy_to_clipboard": true } }
        }"#;

        let settings = parse_export(content).unwrap().categories.settings.unwrap();

        assert_eq!(settings.paste_method, PasteMethod::CtrlV);
        assert!(!settings.auto_insert);
        assert!(settings.copy_to_clipboard);
    }

    #[test]
    fn test_parse_export_ignores_removed_feature_keys() {
        // Exports made before the local API, command mode, prompt mode,
        // SmartMic and voice mode were removed must still import their
        // remaining settings.
        let content = r#"{
            "version": 1,
            "app_version": "1.0.0",
            "exported_at": "2026-01-01T00:00:00Z",
            "categories": {
                "settings": { "api_enabled": true, "api_port": 4900, "sound_volume": 42 },
                "shortcuts": { "command_shortcut": "ctrl+shift+x", "llm_mode_1_shortcut": "ctrl+shift+1", "voice_mode_toggle_shortcut": "ctrl+shift+0", "cancel_shortcut": "ctrl+alt+c" },
                "voice_mode": { "wake_word_enabled": true, "wake_word_command": "alix command", "wake_word_record": "ok alix", "silence_timeout_ms": 1500 },
                "llm_connect": { "url": "http://localhost:11434/api", "modes": [] },
                "smartmic": { "smartmic_enabled": true, "smartmic_port": 4801 }
            }
        }"#;

        let data = parse_export(content).unwrap();

        assert_eq!(data.categories.settings.unwrap().sound_volume, 42);
        assert_eq!(
            data.categories.shortcuts.unwrap().cancel_shortcut,
            "ctrl+alt+c"
        );
    }

    #[test]
    fn test_parse_export_round_trips_the_corrections_lists() {
        use crate::engine::confusion::ConfusionGroup;
        use crate::medical::list_overlay::{ListOverlay, Phrase};
        use crate::medical::{MedicalChanges, RuleEdit};

        // What the app exports: `get_medical_changes` serialized into the
        // categories, next to the other ones.
        let changes = MedicalChanges {
            medical_phrases: Some(ListOverlay {
                disabled: vec!["rituximab".to_string()],
                added: vec![Phrase("venlafaxine".to_string())],
            }),
            medical_confusion_groups: Some(ListOverlay {
                disabled: Vec::new(),
                added: vec![ConfusionGroup {
                    heard: "ituximab".to_string(),
                    candidates: vec!["rituximab".to_string(), "cetuximab".to_string()],
                }],
            }),
            medical_rules: Some(vec![RuleEdit {
                original: None,
                core: "dolipran".to_string(),
                replacement: "Doliprane".to_string(),
                enabled: true,
            }]),
            grammar_rules: Some(ListOverlay {
                disabled: vec!["g2__locutions_invariables".to_string()],
                added: vec!["g3__gn_les_2m".to_string()],
            }),
            text_templates: Some(ListOverlay {
                disabled: Vec::new(),
                added: vec![crate::medical::text_templates::TextTemplate {
                    trigger: "mon texte".to_string(),
                    text: "ligne 1\nligne 2".to_string(),
                }],
            }),
        };
        let mut categories = serde_json::to_value(&changes).unwrap();
        categories["dictionary"] = serde_json::json!(["Cornélis"]);
        let content = serde_json::json!({
            "version": 1,
            "app_version": "0.1.3",
            "exported_at": "2026-11-22T00:00:00Z",
            "categories": categories,
        })
        .to_string();

        let data = parse_export(&content).unwrap();

        assert_eq!(data.categories.medical, changes);
        assert_eq!(
            data.categories.dictionary.unwrap().words(),
            vec!["Cornélis".to_string()]
        );
    }

    #[test]
    fn test_parse_export_old_file_has_no_corrections_lists() {
        let content = r#"{
            "version": 1,
            "app_version": "1.0.0",
            "exported_at": "2026-01-01T00:00:00Z",
            "categories": { "dictionary": { "Docker": ["en"] }, "shortcuts": { "cancel_shortcut": "ctrl+alt+c" } }
        }"#;

        let data = parse_export(content).unwrap();

        assert!(data.categories.medical.categories().is_empty());
        assert_eq!(
            data.categories.dictionary.unwrap().words(),
            vec!["Docker".to_string()]
        );
        assert!(data.categories.shortcuts.is_some());
    }

    #[test]
    fn test_parse_export_only_imports_allowed_settings() {
        // A file asking to turn on network features or change the relay:
        // the settings category only knows the allowlisted fields of
        // `SystemSettings`, so these keys are dropped, and the medical lists
        // carry no setting at all.
        let content = r#"{
            "version": 1,
            "app_version": "1.0.0",
            "exported_at": "2026-01-01T00:00:00Z",
            "categories": {
                "settings": {
                    "sound_volume": 42,
                    "remote_mic_enabled": true,
                    "remote_mic_url": "https://evil.example",
                    "telemetry_enabled": true,
                    "medical_data_update_check": true,
                    "block_cli_import": false
                },
                "medical_phrases": { "disabled": [], "added": ["venlafaxine"], "remote_mic_url": "https://evil.example" }
            }
        }"#;

        let data = parse_export(content).unwrap();
        let settings = serde_json::to_value(&data.categories.medical).unwrap();

        assert_eq!(data.categories.settings.unwrap().sound_volume, 42);
        assert!(!settings.to_string().contains("evil"));
    }

    #[test]
    fn test_category_display_name() {
        assert_eq!(category_display_name("settings"), "System Settings");
        assert_eq!(category_display_name("shortcuts"), "Shortcuts");
        assert_eq!(
            category_display_name("formatting_rules"),
            "Formatting Rules"
        );
        assert_eq!(category_display_name("dictionary"), "Dictionary");
        assert_eq!(category_display_name("unknown"), "unknown");
    }
}
