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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
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
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    domain_source: Option<String>,
    #[serde(default)]
    domain_mode: Option<String>,
    #[serde(default)]
    sub_domains: Vec<crate::goal_domains::SubDomainSelection>,
    #[serde(default)]
    subdomain_progress: Option<crate::intake_subdomains::Progress>,
    #[serde(default)]
    discovery: Option<crate::intake_discovery::Progress>,
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
    if matches!(
        entry.status.as_str(),
        "domain_planning" | "subdomain_planning" | "discovery_planning"
    ) {
        entry.error = Some(
            "Context planning was interrupted. Retry when ready; your earlier choices are preserved.".into(),
        );
        retry_question(&mut entry);
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
                    || (e.status == "discovery_planning"
                        && e.goal.is_none()
                        && crate::intake_discovery::answered(&e.sub_domains, &e.answers) == 0)
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
        domain: if matches!(
            kind,
            "subdomains"
                | "subdomain_detail"
                | "discovery"
                | "discovery_required"
                | "discovery_review"
        ) {
            e.domain.clone()
        } else {
            None
        },
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
    classification: bool,
) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let (settings, key) = crate::settings::snapshot_with_api_key(&state.settings);
    let model = settings.model_for_role(crate::model::role::ModelRole::Agent);
    let profile = if classification {
        crate::agent::unless_stopped(
            &e.token,
            crate::model::protected::discover_classification(
                &settings.base_url,
                key.as_deref(),
                model,
            ),
        )
        .await
        .ok_or("Request interrupted")??
    } else {
        crate::agent::unless_stopped(
            &e.token,
            crate::model::protected::discover(
                &state.http,
                &settings.base_url,
                key.as_deref(),
                model,
            ),
        )
        .await
        .ok_or("Request interrupted")??
    };
    let client = crate::model::client::ChatClient {
        app: Some(app),
        output_limit: settings.output_limit(),
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
    if classification {
        crate::agent::unless_stopped(&e.token, client.call_classification(&request, schema)).await
    } else {
        crate::agent::unless_stopped(&e.token, client.call_json_retrying(&request, schema)).await
    }
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
        &json!({"input":e.text,"answers":e.answers,"domain":domain_context(e),"subDomains":subdomain_context(e)}),
        &schema,
        true,
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
    if needs_discovery(e) {
        return discover(app, e).await;
    }
    if require_consent(app, e)? {
        return Ok(());
    }
    match e.kind.as_str() {
        "task" => dispatch(app, e, None),
        "step" => attach_question(app, e),
        "goal" => {
            e.voice_wav.clear();
            if needs_domain(e) {
                suggest_domain(app, e).await
            } else if needs_subdomains(e) {
                plan_subdomains(app, e).await
            } else {
                clarify(app, e).await
            }
        }
        _ => Err("Choose Goal, Task or Step".into()),
    }
}
fn needs_discovery(e: &Entry) -> bool {
    e.domain_mode.as_deref() == Some("auto")
        && e.approved_goal.is_none()
        && e.discovery
            .as_ref()
            .is_some_and(|d| d.phase != crate::intake_discovery::Phase::Clarifying)
}

fn discovery_review_question(e: &mut Entry, explanation: &str) -> Result<(), String> {
    if e.goal.is_none() || crate::intake_discovery::answered(&e.sub_domains, &e.answers) == 0 {
        return Err("Answer at least one discovery question before reviewing a draft".into());
    }
    question(
        e,
        "discovery_review",
        "Review Rune's proposed goal",
        explanation,
        vec![
            option(
                "continue",
                "Use this draft and clarify",
                "Continue to the existing clarification step. The goal is not approved yet.",
            ),
            option(
                "refine",
                "Edit the draft",
                "Write the changes you want in your own answer.",
            ),
            option(
                "more",
                "Answer more questions",
                "Gather more current context within the remaining question limit.",
            ),
        ],
    );
    Ok(())
}

fn discovery_required_question(e: &mut Entry) {
    question(
        e,
        "discovery_required",
        "One answer is needed before Rune can propose a goal",
        "The questions were skipped, so no goal was generated. Share one detail about your current situation in your own answer, revisit the skipped questions, or cancel. Unknown details stay unknown.",
        vec![
            option(
                "revisit",
                "Revisit skipped questions",
                "Explicitly reopen the selected domain's skipped parts.",
            ),
            option(
                "settings",
                "Review context settings",
                "Review protected storage and local model settings.",
            ),
            option(
                "cancel",
                "Cancel discovery",
                "Finish without creating or approving a goal.",
            ),
        ],
    );
}

fn apply_discovery(
    e: &mut Entry,
    response: crate::intake_discovery::Response,
) -> Result<(), String> {
    use crate::intake_discovery::Phase;
    if response.ready && crate::intake_discovery::answered(&e.sub_domains, &e.answers) == 0 {
        return Err("Answer at least one discovery question before generating a draft".into());
    }
    let progress = e
        .discovery
        .as_mut()
        .ok_or("Discovery progress unavailable")?;
    if response.ready {
        let draft = response.goal.ok_or("Discovery returned no draft")?;
        e.goal = Some(serde_json::to_value(draft).map_err(|_| "Invalid discovery draft")?);
        progress.phase = Phase::Reviewing;
        progress.current = None;
        discovery_review_question(
            e,
            &format!(
                "{} Review the definition below. Edit it or continue to clarification; this does not approve the goal or create work.",
                response.explanation
            ),
        )?;
    } else {
        let id = response
            .sub_domain
            .ok_or("Discovery returned no question topic")?;
        let domain = e
            .domain
            .as_deref()
            .and_then(crate::goal_domains::find)
            .ok_or("Choose a domain first")?;
        let sub = domain
            .sub_domains
            .iter()
            .find(|s| s.id == id)
            .ok_or("Unsupported discovery topic")?;
        progress.current = Some(id);
        let options = response
            .options
            .into_iter()
            .map(|o| option(&o.id, &o.label, &o.explanation))
            .collect();
        question(
            e,
            "discovery",
            &format!("{}: {}", sub.name, response.question),
            &response.explanation,
            options,
        );
    }
    if response.class.is_sensitive() {
        e.class = response.class;
    }
    Ok(())
}

fn cold_start_discovery(e: &Entry) -> bool {
    e.answers.is_empty() && e.goal.is_none()
}

async fn discover(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    use crate::intake_discovery::Phase;
    if e.token.is_cancelled() {
        return Err("Discovery stopped".into());
    }
    let phase = e
        .discovery
        .as_ref()
        .ok_or("Discovery progress unavailable")?
        .phase;
    if phase == Phase::Reviewing {
        if require_consent(app, e)? {
            return Ok(());
        }
        discovery_review_question(
            e,
            "Review the draft, edit it, or continue to clarification. Nothing is approved or executable yet.",
        )?;
        return commit(app, e);
    }
    let domain = e
        .domain
        .as_deref()
        .and_then(crate::goal_domains::find)
        .ok_or("Choose a domain explicitly before Auto discovery")?;
    let available = crate::intake_discovery::available(&e.sub_domains);
    let answered = crate::intake_discovery::answered(&e.sub_domains, &e.answers);
    if available.is_empty() && answered == 0 {
        if require_consent(app, e)? {
            return Ok(());
        }
        discovery_required_question(e);
        return commit(app, e);
    }
    let progress = e
        .discovery
        .as_mut()
        .ok_or("Discovery progress unavailable")?;
    if available.is_empty() {
        progress.phase = Phase::Drafting;
    }
    let draft_only = progress.phase == Phase::Drafting;
    let require_question = !draft_only && (answered == 0 || progress.require_question);
    e.status = "discovery_planning".into();
    e.question = None;
    commit(app, e)?;
    e.persistable = false;
    let first = cold_start_discovery(e);
    let value = tokio::time::timeout(std::time::Duration::from_secs(90), infer(
        app, e, include_str!("../prompts/intake/discovery.md"),
        &json!({"stage":if draft_only { "draft" } else { "question" },"requireQuestion":require_question,
            "domain":crate::intake_subdomains::context(domain),"availableSubDomains":available,
            "subDomains":subdomain_context(e),"answers":e.answers,"previousClass":e.class,"existingDraft":e.goal}),
        &crate::intake_discovery::schema(&available, require_question, draft_only), first,
    )).await.map_err(|_| "Auto discovery timed out".to_string())??;
    if e.token.is_cancelled() {
        return Err("Discovery stopped".into());
    }
    let response =
        crate::intake_discovery::parse(value, &available, answered, require_question, draft_only)?;
    apply_discovery(e, response)?;
    e.persistable = true;
    e.confirm_classification = false;
    if require_consent(app, e)? {
        return Ok(());
    }
    commit(app, e)
}

