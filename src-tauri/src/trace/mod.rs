//! The flight recorder: every run, built-in or by an outside agent, leaves a queryable record in
//! one SQLite file in the app's data folder (`trace.sqlite3`): what was asked, each step (what the
//! model saw, what it chose, what the gate decided, what happened, how long each phase took) and
//! how the run ended. Screenshots stay files in the session folders; rows keep their paths.
//!
//! The recorder must never slow a step, block the agent loop or fail a task (the opposite of the
//! screenshot archive, which fails closed on purpose). So the database belongs to one writer
//! thread, the rest of the app sends it small messages with `try_send`, and a message that cannot
//! be queued is dropped and counted. A recorder that could not open its file is a no-op.

pub mod schema;
pub mod store;

use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
    },
    time::{Duration, Instant},
};

use rusqlite::{Connection, types::Value};

/// Messages waiting for the writer. A full queue drops new messages.
const CHANNEL_CAPACITY: usize = 1024;
/// Longest text of each kind kept in a row (characters).
pub const ARGS_CAP: usize = 2_000;
pub const PROMPT_CAP: usize = 8_000;
pub const EVENT_CAP: usize = 2_000;
const TEXT_CAP: usize = 2_000;
/// How many queued messages the writer puts in one transaction.
const BATCH: usize = 64;
/// A warning about dropped or failed writes is logged at most this often.
const WARN_EVERY: Duration = Duration::from_secs(60);

/// What the settings snapshot, the model and the app version look like is the caller's business;
/// this is only the shape of a run's row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunInfo {
    /// The archive session id, or `norun-<scope id>-<unix seconds>` when no session folder exists.
    pub id: String,
    pub scope_id: Option<i64>,
    pub task: String,
    /// "built-in" or the outside agent's name.
    pub source: String,
    pub started_utc: String,
    pub plan_json: Option<String>,
    pub model: Option<String>,
    pub app_version: Option<String>,
    /// A snapshot of a few settings; never the key, the base URL or the hotkeys.
    pub settings_json: Option<String>,
    pub session_dir: Option<String>,
}

/// One step of a run. Everything but the key is optional: the loop may send a step in pieces (the
/// decision first, the result later) and each piece only overwrites what it carries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepRecord {
    pub run_id: String,
    pub step: u32,
    pub started_utc: Option<String>,
    pub goal: Option<i64>,
    pub tool: Option<String>,
    pub args_json: Option<String>,
    pub reason: Option<String>,
    pub risk_stated: Option<String>,
    pub risk_final: Option<String>,
    pub risk_reasons_json: Option<String>,
    /// `allowed`, `approved`, `declined`, `no answer in time`, `stopped`, or `none`.
    pub gate: Option<String>,
    /// `ok`, `failed`, `cancelled` or `skipped`.
    pub result: Option<String>,
    pub error: Option<String>,
    pub observation_path: Option<String>,
    pub elements_hash: Option<String>,
    pub prompt_text: Option<String>,
    pub thinking_chars: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub observe_ms: Option<i64>,
    pub screen_check_ms: Option<i64>,
    pub finish_check_ms: Option<i64>,
    pub decide_ms: Option<i64>,
    pub gate_ms: Option<i64>,
    pub act_ms: Option<i64>,
    pub settle_ms: Option<i64>,
}

/// The independent check of a finished run (`agent::verify`). One per run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VerdictRecord {
    pub run_id: String,
    /// `background` or `blocking`.
    pub mode: String,
    /// `verified`, `contradicted`, `unsure`, `error` or `skipped`.
    pub result: String,
    /// One short sentence naming what the checker saw on the screen (cut to 400 characters).
    pub evidence: Option<String>,
    pub error: Option<String>,
    pub model: Option<String>,
    pub latency_ms: Option<i64>,
    /// Whether the agent claimed the task succeeded.
    pub claimed_success: bool,
    /// Blocking mode asked the agent to keep going because the check contradicted it.
    pub challenged: bool,
    pub created_utc: String,
}

/// One idle reflection run (`reflect`). The job is a stub in R5.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReflectionRecord {
    pub id: String,
    pub started_utc: String,
    pub ended_utc: Option<String>,
    /// `completed`, `preempted` or `failed`.
    pub outcome: String,
    /// How many stages it got through.
    pub stages: i64,
}

