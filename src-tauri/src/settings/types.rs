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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct OnboardingState {
    pub used_home_shortcut: bool,
    pub transcribed_outside_app: bool,
    pub added_dictionary_word: bool,
    pub congrats_dismissed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AppSettings {
    pub record_shortcut: String,
    pub last_transcript_shortcut: String,
    pub voice_mode_toggle_shortcut: String,
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
    pub onboarding: OnboardingState,
    pub cancel_shortcut: String,
    pub mic_id: Option<String>,
    pub mic_label: Option<String>, // Persisted so the disconnected-mic UI can still show a friendly name
    pub log_level: String,         // "info" | "debug" | "trace" | "warn" | "error"
    // Debug aid: keep the recorded WAV files in the system temp dir after
    // transcription instead of deleting them.
    pub keep_recordings: bool,
    pub wake_word_enabled: bool,
    pub wake_word_record: String,
    pub wake_word_cancel: String,
    pub wake_word_validate: String,
    pub wake_word_submit: String,
    pub auto_enter_after_wake_word: bool,
    pub silence_timeout_ms: u64,
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
    // Lockdown for managed devices: refuses `plumed import` from the CLI.
    // Never exported, so a settings file cannot flip it.
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
    // to skip. Never exported.
    pub medical_data_update_check: bool,
    pub medical_data_skipped_version: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            record_shortcut: "ctrl+space".to_string(),
            last_transcript_shortcut: "ctrl+shift+space".to_string(),
            voice_mode_toggle_shortcut: "ctrl+shift+0".to_string(),
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
            paste_method: PasteMethod::default(),
            auto_insert: true,
            persist_history: false,
            language: "default".to_string(),
            sound_enabled: true,
            sound_volume: 50,
            lower_output_while_recording: false,
            output_volume_while_recording: 50,
            onboarding: OnboardingState::default(),
            cancel_shortcut: "ctrl+backspace".to_string(),
            mic_id: None,
            mic_label: None,
            log_level: "info".to_string(),
            keep_recordings: false,
            wake_word_enabled: false,
            wake_word_record: "ok alix".to_string(),
            wake_word_cancel: "alix cancel".to_string(),
            wake_word_validate: "alix validate".to_string(),
            wake_word_submit: "thank you alix".to_string(),
            auto_enter_after_wake_word: false,
            silence_timeout_ms: 1500,
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
            block_cli_import: false,
            short_dictation_context: true,
            remote_mic_enabled: false,
            remote_mic_url: "https://turn.plumed.fr".to_string(),
            remote_mic_prompt_version: String::new(),
            telemetry_enabled: false,
            telemetry_asked: false,
            medical_data_update_check: true,
            medical_data_skipped_version: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_shortcut_survives_serde_round_trip() {
        let mut settings = AppSettings::default();
        settings.record_shortcut = String::new();

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.record_shortcut, "");
    }
}
