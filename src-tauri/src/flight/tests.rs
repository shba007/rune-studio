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
fn insight_history_exposes_capture_and_answer_without_inventing_action_steps() {
    let dir = TempDir::new();
    let db = crate::trace::db_path(&dir.0);
    let folder = dir.0.join("session");
    std::fs::create_dir_all(folder.join("screenshots")).unwrap();
    let capture = folder.join("screenshots/capture.png");
    std::fs::write(&capture, b"png").unwrap();
    let rec = Recorder::start(&db);
    rec.begin_run(RunInfo {
        id: "insight-test".into(),
        source: "screen-insight".into(),
        started_utc: "2026-10-20T10:00:00Z".into(),
        ..Default::default()
    });
    rec.analysis(trace::AnalysisRecord {
        id: "insight-test".into(),
        answer: "An error is visible.".into(),
        findings_json: r#"[{"n":1,"label":"Error"}]"#.into(),
        model: Some("vision".into()),
        status: "ok".into(),
        screenshot_path: Some(capture.display().to_string()),
        ..Default::default()
    });
    rec.end_run("insight-test", "succeeded", "An error is visible.");
    rec.flush();
    let head = header(&db, "insight-test").unwrap().unwrap();
    assert_eq!(head.steps, 0);
    assert_eq!(
        runs(&db, 10, None).unwrap()[0].thumb.as_deref(),
        Some("screenshots/capture.png")
    );
    let analysis = head.analysis.unwrap();
    assert_eq!(analysis.answer, "An error is visible.");
    assert_eq!(analysis.model.as_deref(), Some("vision"));
    assert_eq!(
        analysis.capture_file.as_deref(),
        Some("screenshots/capture.png")
    );
    assert_eq!(analysis.findings[0]["label"], "Error");
    let resolved = session_dir(&db, "insight-test").unwrap().unwrap();
    assert_eq!(
        files::resolve(&resolved, "screenshots/capture.png").unwrap(),
        capture.canonicalize().unwrap()
    );
    std::fs::remove_file(capture).unwrap();
    assert!(
        header(&db, "insight-test")
            .unwrap()
            .unwrap()
            .analysis
            .unwrap()
            .capture_file
            .is_some()
    );
    assert_eq!(
        files::resolve(&resolved, "screenshots/capture.png"),
        Err(files::Refused::Missing)
    );

    rec.begin_run(RunInfo {
        id: "insight-old".into(),
        source: "screen-insight".into(),
        ..Default::default()
    });
    rec.analysis(trace::AnalysisRecord {
        id: "insight-old".into(),
        answer: "An older answer".into(),
        findings_json: "[]".into(),
        ..Default::default()
    });
    rec.flush();
    let old = header(&db, "insight-old")
        .unwrap()
        .unwrap()
        .analysis
        .unwrap();
    assert_eq!(old.answer, "An older answer");
    assert!(old.capture_file.is_none());
    assert!(session_dir(&db, "insight-old").unwrap().is_none());
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

#[test]
fn a_run_id_is_made_safe_for_a_url() {
    assert_eq!(url_safe("120000-1-open-notepad"), "120000-1-open-notepad");
    assert_eq!(url_safe("a b&c=d/../é"), "a%20b%26c%3Dd%2F%2E%2E%2F%C3%A9");
}

#[test]
fn original_prompt_is_complete_and_deletion_does_not_resurrect_from_late_writes() {
    let dir = TempDir::new();
    let db = recorded(&dir);
    let rec = Recorder::start(&db);
    let raw = format!("  Open Notepad\nType exactly: {}  ", "বাংলা ".repeat(400));
    rec.user_prompt("new", &raw);
    rec.analysis(trace::AnalysisRecord {
        id: "new".into(),
        status: "ok".into(),
        ..Default::default()
    });
    rec.flush();
    assert_eq!(
        original_prompt(&db, "new").unwrap().as_deref(),
        Some(raw.as_str())
    );
    assert!(original_prompt(&db, "old").unwrap().is_none());
    rec.block_session("new").unwrap();
    delete_record(&db, "new").unwrap();
    assert!(
        rec.block_session("new").is_err(),
        "a repeated delete cannot reopen the trace write guard"
    );
    rec.event("new", "late", "{}");
    rec.verdict(crate::trace::VerdictRecord {
        run_id: "new".into(),
        ..Default::default()
    });
    rec.analysis(trace::AnalysisRecord {
        id: "new".into(),
        ..Default::default()
    });
    rec.flush();
    assert!(store::get_run(&db, "new").unwrap().is_none());
    assert!(store::get_analysis(&db, "new").unwrap().is_none());
    assert!(calls(&db, "new", None).unwrap().is_empty());
    assert_eq!(steps(&db, "old").unwrap().len(), 1);
    rec.begin_run(RunInfo {
        id: "active".into(),
        ..Default::default()
    });
    assert!(
        rec.block_session("active")
            .unwrap_err()
            .contains("Finish or stop")
    );
}
