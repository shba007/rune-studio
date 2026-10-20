use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;
use crate::trace::{CallRecord, Recorder, RunInfo, StepRecord};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rune-flight-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn call(run: &str, step: Option<i64>, attempt: i64, outcome: &str) -> CallRecord {
    CallRecord {
        run_id: run.into(),
        step,
        kind: "decide".into(),
        attempt,
        started_utc: "2026-10-20T10:00:01Z".into(),
        duration_ms: 90,
        wait_before_ms: if attempt > 1 { 500 } else { 0 },
        status: Some(if outcome == "retried" { 503 } else { 200 }),
        outcome: outcome.into(),
        request_json: r#"{"messages":[]}"#.into(),
        response_json: r#"{"ok":true}"#.into(),
        model: "m".into(),
        ..CallRecord::default()
    }
}

/// A database with two runs: `old` (one step, no attempts) and `new` (two steps, three attempts).
fn recorded(dir: &TempDir) -> PathBuf {
    let db = crate::trace::db_path(&dir.0);
    let rec = Recorder::start(&db);
    for (id, started) in [
        ("old", "2026-10-19T09:00:00Z"),
        ("new", "2026-10-20T10:00:00Z"),
    ] {
        rec.begin_run(RunInfo {
            id: id.into(),
            task: format!("task {id}"),
            source: "built-in".into(),
            started_utc: started.into(),
            ..RunInfo::default()
        });
    }
    rec.step(StepRecord {
        run_id: "old".into(),
        step: 1,
        tool: Some("click".into()),
        ..StepRecord::default()
    });
    for n in 1..=2 {
        rec.step(StepRecord {
            run_id: "new".into(),
            step: n,
            tool: Some("type".into()),
            reason: Some("because".into()),
            decide_ms: Some(40),
            ..StepRecord::default()
        });
    }
    rec.call(call("new", Some(1), 1, "retried"));
    rec.call(call("new", Some(1), 2, "ok"));
    rec.call(call("new", Some(2), 1, "ok"));
    rec.end_run("old", "succeeded", "done");
    rec.end_run("new", "succeeded", "done");
    rec.flush();
    db
}

#[test]
fn runs_come_newest_first_and_page_by_start_time() {
    let dir = TempDir::new();
    let db = recorded(&dir);
    let all = runs(&db, 10, None).unwrap();
    assert_eq!(
        all.iter().map(|run| run.id.as_str()).collect::<Vec<_>>(),
        ["new", "old"]
    );
    assert_eq!(all[0].steps, 2);
    assert_eq!(runs(&db, 1, None).unwrap().len(), 1);
    let older = runs(&db, 10, Some("2026-10-20T10:00:00Z")).unwrap();
    assert_eq!(
        older.iter().map(|run| run.id.as_str()).collect::<Vec<_>>(),
        ["old"]
    );
}

#[test]
fn a_header_counts_steps_and_retries_and_a_missing_run_is_none() {
    let dir = TempDir::new();
    let db = recorded(&dir);
    let head = header(&db, "new").unwrap().unwrap();
    assert_eq!((head.steps, head.retries), (2, 1));
    assert_eq!(head.outcome.as_deref(), Some("succeeded"));
    assert!(head.video.is_none());
    assert_eq!(header(&db, "old").unwrap().unwrap().retries, 0);
    assert!(header(&db, "nope").unwrap().is_none());
}

#[test]
fn steps_and_events_are_in_order() {
    let dir = TempDir::new();
    let db = recorded(&dir);
    let list = steps(&db, "new").unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[1]["step"], 2);
    assert_eq!(list[0]["phases"]["decide"], 40);
    assert!(events(&db, "nope").unwrap().is_empty());
}

#[test]
fn calls_come_without_bodies_and_bodies_load_one_at_a_time() {
    let dir = TempDir::new();
    let db = recorded(&dir);
    let all = calls(&db, "new", None).unwrap();
    assert_eq!(
        all.iter()
            .map(|c| (c.step, c.attempt, c.outcome.as_str()))
            .collect::<Vec<_>>(),
        [
            (Some(1), 1, "retried"),
            (Some(1), 2, "ok"),
            (Some(2), 1, "ok")
        ]
    );
    assert_eq!(all[0].status, Some(503));
    assert_eq!(calls(&db, "new", Some(2)).unwrap().len(), 1);
    assert!(calls(&db, "old", None).unwrap().is_empty());
    let (request, response) = call_bodies(&db, all[0].id).unwrap().unwrap();
    assert_eq!(
        (request.as_str(), response.as_str()),
        (r#"{"messages":[]}"#, r#"{"ok":true}"#)
    );
    assert!(call_bodies(&db, 9_999).unwrap().is_none());
}

#[test]
fn a_missing_database_is_an_error_not_a_panic() {
    let dir = TempDir::new();
    assert!(runs(&dir.0.join("none.db"), 5, None).is_err());
}
