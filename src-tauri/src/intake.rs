//! Resumable input decisions. Bodies live only in the consented, protected Life store.
use crate::{
    events::AppState,
    exec::CancelToken,
    life::{DataClass, LifeCrypto, LifeStore, NewRecord, RecordKind, RecordSource},
    sync::LockExt,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{AppHandle, Emitter, Manager};

static ENTRIES: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(Default::default);
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionItem {
    id: String,
    label: String,
    explanation: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    id: String,
    revision: u64,
    kind: String,
    title: String,
    explanation: String,
    options: Vec<OptionItem>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub(crate) id: String,
    pub(crate) revision: u64,
    text: String,
    pub(crate) class: DataClass,
    pub(crate) kind: String,
    status: String,
    #[serde(default)]
    category: String,
    question: Option<Question>,
    answers: Vec<Value>,
    goal: Option<Value>,
    #[serde(default)]
    pub(crate) approved_goal: Option<Value>,
    #[serde(default)]
    outcome_metric: Option<crate::workspace_tasks::OutcomeMetric>,
    #[serde(default)]
    requested_kind: Option<String>,
    #[serde(default)]
    pathways: Option<crate::intake_pathways::PathwaySet>,
    #[serde(default)]
    task_proposals: Option<crate::task_proposals::Draft>,
    error: Option<String>,
    created: String,
    task_id: Option<String>,
    attempt_id: Option<String>,
    #[serde(default)]
    confirm_classification: bool,
    #[serde(default)]
    persistable: bool,
    #[serde(skip)]
    token: CancelToken,
    #[serde(skip)]
    keep_mic: bool,
    #[serde(skip)]
    voice_wav: Vec<u8>,
}
pub(crate) fn uid() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(format!(
        "intake-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
pub(crate) fn consent(app: &AppHandle, class: DataClass) -> bool {
    let s = app.state::<AppState>();
    let s = s.settings.locked();
    s.life_enabled
        && s.life_consents
            .get(class.as_str())
            .copied()
            .unwrap_or(false)
}
pub(crate) fn store(app: &AppHandle) -> Result<LifeStore, String> {
    let path = crate::life::db_path(&app.path().app_config_dir().map_err(|e| e.to_string())?);
    let crypto = if path.exists() {
        LifeCrypto::from_keyring_existing()
    } else {
        LifeCrypto::from_keyring_or_create()
    }
    .map_err(|e| e.to_string())?;
    let store = LifeStore::open(&path, crypto).map_err(|e| e.to_string())?;
    for class in DataClass::all() {
        let enabled = consent(app, *class);
        if store.get_consent(*class).map_err(|e| e.to_string())? != enabled {
            store
                .set_consent(*class, enabled, "studio")
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(store)
}
fn commit(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    if matches!(e.status.as_str(), "submitted" | "cancelled" | "goal_ready") {
        e.voice_wav.clear();
    }
    e.revision += 1;
    if let Some(q) = &mut e.question {
        q.id = e.id.clone();
        q.revision = e.revision;
    }
    if e.persistable && consent(app, e.class) {
        let store = store(app)?;
        store_entry(&store, e)?;
    }
    ENTRIES.locked().insert(e.id.clone(), e.clone());
    let _ = app.emit("decisions_changed", ());
    Ok(())
}
fn store_entry(store: &LifeStore, e: &Entry) -> Result<(), String> {
    let body = store
        .crypto()
        .encrypt(&serde_json::to_string(e).map_err(|err| err.to_string())?)
        .map_err(|err| err.to_string())?;
    store
        .write_record(
            NewRecord {
                id: Some(format!("{}-{}", e.id, e.revision)),
                class: e.class,
                kind: RecordKind::Note,
                body,
                source: RecordSource::UserSaid,
                grade: 0,
                occurred_utc: None,
                supports_json: None,
            },
            "studio-intake",
        )
        .map_err(|err| err.to_string())?;
    Ok(())
}
fn decode_entry(store: &LifeStore, body: &str) -> Result<Entry, String> {
    let decoded = store
        .crypto()
        .decrypt(body)
        .map_err(|err| err.to_string())?;
    let mut entry: Entry = serde_json::from_str(&decoded).map_err(|err| err.to_string())?;
    if entry.status == "dispatching" {
        entry.status = "interrupted".into();
        entry.error=Some("Execution dispatch was interrupted. Inspect task history before submitting another attempt.".into());
    }
    if matches!(entry.status.as_str(), "pathway_planning" | "task_planning") {
        entry.status = "goal_ready".into();
        entry.error=Some("Planning was interrupted. Earlier approved choices and drafts are preserved; retry when ready.".into());
    }
    Ok(entry)
}
fn load(app: &AppHandle) -> Result<(), String> {
    if !app.state::<AppState>().settings.locked().life_enabled {
        return Ok(());
    }
    let store = store(app)?;
    for class in DataClass::all().iter().filter(|c| consent(app, **c)) {
        for r in store
            .list_records(Some(*class), false)
            .map_err(|e| e.to_string())?
        {
            if !r.id.starts_with("intake-") {
                continue;
            }
            let entry = decode_entry(&store, &r.body)?;
            let mut entries = ENTRIES.locked();
            if entries
                .get(&entry.id)
                .is_none_or(|old| old.revision < entry.revision)
            {
                entries.insert(entry.id.clone(), entry);
            }
        }
    }
    Ok(())
}
#[tauri::command]
pub fn decisions_state(app: AppHandle) -> Result<Vec<Entry>, String> {
    load(&app)?;
    let mut entries = ENTRIES
        .locked()
        .values()
        .filter(|e| {
            e.status != "cancelled"
                && e.status != "submitted"
                && (matches!(e.status.as_str(), "consent" | "retry" | "classifying")
                    || consent(&app, e.class))
        })
        .cloned()
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.created.cmp(&b.created).then(a.id.cmp(&b.id)));
    Ok(entries)
}
fn question(e: &mut Entry, kind: &str, title: &str, explanation: &str, options: Vec<OptionItem>) {
    e.status = kind.into();
    e.question = Some(Question {
        id: e.id.clone(),
        revision: e.revision,
        kind: kind.into(),
        title: title.into(),
        explanation: explanation.into(),
        options,
    });
}
fn option(id: &str, label: &str, explanation: &str) -> OptionItem {
    OptionItem {
        id: id.into(),
        label: label.into(),
        explanation: explanation.into(),
    }
}
fn require_consent(app: &AppHandle, e: &mut Entry) -> Result<bool, String> {
    if consent(app, e.class) {
        return Ok(false);
    }
    let class = e.class.as_str();
    question(
        e,
        "consent",
        "Save this conversation securely?",
        &format!(
            "This request needs {class} context storage to resume after restart. Sensitive records are encrypted and local-only. Nothing is saved before you opt in."
        ),
        vec![
            option(
                "enable",
                &format!("Enable {class} storage and continue"),
                "Allow this domain to be saved in the protected Life store.",
            ),
            option(
                "settings",
                "Review storage settings",
                "Review domain permissions before continuing.",
            ),
            option(
                "cancel",
                "Cancel this request",
                "Do not save this conversation.",
            ),
        ],
    );
    commit(app, e)?;
    Ok(true)
}
async fn infer(
    app: &AppHandle,
    e: &Entry,
    system: &str,
    user: &Value,
    schema: &Value,
) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let settings = state.settings.locked().clone();
    let key = crate::settings::api_key();
    let model = settings.model_for_role(crate::model::role::ModelRole::Agent);
    let profile = crate::agent::unless_stopped(
        &e.token,
        crate::model::protected::discover(&state.http, &settings.base_url, key.as_deref(), model),
    )
    .await
    .ok_or("Request interrupted")??;
    let client = crate::model::client::ChatClient {
        http: &state.http,
        base_url: &settings.base_url,
        api_key: key.as_deref(),
        model,
        profile: Some(&profile),
        sink: None,
        no_swap: false,
        on_loading: None,
    };
    let text = user.to_string();
    let system = format!("{system}{}", crate::personalities::style(app, &e.category)?);
    let request = crate::model::client::Request {
        system: &system,
        user_text: &text,
        max_tokens: Some(2500),
        data_class: Some(DataClass::Finance),
        background: false,
        ..Default::default()
    };
    crate::agent::unless_stopped(&e.token, client.call_json_retrying(&request, schema))
        .await
        .ok_or("Request interrupted")?
        .map_err(|err| err.to_string())
}
async fn classify(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    e.persistable = false;
    let schema = json!({"type":"object","additionalProperties":false,"properties":{"kind":{"type":"string","enum":["goal","task","step"]},"class":{"type":"string","enum":["general","health","mind","finance"]},"category":{"type":"string"},"ambiguous":{"type":"boolean"},"reason":{"type":"string"}},"required":["kind","class","category","ambiguous","reason"]});
    let value = infer(
        app,
        e,
        include_str!("../prompts/intake/classify.md"),
        &json!({"input":e.text,"answers":e.answers}),
        &schema,
    )
    .await?;
    if e.token.is_cancelled() {
        return Err("Request interrupted".into());
    }
    e.kind = value["kind"]
        .as_str()
        .ok_or("Missing input classification")?
        .into();
    e.category = value["category"]
        .as_str()
        .unwrap_or("general")
        .chars()
        .take(80)
        .collect();
    e.class = serde_json::from_value(value["class"].clone()).map_err(|err| err.to_string())?;
    if let Some(kind) = e.requested_kind.take() {
        e.kind = kind;
    }
    e.persistable = true;
    if !matches!(e.kind.as_str(), "goal" | "task" | "step") {
        return Err("Invalid input classification".into());
    }
    let confirm = app
        .state::<AppState>()
        .settings
        .locked()
        .input_classification
        == crate::settings::InputClassification::AlwaysConfirm
        || value["ambiguous"] != false;
    e.confirm_classification = confirm;
    if e.kind == "task" && !confirm {
        return dispatch(app, e, None);
    }
    e.status = if confirm {
        "classification"
    } else {
        "continue"
    }
    .into();
    if require_consent(app, e)? {
        return Ok(());
    }
    if confirm {
        classification_question(
            e,
            value["reason"]
                .as_str()
                .unwrap_or("Choose the intended scope."),
        );
        commit(app, e)
    } else {
        advance(app, e).await
    }
}
fn classification_question(e: &mut Entry, reason: &str) {
    let mut kinds = vec![e.kind.clone()];
    for kind in ["goal", "task", "step"] {
        if kind != e.kind {
            kinds.push(kind.into());
        }
    }
    let options = kinds
        .iter()
        .map(|kind| match kind.as_str() {
            "goal" => option(
                "goal",
                "Goal",
                "A lasting outcome that needs clarification and a pathway.",
            ),
            "step" => option(
                "step",
                "Step",
                "An additional action attached to an existing task.",
            ),
            _ => option(
                "task",
                "Task",
                "A bounded piece of work to execute under existing permissions.",
            ),
        })
        .collect();
    question(
        e,
        "classification",
        "How should Rune treat this input?",
        reason,
        options,
    );
}
async fn advance(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    if e.token.is_cancelled() {
        return Err("Request interrupted".into());
    }
    if require_consent(app, e)? {
        return Ok(());
    }
    match e.kind.as_str() {
        "task" => dispatch(app, e, None),
        "step" => attach_question(app, e),
        "goal" => {
            e.voice_wav.clear();
            clarify(app, e).await
        }
        _ => Err("Choose Goal, Task or Step".into()),
    }
}
fn attach_question(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    let db = app
        .state::<AppState>()
        .trace
        .db_path()
        .ok_or("Task history unavailable")?
        .to_path_buf();
    let runs = crate::flight::runs(&db, 200, None)
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|r| api_task_source(&r.source))
        .take(3)
        .collect::<Vec<_>>();
    let options = if runs.is_empty() {
        vec![
            option(
                "new_task",
                "Create a task for this step",
                "Execute it as a new bounded task.",
            ),
            option(
                "reclassify",
                "Review the input",
                "Choose a different scope.",
            ),
            option("cancel", "Cancel", "Do not execute this step."),
        ]
    } else {
        runs.into_iter().map(|r|option(&r.id,&r.task,"Attach this step and create a new execution attempt. You can also enter another exact task ID or title below.")).collect()
    };
    question(
        e,
        "attachment",
        "Which task owns this step?",
        "The original execution and recording stay available. The new step runs as another attempt of the selected task.",
        options,
    );
    commit(app, e)
}
async fn clarify(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    e.persistable = false;
    e.status = "clarifying".into();
    e.question = None;
    commit(app, e)?;
    let schema = json!({"type":"object","additionalProperties":false,"properties":{"class":{"type":"string","enum":["general","health","mind","finance"]},"ready":{"type":"boolean"},"question":{"type":"string"},"explanation":{"type":"string"},"options":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"object","additionalProperties":false,"properties":{"id":{"type":"string"},"label":{"type":"string"},"explanation":{"type":"string"}},"required":["id","label","explanation"]}},"goal":{"type":"object","additionalProperties":false,"properties":{"outcome":{"type":"string"},"current":{"type":"string"},"success":{"type":"string"},"horizon":{"type":"string"},"resources":{"type":"string"},"constraints":{"type":"string"}},"required":["outcome","current","success","horizon","resources","constraints"]}},"required":["class","ready","question","explanation","options","goal"]});
    let value = infer(
        app,
        e,
        include_str!("../prompts/intake/clarify.md"),
        &json!({"input":e.text,"answers":e.answers,"draft":e.goal}),
        &schema,
    )
    .await?;
    if e.token.is_cancelled() {
        return Err("Request interrupted".into());
    }
    e.goal = Some(value["goal"].clone());
    let detected: DataClass =
        serde_json::from_value(value["class"].clone()).map_err(|err| err.to_string())?;
    if detected.is_sensitive() {
        e.class = detected;
    }
    e.persistable = true;
    if require_consent(app, e)? {
        return Ok(());
    }
    if value["ready"] == true
        && e.answers
            .iter()
            .any(|a| a["kind"] == "clarification" || a["kind"] == "refinement")
    {
        question(
            e,
            "goal_review",
            "Does this goal reflect what you want?",
            "Review the definition below. Approval saves the goal; it does not authorize pathways, schedules or executable tasks.",
            vec![
                option(
                    "approve",
                    "Approve this goal",
                    "Save the clarified outcome and constraints.",
                ),
                option(
                    "refine",
                    "Refine the goal",
                    "Explain what should change in your own answer.",
                ),
                option("cancel", "Cancel", "Keep no active goal from this request."),
            ],
        );
    } else {
        let options: Vec<OptionItem> =
            serde_json::from_value(value["options"].clone()).map_err(|err| err.to_string())?;
        if options.len() != 3
            || options
                .iter()
                .any(|o| o.id.is_empty() || matches!(o.id.as_str(), "cancel" | "settings"))
            || options
                .iter()
                .map(|o| &o.id)
                .collect::<std::collections::HashSet<_>>()
                .len()
                != 3
        {
            return Err("Invalid clarification options".into());
        }
        question(
            e,
            "clarification",
            value["question"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("Missing clarification question")?,
            value["explanation"].as_str().unwrap_or(""),
            options,
        );
    }
    commit(app, e)
}
fn dispatch(app: &AppHandle, e: &mut Entry, parent: Option<&str>) -> Result<(), String> {
    if e.token.is_cancelled() {
        return Err("Request interrupted".into());
    }
    let _guard = crate::domain::MUTATION.locked();
    let attempt = format!("{}-execution", e.id);
    e.status = "dispatching".into();
    e.question = None;
    e.attempt_id = Some(attempt.clone());
    commit(app, e)?;
    let state = app.state::<AppState>();
    state.trace.sync()?;
    if let Some(parent) = parent {
        let db = state.trace.db_path().ok_or("Task history unavailable")?;
        if let Err(error) = crate::domain::link_attempt(db, parent, &attempt) {
            e.attempt_id = None;
            return Err(error.to_string());
        }
    }
    let context = parent
        .map(|parent| {
            let db = state.trace.db_path().ok_or("Task history unavailable")?;
            crate::flight::original_prompt(db, parent)
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "The original task scope was not retained. Start a new task with explicit targets and constraints instead of attaching to this older record.".to_string())
        })
        .transpose();
    let context = match context {
        Ok(context) => context,
        Err(error) => {
            e.attempt_id = None;
            return Err(error);
        }
    };
    if e.token.is_cancelled() {
        return Err("Request interrupted".into());
    }
    let voice = if e.voice_wav.is_empty() {
        None
    } else {
        Some(crate::archive::save_unscoped_voice(&e.voice_wav).map_err(|err| err.to_string())?)
    };
    if let Err(error) =
        crate::tasks::submit_with_id(app, &e.text, e.keep_mic, Some(&attempt), context, voice)
    {
        e.attempt_id = None;
        return Err(error);
    }
    state.trace.sync()?;
    e.task_id = Some(parent.unwrap_or(&attempt).into());
    e.status = "submitted".into();
    commit(app, e)
}
pub fn submit(app: &AppHandle, text: &str, keep_mic: bool) -> Result<u64, String> {
    submit_voice(app, text, keep_mic, Vec::new())
}
pub fn submit_voice(
    app: &AppHandle,
    text: &str,
    keep_mic: bool,
    voice_wav: Vec<u8>,
) -> Result<u64, String> {
    submit_intent(app, text, keep_mic, voice_wav, None, None)
}
fn submit_intent(
    app: &AppHandle,
    text: &str,
    keep_mic: bool,
    voice_wav: Vec<u8>,
    requested_kind: Option<String>,
    supplied_id: Option<String>,
) -> Result<u64, String> {
    if text.trim().is_empty() || text.chars().count() > 4000 {
        return Err("Enter a request of 1 to 4000 characters".into());
    }
    if app.state::<AppState>().settings.locked().external_agent {
        return Err("Start tasks in your external agent host".into());
    }
    if ENTRIES
        .locked()
        .values()
        .filter(|e| !matches!(e.status.as_str(), "submitted" | "cancelled" | "goal_ready"))
        .count()
        >= 24
    {
        return Err("Resolve or cancel pending decisions before adding more requests".into());
    }
    let mut e = Entry {
        id: supplied_id.map_or_else(uid, Ok)?,
        revision: 0,
        text: text.trim().into(),
        class: DataClass::General,
        kind: requested_kind.clone().unwrap_or_default(),
        category: String::new(),
        status: "classifying".into(),
        question: None,
        answers: Vec::new(),
        goal: None,
        approved_goal: None,
        outcome_metric: None,
        requested_kind,
        pathways: None,
        task_proposals: None,
        error: None,
        created: crate::trace::now_utc(),
        task_id: None,
        attempt_id: None,
        confirm_classification: false,
        persistable: false,
        token: CancelToken::default(),
        keep_mic,
        voice_wav,
    };
    ENTRIES.locked().insert(e.id.clone(), e.clone());
    let _ = app.emit("decisions_changed", ());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _serial = SERIAL.lock().await;
        let result = classify(&app, &mut e).await;
        if let Err(err) = result {
            fail(&app, &mut e, err);
        }
    });
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Ok(NEXT.fetch_add(1, Ordering::Relaxed))
}
fn fail(app: &AppHandle, e: &mut Entry, error: String) {
    e.error = Some(error);
    question(
        e,
        "retry",
        "This request needs attention",
        "Nothing will be retried automatically. Review any existing attempt before continuing.",
        vec![
            option(
                "retry",
                "Retry this decision",
                "Use the saved input and answers.",
            ),
            option(
                "settings",
                "Review settings",
                "Check the local model and storage permissions.",
            ),
            option("cancel", "Cancel request", "Do not continue."),
        ],
    );
    let _ = commit(app, e);
    ENTRIES.locked().insert(e.id.clone(), e.clone());
    let _ = app.emit("decisions_changed", ());
}
pub fn stop() {
    for entry in ENTRIES.locked().values() {
        if matches!(
            entry.status.as_str(),
            "classifying" | "clarifying" | "dispatching" | "pathway_planning" | "task_planning"
        ) {
            entry.token.cancel();
        }
    }
}
pub fn busy() -> bool {
    ENTRIES.locked().values().any(|e| {
        matches!(
            e.status.as_str(),
            "classifying" | "clarifying" | "dispatching" | "pathway_planning" | "task_planning"
        )
    })
}
fn api_task_source(source: &str) -> bool {
    matches!(
        source,
        "" | "built-in" | "task-request" | "screen-insight" | "rune"
    )
}

