pub mod encode;
pub mod schedule;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use crate::{
    events::{self, AgentState, AppState},
    sync::LockExt,
    trace::{self, store},
};
use encode::EncodeError;
use schedule::{schedule, total_ms};

const EVERY: Duration = Duration::from_secs(5);
const LOOK_BACK: usize = 10;

pub fn may_encode(
    task_running: bool,
    lease_held: bool,
    analysis_busy: bool,
    reflecting: bool,
) -> bool {
    !(task_running || lease_held || analysis_busy || reflecting)
}

fn idle(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    may_encode(
        events::agent_state(app).is_running(),
        state.lease.locked().is_held(),
        crate::analysis::state::in_flight(state.analysis.locked().status),
        events::agent_state(app) == AgentState::Reflecting,
    )
}

pub fn frames_of(detail: &store::RunDetail) -> (Vec<(String, u64)>, Option<u64>) {
    let start = trace::parse_iso(&detail.run.info.started_utc);
    let at = |text: &str| trace::parse_iso(text);
    let outside = !matches!(detail.run.info.source.as_str(), "" | "built-in");
    let offset = |step: &trace::StepRecord| -> u64 {
        let begun = step
            .started_utc
            .as_deref()
            .and_then(at)
            .zip(start)
            .map_or(0, |(t, s)| t.saturating_sub(s) * 1000);
        let shown_after = if outside {
            step.act_ms.unwrap_or(0) + step.settle_ms.unwrap_or(0)
        } else {
            step.observe_ms.unwrap_or(0)
        };
        begun + u64::try_from(shown_after).unwrap_or(0)
    };
    let frames = detail
        .steps
        .iter()
        .filter_map(|step| {
            step.observation_path
                .as_ref()
                .filter(|path| !path.is_empty())
                .map(|path| (path.clone(), offset(step)))
        })
        .collect();
    let end = detail
        .run
        .ended_utc
        .as_deref()
        .and_then(at)
        .zip(start)
        .map(|(e, s)| e.saturating_sub(s) * 1000);
    (frames, end)
}

pub fn set_manifest(dir: &Path, video: Value) -> std::io::Result<()> {
    let path = dir.join("session.json");
    let mut manifest: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}));
    manifest["video"] = video;
    std::fs::write(
        path,
        serde_json::to_string_pretty(&manifest).map_err(std::io::Error::other)?,
    )
}

pub fn video_value(
    file: Option<&str>,
    frames: usize,
    seconds: f64,
    unavailable: Option<&str>,
) -> Value {
    let mut value =
        json!({ "file": file, "frames": frames, "seconds": seconds, "kind": "storyboard" });
    if let Some(why) = unavailable {
        value["unavailable"] = json!(why);
    }
    value
}

