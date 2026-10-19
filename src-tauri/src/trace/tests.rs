use std::{
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};

use rusqlite::Connection;

use super::{store::*, *};

/// A fresh empty folder for one test's database, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!("rune-trace-test-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn db(&self) -> PathBuf {
        db_path(&self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(id: &str, started: &str) -> RunInfo {
    RunInfo { id: id.into(), scope_id: Some(7), task: "open notepad".into(), source: "built-in".into(), started_utc: started.into(), ..RunInfo::default() }
}

fn version_of(path: &std::path::Path) -> i64 {
    Connection::open(path).unwrap().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
}

fn tables_of(path: &std::path::Path) -> Vec<String> {
    let db = Connection::open(path).unwrap();
    let mut statement = db.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn a_new_database_is_created_at_the_current_version() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.flush();
    assert_eq!(version_of(&dir.db()), 3);
    assert_eq!(tables_of(&dir.db()), ["analyses", "events", "runs", "steps", "verdicts"]);
}

#[test]
fn opening_twice_is_idempotent() {
    let dir = TempDir::new();
    let first = Recorder::start(&dir.db());
    first.begin_run(run("r1", "2026-10-19T10:00:00Z"));
    first.flush();
    let second = Recorder::start(&dir.db());
    second.flush();
    assert_eq!(version_of(&dir.db()), 3);
    assert_eq!(list_runs(&dir.db(), 10).unwrap().len(), 1, "the second open keeps the data");
}

#[test]
fn a_newer_database_is_left_untouched_and_nothing_is_recorded() {
    let dir = TempDir::new();
    {
        let db = Connection::open(dir.db()).unwrap();
        db.execute_batch("CREATE TABLE future (x INTEGER); PRAGMA user_version = 5;").unwrap();
    }
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("r1", "2026-10-19T10:00:00Z"));
    assert!(recorder.tx.is_none(), "a newer schema means a no-op recorder");
    assert_eq!(version_of(&dir.db()), 5);
    assert_eq!(tables_of(&dir.db()), ["future"]);
    assert_eq!(list_runs(&dir.db(), 10), Err(StoreError::Newer(5)));
}