fn validate_answer(
    e: &Entry,
    revision: u64,
    option_id: Option<&str>,
    text: &str,
) -> Result<Question, String> {
    if e.revision != revision {
        return Err("This question changed. Review its latest version before answering.".into());
    }
    let q = e
        .question
        .clone()
        .ok_or("This decision has already been answered")?;
    if option_id.is_some_and(|id| id != "cancel" && !q.options.iter().any(|o| o.id == id)) {
        return Err("That option is not part of this question".into());
    }
    if option_id.is_none() && text.trim().is_empty() {
        return Err("Choose an option or write an answer".into());
    }
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(class: DataClass) -> Entry {
        Entry {
            id: "intake-test".into(),
            revision: 1,
            text: "SENSITIVE_GOAL_CANARY".into(),
            class,
            category: "fitness".into(),
            kind: "goal".into(),
            status: "clarification".into(),
            question: Some(Question {
                id: "intake-test".into(),
                revision: 1,
                kind: "clarification".into(),
                title: "What is your starting position?".into(),
                explanation: String::new(),
                options: vec![option("baseline", "Baseline", "Describe it.")],
            }),
            answers: vec![json!({"kind":"clarification","answer":"CANARY_ANSWER"})],
            goal: None,
            approved_goal: None,
            outcome_metric: None,
            requested_kind: None,
            pathways: None,
            task_proposals: None,
            error: None,
            created: crate::trace::now_utc(),
            task_id: None,
            attempt_id: None,
            confirm_classification: false,
            persistable: true,
            token: CancelToken::default(),
            keep_mic: false,
            voice_wav: Vec::new(),
        }
    }
    #[test]
    fn protected_questions_require_consent_and_restore_without_plaintext_bodies() {
        for class in DataClass::all() {
            let path = std::env::temp_dir().join(format!("rune-intake-{}.sqlite3", uid().unwrap()));
            let store = LifeStore::open(&path, LifeCrypto::in_memory_random()).unwrap();
            let mut e = sample(*class);
            e.voice_wav = b"VOICE_BYTES_MUST_REMAIN_EPHEMERAL".to_vec();
            e.pathways = Some(crate::intake_pathways::PathwaySet {
                assessment: "PATHWAY_CANARY".into(),
                similar_outcome: String::new(),
                options: vec![],
                goal: json!({"outcome":"PATHWAY_GOAL_CANARY"}),
                selected: None,
                approved: None,
            });
            e.task_proposals = Some(crate::task_proposals::tests::sample());
            assert!(store_entry(&store, &e).is_err());
            store.set_consent(*class, true, "test").unwrap();
            store_entry(&store, &e).unwrap();
            let saved = store.get_record("intake-test-1").unwrap().unwrap();
            {
                let raw = rusqlite::Connection::open(&path).unwrap();
                let body: String = raw
                    .query_row(
                        "SELECT body FROM records WHERE id='intake-test-1'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                assert!(LifeCrypto::is_encrypted(&body));
                assert!(!body.contains("CANARY"));
            }
            let restored = decode_entry(&store, &saved.body).unwrap();
            assert_eq!(restored.question.unwrap().revision, 1);
            assert_eq!(restored.answers, e.answers);
            assert_eq!(restored.text, e.text);
            assert!(restored.voice_wav.is_empty());
            assert_eq!(
                restored.task_proposals.as_ref().unwrap().assessment,
                "PROPOSAL_ASSESSMENT_CANARY"
            );
            assert_eq!(
                restored.pathways.as_ref().unwrap().assessment,
                "PATHWAY_CANARY"
            );
            e.revision = 2;
            e.status = "dispatching".into();
            e.question = None;
            store_entry(&store, &e).unwrap();
            let saved = store.get_record("intake-test-2").unwrap().unwrap();
            let recovered = decode_entry(&store, &saved.body).unwrap();
            assert_eq!(recovered.status, "interrupted");
            assert!(recovered.question.is_none());
            e.revision = 3;
            e.status = "pathway_planning".into();
            store_entry(&store, &e).unwrap();
            let saved = store.get_record("intake-test-3").unwrap().unwrap();
            let recovered = decode_entry(&store, &saved.body).unwrap();
            assert_eq!(recovered.status, "goal_ready");
            assert_eq!(recovered.pathways.unwrap().assessment, "PATHWAY_CANARY");
            e.revision = 4;
            e.status = "task_planning".into();
            store_entry(&store, &e).unwrap();
            let saved = store.get_record("intake-test-4").unwrap().unwrap();
            let recovered = decode_entry(&store, &saved.body).unwrap();
            assert_eq!(recovered.status, "goal_ready");
            assert!(
                recovered.task_proposals.unwrap().tasks[0]
                    .created_task_id
                    .is_none()
            );
            drop(store);
            let _ = std::fs::remove_file(&path);
        }
    }
    #[test]
    fn stale_duplicate_and_fabricated_answers_are_rejected_before_dispatch() {
        let mut e = sample(DataClass::General);
        assert!(validate_answer(&e, 0, Some("baseline"), "").is_err());
        assert!(validate_answer(&e, 1, Some("invented"), "").is_err());
        assert!(validate_answer(&e, 1, None, "").is_err());
        assert!(validate_answer(&e, 1, None, "My own answer").is_ok());
        e.question = None;
        assert!(validate_answer(&e, 1, Some("baseline"), "").is_err());
    }
}

#[tauri::command]
pub async fn decision_refine(app: AppHandle, id: String, revision: u64) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    load(&app)?;
    let mut e = ENTRIES
        .locked()
        .get(&id)
        .cloned()
        .ok_or("This goal no longer exists")?;
    if e.revision != revision || e.kind != "goal" || e.status != "goal_ready" {
        return Err("Review the latest goal before refining it".into());
    }
    question(
        &mut e,
        "clarification",
        "What would you like to change?",
        "The approved definition stays in the saved history. Review and approve the revised goal before it replaces the current definition.",
        vec![
            option(
                "outcome",
                "Change the outcome",
                "Clarify what you want to achieve.",
            ),
            option(
                "horizon",
                "Change the timeframe",
                "Choose a different deadline or horizon.",
            ),
            option(
                "constraints",
                "Change resources or constraints",
                "Update what is available or limiting the goal.",
            ),
        ],
    );
    commit(&app, &mut e)
}

