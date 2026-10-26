use std::{
    io::{BufRead, Write},
    net::{SocketAddr, TcpStream},
    process::{Command as Process, Stdio},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::{
    agents, identity, server,
    skill::{self, GUIDE},
    token,
};
use crate::{
    analysis::report::Finding,
    settings,
    trace::{
        self, AnalysisRecord, StepRecord,
        store::{RunDetail, RunSummary},
    },
};

const START_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_RUNS: usize = 20;

#[derive(Debug, PartialEq, Eq)]
pub enum LifeCommand {
    Status,
    Export {
        class: Option<String>,
        folder: String,
    },
    Delete {
        class: Option<String>,
        yes: bool,
    },
    Audit {
        limit: usize,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    App,
    Mcp(Option<String>),
    Status,
    Enable,
    Disable,
    Connect(String),
    Guide,
    Skill(String),
    Disconnect(String),
    Runs(usize),
    Run(String, bool),
    Analyses(usize),
    Analysis(String),
    Reflections(usize),
    Life(LifeCommand),
    Help,
    Unknown(String),
}

pub fn parse(args: &[String]) -> Command {
    let Some(first) = args.first() else {
        return Command::App;
    };
    match first.as_str() {
        "mcp" => match args.get(1).map(String::as_str) {
            None => Command::Mcp(None),
            Some("--agent") => match args.get(2).filter(|id| !id.starts_with('-')) {
                Some(id) => Command::Mcp(Some(id.clone())),
                None => Command::Unknown("mcp --agent (needs an id)".into()),
            },
            Some(other) => Command::Unknown(format!("mcp {other}")),
        },
        "status" => Command::Status,
        "enable" => Command::Enable,
        "disable" => Command::Disable,
        "guide" => Command::Guide,
        "skill" => Command::Skill(args.get(1).cloned().unwrap_or_default()),
        "disconnect" => Command::Disconnect(args.get(1).cloned().unwrap_or_default()),
        "runs" => match args.get(1).map(String::as_str) {
            None => Command::Runs(DEFAULT_RUNS),
            Some("--limit") => match args.get(2).and_then(|number| number.parse::<usize>().ok()) {
                Some(limit) => Command::Runs(limit),
                None => Command::Unknown("runs --limit (needs a number)".into()),
            },
            Some(other) => Command::Unknown(format!("runs {other}")),
        },
        "run" => {
            let rest = &args[1.min(args.len())..];
            let calls = rest.iter().any(|word| word == "--calls");
            let id = rest
                .iter()
                .find(|word| !word.starts_with("--"))
                .cloned()
                .unwrap_or_default();
            match rest
                .iter()
                .find(|word| word.starts_with("--") && *word != "--calls")
            {
                Some(other) => Command::Unknown(format!("run {other}")),
                None => Command::Run(id, calls),
            }
        }
        "analyses" => match args.get(1).map(String::as_str) {
            None => Command::Analyses(DEFAULT_RUNS),
            Some("--limit") => match args.get(2).and_then(|number| number.parse::<usize>().ok()) {
                Some(limit) => Command::Analyses(limit),
                None => Command::Unknown("analyses --limit (needs a number)".into()),
            },
            Some(other) => Command::Unknown(format!("analyses {other}")),
        },
        "reflections" => match args.get(1).map(String::as_str) {
            None => Command::Reflections(DEFAULT_RUNS),
            Some("--limit") => match args.get(2).and_then(|number| number.parse::<usize>().ok()) {
                Some(limit) => Command::Reflections(limit),
                None => Command::Unknown("reflections --limit (needs a number)".into()),
            },
            Some(other) => Command::Unknown(format!("reflections {other}")),
        },
        "analysis" => Command::Analysis(args.get(1).cloned().unwrap_or_default()),
        "life" => {
            let sub = args.get(1).map(String::as_str);
            match sub {
                Some("status") => Command::Life(LifeCommand::Status),
                Some("export") => {
                    let target = args.get(2).cloned().unwrap_or_default();
                    let folder = args.get(3).cloned().unwrap_or_default();
                    if target.is_empty() || folder.is_empty() {
                        Command::Unknown("life export <class|all> <folder>".into())
                    } else {
                        let class = if target.eq_ignore_ascii_case("all") {
                            None
                        } else {
                            Some(target)
                        };
                        Command::Life(LifeCommand::Export { class, folder })
                    }
                }
                Some("delete") => {
                    let target = args.get(2).cloned().unwrap_or_default();
                    let yes = args.iter().any(|arg| arg == "--yes");
                    if target.is_empty() {
                        Command::Unknown("life delete <class|all> --yes".into())
                    } else {
                        let class = if target.eq_ignore_ascii_case("all") {
                            None
                        } else {
                            Some(target)
                        };
                        Command::Life(LifeCommand::Delete { class, yes })
                    }
                }
                Some("audit") => {
                    let limit = match args.get(2).map(String::as_str) {
                        Some("--limit") => args
                            .get(3)
                            .and_then(|n| n.parse::<usize>().ok())
                            .unwrap_or(20),
                        _ => 20,
                    };
                    Command::Life(LifeCommand::Audit { limit })
                }
                Some(other) => Command::Unknown(format!("life {other}")),
                None => Command::Unknown("life <status|export|delete|audit>".into()),
            }
        }
        "help" | "--help" | "-h" => Command::Help,
        "connect" => Command::Connect(args.get(1).cloned().unwrap_or_default()),
        other => Command::Unknown(other.to_string()),
    }
}

const HELP: &str = "\
Rune Studio

  rune-studio-cli                  Open the app.
  rune-studio-cli connect claude   Connect Claude Code to Rune Studio (works in every project):
                               registers the tools and installs the Rune Studio skill.
  rune-studio-cli connect antigravity  Connect Antigravity the same way (settings file and skill).
  rune-studio-cli connect --print  Show the settings to paste into any other MCP-capable agent.
  rune-studio-cli skill            Install or refresh the Claude Code skill only (--remove deletes it).
  rune-studio-cli disconnect <agent>  Remove Rune Studio from claude or antigravity (tools and skill).
  rune-studio-cli enable           Allow outside agents to see and use this PC through Rune Studio.
  rune-studio-cli disable          Turn that off again.
  rune-studio-cli status           Show whether Rune Studio is running and what is allowed.
  rune-studio-cli guide            Print the guide an agent reads.
  rune-studio-cli runs [--limit N]  List the newest recorded runs (default 20).
  rune-studio-cli run <id> [--calls]  Show one recorded run: its steps and events (--calls: every model attempt).
  rune-studio-cli analyses [--limit N]  List the newest screen analyses (default 20).
  rune-studio-cli reflections [--limit N]  List the newest idle reflection runs (default 20).
  rune-studio-cli analysis <id>    Show one visual analysis: question, answer and findings.
  rune-studio-cli life status      Show life module status, consents, and counts.
  rune-studio-cli life export <class|all> <folder>  Export records as JSON and Markdown.
  rune-studio-cli life delete <class|all> --yes     Permanently delete records and vacuum.
  rune-studio-cli life audit [--limit N]            List life audit trail.
  rune-studio-cli mcp              Used by agents: speaks MCP on stdin/stdout and opens the app if needed.

You are always in control: press Stop in the app (Ctrl+Alt+Esc) and the agent loses control at once.
";

pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Command::App => open_app(),
        Command::Mcp(agent) => run_mcp(agent.as_deref()),
        Command::Status => status(),
        Command::Enable => set_switch(true),
        Command::Disable => set_switch(false),
        Command::Connect(target) => connect(&target),
        Command::Skill(option) => skill_command(&option),
        Command::Disconnect(target) => disconnect(&target),
        Command::Runs(limit) => runs_command(limit),
        Command::Run(id, calls) => run_command(&id, calls),
        Command::Analyses(limit) => analyses_command(limit),
        Command::Analysis(id) => analysis_command(&id),
        Command::Reflections(limit) => reflections_command(limit),
        Command::Life(cmd) => life_command(cmd),
        Command::Guide => {
            say(GUIDE);
            0
        }
        Command::Help => {
            say(HELP);
            0
        }
        Command::Unknown(name) => {
            say(&format!(
                "Unknown command \"{name}\".

{HELP}"
            ));
            2
        }
    }
}

