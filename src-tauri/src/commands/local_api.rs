use tauri::{command, AppHandle};

use crate::local_api::{self, LocalApiStatus};

#[command]
pub fn get_local_api_status(app: AppHandle) -> LocalApiStatus {
    local_api::status(&app)
}

#[command]
pub fn set_local_api_enabled(app: AppHandle, enabled: bool) -> Result<LocalApiStatus, String> {
    let mut s = crate::settings::load_settings(&app);
    s.local_api_enabled = enabled;
    crate::settings::save_settings(&app, &s)?;
    local_api::apply_settings(&app)?;
    Ok(local_api::status(&app))
}

#[command]
pub fn set_local_api_port(app: AppHandle, port: u16) -> Result<LocalApiStatus, String> {
    if port == 0 {
        return Err("Invalid port: 0".to_string());
    }
    let mut s = crate::settings::load_settings(&app);
    s.local_api_port = port;
    crate::settings::save_settings(&app, &s)?;
    local_api::apply_settings(&app)?;
    Ok(local_api::status(&app))
}

#[command]
pub fn regenerate_local_api_token(app: AppHandle) -> Result<LocalApiStatus, String> {
    local_api::regenerate_token(&app)?;
    Ok(local_api::status(&app))
}
