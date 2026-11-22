//! The thread that owns Grammalecte: QuickJS is single-threaded, and loading takes about a
//! second, so one thread loads it once (lazily, never on the main thread) and keeps it in
//! memory; dictations send it their text and wait for the answer. Written with Claude Code.

use super::engine::{Grammalecte, JS_STACK_BYTES};
use super::mask;
use super::procedure::{self, Config};
use super::rules::{RuleSet, DATA};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// How long a dictation waits for the answer, loading included, before it goes on
/// without the grammar step.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(20);

struct Job {
    text: String,
    rules: RuleSet,
    reply: Sender<Result<String, String>>,
}

static WORKER: OnceLock<Sender<Job>> = OnceLock::new();

fn run(dir: PathBuf, jobs: Receiver<Job>) {
    let started = Instant::now();
    let loaded = mask::compile(&DATA.masks)
        .and_then(|masks| Grammalecte::load(&dir).map(|engine| (engine, masks)));
    match &loaded {
        Ok(_) => log::info!("Grammalecte loaded in {:?}", started.elapsed()),
        Err(e) => log::warn!("Grammalecte could not load, grammar step off: {e}"),
    }
    let skip_types: HashSet<String> = DATA.skip_types.iter().cloned().collect();
    let mut loaded = loaded;
    for job in jobs {
        let answer = match &mut loaded {
            Ok((engine, masks)) => {
                let config = Config {
                    rules: &job.rules,
                    masks,
                    skip_types: &skip_types,
                };
                procedure::correct(&job.text, engine, &config)
            }
            Err(e) => Err(e.clone()),
        };
        // The dictation may have stopped waiting.
        let _ = job.reply.send(answer);
    }
}

/// The worker's queue, started on first use with Grammalecte from `dir`.
fn worker(dir: impl FnOnce() -> Option<PathBuf>) -> &'static Sender<Job> {
    WORKER.get_or_init(|| {
        let (sender, jobs) = mpsc::channel();
        match dir() {
            Some(dir) => {
                let spawned = std::thread::Builder::new()
                    .name("grammalecte".to_string())
                    .stack_size(2 * JS_STACK_BYTES)
                    .spawn(move || run(dir, jobs));
                if let Err(e) = spawned {
                    log::warn!("Could not start the grammar thread: {e}");
                }
            }
            None => log::warn!("Grammalecte not found in the resources, grammar step off"),
        }
        sender
    })
}

/// Starts loading Grammalecte in the background, if not done yet.
pub fn warm_up(dir: impl FnOnce() -> Option<PathBuf>) {
    worker(dir);
}

/// `text` with the fixes of `rules` applied, waiting for Grammalecte to load if needed.
pub fn correct(
    dir: impl FnOnce() -> Option<PathBuf>,
    text: &str,
    rules: RuleSet,
) -> Result<String, String> {
    let (reply, answer) = mpsc::channel();
    let job = Job {
        text: text.to_string(),
        rules,
        reply,
    };
    worker(dir)
        .send(job)
        .map_err(|_| "Grammalecte is not available".to_string())?;
    answer
        .recv_timeout(ANSWER_TIMEOUT)
        .map_err(|e| format!("No answer from Grammalecte: {e}"))?
}
