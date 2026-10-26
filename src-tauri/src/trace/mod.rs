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

const CHANNEL_CAPACITY: usize = 1024;
pub const ARGS_CAP: usize = 2_000;
pub const PROMPT_CAP: usize = 8_000;
pub const EVENT_CAP: usize = 2_000;
const TEXT_CAP: usize = 2_000;
const BATCH: usize = 64;
const WARN_EVERY: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunInfo {
    pub id: String,
    pub scope_id: Option<i64>,
    pub task: String,
    pub source: String,
    pub started_utc: String,
    pub plan_json: Option<String>,
    pub model: Option<String>,
    pub app_version: Option<String>,
    pub settings_json: Option<String>,
    pub session_dir: Option<String>,
}

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
    pub gate: Option<String>,
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
    pub served_model: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VerdictRecord {
    pub run_id: String,
    pub mode: String,
    pub result: String,
    pub evidence: Option<String>,
    pub error: Option<String>,
    pub model: Option<String>,
    pub latency_ms: Option<i64>,
    pub claimed_success: bool,
    pub challenged: bool,
    pub created_utc: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallRecord {
    pub run_id: String,
    pub step: Option<i64>,
    pub kind: String,
    pub attempt: i64,
    pub started_utc: String,
    pub duration_ms: i64,
    pub wait_before_ms: i64,
    pub status: Option<i64>,
    pub outcome: String,
    pub error: Option<String>,
    pub request_json: String,
    pub response_json: String,
    pub truncated: bool,
    pub model: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub served_model: Option<String>,
    pub locality: Option<String>,
    pub swap_ms: Option<i64>,
}

pub const BODY_CAP: usize = 64 * 1024;

pub fn redact(value: &serde_json::Value) -> (String, bool) {
    fn walk(value: &serde_json::Value, key: &str) -> serde_json::Value {
        use serde_json::Value;
        match value {
            Value::String(text) if text.starts_with("data:") => {
                let bytes = text.len();
                Value::String(format!("[image {bytes} bytes]"))
            }
            Value::String(text) if key == "data" && text.len() > 256 => {
                Value::String(format!("[audio {} bytes]", text.len()))
            }
            Value::String(_) if is_secret_key(key) => Value::String("[removed]".into()),
            Value::Object(map) => {
                Value::Object(map.iter().map(|(k, v)| (k.clone(), walk(v, k))).collect())
            }
            Value::Array(items) => Value::Array(items.iter().map(|v| walk(v, key)).collect()),
            other => other.clone(),
        }
    }
    fn is_secret_key(key: &str) -> bool {
        let key = key.to_ascii_lowercase();
        [
            "authorization",
            "api_key",
            "apikey",
            "api-key",
            "token",
            "secret",
            "password",
        ]
        .iter()
        .any(|word| key.contains(word))
    }
    let text = walk(value, "").to_string();
    if text.len() <= BODY_CAP {
        return (text, false);
    }
    let mut end = BODY_CAP;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

pub struct CallLog {
    recorder: Recorder,
    run_id: String,
    context: Mutex<(String, Option<i64>)>,
}

impl CallLog {
    pub fn new(recorder: &Recorder, run_id: &str, kind: &str) -> Self {
        Self {
            recorder: recorder.clone(),
            run_id: run_id.to_string(),
            context: Mutex::new((kind.to_string(), None)),
        }
    }

    pub fn set(&self, kind: &str, step: Option<u32>) {
        if let Ok(mut context) = self.context.lock() {
            *context = (kind.to_string(), step.map(i64::from));
        }
    }
}

impl crate::model::attempts::AttemptSink for CallLog {
    fn record(&self, attempt: crate::model::attempts::Attempt) {
        let (kind, step) = self
            .context
            .lock()
            .map(|context| context.clone())
            .unwrap_or_default();
        let (request_json, request_cut) = redact(&attempt.request);
        let (response_json, response_cut) = redact(&attempt.response);
        self.recorder.call(CallRecord {
            run_id: self.run_id.clone(),
            step,
            kind,
            attempt: i64::from(attempt.attempt),
            started_utc: attempt.started_utc,
            duration_ms: i64::try_from(attempt.duration_ms).unwrap_or(i64::MAX),
            wait_before_ms: i64::try_from(attempt.wait_before_ms).unwrap_or(i64::MAX),
            status: attempt.status.map(i64::from),
            outcome: attempt.outcome.to_string(),
            error: attempt.error.map(|text| cap(&text, TEXT_CAP)),
            request_json,
            response_json,
            truncated: request_cut || response_cut,
            model: attempt.model,
            prompt_tokens: attempt.prompt_tokens.and_then(|n| i64::try_from(n).ok()),
            completion_tokens: attempt
                .completion_tokens
                .and_then(|n| i64::try_from(n).ok()),
            locality: attempt.locality,
            served_model: attempt.served_model,
            swap_ms: attempt.swap_ms.and_then(|n| i64::try_from(n).ok()),
        });
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReflectionRecord {
    pub id: String,
    pub started_utc: String,
    pub ended_utc: Option<String>,
    pub outcome: String,
    pub stages: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnalysisRecord {
    pub id: String,
    pub created_utc: String,
    pub question: String,
    pub answer: String,
    pub findings_json: String,
    pub model: Option<String>,
    pub latency_ms: Option<i64>,
    pub status: String,
    pub error: Option<String>,
    pub screenshot_path: Option<String>,
    pub monitor_json: Option<String>,
    pub boxes_rejected: i64,
}

pub const FINDINGS_CAP: usize = 20_000;

pub const EVIDENCE_CAP: usize = 400;

pub const STEP_COLUMNS: [&str; 24] = [
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
    "served_model",
];

impl StepRecord {
    pub fn new(run_id: &str, step: u32) -> Self {
        Self {
            run_id: run_id.to_string(),
            step,
            ..Self::default()
        }
    }

    pub fn values(&self) -> [Value; 24] {
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
            text(&self.served_model),
        ]
    }

    fn capped(mut self) -> Self {
        self.args_json = self.args_json.map(|text| cap(&text, ARGS_CAP));
        self.prompt_text = self.prompt_text.map(|text| cap(&text, PROMPT_CAP));
        self.reason = self.reason.map(|text| cap(&text, TEXT_CAP));
        self.error = self.error.map(|text| cap(&text, TEXT_CAP));
        self.risk_reasons_json = self.risk_reasons_json.map(|text| cap(&text, TEXT_CAP));
        self
    }
}

pub fn cap(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

pub fn now_utc() -> String {
    crate::archive::iso(crate::archive::now())
}

pub fn pointer_now() -> Option<[f64; 2]> {
    let (x, y) = crate::exec::input::cursor_position()?;
    let monitor = crate::observe::screenshot::monitor_at(x, y).ok()?;
    let (mx, my) = crate::model::coords::pixels_to_model(&monitor, x, y);
    Some([(mx * 10.0).round() / 10.0, (my * 10.0).round() / 10.0])
}

pub fn with_pointer(args_json: Option<&str>, pointer: Option<[f64; 2]>) -> Option<String> {
    let Some(pointer) = pointer else {
        return args_json.map(str::to_string);
    };
    let mut value: serde_json::Value = args_json
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("_pointer".into(), serde_json::json!(pointer));
    }
    Some(value.to_string())
}

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
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    u64::try_from(days * 86_400 + hour * 3_600 + minute * 60 + second).ok()
}

pub fn text_hash(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

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

pub fn model_and_snapshot(settings: &Mutex<crate::settings::Settings>) -> (String, String) {
    use crate::sync::LockExt;
    let settings = settings.locked();
    (
        settings
            .model_for_role(crate::model::role::ModelRole::Agent)
            .to_string(),
        settings_snapshot(&settings),
    )
}

pub fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("trace.sqlite3")
}

#[cfg(feature = "agent-bridge")]
pub fn db_path_without_app() -> Option<std::path::PathBuf> {
    crate::settings::path_without_app().and_then(|settings| settings.parent().map(db_path))
}

pub enum Msg {
    BeginConcurrentRun(Box<RunInfo>),
    TaskHistory {
        request_id: String,
        revision: i64,
        at_ms: i64,
        snapshot_json: String,
    },
    TaskLink {
        run_id: String,
        request_id: String,
        task_id: i64,
    },
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
    PurgeSessions(Vec<String>),
    Verdict(Box<VerdictRecord>),
    Analysis(Box<AnalysisRecord>),
    Reflection(Box<ReflectionRecord>),
    Call(Box<CallRecord>),
    Flush(std::sync::mpsc::Sender<()>),
}

#[derive(Clone)]
pub struct Recorder {
    tx: Option<SyncSender<Msg>>,
    path: Option<std::path::PathBuf>,
    dropped: Arc<AtomicU64>,
    open: Arc<Mutex<Option<String>>>,
    concurrent: Arc<Mutex<std::collections::HashSet<String>>>,
    deleted: Arc<Mutex<std::collections::HashSet<String>>>,
}

static LAST_WARNING: Mutex<Option<Instant>> = Mutex::new(None);

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
    pub fn noop() -> Self {
        Self {
            tx: None,
            path: None,
            dropped: Arc::default(),
            open: Arc::default(),
            concurrent: Arc::default(),
            deleted: Arc::default(),
        }
    }

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
                path: Some(path.to_path_buf()),
                dropped: Arc::default(),
                open: Arc::default(),
                concurrent: Arc::default(),
                deleted: Arc::default(),
            },
            Err(err) => {
                log::error!("could not start the trace writer: {err}");
                Self::noop()
            }
        }
    }

    pub fn db_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    fn send(&self, message: Msg) {
        let deleted = self
            .deleted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = match &message {
            Msg::BeginRun(info) | Msg::BeginConcurrentRun(info) => Some(info.id.as_str()),
            Msg::TaskHistory { request_id, .. } => Some(request_id.as_str()),
            Msg::SetPlan { run_id, .. }
            | Msg::TaskLink { run_id, .. }
            | Msg::Event { run_id, .. }
            | Msg::EndRun { run_id, .. } => Some(run_id.as_str()),
            Msg::Step(row) => Some(row.run_id.as_str()),
            Msg::Verdict(row) => Some(row.run_id.as_str()),
            Msg::Call(row) => Some(row.run_id.as_str()),
            Msg::Analysis(row) => Some(row.id.as_str()),
            Msg::Reflection(row) => Some(row.id.as_str()),
            _ => None,
        };
        if id.is_some_and(|id| deleted.contains(id)) {
            return;
        }
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

    pub fn begin_background_run(&self, info: RunInfo) {
        self.concurrent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(info.id.clone());
        self.send(Msg::BeginConcurrentRun(Box::new(RunInfo {
            task: cap(&info.task, TEXT_CAP),
            ..info
        })));
    }

    pub fn task_history(
        &self,
        request_id: &str,
        revision: u64,
        at_ms: u64,
        snapshot: &serde_json::Value,
    ) {
        let snapshot_json = snapshot.to_string();
        if snapshot_json.len() > 1_048_576 {
            warn_rarely(|| "Task history snapshot exceeded its size bound; not recorded".into());
            return;
        }
        self.send(Msg::TaskHistory {
            request_id: request_id.into(),
            revision: revision.min(i64::MAX as u64) as i64,
            at_ms: at_ms.min(i64::MAX as u64) as i64,
            snapshot_json,
        });
    }

    pub fn task_link(&self, run_id: &str, request_id: &str, task_id: u64) {
        self.send(Msg::TaskLink {
            run_id: run_id.into(),
            request_id: request_id.into(),
            task_id: task_id.min(i64::MAX as u64) as i64,
        });
    }

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

    pub fn end_run(&self, run_id: &str, outcome: &str, summary: &str) {
        self.concurrent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(run_id);
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

    pub fn verdict(&self, record: VerdictRecord) {
        let record = VerdictRecord {
            evidence: record.evidence.map(|text| cap(&text, EVIDENCE_CAP)),
            error: record.error.map(|text| cap(&text, TEXT_CAP)),
            ..record
        };
        self.send(Msg::Verdict(Box::new(record)));
    }

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

    pub fn call(&self, record: CallRecord) {
        self.send(Msg::Call(Box::new(record)));
    }

    pub fn reflection(&self, record: ReflectionRecord) {
        self.send(Msg::Reflection(Box::new(record)));
    }

    pub fn call_log(&self, run_id: &str, kind: &str) -> CallLog {
        CallLog::new(self, run_id, kind)
    }

    pub fn purge_sessions(&self, session_dirs: Vec<String>) {
        self.send(Msg::PurgeSessions(session_dirs));
    }

    pub fn sync(&self) -> Result<(), String> {
        let (done, wait) = std::sync::mpsc::channel();
        self.tx
            .as_ref()
            .ok_or("Session history is unavailable")?
            .send(Msg::Flush(done))
            .map_err(|_| "Session history writer is closed")?;
        wait.recv()
            .map_err(|_| "Session history writer stopped".into())
    }

    pub fn block_session(&self, id: &str) -> Result<(), String> {
        if self.is_active(id) {
            return Err("Finish or stop this session before deleting it".into());
        }
        let inserted = self
            .deleted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id.to_string());
        if !inserted {
            return Err("This session is already deleted or being deleted".into());
        }
        if let Err(error) = self.sync() {
            self.unblock_session(id);
            return Err(error);
        }
        Ok(())
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.open_run().as_deref() == Some(id)
            || self
                .concurrent
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains(id)
    }

    pub fn unblock_session(&self, id: &str) {
        self.deleted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id);
    }

    pub fn user_prompt(&self, id: &str, text: &str) {
        let complete = text.chars().count() <= PROMPT_CAP;
        let text = if complete {
            text.to_string()
        } else {
            cap(text, PROMPT_CAP)
        };
        self.send(Msg::Event {
            run_id: id.to_string(),
            at_utc: now_utc(),
            kind: "user_prompt".into(),
            data_json: serde_json::json!({"text":text,"complete":complete}).to_string(),
        });
    }

    #[cfg(test)]
    pub(crate) fn flush(&self) {
        self.sync().expect("the writer answers");
    }

    #[cfg(test)]
    fn undrained(capacity: usize) -> (Self, Receiver<Msg>) {
        let (tx, rx) = sync_channel(capacity);
        (
            Self {
                tx: Some(tx),
                path: None,
                dropped: Arc::default(),
                open: Arc::default(),
                concurrent: Arc::default(),
                deleted: Arc::default(),
            },
            rx,
        )
    }
}

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
        for message in batch {
            if let Msg::Flush(done) = message {
                let _ = done.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests;
