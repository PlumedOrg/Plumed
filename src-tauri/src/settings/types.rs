use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PasteMethod {
    #[default]
    #[serde(alias = "CtrlV")]
    CtrlV,
    #[serde(alias = "CtrlShiftV")]
    CtrlShiftV,
    #[serde(alias = "Direct")]
    Direct,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppSettings {
    pub record_shortcut: String,
    pub last_transcript_shortcut: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dictionary: Vec<String>,
    pub record_mode: String,      // "push_to_talk" | "toggle_to_talk"
    pub overlay_mode: String,     // "hidden" | "recording" | "always"
    pub overlay_position: String, // "top" | "bottom"
    pub copy_to_clipboard: bool,  // Keeps transcription in clipboard after recording finishes
    pub paste_method: PasteMethod,
    pub auto_insert: bool,
    pub persist_history: bool, // Persists last 5 transcriptions to disk
    pub language: String,      // UI language code (e.g., "en", "fr")
    pub sound_enabled: bool,
    pub sound_volume: u8,
    pub lower_output_while_recording: bool,
    pub output_volume_while_recording: u8,
    pub cancel_shortcut: String,
    pub mic_id: Option<String>,
    pub mic_label: Option<String>, // Persisted so the disconnected-mic UI can still show a friendly name
    pub log_level: String,         // "info" | "debug" | "trace" | "warn" | "error"
    // Debug aid: keep the recorded WAV files in the system temp dir after
    // transcription instead of deleting them.
    pub keep_recordings: bool,
    pub show_in_dock: bool,
    pub streaming_preview: bool,
    pub overlay_size: String, // "small" | "medium" | "large"
    pub streaming_text_width: u32,
    pub streaming_font_size: u32,
    pub streaming_max_lines: u32,
    pub result_panel_mode: String, // "off" | "all"
    pub result_panel_duration_secs: u64,
    pub remove_hesitations: bool,
    // Linux only. Persists the user's dismissal of `WaylandModeNotice`
    // so the onboarding banner does not reappear on next launch.
    pub wayland_notice_dismissed: bool,
    // Linux only. Persists the user's dismissal of the Wayland clipboard
    // fallback onboarding card (manual Ctrl+V hint when auto-paste fails).
    pub wayland_clipboard_fallback_dismissed: bool,
    // Refuses `plumed import` from the CLI, on by default (Plumed fork): any
    // program running as the user could otherwise replace the settings. Never
    // exported, so a settings file cannot flip it. Settings files saved before
    // keep their value (every field is written on save).
    pub block_cli_import: bool,
    // Plumed fork: re-transcribe short dictations behind the previous one (RAM only).
    pub short_dictation_context: bool,
    // Plumed fork: phone microphone through a ParakeetWeb relay. Never exported,
    // so a settings file cannot redirect dictation audio to another server.
    pub remote_mic_enabled: bool,
    pub remote_mic_url: String,
    // Plumed fork: app version on which the user answered the startup question
    // "use your phone as microphone?" (asked when no microphone is found).
    // Empty means never answered. Never exported.
    pub remote_mic_prompt_version: String,
    // Plumed fork: opt-in anonymous usage statistics (see `telemetry`).
    // `telemetry_asked` records that the consent dialog was answered, either
    // way. Never exported, so a settings file cannot turn telemetry on.
    pub telemetry_enabled: bool,
    pub telemetry_asked: bool,
    // Plumed fork: look for a newer signed medical data set at startup (only
    // asks, never downloads without a click), and the version the user chose
    // to skip. `medical_data_update_asked` records that the user answered the
    // first launch question (or used the toggle): no check runs before. Never
    // exported, so a settings file cannot turn the network check on.
    pub medical_data_update_check: bool,
    pub medical_data_skipped_version: String,
    pub medical_data_update_asked: bool,
}

/// Plumed: new installs type the text (Direct), which needs no clipboard and
/// works in every application. Wayland keeps Ctrl+V: Direct is experimental
/// there (keyboard layout detection can fail). Existing settings files keep
/// their value, `paste_method` is always saved.
fn default_paste_method(is_wayland: bool) -> PasteMethod {
    match is_wayland {
        true => PasteMethod::CtrlV,
        false => PasteMethod::Direct,
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            record_shortcut: "ctrl+space".to_string(),
            last_transcript_shortcut: "ctrl+shift+space".to_string(),
            dictionary: Vec::new(),
            // Wayland CLI mode binds shortcuts at the OS level, so Murmure receives no key-release events. Push-to-talk is impossible.
            record_mode: {
                #[cfg(target_os = "linux")]
                {
                    if crate::utils::platform::is_wayland_session() {
                        "toggle_to_talk".to_string()
                    } else {
                        "push_to_talk".to_string()
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    "push_to_talk".to_string()
                }
            },
            // gtk-layer-shell handles focus/positioning only on wlr-layer-shell
            // compositors. Mutter (GNOME) and Muffin (Cinnamon) lack it, so the
            // Tauri fallback misplaces the overlay and steals focus there.
            overlay_mode: {
                #[cfg(target_os = "linux")]
                {
                    if crate::utils::platform::is_wayland_session()
                        && !crate::utils::platform::is_portal_reliable_desktop()
                    {
                        "hidden".to_string()
                    } else {
                        "recording".to_string()
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    "recording".to_string()
                }
            },
            overlay_position: "bottom".to_string(),
            // Default to true on Wayland so transcriptions remain accessible
            // via manual Ctrl+V when enigo's key injection cannot reach native
            // Wayland apps. Users remain free to disable it.
            copy_to_clipboard: {
                #[cfg(target_os = "linux")]
                {
                    crate::utils::platform::is_wayland_session()
                }
                #[cfg(not(target_os = "linux"))]
                {
                    false
                }
            },
            paste_method: default_paste_method(crate::utils::platform::is_wayland_session()),
            auto_insert: true,
            persist_history: false,
            language: "default".to_string(),
            sound_enabled: true,
            sound_volume: 50,
            lower_output_while_recording: false,
            output_volume_while_recording: 50,
            cancel_shortcut: "ctrl+backspace".to_string(),
            mic_id: None,
            mic_label: None,
            log_level: "info".to_string(),
            keep_recordings: false,
            show_in_dock: true,
            streaming_preview: true,
            overlay_size: "small".to_string(),
            streaming_text_width: 450,
            streaming_font_size: 11,
            streaming_max_lines: 5,
            result_panel_mode: "off".to_string(),
            result_panel_duration_secs: 5,
            remove_hesitations: false,
            wayland_notice_dismissed: false,
            wayland_clipboard_fallback_dismissed: false,
            block_cli_import: true,
            short_dictation_context: true,
            remote_mic_enabled: false,
            remote_mic_url: "https://turn.plumed.fr".to_string(),
            remote_mic_prompt_version: String::new(),
            telemetry_enabled: false,
            telemetry_asked: false,
            medical_data_update_check: false,
            medical_data_skipped_version: String::new(),
            medical_data_update_asked: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_installs_block_cli_import_but_saved_settings_keep_their_value() {
        assert!(AppSettings::default().block_cli_import);
        let saved: AppSettings = serde_json::from_str(r#"{"block_cli_import":false}"#).unwrap();
        assert!(!saved.block_cli_import);
    }

    #[test]
    fn empty_shortcut_survives_serde_round_trip() {
        let mut settings = AppSettings::default();
        settings.record_shortcut = String::new();

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.record_shortcut, "");
    }

    #[test]
    fn new_installs_type_the_text_except_on_wayland() {
        assert_eq!(default_paste_method(false), PasteMethod::Direct);
        assert_eq!(default_paste_method(true), PasteMethod::CtrlV);
    }

    #[test]
    fn settings_with_the_removed_onboarding_checklist_still_load() {
        let parsed: AppSettings = serde_json::from_str(
            r#"{"sound_volume":42,"onboarding":{"used_home_shortcut":true,"congrats_dismissed":true}}"#,
        )
        .unwrap();
        assert_eq!(parsed.sound_volume, 42);
    }

    #[test]
    fn saved_paste_method_is_kept() {
        let parsed: AppSettings = serde_json::from_str(r#"{"paste_method":"ctrl_v"}"#).unwrap();
        assert_eq!(parsed.paste_method, PasteMethod::CtrlV);
    }
}
