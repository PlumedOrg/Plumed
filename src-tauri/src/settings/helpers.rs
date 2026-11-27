// "all" shows every dictation. Anything else is "off", including the legacy
// "command" and "command_llm" values: those only covered the removed Command
// and Prompt modes, so they never showed a plain dictation.
pub fn normalize_result_panel_mode(mode: &str) -> &'static str {
    match mode {
        "all" => "all",
        _ => "off",
    }
}

pub fn migrate_legacy_paste_method(settings: &mut serde_json::Value) {
    let Some(object) = settings.as_object_mut() else {
        return;
    };
    if matches!(
        object
            .get("paste_method")
            .and_then(serde_json::Value::as_str),
        Some("none" | "None")
    ) {
        object.insert("paste_method".to_string(), "ctrl_v".into());
        object.insert("auto_insert".to_string(), false.into());
    }
}

/// Plumed fork: settings saved before the cancel shortcut defaulted to
/// escape move from the old default (ctrl+backspace) to escape, once: the
/// flag makes a later choice of ctrl+backspace stick. Custom values stay.
pub fn migrate_cancel_shortcut_default(settings: &mut serde_json::Value) {
    let Some(object) = settings.as_object_mut() else {
        return;
    };
    if object.contains_key("cancel_shortcut_escape_migrated") {
        return;
    }
    if object
        .get("cancel_shortcut")
        .and_then(serde_json::Value::as_str)
        == Some("ctrl+backspace")
    {
        object.insert("cancel_shortcut".to_string(), "escape".into());
    }
    object.insert("cancel_shortcut_escape_migrated".to_string(), true.into());
}

/// Plumed fork: whether every key of `cancel` is in `record` (record
/// shift+escape, cancel escape), so starting a recording would cancel it.
pub fn cancel_inside_record(record: &str, cancel: &str) -> bool {
    let record = crate::shortcuts::parse_binding_keys(record);
    let cancel = crate::shortcuts::parse_binding_keys(cancel);
    !cancel.is_empty() && cancel.iter().all(|key| record.contains(key))
}

/// Plumed fork: blanks a cancel shortcut inside the record one (see
/// [`cancel_inside_record`]); true when it did.
pub fn blank_cancel_inside_record(settings: &mut super::AppSettings) -> bool {
    if !cancel_inside_record(&settings.record_shortcut, &settings.cancel_shortcut) {
        return false;
    }
    settings.cancel_shortcut.clear();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_default_cancel_shortcut_moves_to_escape_once() {
        let mut old = serde_json::json!({"cancel_shortcut": "ctrl+backspace"});
        migrate_cancel_shortcut_default(&mut old);
        assert_eq!(old["cancel_shortcut"], "escape");
        assert_eq!(old["cancel_shortcut_escape_migrated"], true);

        let mut chosen_again = serde_json::json!({
            "cancel_shortcut": "ctrl+backspace",
            "cancel_shortcut_escape_migrated": true,
        });
        migrate_cancel_shortcut_default(&mut chosen_again);
        assert_eq!(chosen_again["cancel_shortcut"], "ctrl+backspace");

        let mut custom = serde_json::json!({"cancel_shortcut": "ctrl+x"});
        migrate_cancel_shortcut_default(&mut custom);
        assert_eq!(custom["cancel_shortcut"], "ctrl+x");
    }

    #[test]
    fn cancel_made_of_record_keys_is_blanked() {
        let mut settings = crate::settings::AppSettings {
            record_shortcut: "shift+escape".to_string(),
            ..Default::default()
        };
        assert!(blank_cancel_inside_record(&mut settings));
        assert_eq!(settings.cancel_shortcut, "");
        // Already blank: nothing to do
        assert!(!blank_cancel_inside_record(&mut settings));
    }

    #[test]
    fn cancel_with_a_key_outside_record_is_kept() {
        assert!(!cancel_inside_record("ctrl+space", "escape"));
        assert!(!cancel_inside_record("escape", "shift+escape"));
        assert!(cancel_inside_record("escape", "escape"));
        assert!(!cancel_inside_record("shift+escape", ""));
    }

    #[test]
    fn valid_result_panel_modes_are_kept() {
        for mode in ["off", "all"] {
            assert_eq!(normalize_result_panel_mode(mode), mode);
        }
    }

    #[test]
    fn legacy_and_unknown_result_panel_modes_fall_back_to_off() {
        for mode in ["command", "command_llm", "commands", "", "All"] {
            assert_eq!(normalize_result_panel_mode(mode), "off");
        }
    }
}
