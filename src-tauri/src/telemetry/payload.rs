//! The JSON sent to Umami: an allowlist of aggregated values, never any text,
//! rule, dictionary word, path, hostname or timestamp.

use super::counters::{mean_stdev, Counters};
use serde_json::{json, Map, Value};

/// Hostname and page Umami files the event under (not this computer's name).
const HOSTNAME: &str = "app.plumed.fr";
const URL: &str = "/app";
const EVENT_NAME: &str = "daily";

/// What describes this installation, read from the settings and the build.
pub struct Environment {
    pub version: String,
    pub os: &'static str,
    pub arch: &'static str,
    pub install_kind: Option<String>,
    pub language: String,
    /// The speech model in use: a catalog variant id, `dev` or `none`.
    pub model: &'static str,
}

/// Which features are in use, as booleans or coarse buckets.
pub struct FeatureFlags {
    pub user_rules: &'static str,
    pub medical_rules_edited: bool,
    pub dictionary_words: &'static str,
    pub record_mode: &'static str,
    pub remote_mic: bool,
    pub confusion_rescoring: bool,
    pub phrase_boosting: bool,
    pub grammar: bool,
}

/// Coarse bucket of a count, `bounds` being the inclusive upper bounds:
/// with `[10, 50]`, 0 gives "0", 7 gives "1-10", 30 gives "11-50" and 80
/// gives ">50".
pub fn count_bucket(
    count: usize,
    bounds: &[(usize, &'static str)],
    above: &'static str,
) -> &'static str {
    match count {
        0 => "0",
        n => bounds
            .iter()
            .find(|(bound, _)| n <= *bound)
            .map_or(above, |(_, label)| label),
    }
}

/// Keeps a known value of a free-form setting, so nothing typed leaks.
pub fn known_value(value: &str, known: &[&'static str]) -> &'static str {
    known
        .iter()
        .find(|k| **k == value)
        .copied()
        .unwrap_or("other")
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// The aggregated values of the event (`payload.data`).
pub fn event_data(
    env: &Environment,
    flags: &FeatureFlags,
    counters: &Counters,
    days_since_install: i64,
) -> Map<String, Value> {
    let n = counters.transcriptions;
    let (audio_mean, audio_stdev) =
        mean_stdev(n, counters.audio_secs_sum, counters.audio_secs_sq_sum);
    let (chars_mean, chars_stdev) = mean_stdev(n, counters.chars_sum, counters.chars_sq_sum);
    let rtf_mean = match n {
        0 => 0.0,
        n => counters.rtf_sum / n as f64,
    };
    let [under_10s, from_10_to_30s, from_30_to_120s, over_120s] = counters.audio_buckets;
    let data = json!({
        "version": env.version,
        "os": env.os,
        "arch": env.arch,
        "language": env.language,
        "model": env.model,
        "days_since_install": days_since_install,
        "transcriptions": n,
        "audio_secs_mean": round2(audio_mean),
        "audio_secs_stdev": round2(audio_stdev),
        "audio_under_10s": under_10s,
        "audio_10_30s": from_10_to_30s,
        "audio_30_120s": from_30_to_120s,
        "audio_over_120s": over_120s,
        "chars_mean": round2(chars_mean),
        "chars_stdev": round2(chars_stdev),
        "rtf_mean": round2(rtf_mean),
        "errors_model_load": counters.errors.model_load,
        "errors_microphone": counters.errors.microphone,
        "errors_paste": counters.errors.paste,
        "user_rules": flags.user_rules,
        "medical_rules_edited": flags.medical_rules_edited,
        "dictionary_words": flags.dictionary_words,
        "record_mode": flags.record_mode,
        "remote_mic": flags.remote_mic,
        "confusion_rescoring": flags.confusion_rescoring,
        "phrase_boosting": flags.phrase_boosting,
        "grammar": flags.grammar,
    });
    let mut map = match data {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    if let Some(kind) = &env.install_kind {
        map.insert("install_kind".to_string(), json!(kind));
    }
    map
}

/// The whole body of the Umami 3 `/api/send` request.
pub fn umami_body(website_id: &str, id: &str, language: &str, data: Map<String, Value>) -> Value {
    json!({
        "type": "event",
        "payload": {
            "website": website_id,
            "hostname": HOSTNAME,
            "url": URL,
            "language": language,
            "name": EVENT_NAME,
            "id": id,
            "data": data,
        }
    })
}

/// A page view on the same page, sent with each daily event: Umami's main
/// dashboard (visitors, views) only counts page views, so this makes each
/// active install show up there. Carries no data beyond the event's own.
pub fn umami_pageview(website_id: &str, id: &str, language: &str) -> Value {
    json!({
        "type": "event",
        "payload": {
            "website": website_id,
            "hostname": HOSTNAME,
            "url": URL,
            "language": language,
            "id": id,
        }
    })
}

/// A browser-like User-Agent: Umami drops requests whose User-Agent looks
/// like a bot, and reads the operating system from it.
pub fn user_agent(env: &Environment) -> String {
    let platform = match env.os {
        "windows" => "Windows NT 10.0",
        "macos" => "Macintosh; Mac OS X",
        "linux" => "X11; Linux",
        other => other,
    };
    format!(
        "Mozilla/5.0 ({platform} {}) Plumed/{}",
        env.arch, env.version
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Environment {
        Environment {
            version: "0.1.3".to_string(),
            os: "linux",
            arch: "x86_64",
            install_kind: Some("deb".to_string()),
            language: "fr".to_string(),
            model: "ultra-int8",
        }
    }

    fn flags() -> FeatureFlags {
        FeatureFlags {
            user_rules: "1-10",
            medical_rules_edited: true,
            dictionary_words: "0",
            record_mode: "push_to_talk",
            remote_mic: true,
            confusion_rescoring: true,
            phrase_boosting: true,
            grammar: true,
        }
    }

    #[test]
    fn pageview_carries_no_event_name_nor_data() {
        let body = umami_pageview("website", "id", "fr");
        let mut keys: Vec<&str> = body["payload"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["hostname", "id", "language", "url", "website"]);
    }

    #[test]
    fn payload_holds_only_the_allowlisted_keys() {
        let mut counters = Counters::default();
        counters.add_transcription(12.0, 120, 0.6);
        let data = event_data(&env(), &flags(), &counters, 20);
        let body = umami_body("website", "id", "fr", data);

        let mut keys: Vec<&str> = body["payload"]["data"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        let mut expected = vec![
            "version",
            "os",
            "arch",
            "install_kind",
            "language",
            "model",
            "days_since_install",
            "transcriptions",
            "audio_secs_mean",
            "audio_secs_stdev",
            "audio_under_10s",
            "audio_10_30s",
            "audio_30_120s",
            "audio_over_120s",
            "chars_mean",
            "chars_stdev",
            "rtf_mean",
            "errors_model_load",
            "errors_microphone",
            "errors_paste",
            "user_rules",
            "medical_rules_edited",
            "dictionary_words",
            "record_mode",
            "remote_mic",
            "confusion_rescoring",
            "phrase_boosting",
            "grammar",
        ];
        expected.sort_unstable();
        assert_eq!(keys, expected);

        let mut payload_keys: Vec<&str> = body["payload"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        payload_keys.sort_unstable();
        assert_eq!(
            payload_keys,
            ["data", "hostname", "id", "language", "name", "url", "website"]
        );
        // Every value is a number, a boolean or a short label: no free text.
        for value in body["payload"]["data"].as_object().unwrap().values() {
            match value {
                Value::String(s) => assert!(s.len() <= 32, "unexpected long text {s}"),
                Value::Number(_) | Value::Bool(_) => {}
                other => panic!("unexpected value {other}"),
            }
        }
    }

    #[test]
    fn install_kind_is_omitted_when_unknown() {
        let mut env = env();
        env.install_kind = None;
        let data = event_data(&env, &flags(), &Counters::default(), 0);
        assert!(!data.contains_key("install_kind"));
    }

    #[test]
    fn buckets_and_known_values_hide_details() {
        let bounds = [(10, "1-10"), (50, "11-50")];
        assert_eq!(count_bucket(0, &bounds, ">50"), "0");
        assert_eq!(count_bucket(7, &bounds, ">50"), "1-10");
        assert_eq!(count_bucket(30, &bounds, ">50"), "11-50");
        assert_eq!(count_bucket(80, &bounds, ">50"), ">50");
        assert_eq!(
            known_value("toggle_to_talk", &["push_to_talk", "toggle_to_talk"]),
            "toggle_to_talk"
        );
        assert_eq!(known_value("Dr Dupont", &["push_to_talk"]), "other");
    }

    #[test]
    fn user_agent_names_os_arch_and_version() {
        assert_eq!(
            user_agent(&env()),
            "Mozilla/5.0 (X11; Linux x86_64) Plumed/0.1.3"
        );
    }
}