fn ready_goal(app: &AppHandle, id: &str, revision: u64) -> Result<Entry, String> {
    load(app)?;
    let e = ENTRIES
        .locked()
        .get(id)
        .cloned()
        .ok_or("This goal no longer exists")?;
    if e.revision != revision
        || e.kind != "goal"
        || e.status != "goal_ready"
        || e.approved_goal.is_none()
    {
        return Err(
            "Conflict: review the latest approved goal before changing its pathways".into(),
        );
    }
    if !consent(app, e.class) {
        return Err("Protected storage consent is required for goal pathways".into());
    }
    Ok(e)
}
#[tauri::command]
pub fn workspace_new_goal(app: AppHandle, text: String) -> Result<String, String> {
    let id = uid()?;
    submit_intent(
        &app,
        &text,
        false,
        vec![],
        Some("goal".into()),
        Some(id.clone()),
    )?;
    Ok(id)
}
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri command arguments are the explicit measurement fields.
pub async fn goal_measurement(
    app: AppHandle,
    id: String,
    revision: u64,
    current: f64,
    baseline: f64,
    target: f64,
    unit: String,
    note: String,
) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    let mut e = ready_goal(&app, &id, revision)?;
    let baseline = e.outcome_metric.as_ref().map_or(baseline, |m| m.baseline);
    if let Some(old) = &e.outcome_metric
        && (old.target != target || old.unit != unit.trim())
    {
        return Err("This goal already has a measure. Its target and unit stay fixed while recording progress.".into());
    }
    let metric = crate::workspace_tasks::OutcomeMetric {
        baseline,
        current,
        target,
        unit: unit.trim().into(),
        note,
        updated_at: crate::trace::now_utc(),
    };
    metric.validate()?;
    e.outcome_metric = Some(metric);
    commit(&app, &mut e)
}
#[tauri::command]
pub async fn decision_pathways(app: AppHandle, id: String, revision: u64) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    let mut e = ready_goal(&app, &id, revision)?;
    let approved = e.approved_goal.clone().ok_or("Approve the goal first")?;
    e.token = CancelToken::default();
    e.status = "pathway_planning".into();
    e.error = None;
    commit(&app, &mut e)?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        infer(
            &app,
            &e,
            include_str!("../prompts/intake/pathways.md"),
            &json!({"approvedGoal":approved,"category":e.category}),
            &crate::intake_pathways::schema(),
        ),
    )
    .await
    .map_err(|_| "Pathway comparison timed out".to_string())
    .and_then(|value| value)
    .and_then(|value| crate::intake_pathways::parse(value, &approved));
    e.status = "goal_ready".into();
    if !consent(&app, e.class) {
        e.error = Some("Goal storage consent was removed. No pathway changes were saved.".into());
        ENTRIES.locked().insert(e.id.clone(), e);
        let _ = app.emit("decisions_changed", ());
        return Err("Goal storage consent was removed. No pathway changes were saved.".into());
    }
    if e.token.is_cancelled() {
        e.error = Some("Pathway comparison stopped. Your earlier choice is preserved.".into());
        commit(&app, &mut e)?;
        return Err("Pathway comparison stopped".into());
    }
    match result {
        Ok(mut set) => {
            crate::intake_pathways::preserve_preference(&mut set, e.pathways.as_ref());
            e.pathways = Some(set);
            commit(&app, &mut e)
        }
        Err(error) => {
            e.error = Some(error.clone());
            commit(&app, &mut e)?;
            Err(error)
        }
    }
}
#[tauri::command]
pub async fn decision_select_pathway(
    app: AppHandle,
    id: String,
    revision: u64,
    pathway_id: String,
) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    let mut e = ready_goal(&app, &id, revision)?;
    let approved = e.approved_goal.clone().ok_or("Approve the goal first")?;
    let set = e.pathways.as_mut().ok_or("Compare pathways first")?;
    crate::intake_pathways::select(set, &approved, &pathway_id)?;
    e.error = None;
    commit(&app, &mut e)
}

