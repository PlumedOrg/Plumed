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
        return Err("the remote microphone is disabled in the settings".to_string());
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