#[test]
fn a_run_with_steps_and_events_round_trips() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    let mut info = run("120000-1-open-notepad", "2026-10-19T12:00:00Z");
    info.model = Some("gemma".into());
    info.app_version = Some("0.1.0".into());
    info.settings_json = Some(r#"{"max_steps":30}"#.into());
    info.session_dir = Some("/docs/sessions/2026-10-19/120000-1-open-notepad".into());
    recorder.begin_run(info.clone());
    recorder.set_plan(&info.id, r#"["open","type"]"#);
    let mut step = StepRecord::new(&info.id, 1);
    step.tool = Some("click".into());
    step.gate = Some("allowed".into());
    step.result = Some("ok".into());
    step.decide_ms = Some(812);
    step.act_ms = Some(40);
    recorder.step(step);
    recorder.event(&info.id, "plan", r#"{"goals":2}"#);
    recorder.end_run(&info.id, "succeeded", "Done");
    recorder.flush();

    let detail = get_run(&dir.db(), &info.id).unwrap().expect("the run exists");
    assert_eq!(detail.run.info.task, "open notepad");
    assert_eq!(detail.run.info.scope_id, Some(7));
    assert_eq!(detail.run.info.model.as_deref(), Some("gemma"));
    assert_eq!(detail.run.info.plan_json.as_deref(), Some(r#"["open","type"]"#));
    assert_eq!(detail.run.info.session_dir, info.session_dir);
    assert_eq!(detail.run.outcome.as_deref(), Some("succeeded"));
    assert_eq!(detail.run.summary.as_deref(), Some("Done"));
    assert!(detail.run.ended_utc.is_some());
    assert_eq!(detail.steps.len(), 1);
    assert_eq!((detail.steps[0].step, detail.steps[0].tool.as_deref(), detail.steps[0].gate.as_deref()), (1, Some("click"), Some("allowed")));
    assert_eq!((detail.steps[0].decide_ms, detail.steps[0].act_ms, detail.steps[0].observe_ms), (Some(812), Some(40), None), "a phase that did not run stays NULL");
    assert_eq!(detail.events.len(), 1);
    assert_eq!((detail.events[0].kind.as_deref(), detail.events[0].data_json.as_deref()), (Some("plan"), Some(r#"{"goals":2}"#)));
    assert_eq!(get_run(&dir.db(), "missing").unwrap(), None);
}

#[test]
fn a_step_sent_in_two_pieces_is_one_row() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("r1", "2026-10-19T10:00:00Z"));
    let mut decision = StepRecord::new("r1", 3);
    decision.tool = Some("type_text".into());
    decision.reason = Some("fill the field".into());
    decision.decide_ms = Some(900);
    recorder.step(decision);
    let mut result = StepRecord::new("r1", 3);
    result.result = Some("failed".into());
    result.error = Some("no focus".into());
    result.act_ms = Some(12);
    recorder.step(result);
    recorder.flush();

    let steps = get_run(&dir.db(), "r1").unwrap().unwrap().steps;
    assert_eq!(steps.len(), 1);
    let row = &steps[0];
    assert_eq!((row.tool.as_deref(), row.reason.as_deref(), row.decide_ms), (Some("type_text"), Some("fill the field"), Some(900)), "the first piece survives");
    assert_eq!((row.result.as_deref(), row.error.as_deref(), row.act_ms), (Some("failed"), Some("no focus"), Some(12)), "the second piece is added");
}

#[test]
fn over_long_text_is_cut_to_its_limits() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("r1", "2026-10-19T10:00:00Z"));
    let mut step = StepRecord::new("r1", 1);
    step.args_json = Some("a".repeat(5_000));
    step.prompt_text = Some("é".repeat(20_000));
    recorder.step(step);
    recorder.event("r1", "note", &"n".repeat(9_000));
    recorder.flush();

    let detail = get_run(&dir.db(), "r1").unwrap().unwrap();
    let args = detail.steps[0].args_json.clone().unwrap();
    assert_eq!(args.chars().count(), ARGS_CAP);
    assert!(args.ends_with('…'));
    assert_eq!(detail.steps[0].prompt_text.clone().unwrap().chars().count(), PROMPT_CAP);
    assert_eq!(detail.events[0].data_json.clone().unwrap().chars().count(), EVENT_CAP);
    assert_eq!(cap("short", 10), "short");
    assert_eq!(cap("abcdef", 4), "abc…");
}

#[test]
fn a_full_queue_drops_at_once_and_counts() {
    let (recorder, queue) = Recorder::undrained(2);
    let started = Instant::now();
    for number in 0..5 {
        recorder.event("r1", "note", &number.to_string());
    }
    assert!(started.elapsed() < Duration::from_secs(1), "recording never waits for the writer");
    assert_eq!(recorder.dropped(), 3, "two fit in the queue, three were dropped");
    drop(queue);
    recorder.event("r1", "note", "after the writer is gone");
    assert_eq!(recorder.dropped(), 4, "a closed queue also counts");
    assert_eq!(Recorder::noop().dropped(), 0);
    Recorder::noop().event("r1", "note", "nothing happens");
}

#[test]
fn runs_are_listed_newest_first_up_to_the_limit() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("old", "2026-10-19T09:00:00Z"));
    recorder.begin_run(run("newer", "2026-10-19T10:00:00Z"));
    recorder.begin_run(run("newest", "2026-10-19T11:00:00Z"));
    recorder.step(StepRecord::new("newest", 1));
    recorder.step(StepRecord::new("newest", 2));
    recorder.flush();

    let all = list_runs(&dir.db(), 10).unwrap();
    assert_eq!(all.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(), ["newest", "newer", "old"]);
    assert_eq!(all[0].steps, 2);
    assert_eq!(all[1].steps, 0);
    assert_eq!(list_runs(&dir.db(), 2).unwrap().len(), 2);
    assert_eq!(list_runs(&dir.db(), 0).unwrap().len(), 0);
}

#[test]
fn only_the_first_end_counts_and_a_new_run_replaces_an_open_one() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("a", "2026-10-19T09:00:00Z"));
    recorder.end_run("a", "stopped", "");
    recorder.end_run("a", "succeeded", "late");
    recorder.begin_run(run("b", "2026-10-19T10:00:00Z"));
    recorder.begin_run(run("c", "2026-10-19T11:00:00Z"));
    recorder.end_run("b", "failed", "late end of a replaced run");
    recorder.flush();

    let outcome = |id: &str| get_run(&dir.db(), id).unwrap().unwrap().run.outcome;
    assert_eq!(outcome("a").as_deref(), Some("stopped"));
    assert_eq!(outcome("b").as_deref(), Some("replaced"));
    assert_eq!(outcome("c"), None, "the newest run is still open");
}

