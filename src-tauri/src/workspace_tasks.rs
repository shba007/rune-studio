//! Durable user-managed tasks. The scheduler creates reminders, never execution.
use crate::{
    events::AppState,
    intake,
    life::{DataClass, NewRecord, RecordKind, RecordSource},
    sync::LockExt,
    task_calendar::Schedule,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};
static MUTATION: Mutex<()> = Mutex::new(());
static PRIVATE_EXECUTION: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn enter_private_context() {
    PRIVATE_EXECUTION.store(true, std::sync::atomic::Ordering::SeqCst);
}
pub fn protected_execution_context(app: &AppHandle) -> bool {
    // Dependency snapshots and delayed verifier evidence outlive board dismissal.
    // Keep this process local-only once a protected workspace run enters it.
    let _ = app;
    if crate::tasks::tasks_state()
        .requests
        .iter()
        .any(|request| request.history_id.starts_with("worktask-"))
    {
        enter_private_context();
    }
    PRIVATE_EXECUTION.load(std::sync::atomic::Ordering::SeqCst)
}
pub async fn execution_profile(
    app: &AppHandle,
    base: &str,
    key: Option<&str>,
    model: &str,
) -> Result<crate::model::profile::ModelProfile, String> {
    if protected_execution_context(app) {
        crate::model::protected::discover(&app.state::<AppState>().http, base, key, model).await
    } else {
        Ok(crate::model::profile::resolve_profile(model, None, None))
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeMetric {
    pub baseline: f64,
    pub current: f64,
    pub target: f64,
    pub unit: String,
    pub note: String,
    pub updated_at: String,
}
impl OutcomeMetric {
    pub fn validate(&self) -> Result<(), String> {
        if !self.baseline.is_finite()
            || !self.current.is_finite()
            || !self.target.is_finite()
            || self.unit.trim().is_empty()
            || self.unit.len() > 80
            || self.note.len() > 2000
            || !(self.target - self.baseline).is_finite()
            || !(self.current - self.baseline).is_finite()
        {
            Err("Enter finite values, a unit, and a note up to 2,000 bytes".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub id: String,
    pub due_ms: Option<u64>,
    pub state: String,
    pub run_id: Option<String>,
    pub reported: bool,
    pub completed_ms: Option<u64>,
    #[serde(default)]
    pub work_id: Option<String>,
    #[serde(default)]
    pub snoozed_until_ms: Option<u64>,
}
impl Occurrence {
    pub fn effective_due_ms(&self) -> Option<u64> {
        self.snoozed_until_ms.or(self.due_ms)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTask {
    #[serde(default = "agent_mode")]
    pub execution_mode: String,
    pub id: String,
    pub revision: u64,
    pub title: String,
    pub parent_goal_id: Option<String>,
    pub class: DataClass,
    pub created_ms: u64,
    pub schedule: Option<Schedule>,
    pub paused: bool,
    pub occurrences: Vec<Occurrence>,
    pub error: Option<String>,
    #[serde(default)]
    pub legacy_task_id: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default = "remind_action")]
    pub due_action: String,
}
fn agent_mode() -> String {
    "agent".into()
}
fn remind_action() -> String {
    "remind".into()
}
fn check_due_action(action: &str, mode: &str, scheduled: bool) -> Result<(), String> {
    match action {
        "remind" => Ok(()),
        "run" if mode == "agent" && scheduled => Ok(()),
        "run" => Err("Only scheduled tasks that Rune executes can run automatically".into()),
        _ => Err("Choose to be reminded or to run automatically".into()),
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    #[serde(flatten)]
    pub task: WorkspaceTask,
    pub completed_steps: u64,
    pub total_steps: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInput {
    pub local: String,
    pub zone: String,
    pub repeat: String,
    pub anchor_day: Option<u32>,
}
fn make_schedule(input: ScheduleInput) -> Result<Schedule, String> {
    let mut schedule = Schedule::create(input.local, input.zone, input.repeat, now())?;
    if let Some(day) = input.anchor_day {
        if !(1..=31).contains(&day) {
            return Err("Monthly day must be between 1 and 31".into());
        }
        schedule.anchor_day = day;
    }
    Ok(schedule)
}
fn load(app: &AppHandle) -> Result<Vec<WorkspaceTask>, String> {
    if !app.state::<AppState>().settings.locked().life_enabled {
        return Ok(vec![]);
    }
    let store = intake::store(app)?;
    let mut latest: HashMap<String, WorkspaceTask> = HashMap::new();
    for class in DataClass::all()
        .iter()
        .filter(|c| intake::consent(app, **c))
    {
        for record in store
            .list_records(Some(*class), false)
            .map_err(|e| e.to_string())?
        {
            if !record.id.starts_with("worktask-") {
                continue;
            }
            let plain = store
                .crypto()
                .decrypt(&record.body)
                .map_err(|e| e.to_string())?;
            let task: WorkspaceTask = serde_json::from_str(&plain).map_err(|e| e.to_string())?;
            if latest
                .get(&task.id)
                .is_none_or(|old| old.revision < task.revision)
            {
                latest.insert(task.id.clone(), task);
            }
        }
    }
    let mut tasks = latest.into_values().collect::<Vec<_>>();
    tasks.sort_by_key(|t| t.created_ms);
    Ok(tasks)
}
fn save(app: &AppHandle, task: &mut WorkspaceTask) -> Result<(), String> {
    if !intake::consent(app, task.class) {
        return Err("Enable protected storage consent for this task's privacy category in Settings → Data & privacy.".into());
    }
    task.revision += 1;
    let store = intake::store(app)?;
    let body = store
        .crypto()
        .encrypt(&serde_json::to_string(task).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    store
        .write_record(
            NewRecord {
                id: Some(format!("{}-v{}", task.id, task.revision)),
                class: task.class,
                kind: RecordKind::Note,
                body,
                source: RecordSource::UserSaid,
                grade: 0,
                occurred_utc: None,
                supports_json: None,
            },
            "studio-workspace",
        )
        .map_err(|e| e.to_string())?;
    let _ = app.emit("workspace_tasks_changed", ());
    Ok(())
}
fn parent(app: &AppHandle, id: Option<&str>, fallback: DataClass) -> Result<DataClass, String> {
    let Some(id) = id else { return Ok(fallback) };
    intake::decisions_state(app.clone())?
        .into_iter()
        .find(|g| g.id == id && g.kind == "goal" && g.approved_goal.is_some())
        .map(|g| g.class)
        .ok_or("Choose an existing approved goal with storage consent".into())
}
fn checked(app: &AppHandle, id: &str, revision: u64) -> Result<WorkspaceTask, String> {
    let t = load(app)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or("This task is unavailable")?;
    if t.revision != revision {
        return Err("Conflict: this task changed. Reload before applying your decision.".into());
    }
    Ok(t)
}
fn occurrence(due_ms: Option<u64>) -> Result<Occurrence, String> {
    Ok(Occurrence {
        id: intake::uid()?,
        due_ms,
        state: "pending".into(),
        run_id: None,
        reported: false,
        completed_ms: None,
        work_id: None,
        snoozed_until_ms: None,
    })
}
#[tauri::command]
pub fn workspace_task_import(
    app: AppHandle,
    run_id: String,
    parent_goal_id: Option<String>,
    class: DataClass,
    schedule: Option<ScheduleInput>,
) -> Result<String, String> {
    let _guard = MUTATION.locked();
    let state = app.state::<AppState>();
    let db = state.trace.db_path().ok_or("Task history unavailable")?;
    let root = crate::domain::task_id(db, &run_id).map_err(|e| e.to_string())?;
    if let Some(existing) = load(&app)?.into_iter().find(|t| {
        t.occurrences
            .iter()
            .any(|o| o.run_id.as_deref() == Some(&root))
            || t.legacy_task_id.as_deref() == Some(&root)
    }) {
        return Err(format!(
            "This task is already managed as {}. Reload and open its current record.",
            existing.id
        ));
    }
    let original = crate::flight::header(db, &root)
        .map_err(|e| e.to_string())?
        .ok_or("Task history not found")?;
    if !matches!(
        original.source.as_str(),
        "" | "built-in" | "task-request" | "screen-insight" | "rune"
    ) {
        return Err("External-agent tasks remain separate".into());
    }
    let class = parent(&app, parent_goal_id.as_deref(), class)?;
    let instructions = crate::flight::original_prompt(db, &root).map_err(|e| e.to_string())?;
    let is_analysis = original.analysis.is_some();
    if schedule.is_some() && instructions.is_none() && !is_analysis {
        return Err("The original prompt was not retained. Create a new task with explicit instructions to schedule it.".into());
    }
    let attempts = crate::domain::attempts(db, &root).map_err(|e| e.to_string())?;
    let execution = attempts.last().cloned().unwrap_or_else(|| root.clone());
    let header = crate::flight::header(db, &execution)
        .map_err(|e| e.to_string())?
        .ok_or("Execution history unavailable")?;
    if header.outcome.is_none() || header.outcome.as_deref() == Some("open") {
        return Err("Finish or stop this execution before organizing its history".into());
    }
    let read_time = |time: Option<&str>| -> Option<u64> {
        crate::trace::store::open_read(db)
            .ok()?
            .query_row(
                "SELECT CAST(strftime('%s',?1) AS INTEGER)*1000",
                [time?],
                |row| row.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten()
            .and_then(|time| u64::try_from(time).ok())
    };
    let state = match header.outcome.as_deref() {
        Some("succeeded") => "completed",
        Some("failed") => "failed",
        Some("stopped") => "cancelled",
        _ => "running",
    };
    let mut task = WorkspaceTask {
        execution_mode: if is_analysis { "analysis" } else { "agent" }.into(),
        id: format!("worktask-{}", intake::uid()?),
        revision: 0,
        title: original.task,
        parent_goal_id,
        class,
        created_ms: read_time(Some(&original.started_utc)).unwrap_or_else(now),
        schedule: schedule.map(make_schedule).transpose()?,
        paused: false,
        occurrences: vec![Occurrence {
            id: intake::uid()?,
            due_ms: None,
            state: state.into(),
            run_id: Some(execution),
            reported: false,
            completed_ms: read_time(header.ended_utc.as_deref()),
            work_id: None,
            snoozed_until_ms: None,
        }],
        error: None,
        legacy_task_id: Some(root),
        instructions,
        due_action: remind_action(),
    };
    save(&app, &mut task)?;
    Ok(task.id)
}
#[tauri::command]
pub fn workspace_task_schedule(
    app: AppHandle,
    id: String,
    revision: u64,
    schedule: Option<ScheduleInput>,
) -> Result<(), String> {
    let _guard = MUTATION.locked();
    let mut task = checked(&app, &id, revision)?;
    if schedule.is_some()
        && task.legacy_task_id.is_some()
        && task.instructions.is_none()
        && task.execution_mode != "analysis"
    {
        return Err("This older task has no retained original instructions. Create a new task to schedule it.".into());
    }
    if task.occurrences.last().is_some_and(|o| {
        matches!(
            o.state.as_str(),
            "running" | "dispatching" | "queued" | "planning"
        )
    }) {
        return Err("Wait for the active occurrence before changing its schedule".into());
    }
    let next = schedule.map(make_schedule).transpose()?;
    if let Some(current) = task.occurrences.last_mut().filter(|o| o.state == "pending") {
        current.state = "rescheduled".into();
        current.completed_ms = Some(now());
    }
    task.schedule = next;
    if task.schedule.is_none() {
        task.due_action = remind_action();
    }
    task.paused = false;
    task.error = None;
    if task.schedule.is_none() {
        task.occurrences.push(occurrence(None)?);
    }
    save(&app, &mut task)
}
#[tauri::command]
pub fn workspace_task_create(
    app: AppHandle,
    title: String,
    parent_goal_id: Option<String>,
    class: DataClass,
    schedule: Option<ScheduleInput>,
    execution_mode: Option<String>,
    due_action: Option<String>,
) -> Result<String, String> {
    let _guard = MUTATION.locked();
    if title.trim().is_empty() || title.chars().count() > 4000 {
        return Err("Enter a task of 1 to 4,000 characters".into());
    }
    let class = parent(&app, parent_goal_id.as_deref(), class)?;
    let mode = execution_mode.unwrap_or_else(agent_mode);
    if !matches!(mode.as_str(), "manual" | "agent" | "analysis") {
        return Err("Choose manual activity, Rune execution, or Visual Analysis".into());
    }
    let schedule = schedule.map(make_schedule).transpose()?;
    let due_action = due_action.unwrap_or_else(remind_action);
    check_due_action(&due_action, &mode, schedule.is_some())?;
    let mut task = WorkspaceTask {
        due_action,
        execution_mode: mode,
        id: format!("worktask-{}", intake::uid()?),
        revision: 0,
        title: title.trim().into(),
        parent_goal_id,
        class,
        created_ms: now(),
        schedule,
        paused: false,
        occurrences: vec![],
        error: None,
        legacy_task_id: None,
        instructions: Some(title.trim().into()),
    };
    if task.schedule.is_none() {
        task.occurrences.push(occurrence(None)?);
    }
    save(&app, &mut task)?;
    Ok(task.id)
}
pub(crate) fn proposal_context(
    app: &AppHandle,
    goal_id: &str,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Array(load(app)?.into_iter().filter(|t| t.parent_goal_id.as_deref() == Some(goal_id)).map(|t| serde_json::json!({"title":t.title,"instructions":t.instructions,"mode":t.execution_mode,"schedule":t.schedule,"latestState":t.occurrences.last().map(|o| &o.state)})).collect()))
}
pub(crate) fn create_proposed(
    app: &AppHandle,
    id: String,
    goal_id: &str,
    class: DataClass,
    approval: crate::task_proposals::Approval,
) -> Result<WorkspaceTask, String> {
    let _guard = MUTATION.locked();
    if let Some(existing) = load(app)?.into_iter().find(|t| t.id == id) {
        if existing.parent_goal_id.as_deref() != Some(goal_id) || existing.class != class {
            return Err(
                "Conflict: this proposal's task belongs to a different goal or privacy category"
                    .into(),
            );
        }
        return Ok(existing);
    }
    approval.validate()?;
    let class = parent(app, Some(goal_id), class)?;
    let schedule = approval.schedule.map(make_schedule).transpose()?;
    let mut task = WorkspaceTask {
        due_action: remind_action(),
        id,
        revision: 0,
        title: approval.title.trim().into(),
        instructions: Some(approval.instructions.trim().into()),
        execution_mode: approval.execution_mode,
        parent_goal_id: Some(goal_id.into()),
        class,
        created_ms: now(),
        schedule,
        paused: false,
        occurrences: vec![],
        error: None,
        legacy_task_id: None,
    };
    if task.schedule.is_none() {
        task.occurrences.push(occurrence(None)?);
    }
    save(app, &mut task)?;
    Ok(task)
}
fn progress(app: &AppHandle, task: &WorkspaceTask) -> (u64, u64) {
    let Some(run) = task.occurrences.last().and_then(|o| o.run_id.as_ref()) else {
        return (0, 0);
    };
    progress_run(app, run)
}
fn progress_run(app: &AppHandle, run: &str) -> (u64, u64) {
    let state = app.state::<AppState>();
    let Some(db) = state.trace.db_path() else {
        return (0, 0);
    };
    let Ok(history) = crate::trace::store::task_history(db, run) else {
        return (0, 0);
    };
    let Some(tasks) = history
        .last()
        .and_then(|v| v["snapshot"]["tasks"].as_array())
    else {
        return (0, 0);
    };
    let steps = tasks
        .iter()
        .flat_map(|t| t["steps"].as_array().into_iter().flatten());
    let mut done = 0;
    let mut total = 0;
    for step in steps {
        total += 1;
        if step["phase"] == "done" {
            done += 1;
        }
    }
    (done, total)
}
#[tauri::command]
pub fn workspace_history_progress(
    app: AppHandle,
    ids: Vec<String>,
) -> Result<HashMap<String, (u64, u64)>, String> {
    if ids.len() > 200 {
        return Err("Request at most 200 task summaries".into());
    }
    Ok(ids
        .into_iter()
        .map(|id| {
            let latest = app
                .state::<AppState>()
                .trace
                .db_path()
                .and_then(|db| crate::domain::attempts(db, &id).ok())
                .and_then(|ids| ids.last().cloned())
                .unwrap_or_else(|| id.clone());
            let p = progress_run(&app, &latest);
            (id, p)
        })
        .collect())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItem {
    pub task_id: String,
    pub at_ms: u64,
    pub forecast: bool,
    pub state: String,
}
#[tauri::command]
pub fn workspace_calendar(
    app: AppHandle,
    from_ms: u64,
    to_ms: u64,
) -> Result<Vec<CalendarItem>, String> {
    if to_ms <= from_ms || to_ms - from_ms > 93 * 86_400_000 {
        return Err("Choose a calendar range of at most 93 days".into());
    }
    let _guard = MUTATION.locked();
    let mut items = vec![];
    for task in load(&app)? {
        let mut seen = std::collections::HashSet::new();
        for o in task.occurrences.iter().rev() {
            if !seen.insert(o.work_id.as_deref().unwrap_or(&o.id)) {
                continue;
            }
            if let Some(at) = o.due_ms.filter(|at| *at >= from_ms && *at < to_ms)
                && o.state != "rescheduled"
            {
                items.push(CalendarItem {
                    task_id: task.id.clone(),
                    at_ms: at,
                    forecast: false,
                    state: o.state.clone(),
                });
            }
        }
        if task.paused {
            continue;
        }
        if let Some(mut schedule) = task.schedule.clone() {
            if schedule.next_ms < from_ms && !schedule.next_after(from_ms.saturating_sub(1))? {
                continue;
            }
            for _ in 0..94 {
                let at = schedule.next_ms;
                if at >= to_ms {
                    break;
                }
                if at >= from_ms {
                    items.push(CalendarItem {
                        task_id: task.id.clone(),
                        at_ms: at,
                        forecast: true,
                        state: "scheduled".into(),
                    });
                }
                if !schedule.next_after(at)? {
                    break;
                }
            }
        }
    }
    items.sort_by_key(|i| i.at_ms);
    Ok(items)
}
#[tauri::command]
pub fn workspace_tasks_state(app: AppHandle) -> Result<Vec<TaskView>, String> {
    let _guard = MUTATION.locked();
    Ok(load(&app)?
        .into_iter()
        .map(|task| {
            let (completed_steps, total_steps) = progress(&app, &task);
            TaskView {
                task,
                completed_steps,
                total_steps,
            }
        })
        .collect())
}
fn snooze_occurrence(task: &mut WorkspaceTask, minutes: u64, at: u64) -> Result<(), String> {
    if !(1..=7 * 1440).contains(&minutes) {
        return Err("Choose a snooze of up to 7 days".into());
    }
    let current = task
        .occurrences
        .last_mut()
        .filter(|o| o.state == "pending" && o.effective_due_ms().is_some())
        .ok_or("Only a pending scheduled reminder can be snoozed")?;
    current.snoozed_until_ms = Some(at + minutes * 60_000);
    Ok(())
}
#[tauri::command]
pub fn workspace_task_snooze(
    app: AppHandle,
    id: String,
    revision: u64,
    minutes: u64,
) -> Result<(), String> {
    let _guard = MUTATION.locked();
    let mut task = checked(&app, &id, revision)?;
    snooze_occurrence(&mut task, minutes, now())?;
    save(&app, &mut task)
}
#[tauri::command]
pub fn workspace_task_due_action(
    app: AppHandle,
    id: String,
    revision: u64,
    action: String,
) -> Result<(), String> {
    let _guard = MUTATION.locked();
    let mut task = checked(&app, &id, revision)?;
    check_due_action(&action, &task.execution_mode, task.schedule.is_some())?;
    task.due_action = action;
    save(&app, &mut task)
}
#[tauri::command]
pub fn workspace_task_zone() -> Result<String, String> {
    crate::task_calendar::current_zone()
}
#[tauri::command]
pub fn workspace_task_link(
    app: AppHandle,
    id: String,
    revision: u64,
    parent_goal_id: Option<String>,
) -> Result<(), String> {
    let _guard = MUTATION.locked();
    let mut task = checked(&app, &id, revision)?;
    let class = parent(&app, parent_goal_id.as_deref(), task.class)?;
    if task.class != class {
        return Err("The goal and task have different privacy categories. Create or organize a task under that goal's category instead.".into());
    }
    task.parent_goal_id = parent_goal_id;
    save(&app, &mut task)
}
#[tauri::command]
pub async fn workspace_task_action(
    app: AppHandle,
    id: String,
    revision: u64,
    action: String,
) -> Result<(), String> {
    // A click on Run is required even after the due time; existing task safety gates still apply.
    if action == "run" {
        let state = app.state::<AppState>();
        let (settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
        if settings.external_agent {
            return Err(
                "Use API mode to run Studio tasks; external-agent sessions remain separate".into(),
            );
        }
        crate::model::protected::discover(
            &app.state::<AppState>().http,
            &settings.base_url,
            key.as_deref(),
            settings.model_for_role(crate::model::role::ModelRole::Agent),
        )
        .await?;
    }
    let _guard = MUTATION.locked();
    let mut task = checked(&app, &id, revision)?;
    task.error = None;
    if action == "run" && task.legacy_task_id.is_some() && task.instructions.is_none() {
        return Err("The original instructions are unavailable; create a new task instead of reconstructing this older one.".into());
    }
    if action == "run" && task.execution_mode != "agent" {
        return Err("Use this task's manual or capture action instead of computer tools".into());
    }
    if action == "pause" || action == "resume" {
        task.paused = action == "pause";
        return save(&app, &mut task);
    }
    let current = task
        .occurrences
        .last_mut()
        .ok_or("No occurrence is awaiting a decision")?;
    if current.state != "pending"
        && !(current.state == "in_progress"
            && matches!(action.as_str(), "done" | "skip" | "analysis"))
        && !(matches!(action.as_str(), "retry" | "skip")
            && matches!(
                current.state.as_str(),
                "failed" | "interrupted" | "cancelled"
            ))
    {
        return Err("This occurrence is not awaiting this action".into());
    }
    match action.as_str() {
        "start" => {
            if task.execution_mode != "manual" {
                return Err("This is not a manual activity".into());
            }
            current.state = "in_progress".into();
            save(&app, &mut task)
        }
        "analysis" => {
            let state = app.state::<AppState>();
            if task.execution_mode != "analysis" || state.settings.locked().external_agent {
                return Err("Use API Visual Analysis mode for this task".into());
            }
            if crate::events::agent_state(&app).is_running()
                || matches!(
                    state.analysis.locked().status,
                    crate::analysis::report::Status::Capturing
                        | crate::analysis::report::Status::Asking
                )
            {
                return Err(
                    "Finish or stop the current operation before selecting a new capture".into(),
                );
            }
            current.state = "in_progress".into();
            save(&app, &mut task)?;
            enter_private_context();
            if let Some(window) = app.get_webview_window(crate::window::MAIN) {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
            app.emit(
                "workspace_analysis_request",
                task.instructions.as_deref().unwrap_or(""),
            )
            .map_err(|e| e.to_string())
        }
        "run" => {
            let run = format!("{}-run-{}", task.id, current.id);
            current.run_id = Some(run.clone());
            current.state = "dispatching".into();
            save(&app, &mut task)?;
            match crate::tasks::submit_with_id(
                &app,
                task.instructions.as_deref().unwrap_or(&task.title),
                false,
                Some(&run),
                None,
                None,
            ) {
                Ok(_) => {
                    task.occurrences.last_mut().unwrap().state = "queued".into();
                    save(&app, &mut task)
                }
                Err(error) => {
                    let o = task.occurrences.last_mut().unwrap();
                    o.state = "pending".into();
                    o.run_id = None;
                    task.error = Some(error.clone());
                    save(&app, &mut task)?;
                    Err(error)
                }
            }
        }
        "done" | "skip" => {
            current.state = if action == "done" {
                "completed"
            } else {
                "skipped"
            }
            .into();
            current.reported = action == "done";
            current.completed_ms = Some(now());
            save(&app, &mut task)
        }
        "retry" => {
            let due = current.due_ms;
            let work = current
                .work_id
                .clone()
                .unwrap_or_else(|| current.id.clone());
            let mut retry = occurrence(due)?;
            retry.work_id = Some(work);
            task.occurrences.push(retry);
            save(&app, &mut task)
        }
        _ => Err("Unsupported task action".into()),
    }
}
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            match tick(&app) {
                Ok(all) => crate::alarms::refresh(&app, &all, now()),
                Err(error) => log::debug!("Workspace scheduler unavailable: {error}"),
            }
        }
    });
}
fn materialize_due(task: &mut WorkspaceTask, at: u64) -> Result<bool, String> {
    let waiting = task.occurrences.last().is_some_and(|o| {
        matches!(
            o.state.as_str(),
            "pending"
                | "in_progress"
                | "running"
                | "dispatching"
                | "queued"
                | "planning"
                | "interrupted"
                | "failed"
                | "cancelled"
        )
    });
    if task.paused || waiting {
        return Ok(false);
    }
    let Some(schedule) = &mut task.schedule else {
        return Ok(false);
    };
    if schedule.next_ms > at {
        return Ok(false);
    }
    let due = schedule.next_ms;
    match schedule.next_after(at) {
        Ok(false) => task.schedule = None,
        Ok(true) => {}
        Err(error) => {
            task.error = Some(error);
            task.paused = true;
        }
    }
    task.occurrences.push(occurrence(Some(due))?);
    Ok(true)
}
fn tick(app: &AppHandle) -> Result<Vec<WorkspaceTask>, String> {
    let _guard = MUTATION.locked();
    let state = app.state::<AppState>();
    let snapshot = crate::tasks::tasks_state();
    let mut all = Vec::new();
    for mut task in load(app)? {
        let mut changed = false;
        if let Some(o) = task.occurrences.last_mut().filter(|o| {
            matches!(
                o.state.as_str(),
                "running" | "dispatching" | "queued" | "planning"
            )
        }) && let Some(run) = &o.run_id
        {
            let live = snapshot.requests.iter().find(|r| &r.history_id == run);
            let outcome = state
                .trace
                .db_path()
                .and_then(|db| crate::flight::header(db, run).ok().flatten())
                .and_then(|h| h.outcome);
            if let Some(live) = live {
                let phase = match live.phase {
                    crate::tasks::Phase::Queued => "queued",
                    crate::tasks::Phase::Planning => "planning",
                    crate::tasks::Phase::Running => "running",
                    crate::tasks::Phase::Done => "completed",
                    crate::tasks::Phase::Failed | crate::tasks::Phase::Blocked => "failed",
                    crate::tasks::Phase::Cancelled => "cancelled",
                };
                if o.state != phase {
                    o.state = phase.into();
                    changed = true;
                    if matches!(phase, "completed" | "failed" | "cancelled") {
                        o.completed_ms = Some(now());
                    }
                }
            }
            if let Some(outcome) = outcome {
                if outcome != "open" {
                    o.state = match outcome.as_str() {
                        "succeeded" => "completed",
                        "stopped" => "cancelled",
                        _ => "failed",
                    }
                    .into();
                    o.completed_ms = Some(now());
                    changed = true;
                }
            } else if live.is_none() {
                o.state = "interrupted".into();
                task.error =
                    Some("Execution was interrupted. Review its history before retrying.".into());
                changed = true;
            }
        }
        changed |= materialize_due(&mut task, now())?;
        if changed {
            save(app, &mut task)?;
            let _ = app.emit("workspace_tasks_due", ());
        }
        all.push(task);
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_running_is_limited_to_scheduled_rune_tasks() {
        assert!(check_due_action("remind", "manual", false).is_ok());
        assert!(check_due_action("run", "agent", true).is_ok());
        assert!(check_due_action("run", "agent", false).is_err());
        assert!(check_due_action("run", "manual", true).is_err());
        assert!(check_due_action("run", "analysis", true).is_err());
        assert!(check_due_action("later", "agent", true).is_err());
    }
    #[test]
    fn older_tasks_without_the_field_default_to_remind() {
        let task = scheduled();
        assert_eq!(task.due_action, "remind");
    }
    #[test]
    fn snoozing_moves_the_effective_due_time_without_losing_the_original() {
        let mut task = scheduled();
        task.occurrences.push(occurrence(Some(1_000)).unwrap());
        snooze_occurrence(&mut task, 10, 5_000).unwrap();
        let o = task.occurrences.last().unwrap();
        assert_eq!(o.due_ms, Some(1_000));
        assert_eq!(o.effective_due_ms(), Some(5_000 + 600_000));
        assert!(snooze_occurrence(&mut task, 0, 0).is_err());
        assert!(snooze_occurrence(&mut task, 7 * 1440 + 1, 0).is_err());
        task.occurrences.last_mut().unwrap().state = "completed".into();
        assert!(snooze_occurrence(&mut task, 10, 0).is_err());
    }
    fn scheduled() -> WorkspaceTask {
        serde_json::from_value(serde_json::json!({
            "id":"worktask-test", "revision":1, "title":"Walk", "parentGoalId":null,
            "class":"general", "createdMs":0, "paused":false, "occurrences":[], "error":null,
            "schedule":{"local":"2026-10-27T07:00", "zone":"UTC", "repeat":"once",
            "nextLocal":"2026-10-27T07:00", "nextMs":100, "anchorDay":27}
        }))
        .unwrap()
    }
    #[test]
    fn overdue_reminder_survives_restart_without_dispatching_or_duplicating() {
        let mut task = scheduled();
        assert!(!materialize_due(&mut task, 99).unwrap());
        assert!(materialize_due(&mut task, 500).unwrap());
        let mut restored: WorkspaceTask =
            serde_json::from_str(&serde_json::to_string(&task).unwrap()).unwrap();
        assert!(!materialize_due(&mut restored, 1000).unwrap());
        assert!(restored.schedule.is_none());
        assert_eq!(restored.occurrences.len(), 1);
        let reminder = &restored.occurrences[0];
        assert_eq!(reminder.state, "pending");
        assert_eq!(reminder.due_ms, Some(100));
        assert!(reminder.run_id.is_none());
        assert!(!reminder.reported);
    }
    #[test]
    fn paused_or_unresolved_work_never_creates_another_reminder() {
        let mut task = scheduled();
        task.paused = true;
        assert!(!materialize_due(&mut task, 500).unwrap());
        task.paused = false;
        for state in [
            "pending",
            "in_progress",
            "queued",
            "running",
            "failed",
            "interrupted",
            "cancelled",
        ] {
            let mut o = occurrence(Some(1)).unwrap();
            o.state = state.into();
            task.occurrences = vec![o];
            assert!(!materialize_due(&mut task, 500).unwrap(), "{state}");
        }
    }
}
