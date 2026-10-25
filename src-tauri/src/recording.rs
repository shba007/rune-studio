//! Explicit, bounded recordings. Separate from Insight frames and session video.
use crate::{
    exec::CancelToken,
    model::coords::Monitor,
    observe::screenshot::{self, CaptureTarget},
    sync::LockExt,
};
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

pub const MODEL_CONTRACT: &str = "Use record_screen for a bounded clip when no agent action must happen during capture. To type/click while recording: discover the explicit target, call start_recording and wait for its recording_active=true result, perform the actions, then call stop_recording with the returned recording_id and wait for its saved path. start_recording returns after the first captured frame; it does not block subsequent actions. Choose a maximum up to 60 seconds (default 60) allowing for model decision latency; capture automatically ends at that limit. Never claim that actions after expiry were recorded. Keep start -> recorded actions -> stop/save as ordered atomic steps of one finite task; do not split this chain into supposedly independent tasks. Keep unrelated work parallel. Recording belongs to its parent request, survives task boundaries, and cancellation discards it. No separate recorder app is needed. A completed clip does not need to be recorded again merely because the screen is unchanged.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Manual,
    Request(u64),
    Session(u64),
}
#[derive(Clone)]
pub struct Scope {
    pub owner: Owner,
    pub cancel: CancelToken,
}

pub fn completion_json(snapshot: &Snapshot) -> String {
    serde_json::json!({
        "status": "completed", "recording_active": false,
        "saved_path": snapshot.path, "recorded_seconds": snapshot.seconds,
        "next_action": "The clip is already stopped and saved. Do not start it again to finish this recording goal. Continue with the next feasible goal, or call done with the saved path. Actions after this result are not part of this clip."
    }).to_string()
}

const MAX_SECONDS: u64 = 60;
static NEXT: AtomicU64 = AtomicU64::new(1);
static STATE: Mutex<State> = Mutex::new(State {
    snapshot: Snapshot {
        id: 0,
        phase: Phase::Idle,
        seconds: 0,
        monitor: None,
        path: None,
        error: None,
    },
    control: None,
    owner: None,
    target: None,
    max_seconds: 0,
    finished: VecDeque::new(),
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Idle,
    Starting,
    Recording,
    Saving,
    Done,
    Cancelled,
    Failed,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: u64,
    pub phase: Phase,
    pub seconds: u64,
    pub monitor: Option<Monitor>,
    pub path: Option<String>,
    pub error: Option<String>,
}
struct Control {
    id: u64,
    stop: AtomicBool,
    discard: AtomicBool,
    result: Mutex<Option<Result<String, String>>>,
}
struct State {
    snapshot: Snapshot,
    control: Option<Arc<Control>>,
    owner: Option<Owner>,
    target: Option<CaptureTarget>,
    max_seconds: u64,
    finished: VecDeque<(Owner, Snapshot)>,
}

pub fn snapshot() -> Snapshot {
    STATE.locked().snapshot.clone()
}
pub fn busy() -> bool {
    matches!(
        snapshot().phase,
        Phase::Starting | Phase::Recording | Phase::Saving
    )
}
pub fn capturing() -> bool {
    snapshot().phase == Phase::Recording
}

fn publish(app: &AppHandle) {
    let _ = app.emit("screen_recording", snapshot());
}
pub fn stop(discard: bool) {
    if let Some(control) = &STATE.locked().control {
        if discard {
            control.discard.store(true, Ordering::SeqCst);
        }
        control.stop.store(true, Ordering::SeqCst);
    }
}
pub fn validate_seconds(seconds: u64) -> Result<(), String> {
    if !(1..=MAX_SECONDS).contains(&seconds) {
        return Err("Choose a recording duration from 1 to 60 seconds".into());
    }
    Ok(())
}

fn launch(
    app: &AppHandle,
    target: CaptureTarget,
    seconds: u64,
    cancel: Option<CancelToken>,
    owner: Owner,
) -> Result<Arc<Control>, String> {
    validate_seconds(seconds)?;
    if !cfg!(windows) {
        return Err("Native screen recording is currently available on Windows".into());
    }
    let app_state = app.state::<crate::events::AppState>();
    if crate::analysis::state::in_flight(app_state.analysis.locked().status) {
        return Err("Finish or cancel Screen Insight before recording".into());
    }
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let control = Arc::new(Control {
        id,
        stop: AtomicBool::new(false),
        discard: AtomicBool::new(false),
        result: Mutex::new(None),
    });
    {
        let mut state = STATE.locked();
        if matches!(
            state.snapshot.phase,
            Phase::Starting | Phase::Recording | Phase::Saving
        ) {
            return Err("A screen recording is already running or being saved".into());
        }
        state.snapshot = Snapshot {
            id,
            phase: Phase::Starting,
            seconds: 0,
            monitor: None,
            path: None,
            error: None,
        };
        state.control = Some(control.clone());
        state.owner = Some(owner);
        state.target = Some(target.clone());
        state.max_seconds = seconds;
    }
    publish(app);
    let handle = app.clone();
    let worker = control.clone();
    let spawned = std::thread::Builder::new()
        .name("screen-recording".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                capture(&handle, id, target, seconds, &worker, cancel.as_ref())
            }))
            .unwrap_or_else(|_| Err("Screen recording failed unexpectedly; no video saved".into()));
            {
                let mut state = STATE.locked();
                if state.snapshot.id == id {
                    match &result {
                        Ok(path) => {
                            state.snapshot.phase = Phase::Done;
                            state.snapshot.path = Some(path.clone());
                        }
                        Err(error) => {
                            state.snapshot.phase = if worker.discard.load(Ordering::SeqCst)
                                || cancel.as_ref().is_some_and(CancelToken::is_cancelled)
                            {
                                Phase::Cancelled
                            } else {
                                Phase::Failed
                            };
                            state.snapshot.error = Some(error.clone());
                        }
                    }
                    state.control = None;
                    let completed = state.snapshot.clone();
                    state.finished.push_back((owner, completed));
                    while state.finished.len() > 64 {
                        state.finished.pop_front();
                    }
                }
            }
            *worker.result.locked() = Some(result);
            crate::overlay::recording_changed(&handle);
            publish(&handle);
        });
    if let Err(error) = spawned {
        let mut state = STATE.locked();
        state.snapshot.phase = Phase::Failed;
        state.snapshot.error = Some(error.to_string());
        state.control = None;
        drop(state);
        publish(app);
        return Err(error.to_string());
    }
    Ok(control)
}