#[test]
fn a_step_for_a_run_whose_beginning_was_dropped_is_still_kept() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.step(StepRecord::new("lost", 1));
    recorder.begin_run(run("lost", "2026-10-19T10:00:00Z"));
    recorder.flush();
    let detail = get_run(&dir.db(), "lost").unwrap().unwrap();
    assert_eq!(detail.steps.len(), 1);
    assert_eq!(detail.run.info.task, "open notepad", "the late beginning fills the bare row");
}

#[test]
fn reading_while_the_writer_is_writing_works() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("busy", "2026-10-19T10:00:00Z"));
    recorder.flush();
    for step in 1..=300 {
        recorder.step(StepRecord::new("busy", step));
        if step % 25 == 0 {
            let listed = list_runs(&dir.db(), 5).expect("a read during writes succeeds");
            assert_eq!(listed.len(), 1);
            assert!(get_run(&dir.db(), "busy").expect("so does a full read").is_some());
        }
    }
    recorder.flush();
    assert_eq!(list_runs(&dir.db(), 5).unwrap()[0].steps, 300);
    assert_eq!(recorder.dropped(), 0);
}

#[test]
fn purging_a_session_removes_its_runs_steps_and_events_only() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    for (id, folder) in [("old", "/s/2026-01-01/a"), ("keep", "/s/2026-10-19/b")] {
        let mut info = run(id, "2026-10-19T10:00:00Z");
        info.session_dir = Some(folder.into());
        recorder.begin_run(info);
        recorder.step(StepRecord::new(id, 1));
        recorder.event(id, "note", "x");
    }
    recorder.purge_sessions(vec!["/s/2026-01-01/a".into()]);
    recorder.flush();
    assert_eq!(get_run(&dir.db(), "old").unwrap(), None);
    let kept = get_run(&dir.db(), "keep").unwrap().unwrap();
    assert_eq!((kept.steps.len(), kept.events.len()), (1, 1));
}

#[test]
fn a_missing_file_is_reported_not_created() {
    let dir = TempDir::new();
    assert_eq!(list_runs(&dir.db(), 5), Err(StoreError::NotFound));
    assert!(!dir.db().exists(), "reading never creates the database");
}

#[test]
fn iso_times_parse_back_to_the_seconds_they_came_from() {
    for seconds in [0_u64, 1, 86_399, 86_400, 951_782_400, 1_760_000_000, 1_781_870_352] {
        assert_eq!(parse_iso(&crate::archive::iso(seconds)), Some(seconds));
    }
    assert_eq!(parse_iso("2026-10-19"), None);
    assert_eq!(parse_iso("2026-13-19T00:00:00Z"), None);
    assert_eq!(parse_iso("not a time at all!"), None);
}

/// The bundled SQLite builds and works: open in memory, create, insert, read.
#[test]
fn sqlite_opens_and_stores_a_row() {
    let db = Connection::open_in_memory().unwrap();
    db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)", []).unwrap();
    db.execute("INSERT INTO t (name) VALUES (?1)", ["hello"]).unwrap();
    let name: String = db.query_row("SELECT name FROM t WHERE id = 1", [], |row| row.get(0)).unwrap();
    assert_eq!(name, "hello");
}

#[test]
fn the_text_hash_is_fnv_1a_and_tells_texts_apart() {
    assert_eq!(text_hash(""), "cbf29ce484222325");
    assert_eq!(text_hash("a"), "af63dc4c8601ec8c");
    assert_ne!(text_hash("e1 button OK"), text_hash("e1 button Cancel"));
    assert_eq!(text_hash("e1 button OK"), text_hash("e1 button OK"));
}

#[test]
fn the_settings_snapshot_has_only_the_five_settings() {
    let settings = crate::settings::Settings { max_steps: 12, ask_every_action: true, ..crate::settings::Settings::default() };
    let snapshot: serde_json::Value = serde_json::from_str(&settings_snapshot(&settings)).unwrap();
    let mut keys: Vec<&str> = snapshot.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["ask_every_action", "fast_checks", "max_steps", "show_thinking", "verify_mode"]);
    assert_eq!(snapshot["verify_mode"], "background", "the default mode is stored with every run");
    assert_eq!(snapshot["max_steps"], 12);
    assert_eq!(snapshot["ask_every_action"], true);
}