fn runs_command(limit: usize) -> i32 {
    with_trace(|path| match trace::store::list_runs(path, limit) {
        Ok(rows) if rows.is_empty() => {
            say("No runs are recorded yet.");
            0
        }
        Ok(rows) => {
            say(&runs_table(&rows));
            0
        }
        Err(err) => {
            say(&format!("Could not read the trace database: {err}"));
            1
        }
    })
}

fn run_command(id: &str, calls: bool) -> i32 {
    if id.is_empty() {
        say("Say which run: rune-studio-cli run <id>   (see rune-studio-cli runs for the ids)");
        return 2;
    }
    with_trace(|path| {
        let (text, code) = lookup_outcome(
            "run",
            id,
            "runs",
            trace::store::get_run(path, id).map(|found| found.map(|detail| run_report(&detail))),
        );
        say(&text);
        if calls && code == 0 {
            match crate::flight::calls(path, id, None) {
                Ok(list) => say(&calls_report(&list)),
                Err(err) => say(&format!("Could not read the model attempts: {err}")),
            }
        }
        code
    })
}

fn calls_report(list: &[crate::flight::FlightCall]) -> String {
    if list.is_empty() {
        return "\nModel attempts: none recorded (this run is older than the attempt record)."
            .into();
    }
    let mut out = format!(
        "\nModel attempts ({})\n  STEP  KIND      TRY  WAIT_MS  DUR_MS  STATUS  OUTCOME    TOKENS\n",
        list.len()
    );
    for call in list {
        out.push_str(&format!(
            "  {:>4}  {:<8}  {:>3}  {:>7}  {:>6}  {:>6}  {:<9}  {}{}\n",
            number(call.step),
            call.kind,
            call.attempt,
            call.wait_before_ms,
            call.duration_ms,
            number(call.status),
            call.outcome,
            number(call.prompt_tokens),
            call.error
                .as_deref()
                .map(|error| format!("  {}", one_line(error, 70)))
                .unwrap_or_default(),
        ));
    }
    out
}

fn analyses_command(limit: usize) -> i32 {
    with_trace(|path| match trace::store::list_analyses(path, limit) {
        Ok(rows) if rows.is_empty() => {
            say("No screen analyses are recorded yet.");
            0
        }
        Ok(rows) => {
            say(&analyses_table(&rows));
            0
        }
        Err(err) => {
            say(&format!("Could not read the trace database: {err}"));
            1
        }
    })
}

fn analysis_command(id: &str) -> i32 {
    if id.is_empty() {
        say(
            "Say which analysis: rune-studio-cli analysis <id>   (see rune-studio-cli analyses for the ids)",
        );
        return 2;
    }
    with_trace(|path| {
        let (text, code) = lookup_outcome(
            "analysis",
            id,
            "analyses",
            trace::store::get_analysis(path, id)
                .map(|found| found.map(|record| analysis_report(&record))),
        );
        say(&text);
        code
    })
}

fn lookup_outcome(
    noun: &str,
    id: &str,
    list_command: &str,
    found: Result<Option<String>, trace::store::StoreError>,
) -> (String, i32) {
    match found {
        Ok(Some(text)) => (text, 0),
        Ok(None) => (
            format!("There is no {noun} \"{id}\". See rune-studio-cli {list_command} for the ids."),
            1,
        ),
        Err(err) => (format!("Could not read the trace database: {err}"), 1),
    }
}

fn with_life_store<F>(show: F) -> i32
where
    F: FnOnce(&crate::life::LifeStore, &std::path::Path) -> i32,
{
    let Some(path) = crate::life::db_path_without_app() else {
        say("Cannot tell where Rune Studio keeps its data on this system (no APPDATA folder).");
        return 1;
    };
    let crypto = match crate::life::LifeCrypto::from_keyring_or_create() {
        Ok(c) => c,
        Err(err) => {
            say(&format!("Could not access life encryption key: {err}"));
            return 1;
        }
    };
    match crate::life::LifeStore::open(&path, crypto) {
        Ok(store) => show(&store, &path),
        Err(err) => {
            say(&format!("Could not open life database: {err}"));
            1
        }
    }
}

