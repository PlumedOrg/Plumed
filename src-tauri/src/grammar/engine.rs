//! Grammalecte 2.3.0 (GPL v3+, https://grammalecte.net) in an embedded QuickJS engine: the
//! JavaScript build of its Firefox extension, fetched into `resources/grammalecte/` by
//! `scripts/fetch-grammalecte.sh`. No Python, no extra process. Loading takes about a
//! second (the rule graph is a 3.6 MB script), so it happens once, on the grammar worker
//! thread ([`super::worker`]); a check then takes a few milliseconds per sentence.
//! Written with Claude Code.

use super::procedure::{Checker, GcError};
use rquickjs::{Context, Ctx, Function, Runtime};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Which files to load, in order: see the file itself.
const FILE_LIST: &str = include_str!("../../grammar-data/grammalecte-files.txt");
/// Our glue: data loading, options, the two functions Rust calls.
const HOST_JS: &str = include_str!("host.js");
/// What Grammalecte expects from a browser worker: `self` and a console.
const PRELUDE_JS: &str = "var self = globalThis; \
    var console = { log() {}, info() {}, warn() {}, error() {}, time() {}, timeEnd() {} };";
/// QuickJS stack limit; the worker thread gets twice as much.
pub const JS_STACK_BYTES: usize = 8 * 1024 * 1024;
/// A single check that runs longer than this is interrupted (a runaway rule regex).
const CHECK_TIMEOUT: Duration = Duration::from_secs(5);
/// Loading gets more time: the rule graph alone is a 3.6 MB script.
const LOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// The files of `FILE_LIST` with their kind, e.g. `("js", "grammalecte/text.js")`.
fn listed_files() -> impl Iterator<Item = (&'static str, &'static str)> {
    FILE_LIST
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(kind, _)| matches!(*kind, "js" | "data"))
}

/// Milliseconds since the epoch, the unit of the interrupt deadline.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub struct Grammalecte {
    // Field order matters: the context must be dropped before its runtime.
    context: Context,
    _runtime: Runtime,
    /// Epoch milliseconds after which the running script is interrupted.
    deadline: Arc<AtomicU64>,
}

fn js_error(ctx: &Ctx<'_>, error: rquickjs::Error, what: &str) -> String {
    match error {
        rquickjs::Error::Exception => {
            let exception = ctx.catch();
            let message = exception
                .as_exception()
                .and_then(|e| e.message())
                .unwrap_or_else(|| format!("{exception:?}"));
            format!("Grammalecte {what}: {message}")
        }
        other => format!("Grammalecte {what}: {other}"),
    }
}

impl Grammalecte {
    /// Loads Grammalecte from `dir` (the `resources/grammalecte` folder).
    pub fn load(dir: &Path) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| format!("QuickJS runtime: {e}"))?;
        runtime.set_max_stack_size(JS_STACK_BYTES);
        let deadline = Arc::new(AtomicU64::new(now_ms() + LOAD_TIMEOUT.as_millis() as u64));
        let watched = Arc::clone(&deadline);
        runtime.set_interrupt_handler(Some(Box::new(move || {
            now_ms() > watched.load(Ordering::Relaxed)
        })));
        let context = Context::full(&runtime).map_err(|e| format!("QuickJS context: {e}"))?;
        context.with(|ctx| -> Result<(), String> {
            let root: PathBuf = dir.to_path_buf();
            let read_data = Function::new(ctx.clone(), move |path: String| -> Option<String> {
                // Only the data files of the list, never another path.
                listed_files()
                    .any(|(kind, listed)| kind == "data" && listed == path)
                    .then(|| std::fs::read_to_string(root.join(&path)).ok())
                    .flatten()
            })
            .map_err(|e| e.to_string())?;
            ctx.globals()
                .set("__plumedFile", read_data)
                .map_err(|e| e.to_string())?;
            ctx.eval::<(), _>(PRELUDE_JS)
                .map_err(|e| js_error(&ctx, e, "prelude"))?;
            for (_, path) in listed_files().filter(|(kind, _)| *kind == "js") {
                let source = std::fs::read_to_string(dir.join(path))
                    .map_err(|e| format!("Cannot read {}: {e}", dir.join(path).display()))?;
                ctx.eval::<(), _>(source)
                    .map_err(|e| js_error(&ctx, e, path))?;
            }
            ctx.eval::<(), _>(HOST_JS)
                .map_err(|e| js_error(&ctx, e, "initialization"))
        })?;
        Ok(Self {
            context,
            _runtime: runtime,
            deadline,
        })
    }

    /// Calls one of the host functions on `text`, its JSON answer parsed.
    fn call<T: for<'de> Deserialize<'de>>(&self, function: &str, text: &str) -> Result<T, String> {
        self.deadline.store(
            now_ms() + CHECK_TIMEOUT.as_millis() as u64,
            Ordering::Relaxed,
        );
        let json = self.context.with(|ctx| -> Result<String, String> {
            let f: Function = ctx
                .globals()
                .get(function)
                .map_err(|e| js_error(&ctx, e, function))?;
            f.call((text,)).map_err(|e| js_error(&ctx, e, function))
        })?;
        serde_json::from_str(&json).map_err(|e| format!("Grammalecte {function} answer: {e}"))
    }
}