#[test]
fn the_open_run_is_tracked_so_stop_can_end_it() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    assert_eq!(recorder.open_run_id(), None);
    recorder.end_open_run("stopped", "nothing is open: nothing happens");
    recorder.begin_run(run("a", "2026-10-19T09:00:00Z"));
    assert_eq!(recorder.open_run_id().as_deref(), Some("a"));
    recorder.end_open_run("stopped", "");
    assert_eq!(recorder.open_run_id(), None);
    recorder.begin_run(run("b", "2026-10-19T10:00:00Z"));
    recorder.end_run("a", "failed", "a late end of another run leaves b open");
    assert_eq!(recorder.open_run_id().as_deref(), Some("b"));
    recorder.end_run("b", "succeeded", "Done");
    assert_eq!(recorder.open_run_id(), None);
    recorder.flush();
    let outcome = |id: &str| get_run(&dir.db(), id).unwrap().unwrap().run.outcome;
    assert_eq!((outcome("a").as_deref(), outcome("b").as_deref()), (Some("stopped"), Some("succeeded")));
}

fn verdict(run_id: &str, mode: &str, result: &str) -> VerdictRecord {
    VerdictRecord { run_id: run_id.into(), mode: mode.into(), result: result.into(), claimed_success: true, created_utc: "2026-10-19T12:01:00Z".into(), ..VerdictRecord::default() }
}

#[test]
fn a_version_1_database_is_migrated_without_losing_a_run() {
    let dir = TempDir::new();
    {
        // A database exactly as the first release of the recorder made it.
        let db = Connection::open(dir.db()).unwrap();
        db.execute_batch(&format!("{} PRAGMA user_version = 1;", schema::CREATE_V1)).unwrap();
        db.execute("INSERT INTO runs (id, task, source, started_utc, outcome) VALUES ('old', 'open notepad', 'built-in', '2026-10-01T09:00:00Z', 'succeeded')", []).unwrap();
        db.execute("INSERT INTO steps (run_id, step, tool, result) VALUES ('old', 1, 'click', 'ok')", []).unwrap();
    }
    // Read before the app has opened it: no verdicts table yet, and that is fine.
    let before = get_run(&dir.db(), "old").unwrap().expect("the old run reads");
    assert_eq!((before.steps.len(), before.verdict.clone()), (1, None));
    assert_eq!(list_runs(&dir.db(), 5).unwrap()[0].verdict, None);

    let recorder = Recorder::start(&dir.db());
    recorder.flush();
    assert_eq!(version_of(&dir.db()), 3);
    assert!(tables_of(&dir.db()).contains(&"verdicts".to_string()));
    let after = get_run(&dir.db(), "old").unwrap().expect("and still reads");
    assert_eq!((after.run.info.task.as_str(), after.steps.len(), after.verdict), ("open notepad", 1, None), "a run without a verdict reads as before");
    recorder.verdict(verdict("old", "background", "verified"));
    recorder.flush();
    assert_eq!(get_run(&dir.db(), "old").unwrap().unwrap().verdict.unwrap().result, "verified");
}

#[test]
fn a_verdict_round_trips_and_shows_in_the_list() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("with", "2026-10-19T10:00:00Z"));
    recorder.begin_run(run("without", "2026-10-19T11:00:00Z"));
    recorder.verdict(VerdictRecord {
        evidence: Some("The Notepad window shows hello rune.".into()),
        error: None,
        model: Some("gemma".into()),
        latency_ms: Some(4_200),
        challenged: true,
        ..verdict("with", "blocking", "contradicted")
    });
    recorder.flush();
    let detail = get_run(&dir.db(), "with").unwrap().unwrap();
    let stored = detail.verdict.expect("the verdict is stored");
    assert_eq!(
        stored,
        VerdictRecord {
            run_id: "with".into(),
            mode: "blocking".into(),
            result: "contradicted".into(),
            evidence: Some("The Notepad window shows hello rune.".into()),
            error: None,
            model: Some("gemma".into()),
            latency_ms: Some(4_200),
            claimed_success: true,
            challenged: true,
            created_utc: "2026-10-19T12:01:00Z".into(),
        }
    );
    assert_eq!(get_run(&dir.db(), "without").unwrap().unwrap().verdict, None);
    let listed = list_runs(&dir.db(), 5).unwrap();
    assert_eq!(listed.iter().map(|row| (row.id.as_str(), row.verdict.as_deref())).collect::<Vec<_>>(), [("without", None), ("with", Some("contradicted"))]);
}

#[test]
fn a_new_verdict_for_a_run_replaces_the_old_one_and_the_text_is_capped() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.begin_run(run("r1", "2026-10-19T10:00:00Z"));
    recorder.verdict(verdict("r1", "background", "unsure"));
    recorder.verdict(VerdictRecord { evidence: Some("e".repeat(1_000)), error: Some("x".repeat(5_000)), ..verdict("r1", "background", "verified") });
    recorder.flush();
    let stored = get_run(&dir.db(), "r1").unwrap().unwrap().verdict.unwrap();
    assert_eq!(stored.result, "verified", "one verdict per run, the last one wins");
    assert_eq!(stored.evidence.unwrap().chars().count(), EVIDENCE_CAP);
    assert_eq!(stored.error.unwrap().chars().count(), 2_000);
}