fn life_command(cmd: LifeCommand) -> i32 {
    match cmd {
        LifeCommand::Status => with_life_store(|store, path| {
            let settings = settings::path_without_app()
                .map(|p| settings::read_file(&p))
                .unwrap_or_default();

            say("Rune Life Harness Status");
            say(&format!(
                "  Master switch:   {}",
                if settings.life_enabled {
                    "enabled"
                } else {
                    "disabled (off by default)"
                }
            ));
            say("  Consents:");
            for class in [
                crate::life::DataClass::General,
                crate::life::DataClass::Health,
                crate::life::DataClass::Mind,
                crate::life::DataClass::Finance,
            ] {
                let name = class.as_str();
                let consent =
                    store.get_consent(class).unwrap_or(false) || settings.life_consent(name);
                let ret = settings.retention_days_for(name);
                let ret_str = if ret == 0 {
                    "forever".to_string()
                } else {
                    format!("{ret} days")
                };
                let recs = store
                    .list_records(Some(class), false)
                    .map(|r| r.len())
                    .unwrap_or(0);
                say(&format!(
                    "    {:<8}: consent={:<5} retention={:<10} records={}",
                    name,
                    if consent { "yes" } else { "no" },
                    ret_str,
                    recs
                ));
            }
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            say(&format!(
                "  Database:        {} ({size} bytes)",
                path.display()
            ));
            say("  Encryption:      AES-256-GCM (key in OS keyring)");
            0
        }),
        LifeCommand::Export { class, folder } => with_life_store(|store, _path| {
            let data_class = match class.as_deref() {
                Some(name) => match name.parse::<crate::life::DataClass>() {
                    Ok(c) => Some(c),
                    Err(err) => {
                        say(&format!("Invalid data class '{name}': {err}"));
                        return 2;
                    }
                },
                None => None,
            };
            let out_path = std::path::Path::new(&folder);
            match store.export_class(data_class, out_path) {
                Ok(summary) => {
                    say(&format!(
                        "Exported {} records and {} assumptions to {}\nFiles written: {}",
                        summary.records_count,
                        summary.assumptions_count,
                        out_path.display(),
                        summary.files_written.join(", ")
                    ));
                    0
                }
                Err(err) => {
                    say(&format!("Export failed: {err}"));
                    1
                }
            }
        }),
        LifeCommand::Delete { class, yes } => {
            if !yes {
                say(
                    "Refused: permanent deletion requires '--yes' (e.g. rune-studio-cli life delete all --yes)",
                );
                return 2;
            }
            with_life_store(|store, _path| {
                let data_class = match class.as_deref() {
                    Some(name) => match name.parse::<crate::life::DataClass>() {
                        Ok(c) => Some(c),
                        Err(err) => {
                            say(&format!("Invalid data class '{name}': {err}"));
                            return 2;
                        }
                    },
                    None => None,
                };
                match store.hard_delete_class(data_class, "cli") {
                    Ok(count) => {
                        say(&format!(
                            "Deleted {count} records and associated content audit rows. Database checkpointed and vacuumed."
                        ));
                        0
                    }
                    Err(err) => {
                        say(&format!("Delete failed: {err}"));
                        1
                    }
                }
            })
        }
        LifeCommand::Audit { limit } => {
            with_life_store(|store, _path| match store.list_audit(limit) {
                Ok(rows) if rows.is_empty() => {
                    say("No audit entries yet.");
                    0
                }
                Ok(rows) => {
                    say(&format!(
                        "{:<6} {:<24} {:<12} {:<20} {:<20} DETAIL",
                        "ID", "AT_UTC", "ACTOR", "ACTION", "SUBJECT"
                    ));
                    say(&"-".repeat(95));
                    for r in rows {
                        say(&format!(
                            "{:<6} {:<24} {:<12} {:<20} {:<20} {}",
                            r.id,
                            r.at_utc,
                            r.actor,
                            r.action,
                            r.subject,
                            r.detail.unwrap_or_default()
                        ));
                    }
                    0
                }
                Err(err) => {
                    say(&format!("Could not read audit log: {err}"));
                    1
                }
            })
        }
    }
}

fn reflections_command(limit: usize) -> i32 {
    with_trace(|path| match trace::store::list_reflections(path, limit) {
        Ok(rows) if rows.is_empty() => {
            say("No reflections are recorded yet.");
            0
        }
        Ok(rows) => {
            say(&reflections_table(&rows));
            0
        }
        Err(err) => {
            say(&format!("Could not read the trace database: {err}"));
            1
        }
    })
}

