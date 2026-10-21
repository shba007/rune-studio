use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::settings::VerifyMode;

pub const SUITE_VERSION: u32 = 1;

pub const KILL_ALLOWLIST: [&str; 5] = [
    "notepad.exe",
    "CalculatorApp.exe",
    "Calculator.exe",
    "SystemSettings.exe",
    "mspaint.exe",
];

const MAX_REPEATS: u32 = 20;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    pub version: u32,
    pub name: String,
    #[serde(default = "one")]
    pub repeats: u32,
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub config: Config,
    pub tasks: Vec<Task>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Defaults {
    pub timeout_secs: u64,
    pub max_steps: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            timeout_secs: 240,
            max_steps: 30,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    pub fast_checks: Option<bool>,
    pub show_thinking: Option<bool>,
    pub model: Option<String>,
    pub verify_mode: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    #[default]
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub setup: Vec<Step>,
    pub check: Check,
    #[serde(default)]
    pub expect: Expect,
    #[serde(default)]
    pub accept_outcomes: Option<Vec<String>>,
    #[serde(default)]
    pub teardown: Vec<Step>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    MakeDir { path: String },
    WriteFile { path: String, content: String },
    DeletePath { path: String },
    KillProcess { name: String },
    WaitMs { ms: u64 },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Check {
    FileExists { path: String },
    DirExists { path: String },
    FileContains { path: String, text: String },
    FileAbsent { path: String },
    AnyWindowTitleContains { text: String },
    ProcessRunning { name: String },
    All { checks: Vec<Check> },
    Any { checks: Vec<Check> },
    Not { check: Box<Check> },
}

#[derive(Debug, Clone)]
pub struct Vars {
    pub work: PathBuf,
    pub run: String,
    pub fixtures: PathBuf,
    pub fixtures_url: String,
}

impl Vars {
    pub fn substitute(&self, text: &str) -> String {
        text.replace("{work}", &self.work.display().to_string())
            .replace("{run}", &self.run)
            .replace("{fixtures_url}", &self.fixtures_url)
            .replace("{fixtures}", &self.fixtures.display().to_string())
    }
}

impl Step {
    fn map(&self, f: &dyn Fn(&str) -> String) -> Step {
        match self {
            Step::MakeDir { path } => Step::MakeDir { path: f(path) },
            Step::WriteFile { path, content } => Step::WriteFile {
                path: f(path),
                content: f(content),
            },
            Step::DeletePath { path } => Step::DeletePath { path: f(path) },
            Step::KillProcess { name } => Step::KillProcess { name: f(name) },
            Step::WaitMs { ms } => Step::WaitMs { ms: *ms },
        }
    }

    fn path(&self) -> Option<&str> {
        match self {
            Step::MakeDir { path } | Step::WriteFile { path, .. } | Step::DeletePath { path } => {
                Some(path)
            }
            Step::KillProcess { .. } | Step::WaitMs { .. } => None,
        }
    }
}

impl Check {
    fn map(&self, f: &dyn Fn(&str) -> String) -> Check {
        match self {
            Check::FileExists { path } => Check::FileExists { path: f(path) },
            Check::DirExists { path } => Check::DirExists { path: f(path) },
            Check::FileContains { path, text } => Check::FileContains {
                path: f(path),
                text: f(text),
            },
            Check::FileAbsent { path } => Check::FileAbsent { path: f(path) },
            Check::AnyWindowTitleContains { text } => {
                Check::AnyWindowTitleContains { text: f(text) }
            }
            Check::ProcessRunning { name } => Check::ProcessRunning { name: f(name) },
            Check::All { checks } => Check::All {
                checks: checks.iter().map(|check| check.map(f)).collect(),
            },
            Check::Any { checks } => Check::Any {
                checks: checks.iter().map(|check| check.map(f)).collect(),
            },
            Check::Not { check } => Check::Not {
                check: Box::new(check.map(f)),
            },
        }
    }

    fn collect(&self, paths: &mut Vec<String>) {
        match self {
            Check::FileExists { path }
            | Check::DirExists { path }
            | Check::FileContains { path, .. }
            | Check::FileAbsent { path } => paths.push(path.clone()),
            Check::ProcessRunning { .. } | Check::AnyWindowTitleContains { .. } => {}
            Check::All { checks } | Check::Any { checks } => {
                checks.iter().for_each(|check| check.collect(paths))
            }
            Check::Not { check } => check.collect(paths),
        }
    }
}

impl Task {
    pub fn resolve(&self, vars: &Vars) -> Task {
        let f = |text: &str| vars.substitute(text);
        Task {
            id: self.id.clone(),
            prompt: vars.substitute(&self.prompt),
            setup: self.setup.iter().map(|step| step.map(&f)).collect(),
            check: self.check.map(&f),
            expect: self.expect,
            accept_outcomes: self.accept_outcomes.clone(),
            teardown: self.teardown.iter().map(|step| step.map(&f)).collect(),
            timeout_secs: self.timeout_secs,
        }
    }

    pub fn confine(&self, roots: &[PathBuf]) -> Result<(), String> {
        let mut paths: Vec<String> = Vec::new();
        let mut kills: Vec<String> = Vec::new();
        for step in self.setup.iter().chain(&self.teardown) {
            paths.extend(step.path().map(String::from));
            if let Step::KillProcess { name } = step {
                kills.push(name.clone());
            }
        }
        self.check.collect(&mut paths);
        for path in &paths {
            jail(path, roots).map_err(|why| format!("task {}: {why}", self.id))?;
        }
        for name in &kills {
            if !kill_allowed(name) {
                return Err(format!(
                    "task {}: {name} is not on the list of programs a suite may close",
                    self.id
                ));
            }
        }
        Ok(())
    }

    pub fn accepted_outcomes(&self) -> Vec<String> {
        self.accept_outcomes
            .clone()
            .unwrap_or_else(|| vec!["reported_failure".to_string(), "stopped".to_string()])
    }
}

pub fn parse_verify_mode(word: &str) -> Option<VerifyMode> {
    match word {
        "off" => Some(VerifyMode::Off),
        "background" => Some(VerifyMode::Background),
        "blocking" => Some(VerifyMode::Blocking),
        _ => None,
    }
}

impl Suite {
    pub fn verify_mode(&self) -> VerifyMode {
        self.config
            .verify_mode
            .as_deref()
            .and_then(parse_verify_mode)
            .unwrap_or(VerifyMode::Off)
    }

    pub fn parse(text: &str) -> Result<Suite, String> {
        let suite: Suite =
            serde_json::from_str(text).map_err(|err| format!("the suite is not valid: {err}"))?;
        suite.validate()?;
        Ok(suite)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != SUITE_VERSION {
            return Err(format!(
                "the suite is version {}, this build reads version {SUITE_VERSION}",
                self.version
            ));
        }
        if self.tasks.is_empty() {
            return Err("the suite has no tasks".into());
        }
        if self.repeats == 0 || self.repeats > MAX_REPEATS {
            return Err(format!("repeats must be from 1 to {MAX_REPEATS}"));
        }
        if let Some(word) = &self.config.verify_mode
            && parse_verify_mode(word).is_none()
        {
            return Err(format!(
                "config.verify_mode \"{word}\" must be off, background or blocking"
            ));
        }
        if self.defaults.timeout_secs == 0 || self.defaults.max_steps == 0 {
            return Err("the defaults need a timeout and a step limit above zero".into());
        }
        let mut seen: Vec<&str> = Vec::new();
        for task in &self.tasks {
            let well_formed = !task.id.is_empty()
                && task
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !well_formed {
                return Err(format!(
                    "task id \"{}\" must be lower case letters, digits and dashes",
                    task.id
                ));
            }
            if seen.contains(&task.id.as_str()) {
                return Err(format!("task id \"{}\" is used twice", task.id));
            }
            seen.push(&task.id);
            if task.prompt.trim().is_empty() {
                return Err(format!("task {} has no prompt", task.id));
            }
            if task.timeout_secs == Some(0) {
                return Err(format!("task {} has a zero timeout", task.id));
            }
            for step in task.setup.iter().chain(&task.teardown) {
                if let Step::KillProcess { name } = step
                    && !kill_allowed(name)
                {
                    return Err(format!(
                        "task {}: {name} is not on the list of programs a suite may close",
                        task.id
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn kill_allowed(name: &str) -> bool {
    KILL_ALLOWLIST
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(name))
}

pub fn jail(path: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let candidate = Path::new(path);
    if !candidate.is_absolute() {
        return Err(format!("path \"{path}\" is not an absolute path"));
    }
    if candidate
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(format!("path \"{path}\" has a \"..\" in it"));
    }
    let Some(root) = roots.iter().find(|root| candidate.starts_with(root)) else {
        return Err(format!(
            "path \"{path}\" is outside the work and output folders"
        ));
    };
    let existing = candidate.ancestors().find(|part| part.exists());
    if let Some(existing) = existing.filter(|part| part.starts_with(root))
        && let (Ok(real), Ok(real_root)) = (existing.canonicalize(), root.canonicalize())
        && !real.starts_with(&real_root)
    {
        return Err(format!(
            "path \"{path}\" leads outside the work and output folders through a link"
        ));
    }
    Ok(candidate.to_path_buf())
}