#[test]
fn purging_a_session_removes_its_verdict_too() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    let mut info = run("old", "2026-01-01T10:00:00Z");
    info.session_dir = Some("/s/2026-01-01/a".into());
    recorder.begin_run(info);
    recorder.begin_run(run("keep", "2026-10-19T10:00:00Z"));
    recorder.verdict(verdict("old", "background", "verified"));
    recorder.verdict(verdict("keep", "background", "unsure"));
    recorder.purge_sessions(vec!["/s/2026-01-01/a".into()]);
    recorder.flush();
    assert_eq!(get_run(&dir.db(), "old").unwrap(), None, "a verdict does not keep its run from being purged");
    assert_eq!(get_run(&dir.db(), "keep").unwrap().unwrap().verdict.unwrap().result, "unsure");
}

fn analysis(id: &str, created: &str) -> AnalysisRecord {
    AnalysisRecord {
        id: id.into(),
        created_utc: created.into(),
        question: "what is here?".into(),
        answer: "a settings window".into(),
        findings_json: "[{\"n\":1}]".into(),
        model: Some("m".into()),
        latency_ms: Some(1200),
        status: "ok".into(),
        screenshot_path: Some("a/b.jpg".into()),
        monitor_json: Some("{\"w\":1920}".into()),
        ..AnalysisRecord::default()
    }
}

#[test]
fn a_version_2_database_is_migrated_and_reads_as_having_no_analyses() {
    let dir = TempDir::new();
    {
        let db = Connection::open(dir.db()).unwrap();
        db.execute_batch(&format!("{} {} PRAGMA user_version = 2;", schema::CREATE_V1, schema::CREATE_V2)).unwrap();
        db.execute("INSERT INTO runs (id, task) VALUES ('old', 'x')", []).unwrap();
    }
    assert_eq!(list_analyses(&dir.db(), 5).unwrap(), vec![]);
    assert_eq!(get_analysis(&dir.db(), "any").unwrap(), None);
    let recorder = Recorder::start(&dir.db());
    recorder.flush();
    assert_eq!(version_of(&dir.db()), 3);
    assert!(tables_of(&dir.db()).contains(&"analyses".to_string()));
    assert_eq!(list_runs(&dir.db(), 5).unwrap().len(), 1, "the run survives");
    recorder.analysis(analysis("a1", "2026-10-19T10:00:00Z"));
    recorder.flush();
    assert_eq!(get_analysis(&dir.db(), "a1").unwrap().unwrap().answer, "a settings window");
}

#[test]
fn an_analysis_round_trips_and_the_same_id_replaces_it() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.analysis(analysis("a1", "2026-10-19T10:00:00Z"));
    recorder.flush();
    assert_eq!(get_analysis(&dir.db(), "a1").unwrap(), Some(analysis("a1", "2026-10-19T10:00:00Z")));
    recorder.analysis(AnalysisRecord { status: "error".into(), error: Some("timed out".into()), ..analysis("a1", "2026-10-19T10:00:00Z") });
    recorder.flush();
    let now = get_analysis(&dir.db(), "a1").unwrap().unwrap();
    assert_eq!((now.status.as_str(), now.error.as_deref()), ("error", Some("timed out")));
    assert_eq!(get_analysis(&dir.db(), "missing").unwrap(), None);
}

#[test]
fn analyses_are_listed_newest_first_up_to_the_limit_and_findings_are_capped() {
    let dir = TempDir::new();
    let recorder = Recorder::start(&dir.db());
    recorder.analysis(analysis("old", "2026-10-19T09:00:00Z"));
    recorder.analysis(AnalysisRecord { findings_json: "x".repeat(FINDINGS_CAP + 500), ..analysis("new", "2026-10-19T11:00:00Z") });
    recorder.analysis(analysis("mid", "2026-10-19T10:00:00Z"));
    recorder.flush();
    let ids: Vec<String> = list_analyses(&dir.db(), 10).unwrap().into_iter().map(|record| record.id).collect();
    assert_eq!(ids, ["new", "mid", "old"]);
    assert_eq!(list_analyses(&dir.db(), 2).unwrap().len(), 2);
    let capped = get_analysis(&dir.db(), "new").unwrap().unwrap();
    assert_eq!(capped.findings_json.chars().count(), FINDINGS_CAP);
}
