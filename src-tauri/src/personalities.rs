//! User-approved style profiles. Categories are labels, never privacy classes or permissions.
use crate::sync::LockExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};
use tauri::{AppHandle, Emitter, Manager};

static MUTATION: Mutex<()> = Mutex::new(());
const SYMBOLS: [&str; 6] = [
    "companion",
    "fitness",
    "finance",
    "learning",
    "creativity",
    "focus",
];
static ACTIVE: LazyLock<Mutex<PersonalityBadge>> = LazyLock::new(|| {
    Mutex::new(PersonalityBadge {
        id: "base".into(),
        name: "Rune".into(),
        mark: "companion".into(),
    })
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Personality {
    pub id: String,
    pub name: String,
    pub mark: String,
    pub categories: Vec<String>,
    pub prompt: String,
}
#[derive(Clone, Serialize)]
pub struct PersonalityBadge {
    pub id: String,
    pub name: String,
    pub mark: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    entries: Vec<Personality>,
    active: PersonalityBadge,
    folder: String,
}

fn base() -> Personality {
    Personality {
        id: "base".into(),
        name: "Rune".into(),
        mark: "companion".into(),
        categories: vec![],
        prompt: include_str!("../prompts/personalities/base.md")
            .trim()
            .into(),
    }
}
fn folder(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("prompts").join("personalities"))
        .map_err(|e| e.to_string())
}
fn validate(p: &Personality) -> Result<(), String> {
    if p.id.is_empty()
        || p.id.len() > 64
        || !p
            .id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("Use a lowercase ID containing letters, digits and hyphens".into());
    }
    if p.name.trim().is_empty()
        || p.name.len() > 80
        || !(p.mark.chars().count() == 1 || SYMBOLS.contains(&p.mark.as_str()))
        || p.prompt.trim().is_empty()
        || p.prompt.len() > 8000
        || p.categories.len() > 32
        || p.categories
            .iter()
            .any(|c| c.trim().is_empty() || c.len() > 80)
    {
        return Err("Provide a name, supported symbol, style prompt (up to 8,000 bytes), and valid categories".into());
    }
    if p.id == "base" && !p.categories.is_empty() {
        return Err("The base personality is the fallback for every unassigned category".into());
    }
    Ok(())
}
fn read(folder: &Path) -> Result<Vec<Personality>, String> {
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let base_path = folder.join("base.json");
    if !base_path.exists() {
        use std::io::Write;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&base_path)
        {
            Ok(mut file) => file
                .write_all(
                    serde_json::to_string_pretty(&base())
                        .map_err(|e| e.to_string())?
                        .as_bytes(),
                )
                .map_err(|e| e.to_string())?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut entries = Vec::new();
    for file in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let file = file.map_err(|e| e.to_string())?;
        if file.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if !file.file_type().map_err(|e| e.to_string())?.is_file()
            || file.metadata().map_err(|e| e.to_string())?.len() > 16000
        {
            return Err("Invalid personality file".into());
        }
        let p: Personality =
            serde_json::from_slice(&fs::read(file.path()).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid personality {}: {e}", file.path().display()))?;
        validate(&p)?;
        if file.path().file_stem().and_then(|s| s.to_str()) != Some(p.id.as_str()) {
            return Err("Personality ID must match its filename".into());
        }
        entries.push(p);
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    for (i, p) in entries.iter().enumerate() {
        if p.categories.iter().any(|c| {
            entries[..i].iter().any(|other| {
                other
                    .categories
                    .iter()
                    .any(|x| x.trim().eq_ignore_ascii_case(c.trim()))
            })
        }) {
            return Err("A category is assigned to multiple personalities. Resolve the conflict before continuing.".into());
        }
    }
    Ok(entries)
}
#[tauri::command]
pub fn personalities_state(app: AppHandle) -> Result<Catalog, String> {
    let _guard = MUTATION.locked();
    let folder = folder(&app)?;
    Ok(Catalog {
        entries: read(&folder)?,
        active: ACTIVE.locked().clone(),
        folder: folder.to_string_lossy().into(),
    })
}
fn save(
    folder: &Path,
    profile: &Personality,
    expected: Option<&Personality>,
) -> Result<(), String> {
    validate(profile)?;
    let entries = read(folder)?;
    if entries.iter().find(|p| p.id == profile.id) != expected {
        return Err(
            "Conflict: this personality changed. Reload and review both versions before saving."
                .into(),
        );
    }
    if entries.iter().filter(|p| p.id != profile.id).any(|p| {
        p.categories.iter().any(|c| {
            profile
                .categories
                .iter()
                .any(|other| other.trim().eq_ignore_ascii_case(c.trim()))
        })
    }) {
        return Err(
            "Conflict: a category already has a personality. Edit its assignment first.".into(),
        );
    }
    let temporary = folder.join(format!("{}.pending", profile.id));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(profile).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temporary, folder.join(format!("{}.json", profile.id))).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn personality_save(
    app: AppHandle,
    profile: Personality,
    expected: Option<Personality>,
) -> Result<Catalog, String> {
    {
        let _guard = MUTATION.locked();
        save(&folder(&app)?, &profile, expected.as_ref())?;
    }
    let _ = app.emit("personalities_changed", ());
    personalities_state(app)
}

fn proposed(category: &str, output: &serde_json::Value, id: String) -> Result<Personality, String> {
    let profile = Personality {
        id,
        name: output["name"].as_str().unwrap_or("").into(),
        mark: output["mark"].as_str().unwrap_or("").into(),
        categories: vec![category.into()],
        prompt: output["prompt"].as_str().unwrap_or("").into(),
    };
    validate(&profile)?;
    if !SYMBOLS.contains(&profile.mark.as_str()) {
        return Err("The model returned an unsupported personality symbol".into());
    }
    Ok(profile)
}

#[tauri::command]
pub async fn personality_propose(app: AppHandle, category: String) -> Result<Personality, String> {
    let category = category.trim();
    if category.is_empty() || category.len() > 80 {
        return Err("Enter a category up to 80 bytes".into());
    }
    let state = app.state::<crate::events::AppState>();
    let settings = state.settings.locked().clone();
    let key = crate::settings::api_key();
    let model = settings.model_for_role(crate::model::role::ModelRole::Agent);
    let profile =
        crate::model::protected::discover(&state.http, &settings.base_url, key.as_deref(), model)
            .await?;
    let client = crate::model::client::ChatClient {
        http: &state.http,
        base_url: &settings.base_url,
        api_key: key.as_deref(),
        model,
        profile: Some(&profile),
        sink: None,
        no_swap: true,
        on_loading: None,
    };
    let input = serde_json::json!({"category":category}).to_string();
    let request = crate::model::client::Request {
        system: include_str!("../prompts/personalities/propose.md"),
        user_text: &input,
        max_tokens: Some(1200),
        ..Default::default()
    };
    let schema = serde_json::json!({"type":"object","additionalProperties":false,"properties":{"name":{"type":"string"},"mark":{"type":"string","enum":SYMBOLS},"prompt":{"type":"string"}},"required":["name","mark","prompt"]});
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        client.call_json_retrying(&request, &schema),
    )
    .await
    .map_err(|_| {
        "Personality suggestion timed out. Try again when the model is available.".to_string()
    })?
    .map_err(|e| e.to_string())?;
    let serial = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    proposed(category, &output, format!("personality-{serial:x}"))
}
pub fn style(app: &AppHandle, category: &str) -> Result<String, String> {
    let _guard = MUTATION.locked();
    let entries = read(&folder(app)?)?;
    let profile = entries
        .iter()
        .find(|p| {
            p.categories
                .iter()
                .any(|c| c.trim().eq_ignore_ascii_case(category.trim()))
        })
        .or_else(|| entries.iter().find(|p| p.id == "base"))
        .ok_or("Base personality missing")?;
    let badge = PersonalityBadge {
        id: profile.id.clone(),
        name: profile.name.clone(),
        mark: profile.mark.clone(),
    };
    *ACTIVE.locked() = badge.clone();
    let _ = app.emit("personality_active", badge);
    Ok(format!(
        "\n{}\n{}",
        include_str!("../prompts/personalities/style-contract.md"),
        profile.prompt
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_suggestions_validate_symbols_and_preserve_the_requested_category() {
        let output = serde_json::json!({"name":"Fitness coach","mark":"fitness","prompt":"Encourage consistency and ask about capacity."});
        let profile = proposed("fitness", &output, "fitness-draft".into()).unwrap();
        assert_eq!(profile.categories, vec!["fitness"]);
        assert_eq!(profile.mark, "fitness");
        assert!(
            proposed(
                "fitness",
                &serde_json::json!({"name":"Coach","mark":"unsupported","prompt":"Be clear"}),
                "draft".into()
            )
            .is_err()
        );
        assert!(
            proposed(
                "fitness",
                &serde_json::json!({"name":"Coach","mark":"fitness"}),
                "draft".into()
            )
            .is_err()
        );
    }
    #[test]
    fn rejects_stale_edits_duplicate_assignments_and_unsafe_ids() {
        let dir = tempfile::tempdir().unwrap();
        let original = read(dir.path()).unwrap().remove(0);
        let mut changed = original.clone();
        changed.prompt = "A new communication style".into();
        save(dir.path(), &changed, Some(&original)).unwrap();
        assert!(
            save(dir.path(), &original, Some(&original))
                .unwrap_err()
                .contains("Conflict")
        );
        let mut coach = base();
        coach.id = "fitness".into();
        coach.categories = vec!["Fitness".into()];
        save(dir.path(), &coach, None).unwrap();
        coach.id = "another".into();
        assert!(save(dir.path(), &coach, None).is_err());
        coach.id = "../outside".into();
        assert!(save(dir.path(), &coach, None).is_err());
        assert_eq!(read(dir.path()).unwrap().len(), 2);
    }
}