/// One visual analysis (`analysis`). The id is the archive session id of its folder.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalysisRecord {
    pub id: String,
    pub created_utc: String,
    pub question: String,
    pub answer: String,
    /// The findings as JSON (cut to `FINDINGS_CAP` characters).
    pub findings_json: String,
    pub model: Option<String>,
    pub latency_ms: Option<i64>,
    /// `ok`, `error` or `cancelled`.
    pub status: String,
    pub error: Option<String>,
    pub screenshot_path: Option<String>,
    pub monitor_json: Option<String>,
}

/// Longest findings JSON kept in an analysis row.
pub const FINDINGS_CAP: usize = 20_000;

/// Longest evidence text kept.
pub const EVIDENCE_CAP: usize = 400;

/// The names of `steps` columns other than the key, in the order `StepRecord::values` lists them.
pub const STEP_COLUMNS: [&str; 23] = [
    "started_utc",
    "goal",
    "tool",
    "args_json",
    "reason",
    "risk_stated",
    "risk_final",
    "risk_reasons_json",
    "gate",
    "result",
    "error",
    "observation_path",
    "elements_hash",
    "prompt_text",
    "thinking_chars",
    "completion_tokens",
    "observe_ms",
    "screen_check_ms",
    "finish_check_ms",
    "decide_ms",
    "gate_ms",
    "act_ms",
    "settle_ms",
];

impl StepRecord {
    pub fn new(run_id: &str, step: u32) -> Self {
        Self {
            run_id: run_id.to_string(),
            step,
            ..Self::default()
        }
    }

    /// The values of `STEP_COLUMNS`; `NULL` for what this piece does not carry.
    pub fn values(&self) -> [Value; 23] {
        let text = |value: &Option<String>| value.clone().map_or(Value::Null, Value::Text);
        let number = |value: &Option<i64>| value.map_or(Value::Null, Value::Integer);
        [
            text(&self.started_utc),
            number(&self.goal),
            text(&self.tool),
            text(&self.args_json),
            text(&self.reason),
            text(&self.risk_stated),
            text(&self.risk_final),
            text(&self.risk_reasons_json),
            text(&self.gate),
            text(&self.result),
            text(&self.error),
            text(&self.observation_path),
            text(&self.elements_hash),
            text(&self.prompt_text),
            number(&self.thinking_chars),
            number(&self.completion_tokens),
            number(&self.observe_ms),
            number(&self.screen_check_ms),
            number(&self.finish_check_ms),
            number(&self.decide_ms),
            number(&self.gate_ms),
            number(&self.act_ms),
            number(&self.settle_ms),
        ]
    }

    /// Cuts the free-text fields to their limits.
    fn capped(mut self) -> Self {
        self.args_json = self.args_json.map(|text| cap(&text, ARGS_CAP));
        self.prompt_text = self.prompt_text.map(|text| cap(&text, PROMPT_CAP));
        self.reason = self.reason.map(|text| cap(&text, TEXT_CAP));
        self.error = self.error.map(|text| cap(&text, TEXT_CAP));
        self.risk_reasons_json = self.risk_reasons_json.map(|text| cap(&text, TEXT_CAP));
        self
    }
}