fn discovery_reply(e: &mut Entry, q: &Question, choice: &str, text: &str) -> Result<(), String> {
    use crate::intake_discovery::Phase;
    let progress = e
        .discovery
        .as_mut()
        .ok_or("Discovery progress unavailable")?;
    if progress.phase != Phase::Questioning {
        return Err("This discovery question has already changed".into());
    }
    if q.kind == "discovery_required" {
        if choice == "revisit" {
            for sub in &mut e.sub_domains {
                sub.skipped = false;
            }
            progress.require_question = true;
            e.answers
                .push(json!({"kind":"discovery_revisit","source":"user"}));
        } else if choice.is_empty() && !text.trim().is_empty() {
            e.persistable = false;
            e.answers.push(
                json!({"kind":"discovery","subDomain":null,"answer":text.trim(),"source":"user"}),
            );
            progress.phase = Phase::Drafting;
            progress.require_question = false;
        } else {
            return Err("Revisit questions, share current context, or cancel discovery".into());
        }
    } else if choice == "skip_all" {
        for sub in &mut e.sub_domains {
            if sub.answers.len() < crate::intake_subdomains::MAX_QUESTIONS {
                sub.skipped = true;
            }
        }
        e.answers
            .push(json!({"kind":"discovery_skip","scope":"remaining","source":"user"}));
        if crate::intake_discovery::answered(&e.sub_domains, &e.answers) > 0 {
            progress.phase = Phase::Drafting;
        }
    } else {
        let id = progress
            .current
            .as_deref()
            .ok_or("No discovery question is pending")?;
        let sub = e
            .sub_domains
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("This discovery part is not selected")?;
        if sub.skipped || sub.answers.len() >= crate::intake_subdomains::MAX_QUESTIONS {
            return Err("This discovery part is finished".into());
        }
        if choice == "skip" {
            sub.skipped = true;
            e.answers
                .push(json!({"kind":"discovery_skip","subDomain":id,"source":"user"}));
        } else {
            let reply = q
                .options
                .iter()
                .find(|o| o.id == choice)
                .map(|o| o.label.as_str())
                .unwrap_or(text.trim());
            if reply.is_empty() {
                return Err("Choose an answer or share current context".into());
            }
            e.persistable = false;
            sub.answers.push(
                json!({"kind":"discovery","question":q.title,"answer":reply,"source":"user"}),
            );
            e.answers
                .push(json!({"kind":"discovery","subDomain":id,"source":"user"}));
            progress.require_question = false;
        }
    }
    progress.current = None;
    e.question = None;
    Ok(())
}

fn discovery_review_reply(e: &mut Entry, choice: &str, text: &str) -> Result<bool, String> {
    use crate::intake_discovery::Phase;
    if e.discovery
        .as_ref()
        .is_none_or(|p| p.phase != Phase::Reviewing)
    {
        return Err("This discovery draft has already changed".into());
    }
    if choice == "continue" {
        e.discovery
            .as_mut()
            .ok_or("Discovery progress unavailable")?
            .phase = Phase::Clarifying;
        e.answers.push(
            json!({"kind":"discovery_draft","acceptedForClarification":true,"source":"user"}),
        );
    } else if choice == "more" {
        if crate::intake_discovery::available(&e.sub_domains).is_empty() {
            discovery_review_question(
                e,
                "The selected parts have no remaining questions. Edit the draft or continue to clarification; skipped parts remain skipped.",
            )?;
            return Ok(false);
        }
        let progress = e
            .discovery
            .as_mut()
            .ok_or("Discovery progress unavailable")?;
        progress.phase = Phase::Questioning;
        progress.require_question = true;
    } else if choice == "refine" || choice.is_empty() {
        if text.trim().is_empty() {
            discovery_review_question(
                e,
                "Write the changes you want in your own answer. The current draft is preserved.",
            )?;
            return Ok(false);
        }
        e.persistable = false;
        e.answers
            .push(json!({"kind":"discovery_refinement","answer":text.trim(),"source":"user"}));
        e.discovery
            .as_mut()
            .ok_or("Discovery progress unavailable")?
            .phase = Phase::Drafting;
    } else {
        return Err("Choose how to review this draft".into());
    }
    e.question = None;
    Ok(true)
}
fn needs_domain(e: &Entry) -> bool {
    e.domain_mode.as_deref() == Some("manual")
        && e.domain.is_none()
        && e.goal.is_none()
        && e.approved_goal.is_none()
}

fn needs_subdomains(e: &Entry) -> bool {
    e.domain_mode.as_deref() == Some("manual")
        && e.domain.is_some()
        && e.goal.is_none()
        && e.approved_goal.is_none()
        && e.subdomain_progress.as_ref().is_some_and(|p| !p.finished)
}

fn domain_context(e: &Entry) -> Value {
    e.domain
        .as_deref()
        .and_then(crate::goal_domains::find)
        .map_or(
            Value::Null,
            |d| json!({"id":d.id,"name":d.name,"zone":d.zone}),
        )
}

fn subdomain_context(e: &Entry) -> Value {
    let domain = e.domain.as_deref().and_then(crate::goal_domains::find);
    Value::Array(
        e.sub_domains
            .iter()
            .map(|s| {
                let name = domain
                    .and_then(|d| d.sub_domains.iter().find(|sub| sub.id == s.id))
                    .map(|sub| &sub.name);
                json!({"id":s.id,"name":name,"answers":s.answers,"skipped":s.skipped})
            })
            .collect(),
    )
}