fn reflections_table(rows: &[trace::ReflectionRecord]) -> String {
    let mut lines = vec![["ID", "STARTED", "OUTCOME", "STAGES", "DURATION"].map(String::from)];
    for row in rows {
        lines.push([
            row.id.clone(),
            shown_time(
                Some(&row.started_utc)
                    .filter(|time| !time.is_empty())
                    .map(String::as_str),
            ),
            row.outcome.clone(),
            row.stages.to_string(),
            shown_duration(Some(&row.started_utc), row.ended_utc.as_deref()),
        ]);
    }
    let widths: Vec<usize> = (0..5)
        .map(|column| {
            lines
                .iter()
                .map(|line| line[column].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for line in &lines {
        let cells: Vec<String> = line
            .iter()
            .enumerate()
            .map(|(column, cell)| format!("{cell:<width$}", width = widths[column]))
            .collect();
        out.push_str(cells.join("  ").trim_end());
        out.push('\n');
    }
    out
}

fn with_trace(show: impl FnOnce(&std::path::Path) -> i32) -> i32 {
    let Some(path) = trace::db_path_without_app() else {
        say("Cannot tell where Rune Studio keeps its data on this system (no APPDATA folder).");
        return 1;
    };
    if !path.exists() {
        say(&format!(
            "No trace database yet ({}). It is created the first time a task runs.",
            path.display()
        ));
        return 0;
    }
    show(&path)
}

fn say(text: &str) {
    let mut out = std::io::stdout();
    let _ = out.write_all(text.as_bytes());
    if !text.ends_with('\n') {
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}

fn door_is_open() -> bool {
    let address: SocketAddr = ([127, 0, 0, 1], server::PORT).into();
    TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok()
}

fn switch_is_on() -> bool {
    settings::path_without_app().is_some_and(|path| settings::read_file(&path).external_agent)
}

#[derive(Debug, PartialEq, Eq)]
enum Door {
    Closed,
    Ours,
    Taken,
}

fn classify_door(answer: Option<&str>) -> Door {
    let Some(answer) = answer else {
        return Door::Closed;
    };
    let body = answer.split_once("\r\n\r\n").map_or("", |(_, body)| body);
    let json: Option<Value> = serde_json::from_str(body.trim()).ok();
    let ours = json.is_some_and(|json| {
        json["jsonrpc"] == "2.0" || (json["error"].is_string() && json["message"].is_string())
    });
    if ours { Door::Ours } else { Door::Taken }
}

fn probe_door() -> Door {
    let address: SocketAddr = ([127, 0, 0, 1], server::PORT).into();
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(300)) else {
        return Door::Closed;
    };
    let body = "{}";
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        server::PORT,
        body.len()
    );
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    if stream.write_all(request.as_bytes()).is_err() {
        return Door::Taken;
    }
    let mut answer = Vec::new();
    let _ = std::io::Read::read_to_end(&mut stream, &mut answer);
    classify_door(Some(&String::from_utf8_lossy(&answer)))
}

fn status() -> i32 {
    let app = match probe_door() {
        Door::Ours => "running".to_string(),
        Door::Closed => {
            "not running (it opens when an agent connects, or run: rune-studio)".to_string()
        }
        Door::Taken => format!(
            "another program is using port {}: Rune Studio's agent bridge cannot start while it does",
            server::PORT
        ),
    };
    say(&format!(
        "Rune Studio {}\n  App:               {app}\n  Outside agents:    {}",
        crate::VERSION,
        if switch_is_on() {
            "allowed"
        } else {
            "not allowed (run: rune-studio-cli enable)"
        }
    ));
    0
}

fn set_switch(on: bool) -> i32 {
    let Some(path) = settings::path_without_app() else {
        say("Could not find your settings folder.");
        return 1;
    };
    let mut current = settings::read_file(&path);
    current.external_agent = on;
    if on {
        current.external_agent_since = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
    }
    if let Err(err) = settings::write_file(&path, &current) {
        say(&format!("Could not save the setting: {err}"));
        return 1;
    }
    say(if on {
        "Outside agents are now allowed. Press Stop in Rune Studio (Ctrl+Alt+Esc) to take control back at any time."
    } else {
        "Outside agents are no longer allowed."
    });
    0
}

fn app_path() -> String {
    std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "rune-studio".into())
}

fn app_exe() -> Result<std::path::PathBuf, String> {
    let own = std::env::current_exe().map_err(|err| err.to_string())?;
    #[cfg(unix)]
    let own = std::fs::canonicalize(own).map_err(|err| err.to_string())?;
    let exe = own.with_file_name(super::exe_name("rune-studio"));
    if exe.exists() {
        Ok(exe)
    } else {
        Err(format!(
            "cannot find {} next to this program",
            exe.display()
        ))
    }
}

fn open_app() -> i32 {
    match ensure_app() {
        Ok(()) => {
            say("Rune Studio is running.");
            0
        }
        Err(problem) => {
            say(&problem);
            1
        }
    }
}

const AGENTS: &str = "claude, antigravity";

fn unknown_agent(verb: &str, target: &str) -> i32 {
    if target.is_empty() {
        say(&format!(
            "Say which agent: rune-studio {verb} <agent>   (agents: {AGENTS})"
        ));
    } else {
        say(&format!("\"{target}\" is not an agent I know how to set up. I can {verb}: {AGENTS}.
For any other MCP-capable agent, run: rune-studio connect --print   and paste the result into its MCP settings."));
    }
    2
}

fn connect(target: &str) -> i32 {
    let exe = app_path();
    match target {
        "--print" | "print" => {
            say(&serde_json::to_string_pretty(
                &json!({ "mcpServers": { "rune-studio": { "command": exe, "args": ["mcp"] } } }),
            )
            .unwrap_or_default());
            0
        }
        "claude" => connect_claude(&exe),
        "antigravity" => connect_antigravity(&exe),
        other => unknown_agent("connect", other),
    }
}

fn run_claude(arguments: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    match Process::new("claude").args(arguments).status() {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Process::new("cmd")
            .arg("/C")
            .arg("claude")
            .args(arguments)
            .status(),
        other => other,
    }
}

fn connect_claude(exe: &str) -> i32 {
    let _ = set_switch(true);
    match run_claude(&[
        "mcp",
        "add",
        "--scope",
        "user",
        "rune-studio",
        "--",
        exe,
        "mcp",
        "--agent",
        "claude",
    ]) {
        Ok(status) if status.success() => {
            let skill_note = match install_skill() {
                Ok(dir) => format!(
                    " It also installed the Rune Studio skill ({}).",
                    dir.display()
                ),
                Err(err) => {
                    format!(" (The skill could not be installed: {err}. Run: rune-studio skill)")
                }
            };
            say(&format!(
                "Connected. Claude Code can now use Rune Studio in every project.{skill_note} Restart Claude Code, then ask it to do something on your PC."
            ));
            0
        }
        _ => {
            say(&format!(
                "Could not run Claude Code's \"claude\" command. Connect by hand with:

  claude mcp add --scope user rune-studio -- \"{exe}\" mcp --agent claude
"
            ));
            1
        }
    }
}

fn connect_antigravity(exe: &str) -> i32 {
    let Some(paths) = agents::antigravity() else {
        say("Cannot find your home folder.");
        return 1;
    };
    let _ = set_switch(true);
    let changed = match agents::edit_file(&paths.config, |text| {
        agents::add_server(text, exe).map(Some)
    }) {
        Ok(changed) => changed,
        Err(problem) => {
            say(&format!("Could not update Antigravity's settings ({}): {problem}
Nothing was changed. Add Rune Studio by hand: run  rune-studio connect --print  and paste it into Antigravity's MCP settings (agent panel, MCP Servers, Manage MCP Servers, View raw config).", paths.config.display()));
            return 1;
        }
    };
    let mut skill_dirs = vec![paths.skill.clone()];
    skill_dirs.extend(paths.cli_skill.clone());
    let mut skill_note = String::new();
    for dir in &skill_dirs {
        if let Err(err) = skill::install(dir, GUIDE) {
            skill_note = format!(
                " (The skill could not be installed in {}: {err})",
                dir.display()
            );
        }
    }
    say(&format!(
        "{} Antigravity can now use Rune Studio.{}
  Settings: {}
  Skill:    {}
In Antigravity, refresh MCP servers (agent panel > MCP Servers > Manage MCP Servers, or Settings > Customizations > Installed MCP Servers) or restart it, then ask it to do something on your PC.",
        if changed { "Connected." } else { "Already connected." },
        skill_note,
        paths.config.display(),
        paths.skill.display(),
    ));
    0
}

fn install_skill() -> Result<std::path::PathBuf, String> {
    let dir = skill::default_dir().ok_or("cannot find your home folder")?;
    skill::install(&dir, GUIDE).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn skill_command(option: &str) -> i32 {
    let Some(dir) = skill::default_dir() else {
        say("Cannot find your home folder.");
        return 1;
    };
    if option == "--remove" {
        return match skill::remove(&dir) {
            Ok(()) => {
                say("The Rune Studio skill was removed.");
                0
            }
            Err(err) => {
                say(&format!("Could not remove the skill: {err}"));
                1
            }
        };
    }
    match install_skill() {
        Ok(dir) => {
            say(&format!(
                "Installed the Rune Studio skill in {}. Restart Claude Code to pick it up.",
                dir.display()
            ));
            0
        }
        Err(err) => {
            say(&format!("Could not install the skill: {err}"));
            1
        }
    }
}

fn disconnect(target: &str) -> i32 {
    match target {
        "claude" => {
            let outcome = run_claude(&["mcp", "remove", "--scope", "user", "rune-studio"]);
            let _ = skill_command("--remove");
            match outcome {
                Ok(status) if status.success() => {
                    say("Rune Studio was removed from Claude Code.");
                    0
                }
                _ => {
                    say(
                        "The skill was removed, but Claude Code's \"claude\" command did not remove the tools. Run: claude mcp remove --scope user rune-studio",
                    );
                    1
                }
            }
        }
        "antigravity" => {
            let Some(paths) = agents::antigravity() else {
                say("Cannot find your home folder.");
                return 1;
            };
            let outcome = agents::edit_file(&paths.config, agents::remove_server);
            let mut dirs = vec![paths.skill];
            dirs.extend(paths.cli_skill);
            for dir in dirs {
                let _ = skill::remove(&dir);
            }
            match outcome {
                Ok(true) => {
                    say(
                        "Rune Studio was removed from Antigravity (tools and skill). Refresh or restart Antigravity.",
                    );
                    0
                }
                Ok(false) => {
                    say(
                        "Rune Studio was not in Antigravity's settings. The skill was removed if it was there.",
                    );
                    0
                }
                Err(problem) => {
                    say(&format!(
                        "Could not update Antigravity's settings: {problem}"
                    ));
                    1
                }
            }
        }
        other => unknown_agent("disconnect", other),
    }
}

fn rpc_error(id: &Value, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": message } })
}

fn ensure_app() -> Result<(), String> {
    if door_is_open() {
        return Ok(());
    }
    let mut launch = Process::new(app_exe()?);
    launch
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        launch.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        launch.process_group(0);
    }
    launch
        .spawn()
        .map_err(|err| format!("could not start Rune Studio: {err}"))?;
    let deadline = Instant::now() + START_TIMEOUT;
    while Instant::now() < deadline {
        if door_is_open() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err(
        "Rune Studio started but its door did not open. Is this the build with agent support?"
            .into(),
    )
}

async fn forward(
    client: &reqwest::Client,
    message: &Value,
    agent: Option<&str>,
) -> Result<Option<Value>, String> {
    ensure_app()?;
    let secret = token::get_or_create()?;
    let mut request = client.post(server::url()).bearer_auth(secret);
    if let Some(agent) = agent {
        request = request.header(identity::ATTEST_HEADER, agent);
    }
    let response = request
        .json(message)
        .timeout(Duration::from_secs(300))
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let status = response.status();
    if status == reqwest::StatusCode::ACCEPTED {
        return Ok(None);
    }
    let text = response.text().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        let detail = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|body| {
                Some(format!(
                    "{}: {}",
                    body["error"].as_str()?,
                    body["message"].as_str()?
                ))
            })
            .unwrap_or(text);
        return Err(detail);
    }
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| err.to_string())
}

fn run_mcp(agent: Option<&str>) -> i32 {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return 1;
    };
    let client = reqwest::Client::new();
    let mut stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let reply = match runtime.block_on(forward(&client, &message, agent)) {
            Ok(reply) => reply,
            Err(problem) => message.get("id").map(|id| rpc_error(id, &problem)),
        };
        if let Some(reply) = reply {
            let _ = writeln!(stdout, "{reply}");
            let _ = stdout.flush();
        }
    }
    0
}

fn shown_time(time: Option<&str>) -> String {
    time.map_or_else(
        || "-".to_string(),
        |time| time.trim_end_matches('Z').replacen('T', " ", 1),
    )
}

fn shown_duration(started: Option<&str>, ended: Option<&str>) -> String {
    let (Some(started), Some(ended)) = (
        started.and_then(trace::parse_iso),
        ended.and_then(trace::parse_iso),
    ) else {
        return "-".into();
    };
    let seconds = ended.saturating_sub(started);
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m {:02}s", seconds / 60, seconds % 60),
        _ => format!("{}h {:02}m", seconds / 3600, seconds % 3600 / 60),
    }
}