/// `text` cut to at most `max` characters; a cut text ends with `…`.
pub fn cap(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// `2026-10-19T07:59:12Z` for now.
pub fn now_utc() -> String {
    crate::archive::iso(crate::archive::now())
}

/// Unix seconds for `2026-10-19T07:59:12Z`.
pub fn parse_iso(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let number = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    u64::try_from(days * 86_400 + hour * 3_600 + minute * 60 + second).ok()
}

/// A short, stable fingerprint of a text (FNV-1a, 64 bits, in hex) for noticing that two control
/// lists are the same without storing either. Not for security.
pub fn text_hash(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The few settings a run's row snapshots. Never the API key, the base URL or the hotkeys.
pub fn settings_snapshot(settings: &crate::settings::Settings) -> String {
    serde_json::json!({
        "max_steps": settings.max_steps,
        "show_thinking": settings.show_thinking,
        "ask_every_action": settings.ask_every_action,
        "fast_checks": settings.fast_checks,
        "verify_mode": settings.verify_mode,
    })
    .to_string()
}

/// The model name and the settings snapshot a run's row records, read under **one** lock. Two
/// `locked()` calls in one statement would hold two guards of the same mutex at once and the second
/// would wait for the first forever (the freeze of every task start, found in the field test).
pub fn model_and_snapshot(settings: &Mutex<crate::settings::Settings>) -> (String, String) {
    use crate::sync::LockExt;
    let settings = settings.locked();
    (settings.model.clone(), settings_snapshot(&settings))
}

/// Where the trace database lives, given the app's data folder.
pub fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("trace.sqlite3")
}

/// The database found without a running app (for the command line); the same folder as the
/// settings file.
#[cfg(feature = "agent-bridge")]
pub fn db_path_without_app() -> Option<std::path::PathBuf> {
    crate::settings::path_without_app().and_then(|settings| settings.parent().map(db_path))
}

/// What the writer thread is asked to do.
pub enum Msg {
    BeginRun(Box<RunInfo>),
    SetPlan {
        run_id: String,
        plan_json: String,
    },
    Step(Box<StepRecord>),
    Event {
        run_id: String,
        at_utc: String,
        kind: String,
        data_json: String,
    },
    EndRun {
        run_id: String,
        ended_utc: String,
        outcome: String,
        summary: String,
    },
    /// Delete the runs whose session folder is one of these (retention), with their steps and events.
    PurgeSessions(Vec<String>),
    Verdict(Box<VerdictRecord>),
    Analysis(Box<AnalysisRecord>),
    Reflection(Box<ReflectionRecord>),
    /// Answered once everything queued before it has been written.
    #[cfg(test)]
    Flush(std::sync::mpsc::Sender<()>),
}

/// A cheap-to-clone handle for recording. Every method returns at once and never fails.
#[derive(Clone)]
pub struct Recorder {
    tx: Option<SyncSender<Msg>>,
    dropped: Arc<AtomicU64>,
    /// The run that has begun and not ended, so Stop and the idle timer can end it without knowing its id.
    open: Arc<Mutex<Option<String>>>,
}

static LAST_WARNING: Mutex<Option<Instant>> = Mutex::new(None);

/// Logs `message` as a warning unless one was logged in the last minute.
pub(crate) fn warn_rarely(message: impl FnOnce() -> String) {
    let mut last = LAST_WARNING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if last.is_none_or(|at| at.elapsed() >= WARN_EVERY) {
        *last = Some(Instant::now());
        log::warn!("{}", message());
    }
}

impl Recorder {
    /// A recorder that records nothing (the database could not be opened).
    pub fn noop() -> Self {
        Self {
            tx: None,
            dropped: Arc::default(),
            open: Arc::default(),
        }
    }

    /// Opens (creating if needed) the database at `path` and starts its writer thread. If that
    /// cannot be done the error is logged and the recorder does nothing.
    pub fn start(path: &Path) -> Self {
        Self::start_with(path, CHANNEL_CAPACITY)
    }

    fn start_with(path: &Path, capacity: usize) -> Self {
        let opened = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .map_err(|err| err.to_string())
            .and_then(|()| Connection::open(path).map_err(|err| err.to_string()))
            .and_then(|db| {
                schema::open(&db)
                    .map(|opened| (db, opened))
                    .map_err(|err| err.to_string())
            });
        let db = match opened {
            Ok((db, schema::Opened::Ready)) => db,
            Ok((_, schema::Opened::Newer(version))) => {
                log::warn!(
                    "{} was written by a newer Rune Studio (schema {version}); nothing will be recorded",
                    path.display()
                );
                return Self::noop();
            }
            Err(err) => {
                log::error!(
                    "could not open the trace database {}: {err}; runs will not be recorded",
                    path.display()
                );
                return Self::noop();
            }
        };
        let (tx, rx) = sync_channel(capacity);
        match std::thread::Builder::new()
            .name("trace-writer".into())
            .spawn(move || writer(db, rx))
        {
            Ok(_) => Self {
                tx: Some(tx),
                dropped: Arc::default(),
                open: Arc::default(),
            },
            Err(err) => {
                log::error!("could not start the trace writer: {err}");
                Self::noop()
            }
        }
    }

    /// How many messages were dropped because the queue was full or the writer was gone.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    fn send(&self, message: Msg) {
        let Some(tx) = &self.tx else { return };
        if tx.try_send(message).is_err() {
            let total = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
            warn_rarely(|| {
                format!("the trace queue is full or closed; {total} record(s) dropped so far")
            });
        }
    }

    fn open_run(&self) -> std::sync::MutexGuard<'_, Option<String>> {
        self.open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The id of the run that has begun and not ended, if any.
    pub fn open_run_id(&self) -> Option<String> {
        self.open_run().clone()
    }

    pub fn begin_run(&self, info: RunInfo) {
        let info = RunInfo {
            task: cap(&info.task, TEXT_CAP),
            ..info
        };
        *self.open_run() = Some(info.id.clone());
        self.send(Msg::BeginRun(Box::new(info)));
    }

    /// Ends whichever run is open (Stop, the idle timer, an outside agent's `rune_done`). Does
    /// nothing when none is.
    pub fn end_open_run(&self, outcome: &str, summary: &str) {
        let open = self.open_run().take();
        if let Some(id) = open {
            self.end_run(&id, outcome, summary);
        }
    }

    pub fn set_plan(&self, run_id: &str, plan_json: &str) {
        self.send(Msg::SetPlan {
            run_id: run_id.to_string(),
            plan_json: cap(plan_json, TEXT_CAP),
        });
    }

    /// Adds a piece of a step: only the fields it carries are written, the rest are kept.
    pub fn step(&self, record: StepRecord) {
        self.send(Msg::Step(Box::new(record.capped())));
    }

    pub fn event(&self, run_id: &str, kind: &str, data: &str) {
        self.send(Msg::Event {
            run_id: run_id.to_string(),
            at_utc: now_utc(),
            kind: kind.to_string(),
            data_json: cap(data, EVENT_CAP),
        });
    }

    /// Ends a run. Only the first end counts, like the session folder's.
    pub fn end_run(&self, run_id: &str, outcome: &str, summary: &str) {
        let mut open = self.open_run();
        if open.as_deref() == Some(run_id) {
            *open = None;
        }
        drop(open);
        self.send(Msg::EndRun {
            run_id: run_id.to_string(),
            ended_utc: now_utc(),
            outcome: outcome.to_string(),
            summary: cap(summary, TEXT_CAP),
        });
    }

    /// Stores the check of a finished run (the newest one for a run replaces an older one).
    pub fn verdict(&self, record: VerdictRecord) {
        let record = VerdictRecord {
            evidence: record.evidence.map(|text| cap(&text, EVIDENCE_CAP)),
            error: record.error.map(|text| cap(&text, TEXT_CAP)),
            ..record
        };
        self.send(Msg::Verdict(Box::new(record)));
    }

    /// Stores one visual analysis (the newest one for an id replaces an older one).
    pub fn analysis(&self, record: AnalysisRecord) {
        let record = AnalysisRecord {
            question: cap(&record.question, TEXT_CAP),
            answer: cap(&record.answer, TEXT_CAP),
            findings_json: cap(&record.findings_json, FINDINGS_CAP),
            error: record.error.map(|text| cap(&text, TEXT_CAP)),
            ..record
        };
        self.send(Msg::Analysis(Box::new(record)));
    }

    /// Stores one reflection run (the newest write for an id replaces an older one).
    pub fn reflection(&self, record: ReflectionRecord) {
        self.send(Msg::Reflection(Box::new(record)));
    }

    pub fn purge_sessions(&self, session_dirs: Vec<String>) {
        self.send(Msg::PurgeSessions(session_dirs));
    }

    /// Waits until everything queued so far is written (tests only).
    #[cfg(test)]
    pub(crate) fn flush(&self) {
        let (done, wait) = std::sync::mpsc::channel();
        self.tx
            .as_ref()
            .expect("a started recorder")
            .send(Msg::Flush(done))
            .expect("the writer is alive");
        wait.recv_timeout(Duration::from_secs(10))
            .expect("the writer answers");
    }

    /// A recorder whose queue nobody reads (tests only), to see what a full queue does.
    #[cfg(test)]
    fn undrained(capacity: usize) -> (Self, Receiver<Msg>) {
        let (tx, rx) = sync_channel(capacity);
        (
            Self {
                tx: Some(tx),
                dropped: Arc::default(),
                open: Arc::default(),
            },
            rx,
        )
    }
}

/// The writer thread: owns the connection and applies messages in small transactions until every
/// sender is gone.
fn writer(mut db: Connection, rx: Receiver<Msg>) {
    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        while batch.len() < BATCH {
            match rx.try_recv() {
                Ok(next) => batch.push(next),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        if let Err(err) = store::write_batch(&mut db, &batch) {
            warn_rarely(|| format!("could not write to the trace database: {err}"));
        }
        #[cfg(test)]
        for message in batch {
            if let Msg::Flush(done) = message {
                let _ = done.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests;