/// A grammar error as the host script returns it, offsets in UTF-16 code units.
#[derive(Deserialize)]
struct JsError {
    start: usize,
    end: usize,
    rule_id: String,
    kind: String,
    suggestions: Vec<String>,
}

/// Index in code points of each UTF-16 offset of `text`, up to its length included.
/// Example: "a😀b" (the emoji takes two UTF-16 units) gives [0, 1, 1, 2, 3], so the
/// UTF-16 span [3, 4] of "b" is the code point span [2, 3].
fn utf16_to_chars(text: &str) -> Vec<usize> {
    let mut map = Vec::with_capacity(text.len() + 1);
    for (index, c) in text.chars().enumerate() {
        map.extend(std::iter::repeat_n(index, c.len_utf16()));
    }
    map.push(text.chars().count());
    map
}

fn to_chars(map: &[usize], offset: usize) -> Result<usize, String> {
    map.get(offset)
        .copied()
        .ok_or_else(|| format!("Grammalecte offset {offset} is past the text"))
}

impl Checker for Grammalecte {
    fn unknown_words(&mut self, text: &str) -> Result<Vec<(usize, usize)>, String> {
        let spans: Vec<(usize, usize)> = self.call("plumedUnknownWords", text)?;
        let map = utf16_to_chars(text);
        spans
            .into_iter()
            .map(|(start, end)| Ok((to_chars(&map, start)?, to_chars(&map, end)?)))
            .collect()
    }

    fn grammar_errors(&mut self, text: &str) -> Result<Vec<GcError>, String> {
        let errors: Vec<JsError> = self.call("plumedGrammarErrors", text)?;
        let map = utf16_to_chars(text);
        errors
            .into_iter()
            .map(|e| {
                Ok(GcError {
                    start: to_chars(&map, e.start)?,
                    end: to_chars(&map, e.end)?,
                    rule_id: e.rule_id,
                    kind: e.kind,
                    suggestions: e.suggestions,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offsets_map_to_code_points() {
        assert_eq!(utf16_to_chars("a😀b"), vec![0, 1, 1, 2, 3]);
        assert_eq!(utf16_to_chars("été"), vec![0, 1, 2, 3]);
        assert_eq!(utf16_to_chars(""), vec![0]);
    }

    #[test]
    fn the_file_list_has_the_engine_and_its_data() {
        let js: Vec<&str> = listed_files()
            .filter(|(k, _)| *k == "js")
            .map(|(_, p)| p)
            .collect();
        assert_eq!(js.first(), Some(&"grammalecte/graphspell/helpers.js"));
        assert_eq!(js.last(), Some(&"grammalecte/fr/gc_engine.js"));
        assert_eq!(listed_files().filter(|(k, _)| *k == "data").count(), 4);
    }
}
