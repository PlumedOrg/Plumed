use super::types::FormattingSettings;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "formatting_rules.json";
const SETTINGS_KEY: &str = "settings";

/// Load formatting settings from the store
pub fn load(app: &AppHandle) -> Result<FormattingSettings, String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;

    match store.get(SETTINGS_KEY) {
        Some(value) => serde_json::from_value::<FormattingSettings>(value)
            .map_err(|e| format!("Failed to parse formatting settings: {}", e)),
        None => Ok(FormattingSettings::default()),
    }
}

/// Plumed fork: the rules the settings UI or an import may save. A
/// replacement may hold line breaks and tabs, no other control character.
fn validate(settings: &FormattingSettings) -> Result<(), String> {
    use crate::utils::user_text::{check_entry_count, has_unsafe_chars_besides_lines};
    check_entry_count(settings.rules.len(), "formatting rules")?;
    match settings
        .rules
        .iter()
        .find(|rule| has_unsafe_chars_besides_lines(&rule.replacement))
    {
        Some(rule) => Err(format!(
            "Formatting rule \"{}\": the replacement contains control characters",
            rule.trigger.escape_debug()
        )),
        None => Ok(()),
    }
}

/// Save formatting settings to the store
pub fn save(app: &AppHandle, settings: &FormattingSettings) -> Result<(), String> {
    validate(settings)?;
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;

    let value = serde_json::to_value(settings)
        .map_err(|e| format!("Failed to serialize formatting settings: {}", e))?;

    store.set(SETTINGS_KEY, value);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting_rules::types::FormattingRule;

    fn with_replacement(replacement: &str) -> FormattingSettings {
        FormattingSettings {
            rules: vec![FormattingRule {
                trigger: "à la ligne".to_string(),
                replacement: replacement.to_string(),
                ..FormattingRule::default()
            }],
            ..FormattingSettings::default()
        }
    }

    #[test]
    fn replacements_keep_line_breaks_but_no_control_key() {
        assert!(validate(&with_replacement("\n-HD :\tok\n")).is_ok());
        assert!(validate(&with_replacement("ok\u{8}\u{8}")).is_err());
        assert!(validate(&with_replacement("\u{16}")).is_err());
    }

    /// The UltiMed dictation rules seeded on a fresh install stay savable.
    #[test]
    fn default_rules_are_valid() {
        assert!(validate(&FormattingSettings::default()).is_ok());
    }
}