fn one_line(text: &str, max: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    trace::cap(&flat, max)
}

fn runs_table(rows: &[RunSummary]) -> String {
    let header = [
        "ID", "STARTED", "SOURCE", "OUTCOME", "VERDICT", "STEPS", "DURATION", "TASK",
    ]
    .map(String::from);
    let mut lines = vec![header];
    for row in rows {
        lines.push([
            row.id.clone(),
            shown_time(row.started_utc.as_deref()),
            row.source.clone().unwrap_or_else(|| "-".into()),
            row.outcome.clone().unwrap_or_else(|| "(open)".into()),
            row.verdict.clone().unwrap_or_else(|| "-".into()),
            row.steps.to_string(),
            shown_duration(row.started_utc.as_deref(), row.ended_utc.as_deref()),
            one_line(row.task.as_deref().unwrap_or(""), 50),
        ]);
    }
    let widths: Vec<usize> = (0..8)
        .map(|column| {
            lines
                .iter()
                .map(|line| line[column].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for line in &lines {
        let cells: Vec<String> = line
            .iter()
            .enumerate()
            .map(|(column, cell)| {
                if column == 7 {
                    cell.clone()
                } else {
                    format!("{cell:<width$}", width = widths[column])
                }
            })
            .collect();
        out.push_str(cells.join("  ").trim_end());
        out.push('\n');
    }
    out
}

fn analyses_table(rows: &[AnalysisRecord]) -> String {
    let mut lines = vec![["ID", "CREATED", "STATUS", "TIME", "QUESTION"].map(String::from)];
    for row in rows {
        lines.push([
            row.id.clone(),
            shown_time(
                Some(&row.created_utc)
                    .filter(|time| !time.is_empty())
                    .map(String::as_str),
            ),
            row.status.clone(),
            row.latency_ms
                .map_or_else(|| "-".into(), |ms| format!("{:.1}s", ms as f64 / 1000.0)),
            one_line(&row.question, 50),
        ]);
    }
    let widths: Vec<usize> = (0..5)
        .map(|column| {
            lines
                .iter()
                .map(|line| line[column].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for line in &lines {
        let cells: Vec<String> = line
            .iter()
            .enumerate()
            .map(|(column, cell)| {
                if column == 4 {
                    cell.clone()
                } else {
                    format!("{cell:<width$}", width = widths[column])
                }
            })
            .collect();
        out.push_str(cells.join("  ").trim_end());
        out.push('\n');
    }
    out
}

fn analysis_report(record: &AnalysisRecord) -> String {
    let mut out = format!(
        "Analysis {}\n  Created:   {}\n  Status:    {}",
        record.id,
        shown_time(
            Some(&record.created_utc)
                .filter(|time| !time.is_empty())
                .map(String::as_str)
        ),
        record.status
    );
    if let Some(error) = record.error.as_deref().filter(|error| !error.is_empty()) {
        out.push_str(&format!(" ({error})"));
    }
    let latency = record
        .latency_ms
        .map(|ms| format!(", {:.1}s", ms as f64 / 1000.0))
        .unwrap_or_default();
    out.push_str(&format!(
        "   [{}{latency}]\n",
        record.model.as_deref().unwrap_or("-")
    ));
    if let Some(path) = &record.screenshot_path {
        out.push_str(&format!("  Screenshot: {path}\n"));
    }
    out.push_str(&format!(
        "  Question:  {}\n  Answer:    {}\n",
        one_line(&record.question, 400),
        one_line(&record.answer, 1000)
    ));
    match serde_json::from_str::<Vec<Finding>>(&record.findings_json) {
        Ok(findings) if findings.is_empty() => out.push_str("\nNo findings.\n"),
        Ok(findings) => {
            out.push_str("\nFindings:\n");
            for finding in &findings {
                let at = finding.bbox.map_or_else(
                    || "no location".to_string(),
                    |b| format!("[{:.0}, {:.0}, {:.0}, {:.0}]", b[0], b[1], b[2], b[3]),
                );
                out.push_str(&format!(
                    "  {}. {} ({:?}) {at}\n     {}\n     evidence: {}\n",
                    finding.n, finding.label, finding.kind, finding.description, finding.evidence
                ));
            }
        }
        Err(_) if record.findings_json.is_empty() => out.push_str("\nNo findings.\n"),
        Err(_) => out.push_str("\nThe findings were too long to keep whole and cannot be shown.\n"),
    }
    out
}

fn number(value: Option<i64>) -> String {
    value.map_or_else(|| "-".to_string(), |value| value.to_string())
}

const STEP_HEADER: &str =
    "  N  TOOL            RESULT     GATE                DECIDE_MS  ACT_MS  REASON";

fn step_line(step: &StepRecord) -> String {
    format!(
        "{:>3}  {:<15} {:<10} {:<19} {:>9}  {:>6}  {}",
        step.step,
        step.tool.as_deref().unwrap_or("-"),
        step.result.as_deref().unwrap_or("-"),
        step.gate.as_deref().unwrap_or("-"),
        number(step.decide_ms),
        number(step.act_ms),
        one_line(step.reason.as_deref().unwrap_or(""), 70),
    )
}

fn run_report(detail: &RunDetail) -> String {
    let run = &detail.run;
    let info = &run.info;
    let mut out = format!("Run {}\n", info.id);
    out.push_str(&format!("  Task:      {}\n", one_line(&info.task, 200)));
    out.push_str(&format!(
        "  Source:    {}   Model: {}   App: {}\n",
        info.source,
        info.model.as_deref().unwrap_or("-"),
        info.app_version.as_deref().unwrap_or("-")
    ));
    out.push_str(&format!(
        "  Started:   {}   Ended: {}   Duration: {}\n",
        shown_time(Some(&info.started_utc)),
        shown_time(run.ended_utc.as_deref()),
        shown_duration(Some(&info.started_utc), run.ended_utc.as_deref())
    ));
    out.push_str(&format!(
        "  Outcome:   {}",
        run.outcome.as_deref().unwrap_or("(still open)")
    ));
    if let Some(summary) = run.summary.as_deref().filter(|summary| !summary.is_empty()) {
        out.push_str(&format!(" - {}", one_line(summary, 200)));
    }
    out.push('\n');
    if let Some(verdict) = &detail.verdict {
        out.push_str(&format!("  Verdict:   {}", verdict.result));
        if let Some(evidence) = verdict
            .evidence
            .as_deref()
            .filter(|evidence| !evidence.is_empty())
        {
            out.push_str(&format!(" - {}", one_line(evidence, 200)));
        }
        if let Some(error) = verdict.error.as_deref().filter(|error| !error.is_empty()) {
            out.push_str(&format!(" ({})", one_line(error, 100)));
        }
        let latency = verdict
            .latency_ms
            .map(|ms| format!(", {:.1}s", ms as f64 / 1000.0))
            .unwrap_or_default();
        let challenged = if verdict.challenged {
            ", the agent was asked to keep going"
        } else {
            ""
        };
        out.push_str(&format!("   [{}{latency}{challenged}]\n", verdict.mode));
    }
    if let Some(folder) = &info.session_dir {
        out.push_str(&format!("  Folder:    {folder}\n"));
    }
    if let Some(goals) = info
        .plan_json
        .as_deref()
        .and_then(|plan| serde_json::from_str::<Vec<String>>(plan).ok())
    {
        let plan: Vec<String> = goals
            .iter()
            .enumerate()
            .map(|(index, goal)| format!("{}. {goal}", index + 1))
            .collect();
        out.push_str(&format!("  Plan:      {}\n", plan.join("  ")));
    }
    out.push_str(&format!("\nSteps ({})\n", detail.steps.len()));
    if !detail.steps.is_empty() {
        out.push_str(STEP_HEADER);
        out.push('\n');
    }
    for step in &detail.steps {
        out.push_str(&step_line(step));
        out.push('\n');
    }
    out.push_str(&format!("\nEvents ({})\n", detail.events.len()));
    for event in &detail.events {
        out.push_str(&format!(
            "  {}  {:<16} {}\n",
            shown_time(event.at_utc.as_deref()),
            event.kind.as_deref().unwrap_or("-"),
            one_line(event.data_json.as_deref().unwrap_or(""), 100)
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn no_arguments_open_the_app() {
        assert_eq!(parse(&args(&[])), Command::App);
    }

    #[test]
    fn commands_parse() {
        assert_eq!(parse(&args(&["mcp"])), Command::Mcp(None));
        assert_eq!(
            parse(&args(&["mcp", "--agent", "claude"])),
            Command::Mcp(Some("claude".into()))
        );
        assert!(matches!(
            parse(&args(&["mcp", "--agent"])),
            Command::Unknown(_)
        ));
        assert!(matches!(
            parse(&args(&["mcp", "--bogus"])),
            Command::Unknown(_)
        ));
        assert_eq!(parse(&args(&["status"])), Command::Status);
        assert_eq!(parse(&args(&["enable"])), Command::Enable);
        assert_eq!(parse(&args(&["disable"])), Command::Disable);
        assert_eq!(parse(&args(&["guide"])), Command::Guide);
        assert_eq!(parse(&args(&["skill"])), Command::Skill(String::new()));
        assert_eq!(
            parse(&args(&["skill", "--remove"])),
            Command::Skill("--remove".into())
        );
        assert_eq!(
            parse(&args(&["disconnect", "claude"])),
            Command::Disconnect("claude".into())
        );
        assert_eq!(parse(&args(&["--help"])), Command::Help);
        assert_eq!(
            parse(&args(&["connect", "claude"])),
            Command::Connect("claude".into())
        );
        assert_eq!(parse(&args(&["connect"])), Command::Connect(String::new()));
    }

    #[test]
    fn a_typo_is_an_error_not_a_second_app() {
        assert_eq!(parse(&args(&["statsu"])), Command::Unknown("statsu".into()));
    }

    #[test]
    fn the_help_text_mentions_every_command() {
        for word in [
            "connect claude",
            "connect antigravity",
            "disconnect",
            "skill",
            "enable",
            "disable",
            "status",
            "guide",
            "mcp",
        ] {
            assert!(HELP.contains(word), "help does not mention {word}");
        }
    }

    #[test]
    fn the_run_commands_parse() {
        assert_eq!(parse(&args(&["runs"])), Command::Runs(20));
        assert_eq!(parse(&args(&["runs", "--limit", "5"])), Command::Runs(5));
        assert!(matches!(
            parse(&args(&["runs", "--limit"])),
            Command::Unknown(_)
        ));
        assert!(matches!(
            parse(&args(&["runs", "--limit", "many"])),
            Command::Unknown(_)
        ));
        assert!(matches!(parse(&args(&["runs", "5"])), Command::Unknown(_)));
        assert_eq!(
            parse(&args(&["run", "120000-1-open-notepad"])),
            Command::Run("120000-1-open-notepad".into(), false)
        );
        assert_eq!(parse(&args(&["run"])), Command::Run(String::new(), false));
        assert_eq!(
            parse(&args(&["run", "r1", "--calls"])),
            Command::Run("r1".into(), true)
        );
        assert_eq!(
            parse(&args(&["run", "--calls", "r1"])),
            Command::Run("r1".into(), true)
        );
        assert!(matches!(
            parse(&args(&["run", "r1", "--bogus"])),
            Command::Unknown(_)
        ));
        for word in ["runs", "run <id>", "--calls"] {
            assert!(HELP.contains(word), "help does not mention {word}");
        }
    }

    #[test]
    fn an_unknown_id_exits_non_zero_and_a_found_one_exits_zero() {
        let missing = lookup_outcome("run", "nope", "runs", Ok(None));
        assert_eq!(
            missing,
            (
                "There is no run \"nope\". See rune-studio-cli runs for the ids.".to_string(),
                1
            )
        );
        assert_eq!(
            lookup_outcome("analysis", "a1", "analyses", Ok(Some("report".into()))),
            ("report".to_string(), 0)
        );
        let broken = lookup_outcome("run", "x", "runs", Err(trace::store::StoreError::NotFound));
        assert_eq!(broken.1, 1);
        assert!(broken.0.contains("no trace database"));
    }

    #[test]
    fn the_help_names_only_the_cli_program() {
        for line in HELP
            .lines()
            .filter(|line| line.trim_start().starts_with("rune-studio"))
        {
            assert!(line.trim_start().starts_with("rune-studio-cli"), "{line}");
        }
    }

    #[test]
    fn the_door_is_told_apart_by_what_answers() {
        let refused = "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\n\r\n{\"error\":\"unauthorized\",\"message\":\"Missing or wrong token.\"}";
        let rpc = "HTTP/1.1 200 OK\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}";
        let web = "HTTP/1.1 404 Not Found\r\ncontent-type: text/html\r\n\r\n<html>nope</html>";
        let other_json = "HTTP/1.1 404 Not Found\r\n\r\n{\"detail\":\"Not Found\"}";
        assert_eq!(classify_door(Some(refused)), Door::Ours);
        assert_eq!(classify_door(Some(rpc)), Door::Ours);
        assert_eq!(classify_door(Some(web)), Door::Taken);
        assert_eq!(classify_door(Some(other_json)), Door::Taken);
        assert_eq!(
            classify_door(Some("")),
            Door::Taken,
            "something that accepts and says nothing is not ours"
        );
        assert_eq!(classify_door(None), Door::Closed);
    }

    #[test]
    fn the_reflections_command_parses_and_its_table_is_laid_out() {
        assert_eq!(parse(&args(&["reflections"])), Command::Reflections(20));
        assert_eq!(
            parse(&args(&["reflections", "--limit", "2"])),
            Command::Reflections(2)
        );
        assert!(matches!(
            parse(&args(&["reflections", "x"])),
            Command::Unknown(_)
        ));
        assert!(HELP.contains("reflections [--limit N]"));
        let rows = [
            trace::ReflectionRecord {
                id: "a".into(),
                started_utc: "2026-10-19T10:00:00Z".into(),
                ended_utc: Some("2026-10-19T10:00:03Z".into()),
                outcome: "completed".into(),
                stages: 3,
            },
            trace::ReflectionRecord {
                id: "bb".into(),
                started_utc: "2026-10-19T09:00:00Z".into(),
                ended_utc: None,
                outcome: "preempted".into(),
                stages: 1,
            },
        ];
        assert_eq!(
            reflections_table(&rows),
            "ID  STARTED              OUTCOME    STAGES  DURATION\na   2026-10-19 10:00:00  completed  3       3s\nbb  2026-10-19 09:00:00  preempted  1       -\n"
        );
    }

    #[test]
    fn the_analysis_commands_parse() {
        assert_eq!(parse(&args(&["analyses"])), Command::Analyses(20));
        assert_eq!(
            parse(&args(&["analyses", "--limit", "3"])),
            Command::Analyses(3)
        );
        assert!(matches!(
            parse(&args(&["analyses", "--limit"])),
            Command::Unknown(_)
        ));
        assert!(matches!(
            parse(&args(&["analyses", "3"])),
            Command::Unknown(_)
        ));
        assert_eq!(
            parse(&args(&["analysis", "abc"])),
            Command::Analysis("abc".into())
        );
        assert_eq!(
            parse(&args(&["analysis"])),
            Command::Analysis(String::new())
        );
        for word in ["analyses [--limit N]", "analysis <id>"] {
            assert!(HELP.contains(word), "help does not mention {word}");
        }
    }

    fn stored(id: &str, status: &str, findings: &str) -> AnalysisRecord {
        AnalysisRecord {
            id: id.into(),
            created_utc: "2026-10-19T12:00:05Z".into(),
            question: "which button deletes?".into(),
            answer: "The red one.".into(),
            findings_json: findings.into(),
            model: Some("gemma".into()),
            latency_ms: Some(2500),
            status: status.into(),
            ..AnalysisRecord::default()
        }
    }

    #[test]
    fn the_analysis_table_is_laid_out_exactly() {
        let rows = [
            stored("a1", "ok", "[]"),
            AnalysisRecord {
                latency_ms: None,
                ..stored("long-id-2", "error", "[]")
            },
        ];
        assert_eq!(
            analyses_table(&rows),
            "ID         CREATED              STATUS  TIME  QUESTION\na1         2026-10-19 12:00:05  ok      2.5s  which button deletes?\nlong-id-2  2026-10-19 12:00:05  error   -     which button deletes?\n"
        );
    }

    #[test]
    fn an_analysis_reads_top_to_bottom_with_a_line_per_finding() {
        let findings = r#"[{"n":1,"label":"Delete","kind":"control","box":[100,200,300,260],"description":"Removes the file","evidence":"red button labelled Delete"},
                          {"n":2,"label":"Title bar","kind":"region","box":null,"description":"Window title","evidence":"text at the top"}]"#;
        let report = analysis_report(&stored("a1", "ok", findings));
        assert_eq!(
            report,
            "Analysis a1\n  Created:   2026-10-19 12:00:05\n  Status:    ok   [gemma, 2.5s]\n  Question:  which button deletes?\n  Answer:    The red one.\n\nFindings:\n  1. Delete (Control) [100, 200, 300, 260]\n     Removes the file\n     evidence: red button labelled Delete\n  2. Title bar (Region) no location\n     Window title\n     evidence: text at the top\n"
        );
        assert!(analysis_report(&stored("a2", "ok", "[]")).contains("No findings."));
        assert!(
            analysis_report(&stored("a3", "ok", "[{\"n\":1,\"lab"))
                .contains("too long to keep whole")
        );
        let failed = AnalysisRecord {
            error: Some("timed out".into()),
            ..stored("a4", "error", "")
        };
        assert!(analysis_report(&failed).contains("Status:    error (timed out)"));
    }

    fn summary(
        id: &str,
        started: &str,
        ended: Option<&str>,
        outcome: Option<&str>,
        steps: i64,
        task: &str,
    ) -> RunSummary {
        RunSummary {
            id: id.into(),
            started_utc: Some(started.into()),
            ended_utc: ended.map(String::from),
            source: Some("built-in".into()),
            outcome: outcome.map(String::from),
            task: Some(task.into()),
            steps,
            verdict: None,
            session_dir: None,
            last_shot: None,
        }
    }

    #[test]
    fn the_run_table_is_laid_out_exactly() {
        let mut checked = summary(
            "120000-1-open-notepad",
            "2026-10-19T12:00:00Z",
            Some("2026-10-19T12:03:07Z"),
            Some("succeeded"),
            4,
            "open notepad",
        );
        checked.verdict = Some("contradicted".into());
        let rows = [
            checked,
            summary(
                "130000-2-x",
                "2026-10-19T13:00:00Z",
                None,
                None,
                0,
                "a very long task description that goes on and on past the limit of fifty characters",
            ),
        ];
        let expected = "\
ID                     STARTED              SOURCE    OUTCOME    VERDICT       STEPS  DURATION  TASK
120000-1-open-notepad  2026-10-19 12:00:00  built-in  succeeded  contradicted  4      3m 07s    open notepad
130000-2-x             2026-10-19 13:00:00  built-in  (open)     -             0      -         a very long task description that goes on and on …
";
        assert_eq!(runs_table(&rows), expected);
    }

    #[test]
    fn durations_are_short_and_honest() {
        assert_eq!(
            shown_duration(Some("2026-10-19T12:00:00Z"), Some("2026-10-19T12:00:42Z")),
            "42s"
        );
        assert_eq!(
            shown_duration(Some("2026-10-19T12:00:00Z"), Some("2026-10-19T12:03:07Z")),
            "3m 07s"
        );
        assert_eq!(
            shown_duration(Some("2026-10-19T12:00:00Z"), Some("2026-10-19T13:02:00Z")),
            "1h 02m"
        );
        assert_eq!(shown_duration(Some("2026-10-19T12:00:00Z"), None), "-");
        assert_eq!(
            shown_duration(Some("garbage"), Some("2026-10-19T12:00:00Z")),
            "-"
        );
    }

    #[test]
    fn a_step_line_has_the_columns_in_order_and_dashes_for_what_did_not_run() {
        let mut step = StepRecord::new("r", 3);
        step.tool = Some("click".into());
        step.result = Some("ok".into());
        step.gate = Some("none".into());
        step.decide_ms = Some(812);
        step.act_ms = Some(40);
        step.reason = Some("press the\nsave button".into());
        assert_eq!(
            step_line(&step),
            "  3  click           ok         none                      812      40  press the save button"
        );
        let bare = StepRecord::new("r", 12);
        assert_eq!(
            step_line(&bare),
            " 12  -               -          -                           -       -  "
        );
    }

    #[test]
    fn a_whole_run_reads_top_to_bottom() {
        use crate::trace::{
            RunInfo,
            store::{EventRow, RunRow},
        };
        let mut step = StepRecord::new("r1", 1);
        step.tool = Some("done".into());
        step.result = Some("ok".into());
        step.gate = Some("none".into());
        let detail = RunDetail {
            run: RunRow {
                info: RunInfo {
                    id: "r1".into(),
                    task: "open notepad".into(),
                    source: "Claude".into(),
                    started_utc: "2026-10-19T12:00:00Z".into(),
                    plan_json: Some(r#"["open","type"]"#.into()),
                    model: Some("gemma".into()),
                    app_version: Some("0.1.0".into()),
                    session_dir: Some("D:/Documents/Rune Studio/sessions/2026-10-19/r1".into()),
                    ..RunInfo::default()
                },
                ended_utc: Some("2026-10-19T12:00:09Z".into()),
                outcome: Some("succeeded".into()),
                summary: Some("Done".into()),
            },
            steps: vec![step],
            verdict: None,
            events: vec![EventRow {
                id: 1,
                at_utc: Some("2026-10-19T12:00:01Z".into()),
                kind: Some("plan".into()),
                data_json: Some(r#"["open","type"]"#.into()),
            }],
        };
        let expected = "\
Run r1
  Task:      open notepad
  Source:    Claude   Model: gemma   App: 0.1.0
  Started:   2026-10-19 12:00:00   Ended: 2026-10-19 12:00:09   Duration: 9s
  Outcome:   succeeded - Done
  Folder:    D:/Documents/Rune Studio/sessions/2026-10-19/r1
  Plan:      1. open  2. type

Steps (1)
  N  TOOL            RESULT     GATE                DECIDE_MS  ACT_MS  REASON
  1  done            ok         none                        -       -  

Events (1)
  2026-10-19 12:00:01  plan             [\"open\",\"type\"]
";
        assert_eq!(run_report(&detail), expected);
    }

    #[test]
    fn a_stored_verdict_is_shown_in_the_run_report_and_its_absence_is_not() {
        use crate::trace::{RunInfo, VerdictRecord, store::RunRow};
        let mut detail = RunDetail {
            run: RunRow {
                info: RunInfo {
                    id: "r1".into(),
                    task: "t".into(),
                    source: "built-in".into(),
                    started_utc: "2026-10-19T12:00:00Z".into(),
                    ..RunInfo::default()
                },
                ended_utc: Some("2026-10-19T12:00:09Z".into()),
                outcome: Some("succeeded".into()),
                summary: None,
            },
            steps: vec![],
            events: vec![],
            verdict: None,
        };
        assert!(!run_report(&detail).contains("Verdict:"));
        detail.verdict = Some(VerdictRecord {
            run_id: "r1".into(),
            mode: "blocking".into(),
            result: "contradicted".into(),
            evidence: Some("the Notepad window\nis empty".into()),
            latency_ms: Some(12_500),
            claimed_success: true,
            challenged: true,
            ..VerdictRecord::default()
        });
        let report = run_report(&detail);
        let line = report
            .lines()
            .find(|line| line.contains("Verdict:"))
            .expect("a verdict line");
        assert_eq!(
            line,
            "  Verdict:   contradicted - the Notepad window is empty   [blocking, 12.5s, the agent was asked to keep going]"
        );
        detail.verdict = Some(VerdictRecord {
            run_id: "r1".into(),
            mode: "background".into(),
            result: "error".into(),
            error: Some("timed out".into()),
            ..VerdictRecord::default()
        });
        let report = run_report(&detail);
        assert!(
            report.contains("  Verdict:   error (timed out)   [background]\n"),
            "{report}"
        );
    }

    #[test]
    fn parse_life_subcommands() {
        assert_eq!(
            parse(&["life".into(), "status".into()]),
            Command::Life(LifeCommand::Status)
        );

        assert_eq!(
            parse(&[
                "life".into(),
                "export".into(),
                "all".into(),
                "/tmp/life".into()
            ]),
            Command::Life(LifeCommand::Export {
                class: None,
                folder: "/tmp/life".into()
            })
        );

        assert_eq!(
            parse(&[
                "life".into(),
                "export".into(),
                "health".into(),
                "./out".into()
            ]),
            Command::Life(LifeCommand::Export {
                class: Some("health".into()),
                folder: "./out".into()
            })
        );

        assert_eq!(
            parse(&[
                "life".into(),
                "delete".into(),
                "finance".into(),
                "--yes".into()
            ]),
            Command::Life(LifeCommand::Delete {
                class: Some("finance".into()),
                yes: true
            })
        );

        assert_eq!(
            parse(&["life".into(), "delete".into(), "all".into()]),
            Command::Life(LifeCommand::Delete {
                class: None,
                yes: false
            })
        );

        assert_eq!(
            parse(&["life".into(), "audit".into(), "--limit".into(), "50".into()]),
            Command::Life(LifeCommand::Audit { limit: 50 })
        );
    }
}