fn has_video(dir: &Path) -> bool {
    dir.join("session.mp4").exists()
        || std::fs::read_to_string(dir.join("session.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .is_some_and(|manifest| manifest.get("video").is_some())
}

const MOTION_EVERY: Duration = Duration::from_millis(500);
const MOTION_EDGE: u32 = 1920;
const MOTION_QUALITY: u8 = 60;

pub fn motion_frames(dir: &Path) -> Vec<(String, u64)> {
    let mut found: Vec<(u64, String)> = std::fs::read_dir(dir.join("motion"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let path = entry.path();
                    let at = path.file_stem()?.to_str()?.parse::<u64>().ok()?;
                    Some((at, path.display().to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    found.sort_by_key(|(at, _)| *at);
    let first = found.first().map_or(0, |(at, _)| *at);
    found
        .into_iter()
        .map(|(at, path)| (path, at - first))
        .collect()
}

pub fn start_motion_recorder(app: &AppHandle) {
    let app = app.clone();
    std::thread::Builder::new()
        .name("session-motion".into())
        .spawn(move || {
            loop {
                std::thread::sleep(MOTION_EVERY);
                let state = app.state::<AppState>();
                if state.settings.locked().record_session_video
                    != crate::settings::RecordVideo::Motion
                {
                    continue;
                }
                let working =
                    events::agent_state(&app).is_running() || state.lease.locked().is_held();
                let Some(dir) = crate::archive::open_session_dir().filter(|_| working) else {
                    continue;
                };
                let Ok(jpeg) = crate::observe::screenshot::live_jpeg(MOTION_EDGE, MOTION_QUALITY)
                else {
                    continue;
                };
                let folder = dir.join("motion");
                let millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis());
                if std::fs::create_dir_all(&folder).is_ok() {
                    let _ = std::fs::write(folder.join(format!("{millis}.jpg")), jpeg);
                }
            }
        })
        .ok();
}

pub fn start_worker(app: &AppHandle) {
    let app = app.clone();
    let tried: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    let started = trace::now_utc();
    std::thread::Builder::new()
        .name("session-video".into())
        .spawn(move || {
            loop {
                std::thread::sleep(EVERY);
                if !idle(&app) {
                    continue;
                }
                let Some(db) = app
                    .state::<AppState>()
                    .trace
                    .db_path()
                    .map(Path::to_path_buf)
                else {
                    continue;
                };
                if let Some((id, dir)) = next_candidate(&db, &tried, &started) {
                    tried.locked().insert(id.clone());
                    work(&app, &db, &id, &dir, &tried);
                }
            }
        })
        .ok();
}

fn next_candidate(
    db: &Path,
    tried: &Mutex<HashSet<String>>,
    since: &str,
) -> Option<(String, PathBuf)> {
    let rows = store::list_runs(db, LOOK_BACK).ok()?;
    rows.into_iter()
        .filter(|row| {
            row.ended_utc.is_some() && row.started_utc.as_deref().is_some_and(|s| s >= since)
        })
        .filter(|row| !tried.locked().contains(&row.id))
        .find_map(|row| {
            let detail = store::get_run(db, &row.id).ok()??;
            let dir = PathBuf::from(
                detail
                    .run
                    .info
                    .session_dir
                    .as_deref()
                    .filter(|dir| !dir.is_empty())?,
            );
            (dir.is_dir() && !has_video(&dir)).then_some((row.id, dir))
        })
}

fn work(app: &AppHandle, db: &Path, id: &str, dir: &Path, tried: &Mutex<HashSet<String>>) {
    let Some(detail) = store::get_run(db, id).ok().flatten() else {
        return;
    };
    let (mut frames, end) = frames_of(&detail);
    let motion = motion_frames(dir);
    let kind = if motion.is_empty() {
        "storyboard"
    } else {
        "motion"
    };
    if !motion.is_empty() {
        frames = motion;
    }
    let shown = schedule(&frames, end);
    if shown.is_empty() {
        let _ = set_manifest(
            dir,
            video_value(None, 0, 0.0, Some("no screenshots in this session")),
        );
        return;
    }
    let out = dir.join("session.mp4");
    match encode::encode(&out, &shown, &|| !idle(app)) {
        Ok(_) => {
            let seconds = total_ms(&shown) as f64 / 1000.0;
            let mut value = video_value(Some("session.mp4"), shown.len(), seconds, None);
            value["kind"] = json!(kind);
            let _ = set_manifest(dir, value);
            let _ = std::fs::remove_dir_all(dir.join("motion"));
            log::info!(
                "session video for {id}: {} frames, {seconds:.1} s",
                shown.len()
            );
        }
        Err(EncodeError::Cancelled) => {
            let _ = std::fs::remove_file(&out);
            tried.locked().remove(id);
            log::info!("session video for {id} stopped: the PC is busy again");
        }
        Err(EncodeError::Unavailable(why)) => {
            let _ = std::fs::remove_file(&out);
            log::warn!("no session video for {id}: {why}");
            let _ = set_manifest(
                dir,
                video_value(
                    None,
                    shown.len(),
                    total_ms(&shown) as f64 / 1000.0,
                    Some("unavailable"),
                ),
            );
        }
        Err(EncodeError::Failed(why)) => {
            let _ = std::fs::remove_file(&out);
            log::warn!("session video for {id} failed: {why}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{RunInfo, StepRecord, store::RunRow};

    fn detail(steps: Vec<StepRecord>, ended: Option<&str>) -> store::RunDetail {
        store::RunDetail {
            run: RunRow {
                info: RunInfo {
                    id: "r".into(),
                    started_utc: "2026-10-20T10:00:00Z".into(),
                    ..RunInfo::default()
                },
                ended_utc: ended.map(String::from),
                outcome: Some("succeeded".into()),
                summary: None,
            },
            steps,
            events: vec![],
            verdict: None,
        }
    }

    fn step(n: u32, at: &str, observe: i64, shot: Option<&str>) -> StepRecord {
        StepRecord {
            started_utc: Some(at.into()),
            observe_ms: Some(observe),
            observation_path: shot.map(String::from),
            ..StepRecord::new("r", n)
        }
    }

    #[test]
    fn frames_come_from_steps_with_a_screenshot_at_their_time_since_the_run_started() {
        let run = detail(
            vec![
                step(1, "2026-10-20T10:00:01Z", 300, Some("a.jpg")),
                step(2, "2026-10-20T10:00:08Z", 250, Some("b.jpg")),
                step(3, "2026-10-20T10:00:10Z", 100, None),
            ],
            Some("2026-10-20T10:00:55Z"),
        );
        let (frames, end) = frames_of(&run);
        assert_eq!(
            frames,
            vec![("a.jpg".to_string(), 1_300), ("b.jpg".to_string(), 8_250)]
        );
        assert_eq!(end, Some(55_000));
        let shown = schedule(&frames, end);
        assert_eq!(
            total_ms(&shown),
            55_000,
            "the video is as long as the session"
        );
    }

    #[test]
    fn an_outside_agents_screenshot_is_what_the_action_left_so_it_shows_after_the_action() {
        let mut run = detail(
            vec![step(1, "2026-10-20T10:00:01Z", 0, Some("after.jpg"))],
            Some("2026-10-20T10:00:55Z"),
        );
        run.run.info.source = "Claude".into();
        run.steps[0].act_ms = Some(400);
        run.steps[0].settle_ms = Some(600);
        let (frames, _) = frames_of(&run);
        assert_eq!(frames, vec![("after.jpg".to_string(), 2_000)]);
    }

    #[test]
    fn a_run_without_screenshots_or_an_end_has_nothing_to_encode_or_no_known_length() {
        let (frames, end) = frames_of(&detail(
            vec![step(1, "2026-10-20T10:00:01Z", 0, None)],
            None,
        ));
        assert!(frames.is_empty());
        assert_eq!(end, None);
        assert!(schedule(&frames, end).is_empty());
    }

    #[test]
    fn motion_frames_are_ordered_and_counted_from_the_first() {
        let dir = std::env::temp_dir().join(format!("rune-video-motion-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("motion")).unwrap();
        for name in [
            "1700000001000.jpg",
            "1700000000000.jpg",
            "1700000000500.jpg",
            "notes.txt",
        ] {
            std::fs::write(dir.join("motion").join(name), b"x").unwrap();
        }
        let frames = motion_frames(&dir);
        assert_eq!(
            frames.iter().map(|(_, at)| *at).collect::<Vec<_>>(),
            [0, 500, 1000]
        );
        assert!(frames[0].0.ends_with("1700000000000.jpg"));
        assert!(motion_frames(&dir.join("nothing")).is_empty());
        let shown = schedule(&frames, Some(1_500));
        assert_eq!(total_ms(&shown), 1_500);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn encoding_waits_for_the_pc_to_be_idle() {
        assert!(may_encode(false, false, false, false));
        assert!(!may_encode(true, false, false, false), "a task runs");
        assert!(
            !may_encode(false, true, false, false),
            "an outside agent holds the lease"
        );
        assert!(!may_encode(false, false, true, false), "an analysis works");
        assert!(
            !may_encode(false, false, false, true),
            "the idle review runs"
        );
    }

    #[test]
    fn the_manifest_gets_a_video_entry_and_keeps_the_rest() {
        let dir = std::env::temp_dir().join(format!("rune-video-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("session.json"),
            r#"{"task":"x","outcome":"succeeded"}"#,
        )
        .unwrap();
        assert!(!has_video(&dir));
        set_manifest(&dir, video_value(Some("session.mp4"), 6, 55.0, None)).unwrap();
        let manifest: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("session.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["task"], "x");
        assert_eq!(
            manifest["video"],
            json!({ "file": "session.mp4", "frames": 6, "seconds": 55.0, "kind": "storyboard" })
        );
        assert!(has_video(&dir), "a note counts: it is not tried again");
        let missing = video_value(None, 6, 55.0, Some("unavailable"));
        assert_eq!(missing["unavailable"], "unavailable");
        assert!(missing["file"].is_null());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