async fn plan_subdomains(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    let domain = e
        .domain
        .as_deref()
        .and_then(crate::goal_domains::find)
        .ok_or("Confirm a domain first")?;
    loop {
        let progress = e
            .subdomain_progress
            .as_mut()
            .ok_or("This conversation has no sub-domain step")?;
        let selection = !progress.selection_done;
        let current = if selection {
            None
        } else {
            progress.next(&e.sub_domains)
        };
        if !selection && current.is_none() {
            progress.finished = true;
            progress.current = None;
            return clarify(app, e).await;
        }
        progress.current = current.clone();
        let answered = current
            .as_ref()
            .and_then(|id| e.sub_domains.iter().find(|s| &s.id == id))
            .map_or(0, |s| s.answers.len());
        e.status = "subdomain_planning".into();
        e.question = None;
        commit(app, e)?;
        e.persistable = false;
        let value = tokio::time::timeout(
            std::time::Duration::from_secs(90),
            infer(
                app,
                e,
                include_str!("../prompts/intake/subdomains.md"),
                &json!({"stage":if selection { "selection" } else { "detail" },"input":e.text,
                "domain":crate::intake_subdomains::context(domain),"currentSubDomain":current,
                "subDomains":e.sub_domains,"answers":e.answers,"previousClass":e.class}),
                &crate::intake_subdomains::schema(domain, selection),
                false,
            ),
        )
        .await
        .map_err(|_| "Sub-domain planning timed out".to_string())??;
        if e.token.is_cancelled() {
            return Err("Sub-domain planning stopped".into());
        }
        let response = crate::intake_subdomains::parse(value, domain, selection, answered)?;
        if response.class.is_sensitive() {
            e.class = response.class;
        }
        if response.ready {
            let progress = e
                .subdomain_progress
                .as_mut()
                .ok_or("Sub-domain progress unavailable")?;
            if let Some(id) = progress.current.take() {
                progress.completed.push(id);
            }
        } else {
            let title = current
                .as_ref()
                .and_then(|id| domain.sub_domains.iter().find(|s| &s.id == id))
                .map_or(response.question.clone(), |sub| {
                    format!("{}: {}", sub.name, response.question)
                });
            let options = response
                .options
                .into_iter()
                .map(|o| option(&o.id, &o.label, &o.explanation))
                .collect();
            question(
                e,
                if selection {
                    "subdomains"
                } else {
                    "subdomain_detail"
                },
                &title,
                &response.explanation,
                options,
            );
            if selection {
                e.status = "subdomain_selection".into();
            }
        }
        e.confirm_classification = false;
        e.persistable = true;
        if require_consent(app, e)? {
            return Ok(());
        }
        commit(app, e)?;
        if !response.ready {
            return Ok(());
        }
    }
}

fn subdomain_reply(e: &mut Entry, q: &Question, choice: &str, text: &str) -> Result<(), String> {
    let domain = e
        .domain
        .as_deref()
        .and_then(crate::goal_domains::find)
        .ok_or("Confirm a domain first")?;
    let progress = e
        .subdomain_progress
        .as_mut()
        .ok_or("Sub-domain progress unavailable")?;
    if progress.finished {
        return Err("The sub-domain step has already finished".into());
    }
    if choice == "skip_all" {
        for s in &mut e.sub_domains {
            if !progress.completed.contains(&s.id)
                && s.answers.len() < crate::intake_subdomains::MAX_QUESTIONS
            {
                s.skipped = true;
            }
        }
        progress.selection_done = true;
        progress.finished = true;
        progress.current = None;
        e.answers.push(
            json!({"kind":q.kind,"skipped":true,"scope":"remaining_subdomains","source":"user"}),
        );
    } else if q.kind == "subdomains" {
        if progress.selection_done {
            return Err("Sub-domains were already selected".into());
        }
        let reply = if choice.is_empty() {
            text.trim()
        } else {
            choice
        };
        let ids = if reply.starts_with('[') {
            Some(
                serde_json::from_str::<Vec<String>>(reply)
                    .map_err(|_| "Choose valid sub-domains from this domain")?,
            )
        } else {
            domain
                .sub_domains
                .iter()
                .find(|s| s.id.eq_ignore_ascii_case(reply) || s.name.eq_ignore_ascii_case(reply))
                .map(|s| vec![s.id.clone()])
        };
        if let Some(ids) = ids {
            if ids.is_empty() {
                return Err("Choose a sub-domain or explicitly skip this step".into());
            }
            let selections =
                crate::goal_domains::Selection::manual(Some("manual"), e.domain.clone(), ids)?;
            e.sub_domains = selections.sub_domains;
            progress.selection_done = true;
        } else if !choice.is_empty() {
            return Err("Choose a supported sub-domain".into());
        }
        e.persistable = false;
        e.answers
            .push(json!({"kind":"subdomains","question":q.title,"answer":reply,"source":"user"}));
    } else if q.kind == "subdomain_detail" {
        let id = progress
            .current
            .as_deref()
            .ok_or("No sub-domain question is pending")?;
        let selected = e
            .sub_domains
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("This sub-domain is not selected")?;
        if selected.skipped
            || selected.answers.len() >= crate::intake_subdomains::MAX_QUESTIONS
            || progress.completed.contains(&selected.id)
        {
            return Err("This sub-domain has already finished".into());
        }
        if choice == "skip" {
            selected.skipped = true;
            e.answers.push(
                json!({"kind":"subdomain_detail","subDomain":id,"skipped":true,"source":"user"}),
            );
        } else {
            let reply = q
                .options
                .iter()
                .find(|o| o.id == choice)
                .map(|o| o.label.as_str())
                .unwrap_or(text.trim());
            let answer = json!({"kind":"subdomain_detail","subDomain":id,"question":q.title,"answer":reply,"source":"user"});
            selected.answers.push(answer);
            // Keep the reply once in subDomains; the conversation history links to it.
            e.answers
                .push(json!({"kind":"subdomain_detail","subDomain":id,"source":"user"}));
            e.persistable = false;
        }
    } else {
        return Err("This question is not a sub-domain decision".into());
    }
    e.question = None;
    Ok(())
}