pub fn record(
    app: &AppHandle,
    target: CaptureTarget,
    seconds: u64,
    cancel: &CancelToken,
    owner: Owner,
) -> Result<String, String> {
    cancel.check().map_err(|e| e.to_string())?;
    let control = launch(app, target, seconds, Some(cancel.clone()), owner)?;
    loop {
        if let Some(result) = control.result.locked().clone() {
            return result;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn current_owner() -> Option<Owner> {
    let state = STATE.locked();
    matches!(
        state.snapshot.phase,
        Phase::Starting | Phase::Recording | Phase::Saving
    )
    .then_some(state.owner.unwrap_or(Owner::Manual))
}
pub fn permits(owner: Owner) -> bool {
    current_owner().is_none_or(|active| active == owner)
}
pub fn stop_owner(owner: Owner, discard: bool) {
    let state = STATE.locked();
    if state.owner == Some(owner)
        && let Some(control) = &state.control
    {
        if discard {
            control.discard.store(true, Ordering::SeqCst);
        }
        control.stop.store(true, Ordering::SeqCst);
    }
}
pub fn start_async(
    app: &AppHandle,
    target: CaptureTarget,
    seconds: u64,
    scope: &Scope,
    action_cancel: &CancelToken,
) -> Result<Snapshot, String> {
    action_cancel.check().map_err(|e| e.to_string())?;
    scope.cancel.check().map_err(|e| e.to_string())?;
    let control = launch(
        app,
        target,
        seconds,
        Some(scope.cancel.clone()),
        scope.owner,
    )?;
    loop {
        if action_cancel.is_cancelled() {
            control.discard.store(true, Ordering::SeqCst);
            control.stop.store(true, Ordering::SeqCst);
            return Err("Recording start cancelled; clip discarded".into());
        }
        let latest = snapshot();
        if latest.id == control.id && latest.phase == Phase::Recording {
            return Ok(latest);
        }
        if let Some(result) = control.result.locked().clone() {
            return Err(result.err().unwrap_or(
                "Recording ended before actions could begin; increase its maximum duration".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn owned_snapshot(state: &State, owner: Owner, id: Option<u64>) -> Result<Snapshot, String> {
    if state.owner == Some(owner) && id.is_none_or(|id| id == state.snapshot.id) {
        return Ok(state.snapshot.clone());
    }
    state.finished.iter().rev().find(|(o,s)|*o==owner&&id.is_none_or(|id|id==s.id)).map(|(_,s)|s.clone()).ok_or("Recording ID does not belong to this request/session. Use its start_recording result; do not stop another recording.".into())
}
fn owned_control(state: &State, owner: Owner, id: u64) -> Result<Option<Arc<Control>>, String> {
    owned_snapshot(state, owner, Some(id))?;
    if state.owner == Some(owner) && state.snapshot.id == id {
        Ok(state.control.clone())
    } else {
        Ok(None)
    }
}
pub fn stop_async(
    id: u64,
    discard: bool,
    scope: &Scope,
    action_cancel: &CancelToken,
) -> Result<Snapshot, String> {
    let control = {
        let state = STATE.locked();
        owned_control(&state, scope.owner, id)?
    };
    if let Some(control) = control {
        if discard {
            control.discard.store(true, Ordering::SeqCst);
        }
        control.stop.store(true, Ordering::SeqCst);
        loop {
            if action_cancel.is_cancelled() || scope.cancel.is_cancelled() {
                control.discard.store(true, Ordering::SeqCst);
                return Err("Recording cancelled; no video saved".into());
            }
            if let Some(result) = control.result.locked().clone() {
                if !discard {
                    result?;
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    let state = STATE.locked();
    let snapshot = owned_snapshot(&state, scope.owner, Some(id))?;
    if !discard && snapshot.phase != Phase::Done {
        return Err(snapshot
            .error
            .unwrap_or("Recording did not finish; no video saved".into()));
    }
    Ok(snapshot)
}
fn status_json(state: &State) -> serde_json::Value {
    serde_json::json!({"recording_id":state.snapshot.id,"recording_active":state.snapshot.phase==Phase::Recording,"phase":state.snapshot.phase,"target":state.target,"max_seconds":state.max_seconds,"saved_path":state.snapshot.path,"recorded_seconds":state.snapshot.seconds,"error":state.snapshot.error})
}
pub fn result_json(owner: Owner, id: Option<u64>) -> String {
    let state = STATE.locked();
    let Ok(snapshot) = owned_snapshot(&state, owner, id) else {
        return serde_json::json!({"recording_active":false,"owned_recording":false}).to_string();
    };
    if state.owner == Some(owner) && state.snapshot.id == snapshot.id {
        status_json(&state).to_string()
    } else {
        serde_json::json!({"recording_id":snapshot.id,"recording_active":false,"phase":snapshot.phase,"saved_path":snapshot.path,"recorded_seconds":snapshot.seconds,"error":snapshot.error}).to_string()
    }
}
pub fn scope_json(owner: Owner) -> String {
    result_json(owner, None)
}

fn capture(
    app: &AppHandle,
    id: u64,
    target: CaptureTarget,
    seconds: u64,
    control: &Control,
    cancel: Option<&CancelToken>,
) -> Result<String, String> {
    let max_edge = app
        .state::<crate::events::AppState>()
        .settings
        .locked()
        .recording_resolution
        .max_edge();
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let folder = cache.join(format!("screen-recording-{}-{id}", std::process::id()));
    fs::create_dir(&folder)
        .map_err(|e| format!("Could not create a private recording folder: {e}"))?;
    struct TemporaryFrames(std::path::PathBuf);
    impl Drop for TemporaryFrames {
        fn drop(&mut self) {
            if let Err(error) = fs::remove_dir_all(&self.0) {
                log::warn!("Could not remove temporary recording frames: {error}");
            }
        }
    }
    let _temporary_frames = TemporaryFrames(folder.clone());
    let output = crate::archive::path_for(crate::archive::Kind::Recordings, "screen", "mp4")
        .map_err(|e| e.to_string())?;
    let discarded =
        || control.discard.load(Ordering::SeqCst) || cancel.is_some_and(CancelToken::is_cancelled);
    (|| {
        let started = Instant::now();
        let mut frames = Vec::new();
        while started.elapsed() < Duration::from_secs(seconds)
            && !control.stop.load(Ordering::SeqCst)
            && !discarded()
        {
            let at = started.elapsed().as_millis() as u64;
            let target = target.clone();
            let Some(frame) = bounded_capture(
                move || {
                    screenshot::recording_capture(target)
                        .map_err(|e| format!("Recording target unavailable: {e}"))
                },
                control,
                cancel,
            )?
            else {
                break;
            };
            if discarded() {
                return Err("Recording cancelled; no video saved".into());
            }
            let path = folder.join(format!("{:04}.png", frames.len()));
            frame
                .save_recording_frame(&path, max_edge)
                .map_err(|e| e.to_string())?;
            frames.push((path.to_string_lossy().into_owned(), at));
            let first = {
                let mut state = STATE.locked();
                let first = state.snapshot.phase == Phase::Starting;
                state.snapshot.phase = Phase::Recording;
                state.snapshot.seconds = started.elapsed().as_secs();
                state.snapshot.monitor = Some(frame.monitor);
                first
            };
            if first {
                crate::overlay::recording_changed(app);
            }
            publish(app);
            let next = Duration::from_millis(at + 500);
            while started.elapsed() < next && !control.stop.load(Ordering::SeqCst) && !discarded() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if frames.is_empty() {
            control.discard.store(true, Ordering::SeqCst);
        }
        if discarded() {
            return Err("Recording cancelled; no video saved".into());
        }
        STATE.locked().snapshot.phase = Phase::Saving;
        crate::overlay::recording_changed(app);
        publish(app);
        let shown =
            crate::video::schedule::schedule(&frames, Some(started.elapsed().as_millis() as u64));
        let encoded = folder.join("video.mp4");
        crate::video::encode::encode_at_resolution(&encoded, &shown, &discarded, max_edge)
            .map_err(|e| match e {
                crate::video::encode::EncodeError::Unavailable(e)
                | crate::video::encode::EncodeError::Failed(e) => e,
                crate::video::encode::EncodeError::Cancelled => {
                    "Recording cancelled; no video saved".into()
                }
            })?;
        if discarded() {
            return Err("Recording cancelled; no video saved".into());
        }
        copy_new(&encoded, &output)?;
        if discarded() {
            let _ = fs::remove_file(&output);
            return Err("Recording cancelled; no video saved".into());
        }
        Ok(output.to_string_lossy().into_owned())
    })()
}

fn bounded_capture<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
    control: &Control,
    cancel: Option<&CancelToken>,
) -> Result<Option<T>, String> {
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("recording-frame".into())
        .spawn(move || {
            let _ = send.send(job());
        })
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        if control.stop.load(Ordering::SeqCst)
            || control.discard.load(Ordering::SeqCst)
            || cancel.is_some_and(CancelToken::is_cancelled)
        {
            return Ok(None);
        }
        match receive.recv_timeout(Duration::from_millis(20)) {
            Ok(result) => return result.map(Some),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Recording capture failed unexpectedly".into());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        if started.elapsed() >= Duration::from_secs(10) {
            return Err("Recording capture timed out. Choose another screen or window.".into());
        }
    }
}

fn copy_new(source: &Path, output: &Path) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| {
            format!("Could not save recording without overwriting an existing file: {e}")
        })?;
    let copied = (|| {
        let mut source = fs::File::open(source)?;
        std::io::copy(&mut source, &mut file)?;
        file.sync_all()
    })();
    if let Err(error) = copied {
        drop(file);
        let _ = fs::remove_file(output);
        return Err(error.to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn screen_recording_state() -> Snapshot {
    snapshot()
}
#[tauri::command]
pub fn start_screen_recording(
    app: AppHandle,
    target: CaptureTarget,
    seconds: u64,
) -> Result<Snapshot, String> {
    let state = app.state::<crate::events::AppState>();
    if crate::events::agent_state(&app).is_running() || state.lease.locked().is_held() {
        return Err("Finish or stop the agent session before starting a manual recording".into());
    }
    launch(&app, target, seconds, None, Owner::Manual)?;
    Ok(snapshot())
}
#[tauri::command]
pub fn stop_screen_recording(discard: bool) {
    stop(discard);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_state() -> State {
        State {
            snapshot: Snapshot {
                id: 42,
                phase: Phase::Recording,
                seconds: 1,
                monitor: None,
                path: None,
                error: None,
            },
            control: Some(Arc::new(Control {
                id: 42,
                stop: AtomicBool::new(false),
                discard: AtomicBool::new(false),
                result: Mutex::new(None),
            })),
            owner: Some(Owner::Request(7)),
            target: Some(CaptureTarget::Window { id: 17 }),
            max_seconds: 60,
            finished: VecDeque::new(),
        }
    }
    #[test]
    fn recording_handles_cannot_cross_requests_sessions_or_manual_ownership() {
        let state = fixture_state();
        assert!(
            owned_control(&state, Owner::Request(7), 42)
                .unwrap()
                .is_some()
        );
        for owner in [Owner::Request(8), Owner::Session(7), Owner::Manual] {
            assert!(owned_control(&state, owner, 42).is_err());
        }
        assert!(owned_control(&state, Owner::Request(7), 41).is_err());
        let control = state.control.unwrap();
        assert!(!control.stop.load(Ordering::SeqCst));
        assert!(!control.discard.load(Ordering::SeqCst));
    }
    #[test]
    fn a_completed_handle_remains_available_without_stopping_a_new_recording() {
        let mut state = fixture_state();
        let mut old = state.snapshot.clone();
        old.phase = Phase::Done;
        old.path = Some("first-request.mp4".into());
        state.finished.push_back((Owner::Request(7), old));
        state.owner = Some(Owner::Request(8));
        state.snapshot.id = 43;
        assert!(
            owned_control(&state, Owner::Request(7), 42)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            owned_snapshot(&state, Owner::Request(7), Some(42))
                .unwrap()
                .path
                .as_deref(),
            Some("first-request.mp4")
        );
        assert!(owned_snapshot(&state, Owner::Session(7), Some(42)).is_err());
        assert!(!state.control.unwrap().stop.load(Ordering::SeqCst));
    }
    #[test]
    fn live_status_distinguishes_an_active_recording_from_an_expired_clip() {
        let mut state = fixture_state();
        let active = status_json(&state);
        assert_eq!(active["recording_active"], true);
        assert_eq!(active["recording_id"], 42);
        assert_eq!(
            active["target"],
            serde_json::json!({"kind":"window","id":17})
        );
        state.snapshot.phase = Phase::Done;
        state.snapshot.path = Some("fixture.mp4".into());
        let ended = status_json(&state);
        assert_eq!(ended["recording_active"], false);
        assert_eq!(ended["saved_path"], "fixture.mp4");
    }
    #[test]
    fn stop_does_not_wait_for_or_accept_a_late_capture() {
        let control = Arc::new(Control {
            id: 0,
            stop: AtomicBool::new(false),
            discard: AtomicBool::new(false),
            result: Mutex::new(None),
        });
        let stopper = control.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            stopper.stop.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();
        assert!(
            bounded_capture(
                || {
                    std::thread::sleep(Duration::from_millis(200));
                    Ok(42)
                },
                &control,
                None
            )
            .unwrap()
            .is_none()
        );
        assert!(started.elapsed() < Duration::from_millis(150));
    }
    #[test]
    fn completion_reports_a_saved_stopped_clip_not_an_active_recorder() {
        let snapshot = Snapshot {
            id: 7,
            phase: Phase::Done,
            seconds: 3,
            monitor: None,
            path: Some("fixture.mp4".into()),
            error: None,
        };
        let value: serde_json::Value = serde_json::from_str(&completion_json(&snapshot)).unwrap();
        assert_eq!(value["status"], "completed");
        assert_eq!(value["recording_active"], false);
        assert_eq!(value["saved_path"], "fixture.mp4");
        assert_eq!(value["recorded_seconds"], 3);
        assert!(
            value["next_action"]
                .as_str()
                .unwrap()
                .contains("not part of this clip")
        );
    }

    #[test]
    fn duration_is_bounded() {
        assert!(validate_seconds(0).is_err());
        assert!(validate_seconds(1).is_ok());
        assert!(validate_seconds(60).is_ok());
        assert!(validate_seconds(61).is_err());
    }
    #[test]
    fn saving_never_overwrites_an_existing_file() {
        let folder =
            std::env::temp_dir().join(format!("studio-recording-copy-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let source = folder.join("source");
        let output = folder.join("output");
        fs::write(&source, b"video").unwrap();
        fs::write(&output, b"existing").unwrap();
        assert!(copy_new(&source, &output).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"existing");
        fs::remove_file(&output).unwrap();
        copy_new(&source, &output).unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"video");
        fs::remove_dir_all(folder).unwrap();
    }
}
