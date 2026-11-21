//! Plumed fork: Tauri commands of the remote microphone (phone through a relay).

use crate::remote_mic::client::{origin_of, RelayClient};
use crate::remote_mic::session::{Command, RemoteMicState, Snapshot};
use tauri::{command, AppHandle, State};

#[command]
pub fn set_remote_mic_enabled(
    app: AppHandle,
    state: State<'_, RemoteMicState>,
    enabled: bool,
) -> Result<(), String> {
    let mut s = crate::settings::load_settings(&app);
    s.remote_mic_enabled = enabled;
    if !enabled {
        state.send(Command::Stop);
    }
    crate::settings::save_settings(&app, &s)
}

/// Opens a room on the relay and shows its QR code. Only on an explicit click.
#[command]
pub fn remote_mic_start(app: AppHandle, state: State<'_, RemoteMicState>) -> Result<(), String> {
    let s = crate::settings::load_settings(&app);
    if !s.remote_mic_enabled {
        return Err("PhoneMic is disabled in the settings".to_string());
    }
    let client = RelayClient::new(origin_of(&s.remote_mic_url)?).map_err(|e| e.to_string())?;
    state.start(app, client)
}

#[command]
pub fn remote_mic_stop(state: State<'_, RemoteMicState>) {
    state.send(Command::Stop);
}

/// The user compared the fingerprint shown on both screens. `fingerprint` is
/// the code the UI displayed, so a click never applies to another phone.
#[command]
pub fn remote_mic_confirm(state: State<'_, RemoteMicState>, fingerprint: String, accepted: bool) {
    state.send(Command::Confirm {
        fingerprint,
        accepted,
    });
}

#[command]
pub fn get_remote_mic_state(state: State<'_, RemoteMicState>) -> Snapshot {
    state.snapshot()
}

/// Whether the startup question "use your phone as microphone?" is due: the
/// remote microphone is off, no local microphone was found, and the question
/// was not answered on this version (so it comes back after an update).
fn remote_mic_prompt_is_due(
    enabled: bool,
    answered_version: &str,
    app_version: &str,
    mic_count: usize,
) -> bool {
    !enabled && answered_version != app_version && mic_count == 0
}

/// Async so listing the microphones (`pactl` on Linux) does not block the UI.
#[command(async)]
pub fn remote_mic_prompt_due(app: AppHandle) -> bool {
    let s = crate::settings::load_settings(&app);
    remote_mic_prompt_is_due(
        s.remote_mic_enabled,
        &s.remote_mic_prompt_version,
        &app.package_info().version.to_string(),
        crate::audio::microphone::get_mic_list().len(),
    )
}

/// Records the answer to the startup question for this version.
#[command]
pub fn answer_remote_mic_prompt(app: AppHandle, enable: bool) -> Result<(), String> {
    let mut s = crate::settings::load_settings(&app);
    s.remote_mic_enabled = enable;
    s.remote_mic_prompt_version = app.package_info().version.to_string();
    crate::settings::save_settings(&app, &s)
}

#[cfg(test)]
mod tests {
    use super::remote_mic_prompt_is_due;

    #[test]
    fn prompt_only_without_microphone_and_once_per_version() {
        assert!(remote_mic_prompt_is_due(false, "", "0.1.4", 0));
        assert!(remote_mic_prompt_is_due(false, "0.1.3", "0.1.4", 0));
        assert!(!remote_mic_prompt_is_due(false, "0.1.4", "0.1.4", 0));
        assert!(!remote_mic_prompt_is_due(false, "", "0.1.4", 1));
        assert!(!remote_mic_prompt_is_due(true, "", "0.1.4", 0));
    }
}
