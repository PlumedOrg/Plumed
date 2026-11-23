//! The Tutorial page (Plumed fork). Written with Claude Code.

use crate::settings::AppSettings;
use tauri::{command, AppHandle};

fn update(app: &AppHandle, change: impl FnOnce(&mut AppSettings)) -> Result<(), String> {
    let mut settings = crate::settings::load_settings(app);
    change(&mut settings);
    crate::settings::save_settings(app, &settings)
}

/// The Tutorial page was shown as the landing page: it is not again.
#[command]
pub fn mark_tutorial_opened(app: AppHandle) -> Result<(), String> {
    update(&app, |s| s.tutorial_opened = true)
}

/// "J'ai terminé le tutoriel" (or the box unchecked again).
#[command]
pub fn set_tutorial_completed(app: AppHandle, completed: bool) -> Result<(), String> {
    update(&app, |s| s.tutorial_completed = completed)
}