#[tauri::command]
pub async fn decision_propose_tasks(
    app: AppHandle,
    id: String,
    revision: u64,
) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    let mut e = ready_goal(&app, &id, revision)?;
    let goal = e.approved_goal.clone().ok_or("Approve the goal first")?;
    let pathways = e
        .pathways
        .as_ref()
        .ok_or("Choose a preferred pathway first")?;
    if pathways.goal != goal {
        return Err("Compare pathways for the current approved goal first".into());
    }
    let pathway = pathways
        .approved
        .clone()
        .ok_or("Choose a preferred pathway first")?;
    let zone = crate::task_calendar::current_zone()?;
    let start = crate::task_calendar::local_now(&zone)?;
    let existing = crate::workspace_tasks::proposal_context(&app, &e.id)?;
    let draft_id = uid()?;
    e.token = CancelToken::default();
    e.status = "task_planning".into();
    e.error = None;
    commit(&app, &mut e)?;
    let result = tokio::time::timeout(std::time::Duration::from_secs(90), infer(&app, &e,
        include_str!("../prompts/intake/tasks.md"),
        &json!({"approvedGoal":goal,"preferredPathway":pathway,"currentLocalTime":start,"timezone":zone,"existingLinkedTasks":existing}),
        &crate::task_proposals::schema())).await.map_err(|_| "Task proposal generation timed out".to_string())
        .and_then(|r| r).and_then(|value| crate::task_proposals::parse(value, draft_id, goal, pathway, start, zone));
    e.status = "goal_ready".into();
    if !consent(&app, e.class) {
        e.error = Some("Storage consent was removed. No new task proposals were saved.".into());
        ENTRIES.locked().insert(e.id.clone(), e);
        let _ = app.emit("decisions_changed", ());
        return Err("Storage consent was removed".into());
    }
    if e.token.is_cancelled() {
        e.error = Some("Task planning stopped. Your earlier draft is preserved.".into());
        commit(&app, &mut e)?;
        return Err("Task planning stopped".into());
    }
    match result {
        Ok(draft) => {
            e.task_proposals = Some(draft);
            commit(&app, &mut e)
        }
        Err(error) => {
            e.error = Some(error.clone());
            commit(&app, &mut e)?;
            Err(error)
        }
    }
}
#[tauri::command]
pub async fn decision_review_task(
    app: AppHandle,
    id: String,
    revision: u64,
    proposal_id: String,
    approval: Option<crate::task_proposals::Approval>,
) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    let mut e = ready_goal(&app, &id, revision)?;
    let goal = e.approved_goal.as_ref().ok_or("Approve the goal first")?;
    let pathway = e
        .pathways
        .as_ref()
        .and_then(|p| p.approved.as_ref())
        .ok_or("Choose a preferred pathway first")?;
    let draft = e
        .task_proposals
        .as_mut()
        .ok_or("Generate task proposals first")?;
    draft.check_context(goal, pathway)?;
    let index = draft
        .tasks
        .iter()
        .position(|t| t.id == proposal_id)
        .ok_or("Choose a task in this draft")?;
    if draft.tasks[index].created_task_id.is_some() || draft.tasks[index].dismissed {
        return Err("This proposal was already reviewed".into());
    }
    if let Some(approval) = approval {
        approval.validate()?;
        draft.check_schedule(approval.schedule.as_ref())?;
        let task_id = draft.task_id(&proposal_id)?;
        let created =
            crate::workspace_tasks::create_proposed(&app, task_id, &e.id, e.class, approval)?;
        let proposal = &mut draft.tasks[index];
        proposal.created_task_id = Some(created.id);
        proposal.title = created.title;
        proposal.instructions = created.instructions.unwrap_or_default();
        proposal.execution_mode = created.execution_mode;
        proposal.schedule = created
            .schedule
            .map(|s| crate::workspace_tasks::ScheduleInput {
                local: s.local,
                zone: s.zone,
                repeat: s.repeat,
                anchor_day: Some(s.anchor_day),
            });
    } else {
        draft.tasks[index].dismissed = true;
    }
    e.error = None;
    commit(&app, &mut e)
}
#[tauri::command]
pub async fn decision_answer(
    app: AppHandle,
    id: String,
    revision: u64,
    option_id: Option<String>,
    text: String,
) -> Result<(), String> {
    if text.chars().count() > 4000 {
        return Err("Keep the answer within 4000 characters".into());
    }
    let _serial = SERIAL.lock().await;
    load(&app)?;
    let mut e = ENTRIES
        .locked()
        .get(&id)
        .cloned()
        .ok_or("This decision no longer exists")?;
    let q = validate_answer(&e, revision, option_id.as_deref(), &text)?;
    let choice = option_id.as_deref().unwrap_or("");
    e.token = CancelToken::default();
    e.error = None;
    if choice == "settings" {
        return crate::hub::open(&app, Some("data")).map_err(|err| err.to_string());
    }
    if choice == "cancel" {
        if let Some(approved) = &e.approved_goal {
            e.goal = Some(approved.clone());
            e.status = "goal_ready".into();
        } else {
            e.status = "cancelled".into();
        }
        e.question = None;
        return commit(&app, &mut e);
    }
    let result = match q.kind.as_str() {
        "consent" => {
            if choice != "enable" {
                Err("Choose an explicit storage option".into())
            } else {
                let state = app.state::<AppState>();
                {
                    let mut s = state.settings.locked();
                    let mut next = s.clone();
                    next.life_enabled = true;
                    next.life_consents.insert(e.class.as_str().into(), true);
                    crate::settings::save(&app, &next).map_err(|err| err.to_string())?;
                    *s = next;
                }
                crate::hub::changed(&app);
                let reason = "Confirm how you intend this input to be treated.";
                if e.confirm_classification {
                    classification_question(&mut e, reason);
                    commit(&app, &mut e)
                } else {
                    advance(&app, &mut e).await
                }
            }
        }
        "classification" => {
            if matches!(choice, "goal" | "task" | "step") {
                e.kind = choice.into();
                advance(&app, &mut e).await
            } else {
                e.answers
                    .push(json!({"kind":"classification","question":q.title,"answer":text}));
                classify(&app, &mut e).await
            }
        }
        "clarification" => {
            let reply = q
                .options
                .iter()
                .find(|o| o.id == choice)
                .map(|o| o.label.clone())
                .unwrap_or(text);
            e.answers.push(
                json!({"kind":"clarification","question":q.title,"answer":reply,"source":"user"}),
            );
            clarify(&app, &mut e).await
        }
        "goal_review" => {
            if choice == "approve" {
                if e.approved_goal != e.goal {
                    e.pathways = None;
                    e.task_proposals = None;
                    if e.approved_goal
                        .as_ref()
                        .map(|g| (&g["outcome"], &g["success"]))
                        != e.goal.as_ref().map(|g| (&g["outcome"], &g["success"]))
                    {
                        e.outcome_metric = None;
                    }
                }
                e.approved_goal = e.goal.clone();
                e.status = "goal_ready".into();
                e.question = None;
                commit(&app, &mut e)
            } else {
                e.answers.push(
                    json!({"kind":"refinement","question":"Requested refinement","answer":text}),
                );
                clarify(&app, &mut e).await
            }
        }
        "attachment" => {
            if choice == "new_task" {
                e.kind = "task".into();
                dispatch(&app, &mut e, None)
            } else if choice == "reclassify" {
                classification_question(&mut e, "Choose the intended scope.");
                commit(&app, &mut e)
            } else {
                let state = app.state::<AppState>();
                let db = state.trace.db_path().ok_or("Task history unavailable")?;
                let runs = crate::flight::runs(db, 200, None).map_err(|err| err.to_string())?;
                let matches = runs
                    .iter()
                    .filter(|r| api_task_source(&r.source))
                    .filter(|r| {
                        r.id == choice
                            || (choice.is_empty()
                                && (r.id == text.trim()
                                    || r.task.eq_ignore_ascii_case(text.trim())))
                    })
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    Err("Choose one existing task by its option, exact ID or unique title".into())
                } else {
                    dispatch(&app, &mut e, Some(&matches[0].id))
                }
            }
        }
        "retry" => {
            if e.attempt_id.is_some() {
                Err("Inspect task history first. Submit a new step explicitly to create another attempt.".into())
            } else if e.kind.is_empty() {
                classify(&app, &mut e).await
            } else {
                advance(&app, &mut e).await
            }
        }
        _ => Err("Unsupported decision".into()),
    };
    if let Err(err) = &result {
        fail(&app, &mut e, err.clone());
    }
    result
}