fn domain_question(
    e: &mut Entry,
    suggestion: crate::intake_domain::Suggestion,
) -> Result<(), String> {
    if e.domain.is_some() {
        return Err("A chosen domain cannot be replaced by a suggestion".into());
    }
    let mut ranked = vec![crate::intake_domain::Alternative {
        id: suggestion.domain,
        reason: suggestion.reason,
    }];
    ranked.extend(suggestion.alternatives);
    let options = ranked
        .into_iter()
        .map(|candidate| {
            let domain =
                crate::goal_domains::find(&candidate.id).ok_or("Unsupported suggested domain")?;
            Ok(option(&domain.id, &domain.name, &candidate.reason))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let uncertainty = if suggestion.confidence < 0.5 {
        "The match is uncertain. "
    } else {
        ""
    };
    question(
        e,
        "domain",
        "Which domain fits this goal?",
        &format!(
            "{uncertainty}Confirm a suggested domain, choose any other domain, or explain in your own words. This saves a domain choice only; it does not approve the goal."
        ),
        options,
    );
    e.status = "domain_selection".into();
    Ok(())
}

async fn suggest_domain(app: &AppHandle, e: &mut Entry) -> Result<(), String> {
    if !needs_domain(e) {
        return Err("This goal does not need a domain suggestion".into());
    }
    e.status = "domain_planning".into();
    e.question = None;
    // Only already-classified context may be checkpointed; new freeform replies stay in memory.
    commit(app, e)?;
    e.persistable = false;
    let value = tokio::time::timeout(std::time::Duration::from_secs(90), infer(
        app, e, include_str!("../prompts/intake/domain.md"),
        &json!({"input":e.text,"answers":e.answers,"previousClass":e.class,"registry":crate::goal_domains::context()}),
        &crate::intake_domain::schema(), false,
    )).await.map_err(|_| "Domain suggestion timed out".to_string())??;
    if e.token.is_cancelled() {
        return Err("Domain suggestion stopped".into());
    }
    let suggestion = crate::intake_domain::parse(value)?;
    if suggestion.class.is_sensitive() {
        e.class = suggestion.class;
    }
    domain_question(e, suggestion)?;
    e.confirm_classification = false;
    e.persistable = true;
    if require_consent(app, e)? {
        return Ok(());
    }
    commit(app, e)
}

fn domain_reply(e: &mut Entry, q: &Question, choice: &str, text: &str) -> Result<bool, String> {
    if e.domain.is_some() {
        return Err("This goal already has a chosen domain; refresh its current decision".into());
    }
    let reply = if choice.is_empty() {
        text.trim()
    } else {
        choice
    };
    let chosen = crate::goal_domains::find(reply);
    if !choice.is_empty() && chosen.is_none() {
        return Err("Choose a supported goal domain".into());
    }
    // New answers may disclose a different sensitive class. Reclassify via protected inference
    // before saving them, whether proceeding to clarification or asking for another suggestion.
    e.persistable = false;
    e.answers
        .push(json!({"kind":"domain","question":q.title,"answer":reply,"source":"user"}));
    e.question = None;
    if let Some(domain) = chosen {
        e.domain = Some(domain.id.clone());
        e.domain_source = Some("user".into());
    }
    Ok(chosen.is_some())
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
        &json!({"mode":e.domain_mode,"input":e.text,"answers":e.answers,"draft":e.goal,"domain":domain_context(e),"subDomains":subdomain_context(e)}),
        &schema,
        false,
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
    submit_intent(app, text, keep_mic, voice_wav, None, None, None)
}
fn submit_intent(
    app: &AppHandle,
    text: &str,
    keep_mic: bool,
    voice_wav: Vec<u8>,
    requested_kind: Option<String>,
    supplied_id: Option<String>,
    selection: Option<crate::goal_domains::Selection>,
) -> Result<u64, String> {
    let auto = selection
        .as_ref()
        .is_some_and(|s| s.mode == crate::goal_domains::Mode::Auto);
    if (!auto && text.trim().is_empty())
        || text.chars().count() > 4000
        || (auto && (!text.is_empty() || requested_kind.as_deref() != Some("goal")))
    {
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
    let e = new_entry(
        supplied_id.map_or_else(uid, Ok)?,
        text,
        keep_mic,
        voice_wav,
        requested_kind,
        selection,
    );
    ENTRIES.locked().insert(e.id.clone(), e.clone());
    let _ = app.emit("decisions_changed", ());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _serial = SERIAL.lock().await;
        // A goal can be deleted while its first inference is still queued.
        if !ENTRIES.locked().contains_key(&e.id) {
            return;
        }
        let mut e = e;
        let result = if auto {
            discover(&app, &mut e).await
        } else {
            classify(&app, &mut e).await
        };
        if let Err(err) = result {
            fail(&app, &mut e, err);
        }
    });
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Ok(NEXT.fetch_add(1, Ordering::Relaxed))
}

fn new_entry(
    id: String,
    text: &str,
    keep_mic: bool,
    voice_wav: Vec<u8>,
    requested_kind: Option<String>,
    selection: Option<crate::goal_domains::Selection>,
) -> Entry {
    let mode = selection
        .as_ref()
        .map_or(crate::goal_domains::Mode::Manual, |s| s.mode);
    let auto = mode == crate::goal_domains::Mode::Auto;
    let subdomain_progress = (!auto).then(|| {
        crate::intake_subdomains::Progress::new(
            selection
                .as_ref()
                .is_some_and(|s| !s.sub_domains.is_empty()),
        )
    });
    let mut e = Entry {
        id,
        revision: 0,
        text: text.trim().into(),
        class: DataClass::General,
        kind: requested_kind.clone().unwrap_or_default(),
        category: String::new(),
        domain_source: selection
            .as_ref()
            .and_then(|s| s.domain.as_ref())
            .map(|_| "user".into()),
        // Mark new intake at creation so goals reached through the composer also confirm a domain.
        // Older stored entries retain None and are never enrolled by a load or retry.
        domain_mode: Some(mode.as_str().into()),
        domain: selection.as_ref().and_then(|s| s.domain.clone()),
        sub_domains: selection.map(|s| s.sub_domains).unwrap_or_default(),
        subdomain_progress,
        discovery: auto.then(Default::default),
        status: if auto {
            "discovery_planning"
        } else {
            "classifying"
        }
        .into(),
        question: None,
        answers: Vec::new(),
        goal: None,
        approved_goal: None,
        outcome_metric: None,
        requested_kind: if auto { None } else { requested_kind },
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
    if auto
        && e.sub_domains.is_empty()
        && let Some(domain) = e.domain.as_deref().and_then(crate::goal_domains::find)
    {
        e.sub_domains = domain
            .sub_domains
            .iter()
            .map(|s| crate::goal_domains::SubDomainSelection {
                id: s.id.clone(),
                answers: vec![],
                skipped: false,
            })
            .collect();
    }
    e
}
fn fail(app: &AppHandle, e: &mut Entry, error: String) {
    e.error = Some(error);
    retry_question(e);
    let _ = commit(app, e);
    ENTRIES.locked().insert(e.id.clone(), e.clone());
    let _ = app.emit("decisions_changed", ());
}
fn retry_question(e: &mut Entry) {
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
}
pub fn stop() {
    for entry in ENTRIES.locked().values() {
        if processing_status(&entry.status) {
            entry.token.cancel();
        }
    }
}
pub fn busy() -> bool {
    ENTRIES
        .locked()
        .values()
        .any(|e| processing_status(&e.status))
}
fn processing_status(status: &str) -> bool {
    matches!(
        status,
        "classifying"
            | "domain_planning"
            | "subdomain_planning"
            | "discovery_planning"
            | "clarifying"
            | "dispatching"
            | "pathway_planning"
            | "task_planning"
    )
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
    let skip = option_id.is_some_and(|id| {
        (id == "skip_all"
            && matches!(
                q.kind.as_str(),
                "subdomains" | "subdomain_detail" | "discovery"
            ))
            || (id == "skip" && matches!(q.kind.as_str(), "subdomain_detail" | "discovery"))
    });
    if option_id.is_some_and(|id| id != "cancel" && !skip && !q.options.iter().any(|o| o.id == id))
    {
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
            domain: None,
            domain_source: None,
            domain_mode: None,
            sub_domains: vec![],
            subdomain_progress: None,
            discovery: None,
            kind: "goal".into(),
            status: "clarification".into(),
            question: Some(Question {
                id: "intake-test".into(),
                revision: 1,
                kind: "clarification".into(),
                title: "What is your starting position?".into(),
                explanation: String::new(),
                options: vec![option("baseline", "Baseline", "Describe it.")],
                domain: None,
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
    fn only_new_unassigned_goals_enter_domain_confirmation() {
        let mut e = sample(DataClass::General);
        assert!(!needs_domain(&e)); // Legacy entries are not migrated.
        e.domain_mode = Some("manual".into());
        assert!(needs_domain(&e));
        e.goal = Some(json!({"outcome":"An existing draft"}));
        assert!(!needs_domain(&e));
        e.goal = None;
        e.approved_goal = Some(json!({"outcome":"An existing approved goal"}));
        assert!(!needs_domain(&e));
        e.approved_goal = None;
        e.domain = Some("physical".into());
        assert!(!needs_domain(&e));
        assert!(processing_status("domain_planning"));
        assert!(!processing_status("domain_selection"));
    }

    fn domain_proposal() -> crate::intake_domain::Suggestion {
        crate::intake_domain::parse(json!({"class":"general","domain":"career","confidence":0.3,
            "reason":"May concern work; context is limited.","alternatives":[
                {"id":"cognitive","reason":"May concern learning."},
                {"id":"purpose","reason":"May concern meaning."}]}))
        .unwrap()
    }

    #[test]
    fn suggestions_never_assign_a_domain_and_all_registry_choices_require_a_versioned_answer() {
        let mut e = sample(DataClass::General);
        e.domain_mode = Some("manual".into());
        domain_question(&mut e, domain_proposal()).unwrap();
        assert_eq!(e.status, "domain_selection");
        assert!(e.domain.is_none());
        assert!(e.domain_source.is_none());
        assert!(
            e.question
                .as_ref()
                .unwrap()
                .explanation
                .contains("uncertain")
        );
        assert_eq!(e.question.as_ref().unwrap().options.len(), 3);
        assert!(validate_answer(&e, e.revision - 1, Some("career"), "").is_err());
        assert!(validate_answer(&e, e.revision, Some("physical"), "").is_err());
        for domain in crate::goal_domains::REGISTRY.iter() {
            let mut selected = e.clone();
            // The all-domain picker travels through the existing freeform answer contract.
            let q = validate_answer(&selected, selected.revision, None, &domain.id).unwrap();
            assert!(domain_reply(&mut selected, &q, "", &domain.id).unwrap());
            assert_eq!(selected.domain.as_deref(), Some(domain.id.as_str()));
            assert_eq!(selected.domain_source.as_deref(), Some("user"));
            assert!(!selected.persistable); // Classified before newly disclosed content is saved.
            assert!(selected.question.is_none());
            assert!(validate_answer(&selected, selected.revision, None, &domain.id).is_err());
            let saved = selected.answers.clone();
            assert!(domain_question(&mut selected, domain_proposal()).is_err());
            assert!(domain_reply(&mut selected, &q, "career", "").is_err());
            assert_eq!(selected.domain.as_deref(), Some(domain.id.as_str()));
            assert_eq!(selected.answers, saved);
        }
    }

    #[test]
    fn freeform_context_requests_another_proposal_while_exact_names_confirm_a_choice() {
        let mut e = sample(DataClass::General);
        e.domain_mode = Some("manual".into());
        domain_question(&mut e, domain_proposal()).unwrap();
        let q = e.question.clone().unwrap();
        assert!(
            !domain_reply(
                &mut e,
                &q,
                "",
                "I meant relationships, not professional work"
            )
            .unwrap()
        );
        assert!(needs_domain(&e));
        assert_eq!(e.answers.last().unwrap()["kind"], "domain");
        assert!(!e.persistable);
        domain_question(&mut e, domain_proposal()).unwrap();
        let q = e.question.clone().unwrap();
        assert!(domain_reply(&mut e, &q, "", "  Family & Kinship  ").unwrap());
        assert_eq!(e.domain.as_deref(), Some("family"));
    }

    #[test]
    fn domain_decisions_restore_and_interrupted_suggestions_offer_explicit_retry() {
        let dir = tempfile::tempdir().unwrap();
        let store = LifeStore::open(
            &dir.path().join("intake.sqlite3"),
            LifeCrypto::in_memory_random(),
        )
        .unwrap();
        store.set_consent(DataClass::General, true, "test").unwrap();
        let mut e = sample(DataClass::General);
        e.domain_mode = Some("manual".into());
        domain_question(&mut e, domain_proposal()).unwrap();
        store_entry(&store, &e).unwrap();
        let saved = store.get_record("intake-test-1").unwrap().unwrap();
        let restored = decode_entry(&store, &saved.body).unwrap();
        assert_eq!(restored.status, "domain_selection");
        assert!(restored.domain.is_none());
        assert_eq!(restored.question.unwrap().options.len(), 3);
        e.revision = 2;
        e.status = "domain_planning".into();
        e.question = None;
        store_entry(&store, &e).unwrap();
        let saved = store.get_record("intake-test-2").unwrap().unwrap();
        let restored = decode_entry(&store, &saved.body).unwrap();
        assert_eq!(restored.status, "retry");
        assert!(needs_domain(&restored));
        assert_eq!(restored.question.as_ref().unwrap().kind, "retry");
        assert!(validate_answer(&restored, restored.revision, Some("retry"), "").is_ok());
        assert!(!processing_status(&restored.status));
    }

    #[test]
    fn manual_subdomain_selection_is_explicit_and_validates_parent_membership() {
        let mut e = sample(DataClass::General);
        e.domain = Some("environment".into());
        e.domain_mode = Some("manual".into());
        assert!(!needs_subdomains(&e)); // Stored conversations without progress are not enrolled.
        e.subdomain_progress = Some(crate::intake_subdomains::Progress::new(false));
        assert!(needs_subdomains(&e));
        question(
            &mut e,
            "subdomains",
            "Which parts?",
            "Choose or skip.",
            vec![option(
                "digital-hygiene",
                "Digital Hygiene",
                "Reduce stated distractions.",
            )],
        );
        let q = validate_answer(
            &e,
            e.revision,
            None,
            "[\"digital-hygiene\",\"geographic-fit\"]",
        )
        .unwrap();
        assert_eq!(q.domain.as_deref(), Some("environment"));
        assert!(validate_answer(&e, e.revision - 1, Some("digital-hygiene"), "").is_err());
        assert!(subdomain_reply(&mut e, &q, "", "[\"cardiorespiratory-capacity\"]").is_err());
        assert!(
            subdomain_reply(&mut e, &q, "", "[\"digital-hygiene\",\"digital-hygiene\"]").is_err()
        );
        assert!(e.sub_domains.is_empty());
        subdomain_reply(&mut e, &q, "", "[\"digital-hygiene\",\"geographic-fit\"]").unwrap();
        assert_eq!(e.sub_domains.len(), 2);
        assert!(e.subdomain_progress.as_ref().unwrap().selection_done);
        assert!(!e.persistable);
        assert!(e.question.is_none());
        assert_eq!(subdomain_context(&e)[1]["name"], "Sensory Fit");
        assert_eq!(subdomain_context(&e)[1]["id"], "geographic-fit");
        assert!(subdomain_reply(&mut e, &q, "digital-hygiene", "").is_err());
    }

    #[test]
    fn subdomain_freeform_is_context_not_an_automatic_selection() {
        let mut e = sample(DataClass::General);
        e.domain = Some("environment".into());
        e.subdomain_progress = Some(crate::intake_subdomains::Progress::new(false));
        question(&mut e, "subdomains", "Which parts?", "", vec![]);
        let q = e.question.clone().unwrap();
        subdomain_reply(&mut e, &q, "", "I am unsure; noise seems to be the issue").unwrap();
        assert!(!e.subdomain_progress.as_ref().unwrap().selection_done);
        assert!(e.sub_domains.is_empty());
        assert!(!e.persistable);
        assert_eq!(
            e.answers.last().unwrap()["answer"],
            "I am unsure; noise seems to be the issue"
        );
    }

    fn detail_entry() -> Entry {
        let mut e = sample(DataClass::General);
        e.domain = Some("environment".into());
        e.domain_mode = Some("manual".into());
        e.sub_domains = crate::goal_domains::Selection::manual(
            None,
            e.domain.clone(),
            vec!["digital-hygiene".into(), "geographic-fit".into()],
        )
        .unwrap()
        .sub_domains;
        let mut progress = crate::intake_subdomains::Progress::new(true);
        progress.current = Some("digital-hygiene".into());
        e.subdomain_progress = Some(progress);
        question(
            &mut e,
            "subdomain_detail",
            "What interrupts you?",
            "",
            vec![option("messages", "Messages", "Your stated interruption.")],
        );
        e
    }

    #[test]
    fn details_are_versioned_bounded_and_skips_preserve_partial_answers() {
        let mut e = detail_entry();
        for _ in 0..crate::intake_subdomains::MAX_QUESTIONS {
            let q = validate_answer(&e, e.revision, Some("messages"), "").unwrap();
            subdomain_reply(&mut e, &q, "messages", "").unwrap();
            assert!(!e.persistable);
            assert!(e.question.is_none());
            e.revision += 1;
            question(
                &mut e,
                "subdomain_detail",
                "Another focused question?",
                "",
                vec![option("messages", "Messages", "Context.")],
            );
        }
        assert_eq!(e.sub_domains[0].answers.len(), 3);
        let q = e.question.clone().unwrap();
        assert!(subdomain_reply(&mut e, &q, "messages", "").is_err());
        assert_eq!(
            e.subdomain_progress
                .as_ref()
                .unwrap()
                .next(&e.sub_domains)
                .as_deref(),
            Some("geographic-fit")
        );
        let mut partial = detail_entry();
        let q = partial.question.clone().unwrap();
        subdomain_reply(&mut partial, &q, "", "A specific interruption").unwrap();
        question(
            &mut partial,
            "subdomain_detail",
            "More context?",
            "",
            vec![],
        );
        let q = validate_answer(&partial, partial.revision, Some("skip"), "").unwrap();
        subdomain_reply(&mut partial, &q, "skip", "").unwrap();
        assert!(partial.sub_domains[0].skipped);
        assert_eq!(partial.sub_domains[0].answers.len(), 1);
        assert_eq!(subdomain_context(&partial)[0]["skipped"], true);
    }

    #[test]
    fn skipping_remaining_parts_finishes_context_without_approving_the_goal() {
        let mut e = detail_entry();
        let q = validate_answer(&e, e.revision, Some("skip_all"), "").unwrap();
        subdomain_reply(&mut e, &q, "skip_all", "").unwrap();
        assert!(e.subdomain_progress.as_ref().unwrap().finished);
        assert!(e.sub_domains.iter().all(|s| s.skipped));
        assert!(!needs_subdomains(&e));
        assert!(e.goal.is_none());
        assert!(e.approved_goal.is_none());
        let unrelated = sample(DataClass::General);
        assert!(validate_answer(&unrelated, unrelated.revision, Some("skip"), "").is_err());
        assert!(validate_answer(&unrelated, unrelated.revision, Some("skip_all"), "").is_err());
        let mut selection = detail_entry();
        selection.sub_domains.clear();
        selection.subdomain_progress = Some(crate::intake_subdomains::Progress::new(false));
        question(&mut selection, "subdomains", "Which parts?", "", vec![]);
        let q = validate_answer(&selection, selection.revision, Some("skip_all"), "").unwrap();
        subdomain_reply(&mut selection, &q, "skip_all", "").unwrap();
        assert!(!needs_subdomains(&selection));
        assert!(selection.sub_domains.is_empty());
    }

    #[test]
    fn interrupted_subdomain_planning_restores_progress_for_explicit_retry() {
        let dir = tempfile::tempdir().unwrap();
        let store = LifeStore::open(
            &dir.path().join("intake.sqlite3"),
            LifeCrypto::in_memory_random(),
        )
        .unwrap();
        store.set_consent(DataClass::General, true, "test").unwrap();
        let mut e = detail_entry();
        e.status = "subdomain_planning".into();
        e.question = None;
        e.sub_domains[0]
            .answers
            .push(json!({"answer":"Previously classified context"}));
        store_entry(&store, &e).unwrap();
        let saved = store.get_record("intake-test-1").unwrap().unwrap();
        let restored = decode_entry(&store, &saved.body).unwrap();
        assert_eq!(restored.status, "retry");
        assert!(needs_subdomains(&restored));
        assert_eq!(restored.sub_domains[0].answers.len(), 1);
        assert_eq!(
            restored
                .subdomain_progress
                .as_ref()
                .unwrap()
                .current
                .as_deref(),
            Some("digital-hygiene")
        );
        assert!(processing_status("subdomain_planning"));
        assert!(!processing_status("subdomain_detail"));
        assert!(validate_answer(&restored, restored.revision, Some("retry"), "").is_ok());
    }

    #[test]
    fn auto_starts_with_domain_context_without_synthesizing_goal_text() {
        let e = auto_entry(vec![]);
        assert!(e.text.is_empty());
        assert_eq!(e.kind, "goal");
        assert_eq!(e.domain_mode.as_deref(), Some("auto"));
        assert_eq!(e.domain_source.as_deref(), Some("user"));
        assert_eq!(e.domain.as_deref(), Some("physical"));
        assert_eq!(e.sub_domains.len(), 6);
        assert!(e.goal.is_none() && e.approved_goal.is_none());
        assert!(e.requested_kind.is_none());
        assert!(!e.persistable);
        assert!(e.subdomain_progress.is_none());
        assert!(needs_discovery(&e));
        assert!(!needs_subdomains(&e));
        assert!(processing_status("discovery_planning"));
        assert!(!processing_status("discovery"));
        let restricted = auto_entry(vec!["sleep-and-circadian-architecture".into()]);
        assert_eq!(restricted.sub_domains.len(), 1);
    }

    fn auto_entry(parts: Vec<String>) -> Entry {
        let (text, selection) = crate::goal_domains::Selection::create(
            None,
            Some("physical".into()),
            Some("auto"),
            parts,
        )
        .unwrap();
        new_entry(
            "intake-auto".into(),
            &text,
            false,
            vec![],
            Some("goal".into()),
            Some(selection),
        )
    }

    #[test]
    fn only_the_first_discovery_call_may_cold_load_the_model() {
        let mut e = auto_entry(vec![]);
        assert!(cold_start_discovery(&e));
        e.answers.push(json!({"answer":"Usually rested"}));
        assert!(!cold_start_discovery(&e));
        let mut e = auto_entry(vec![]);
        e.goal = Some(Default::default());
        assert!(!cold_start_discovery(&e));
    }

    fn discovery_question_response(e: &Entry) -> crate::intake_discovery::Response {
        let available = crate::intake_discovery::available(&e.sub_domains);
        crate::intake_discovery::parse(json!({"class":"health","ready":false,"subDomain":available[0],
            "question":"How rested do you usually feel?","explanation":"Start with current recovery context.",
            "options":[{"id":"steady","label":"Usually rested","explanation":"A reported steady baseline."},
                {"id":"mixed","label":"It varies","explanation":"A reported inconsistency."},
                {"id":"unknown","label":"I am unsure","explanation":"The baseline stays unknown."}],"goal":null}),
            &available, crate::intake_discovery::answered(&e.sub_domains, &e.answers), true, false).unwrap()
    }

    fn discovery_draft_response() -> crate::intake_discovery::Response {
        crate::intake_discovery::parse(json!({"class":"health","ready":true,"subDomain":null,"question":"","explanation":"A provisional draft from current context.","options":[],
            "goal":{"outcome":"Explore a consistent wind-down routine","current":"Sleep varies","success":"Unknown","horizon":"Unknown","resources":"Unknown","constraints":"Unknown"}}),
            &[], 1, false, true).unwrap()
    }

    #[test]
    fn auto_draft_review_keeps_clarification_and_goal_approval_separate() {
        let mut e = auto_entry(vec![]);
        let first = discovery_question_response(&e);
        apply_discovery(&mut e, first).unwrap();
        assert_eq!(e.class, DataClass::Health);
        assert!(e.goal.is_none());
        let q = e.question.clone().unwrap();
        assert_eq!(q.kind, "discovery");
        assert_eq!(q.domain.as_deref(), Some("physical"));
        assert!(validate_answer(&e, e.revision + 1, Some("mixed"), "").is_err());
        assert!(validate_answer(&e, e.revision, Some("approve"), "").is_err());
        e.persistable = true; // The actual runner checks class consent before exposing questions.
        discovery_reply(&mut e, &q, "mixed", "").unwrap();
        assert!(!e.persistable);
        assert!(e.text.is_empty());
        assert_eq!(
            crate::intake_discovery::answered(&e.sub_domains, &e.answers),
            1
        );
        apply_discovery(&mut e, discovery_draft_response()).unwrap();
        assert_eq!(e.question.as_ref().unwrap().kind, "discovery_review");
        assert!(e.goal.is_some() && e.approved_goal.is_none());
        assert_eq!(e.domain.as_deref(), Some("physical"));
        assert!(validate_answer(&e, e.revision, Some("approve"), "").is_err());
        assert!(discovery_review_reply(&mut e, "continue", "").unwrap());
        assert!(e.discovery.as_ref().unwrap().phase == crate::intake_discovery::Phase::Clarifying);
        assert!(!needs_discovery(&e));
        assert!(e.approved_goal.is_none() && e.pathways.is_none() && e.task_proposals.is_none());
    }

    #[test]
    fn zero_answer_skips_cannot_generate_a_goal_and_revisiting_is_explicit() {
        let mut e = auto_entry(vec![]);
        assert!(apply_discovery(&mut e, discovery_draft_response()).is_err());
        let first = discovery_question_response(&e);
        apply_discovery(&mut e, first).unwrap();
        let q = validate_answer(&e, e.revision, Some("skip_all"), "").unwrap();
        discovery_reply(&mut e, &q, "skip_all", "").unwrap();
        assert_eq!(
            crate::intake_discovery::answered(&e.sub_domains, &e.answers),
            0
        );
        assert!(crate::intake_discovery::available(&e.sub_domains).is_empty());
        assert!(apply_discovery(&mut e, discovery_draft_response()).is_err());
        assert!(e.goal.is_none());
        discovery_required_question(&mut e);
        let q = e.question.clone().unwrap();
        assert!(validate_answer(&e, e.revision, Some("skip_all"), "").is_err());
        discovery_reply(&mut e, &q, "revisit", "").unwrap();
        assert!(e.sub_domains.iter().all(|s| !s.skipped));
        assert_eq!(
            crate::intake_discovery::answered(&e.sub_domains, &e.answers),
            0
        );
        assert!(e.discovery.as_ref().unwrap().require_question);
    }

    #[test]
    fn a_domain_level_answer_after_skips_is_classified_before_a_provisional_draft() {
        let mut e = auto_entry(vec![]);
        for sub in &mut e.sub_domains {
            sub.skipped = true;
        }
        discovery_required_question(&mut e);
        let q = e.question.clone().unwrap();
        e.persistable = true;
        discovery_reply(&mut e, &q, "", "My current routine feels inconsistent").unwrap();
        assert!(!e.persistable);
        assert_eq!(
            crate::intake_discovery::answered(&e.sub_domains, &e.answers),
            1
        );
        assert!(e.sub_domains.iter().all(|s| s.skipped));
        assert!(e.discovery.as_ref().unwrap().phase == crate::intake_discovery::Phase::Drafting);
        apply_discovery(&mut e, discovery_draft_response()).unwrap();
        assert!(e.goal.is_some() && e.approved_goal.is_none());
    }

    #[test]
    fn draft_edits_preserve_the_previous_definition_and_more_questions_obey_the_budget() {
        let mut e = auto_entry(vec!["sleep-and-circadian-architecture".into()]);
        let first = discovery_question_response(&e);
        apply_discovery(&mut e, first).unwrap();
        let q = e.question.clone().unwrap();
        discovery_reply(&mut e, &q, "mixed", "").unwrap();
        apply_discovery(&mut e, discovery_draft_response()).unwrap();
        let original = e.goal.clone();
        assert!(!discovery_review_reply(&mut e, "refine", "").unwrap());
        assert_eq!(e.goal, original);
        assert!(
            discovery_review_reply(
                &mut e,
                "refine",
                "Keep the draft small; my schedule changes often"
            )
            .unwrap()
        );
        assert_eq!(e.goal, original);
        assert!(!e.persistable);
        assert_eq!(e.answers.last().unwrap()["kind"], "discovery_refinement");
        apply_discovery(&mut e, discovery_draft_response()).unwrap();
        assert!(discovery_review_reply(&mut e, "more", "").unwrap());
        assert!(e.discovery.as_ref().unwrap().require_question);
        for _ in 1..crate::intake_subdomains::MAX_QUESTIONS {
            let next = discovery_question_response(&e);
            apply_discovery(&mut e, next).unwrap();
            let q = e.question.clone().unwrap();
            discovery_reply(&mut e, &q, "mixed", "").unwrap();
        }
        assert!(crate::intake_discovery::available(&e.sub_domains).is_empty());
        assert_eq!(e.sub_domains[0].answers.len(), 3);
        apply_discovery(&mut e, discovery_draft_response()).unwrap();
        assert!(!discovery_review_reply(&mut e, "more", "").unwrap());
        assert_eq!(e.question.as_ref().unwrap().kind, "discovery_review");
        assert!(e.goal.is_some() && e.approved_goal.is_none());
    }

    #[test]
    fn discovery_answers_drafts_and_accepted_phase_restore_from_protected_storage() {
        let dir = tempfile::tempdir().unwrap();
        let store = LifeStore::open(
            &dir.path().join("intake.sqlite3"),
            LifeCrypto::in_memory_random(),
        )
        .unwrap();
        let mut e = auto_entry(vec![]);
        let first = discovery_question_response(&e);
        apply_discovery(&mut e, first).unwrap();
        assert!(store_entry(&store, &e).is_err());
        store.set_consent(DataClass::Health, true, "test").unwrap();
        let q = e.question.clone().unwrap();
        discovery_reply(&mut e, &q, "", "PRIVATE_DISCOVERY_CANARY").unwrap();
        e.persistable = true; // Represents a successful protected reclassification of the answer.
        e.status = "discovery_planning".into();
        store_entry(&store, &e).unwrap();
        let saved = store.get_record("intake-auto-0").unwrap().unwrap();
        assert!(!saved.body.contains("PRIVATE_DISCOVERY_CANARY"));
        let mut restored = decode_entry(&store, &saved.body).unwrap();
        assert_eq!(restored.status, "retry");
        assert!(needs_discovery(&restored));
        assert!(restored.text.is_empty());
        assert_eq!(
            crate::intake_discovery::answered(&restored.sub_domains, &restored.answers),
            1
        );
        restored.revision = 1;
        apply_discovery(&mut restored, discovery_draft_response()).unwrap();
        store_entry(&store, &restored).unwrap();
        let saved = store.get_record("intake-auto-1").unwrap().unwrap();
        let mut reviewing = decode_entry(&store, &saved.body).unwrap();
        assert_eq!(
            reviewing.question.as_ref().unwrap().kind,
            "discovery_review"
        );
        assert!(reviewing.approved_goal.is_none());
        discovery_review_reply(&mut reviewing, "continue", "").unwrap();
        reviewing.revision = 2;
        reviewing.status = "discovery_planning".into();
        store_entry(&store, &reviewing).unwrap();
        let saved = store.get_record("intake-auto-2").unwrap().unwrap();
        let accepted = decode_entry(&store, &saved.body).unwrap();
        assert!(!needs_discovery(&accepted));
        assert!(accepted.goal.is_some() && accepted.approved_goal.is_none());
    }

    #[test]
    fn legacy_entries_decode_without_domain_metadata() {
        let original = sample(DataClass::Mind);
        let mut value = serde_json::to_value(&original).unwrap();
        for key in [
            "domain",
            "domainSource",
            "domainMode",
            "subDomains",
            "subdomainProgress",
            "discovery",
        ] {
            value.as_object_mut().unwrap().remove(key);
        }
        let restored: Entry = serde_json::from_value(value).unwrap();
        assert!(restored.domain.is_none());
        assert!(restored.domain_source.is_none());
        assert!(restored.domain_mode.is_none());
        assert!(restored.sub_domains.is_empty());
        assert!(restored.subdomain_progress.is_none());
        assert!(restored.discovery.is_none());
        assert_eq!(restored.answers, original.answers);
        assert_eq!(restored.class, original.class);
        assert_eq!(restored.status, original.status);
        assert_eq!(workspace_goal_modes(), vec!["manual", "auto"]);
    }

    #[test]
    fn protected_questions_require_consent_and_restore_without_plaintext_bodies() {
        for class in DataClass::all() {
            let path = std::env::temp_dir().join(format!("rune-intake-{}.sqlite3", uid().unwrap()));
            let store = LifeStore::open(&path, LifeCrypto::in_memory_random()).unwrap();
            let mut e = sample(*class);
            e.domain = Some("physical".into());
            e.domain_source = Some("user".into());
            e.domain_mode = Some("manual".into());
            e.sub_domains = vec![crate::goal_domains::SubDomainSelection {
                id: "cardiorespiratory-capacity".into(),
                answers: vec![json!({"answer":"SUBDOMAIN_CANARY"})],
                skipped: false,
            }];
            let mut progress = crate::intake_subdomains::Progress::new(true);
            progress.current = Some("cardiorespiratory-capacity".into());
            e.subdomain_progress = Some(progress);
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
            assert_eq!(restored.domain, e.domain);
            assert_eq!(restored.domain_source, e.domain_source);
            assert_eq!(restored.domain_mode, e.domain_mode);
            assert_eq!(restored.sub_domains, e.sub_domains);
            assert_eq!(
                restored.subdomain_progress.unwrap().current.as_deref(),
                Some("cardiorespiratory-capacity")
            );
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
    fn deleting_a_goal_removes_all_saved_revisions_without_deleting_related_records() {
        let store = LifeStore::in_memory_random().unwrap();
        store.set_consent(DataClass::General, true, "test").unwrap();
        store.set_consent(DataClass::Health, true, "test").unwrap();
        let mut e = sample(DataClass::General);
        store_entry(&store, &e).unwrap();
        e.class = DataClass::Health;
        e.revision = 2;
        store_entry(&store, &e).unwrap();
        for id in [
            "intake-test-other",
            "worktask-linked",
            "intake-test-2-other",
        ] {
            let mut other = sample(DataClass::General);
            other.id = id.into();
            store_entry(&store, &other).unwrap();
        }
        // Older revisions must stay deleted even if their class consent is later enabled again.
        store
            .set_consent(DataClass::General, false, "test")
            .unwrap();
        let token = e.token.clone();
        let mut entries = HashMap::from([(e.id.clone(), e)]);
        delete_goal(&mut entries, Some(&store), "intake-test", 2).unwrap();
        assert!(entries.is_empty());
        assert!(token.is_cancelled());
        store.set_consent(DataClass::General, true, "test").unwrap();
        let remaining = store.list_records(None, false).unwrap();
        assert_eq!(remaining.len(), 3);
        assert!(
            remaining
                .iter()
                .all(|r| r.id != "intake-test-1" && r.id != "intake-test-2")
        );
        assert!(
            store
                .get_record("intake-test-1")
                .unwrap()
                .unwrap()
                .deleted
                .is_some()
        );
        assert!(
            store
                .get_record("intake-test-2")
                .unwrap()
                .unwrap()
                .deleted
                .is_some()
        );
    }

    #[test]
    fn deleting_requires_the_current_goal_revision_and_supports_unsaved_discovery() {
        let e = sample(DataClass::General);
        let token = e.token.clone();
        let mut entries = HashMap::from([(e.id.clone(), e)]);
        assert!(delete_goal(&mut entries, None, "intake-test", 0).is_err());
        assert!(!token.is_cancelled());
        entries.get_mut("intake-test").unwrap().kind = "task".into();
        assert!(delete_goal(&mut entries, None, "intake-test", 1).is_err());
        assert_eq!(entries.len(), 1);
        let e = auto_entry(vec![]);
        let token = e.token.clone();
        entries.insert(e.id.clone(), e);
        delete_goal(&mut entries, None, "intake-auto", 0).unwrap();
        assert!(token.is_cancelled());
        assert!(!entries.contains_key("intake-auto"));
        assert!(entries.contains_key("intake-test"));
        assert!(delete_goal(&mut entries, None, "intake-auto", 0).is_err());
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

fn delete_goal(
    entries: &mut HashMap<String, Entry>,
    store: Option<&LifeStore>,
    id: &str,
    revision: u64,
) -> Result<(), String> {
    let entry = entries.get(id).ok_or("This goal no longer exists")?;
    if entry.kind != "goal" || entry.revision != revision {
        return Err("Review the latest goal before deleting it".into());
    }
    if let Some(store) = store {
        store
            .soft_delete_record_series(id, "studio-intake")
            .map_err(|e| e.to_string())?;
    }
    entry.token.cancel();
    entries.remove(id);
    Ok(())
}

#[tauri::command]
pub async fn decision_delete(app: AppHandle, id: String, revision: u64) -> Result<(), String> {
    let _serial = SERIAL.lock().await;
    load(&app)?;
    let path = crate::life::db_path(&app.path().app_config_dir().map_err(|e| e.to_string())?);
    let store = path.exists().then(|| store(&app)).transpose()?;
    delete_goal(&mut ENTRIES.locked(), store.as_ref(), &id, revision)?;
    let _ = app.emit("decisions_changed", ());
    Ok(())
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
pub fn workspace_goal_modes() -> Vec<String> {
    vec!["manual".into(), "auto".into()]
}
#[tauri::command]
pub fn workspace_new_goal(
    app: AppHandle,
    text: Option<String>,
    domain: Option<String>,
    mode: Option<String>,
    sub_domains: Option<Vec<String>>,
) -> Result<String, String> {
    let (text, selection) = crate::goal_domains::Selection::create(
        text,
        domain,
        mode.as_deref(),
        sub_domains.unwrap_or_default(),
    )?;
    let id = uid()?;
    submit_intent(
        &app,
        &text,
        false,
        vec![],
        Some("goal".into()),
        Some(id.clone()),
        Some(selection),
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
            &json!({"approvedGoal":approved,"category":e.category,"domain":domain_context(&e),"subDomains":subdomain_context(&e)}),
            &crate::intake_pathways::schema(),
            false,
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
        &json!({"approvedGoal":goal,"preferredPathway":pathway,"currentLocalTime":start,"timezone":zone,"existingLinkedTasks":existing,"domain":domain_context(&e),"subDomains":subdomain_context(&e)}),
        &crate::task_proposals::schema(), false)).await.map_err(|_| "Task proposal generation timed out".to_string())
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
        "domain" => match domain_reply(&mut e, &q, choice, &text) {
            Ok(true) => advance(&app, &mut e).await,
            Ok(false) => suggest_domain(&app, &mut e).await,
            Err(error) => Err(error),
        },
        "subdomains" | "subdomain_detail" => match subdomain_reply(&mut e, &q, choice, &text) {
            Ok(()) => {
                if e.persistable {
                    e.status = "subdomain_planning".into();
                    commit(&app, &mut e)?;
                }
                advance(&app, &mut e).await
            }
            Err(error) => Err(error),
        },
        "discovery" | "discovery_required" => match discovery_reply(&mut e, &q, choice, &text) {
            Ok(()) => {
                if e.persistable {
                    e.status = "discovery_planning".into();
                    commit(&app, &mut e)?;
                }
                advance(&app, &mut e).await
            }
            Err(error) => Err(error),
        },
        "discovery_review" => match discovery_review_reply(&mut e, choice, &text) {
            Ok(true) => {
                if e.persistable {
                    e.status = "discovery_planning".into();
                    commit(&app, &mut e)?;
                }
                advance(&app, &mut e).await
            }
            Ok(false) => commit(&app, &mut e),
            Err(error) => Err(error),
        },
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
