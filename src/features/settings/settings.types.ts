/** Plumed fork: which app releases the updater offers (`src-tauri/src/app_update.rs`). */
export type AppUpdateChannel = 'stable' | 'beta' | 'disabled';

export interface SystemSettings {
    record_mode: string;
    overlay_mode: string;
    overlay_position: string;
    copy_to_clipboard: boolean;
    paste_method: string;
    auto_insert: boolean;
    persist_history: boolean;
    language: string;
    sound_enabled: boolean;
    sound_volume: number;
    lower_output_while_recording: boolean;
    output_volume_while_recording: number;
    log_level: string;
    keep_recordings: boolean;
    show_in_dock: boolean;
    streaming_preview: boolean;
    overlay_size: string;
    streaming_text_width: number;
    streaming_font_size: number;
    streaming_max_lines: number;
    result_panel_mode: string;
    result_panel_duration_secs: number;
    remove_hesitations: boolean;
    wayland_notice_dismissed: boolean;
    wayland_clipboard_fallback_dismissed: boolean;
    block_cli_import: boolean;
    short_dictation_context: boolean;
    slow_down_audio: boolean;
    meeting_backup_audio: boolean;
    meeting_backup_text: boolean;
    meeting_backup_metadata: boolean;
    meeting_backup_persistent: boolean;
    remote_mic_enabled: boolean;
    remote_mic_url: string;
    remote_mic_prompt_version: string;
    telemetry_enabled: boolean;
    telemetry_asked: boolean;
    medical_data_update_check: boolean;
    medical_data_skipped_version: string;
    medical_data_update_asked: boolean;
    app_update_channel: AppUpdateChannel;
    app_update_channel_asked: boolean;
    tutorial_opened: boolean;
    tutorial_completed: boolean;
}

export interface ShortcutSettings {
    record_shortcut: string;
    last_transcript_shortcut: string;
    cancel_shortcut: string;
}

export interface AppSettings extends SystemSettings, ShortcutSettings {}
