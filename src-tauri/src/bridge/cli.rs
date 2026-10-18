//! The `rune-studio` command line, for people and agents on a machine with nothing but the app.
//!
//! ```text
//! rune-studio                 open the app
//! rune-studio mcp             speak MCP on stdin/stdout (what an agent runs); starts the app if needed
//! rune-studio connect claude  register Rune Studio with Claude Code, for every project
//! rune-studio enable          allow outside agents      (rune-studio disable turns it off)
//! rune-studio status          show what is running and allowed
//! rune-studio guide           print the guide an agent reads
//! ```
//!
//! This is its own small console program (`rune-studio.exe`), next to the windowed app
//! (`rune-studio-app.exe`), so a terminal waits for it and shows what it prints. It is plain Rust:
//! no Node, Python or other tool is needed. `mcp` is a small bridge between an agent's stdio and
//! the app's local door.

use std::{
    io::{BufRead, Write},
    net::{SocketAddr, TcpStream},
    process::{Command as Process, Stdio},
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::{
    agents, server,
    skill::{self, GUIDE},
    token,
};
use crate::settings;

const START_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// No arguments: open the app.
    App,
    Mcp,
    Status,
    Enable,
    Disable,
    Connect(String),
    Guide,
    /// Install (or with `--remove`, remove) the Claude Code skill.
    Skill(String),
    Disconnect(String),
    Help,
    /// Not a command we know: shown as an error with the help text.
    Unknown(String),
}

pub fn parse(args: &[String]) -> Command {
    let Some(first) = args.first() else {
        return Command::App;
    };
    match first.as_str() {
        "mcp" => Command::Mcp,
        "status" => Command::Status,
        "enable" => Command::Enable,
        "disable" => Command::Disable,
        "guide" => Command::Guide,
        "skill" => Command::Skill(args.get(1).cloned().unwrap_or_default()),
        "disconnect" => Command::Disconnect(args.get(1).cloned().unwrap_or_default()),
        "help" | "--help" | "-h" => Command::Help,
        "connect" => Command::Connect(args.get(1).cloned().unwrap_or_default()),
        other => Command::Unknown(other.to_string()),
    }
}

const HELP: &str = "\
Rune Studio

  rune-studio                  Open the app.
  rune-studio connect claude   Connect Claude Code to Rune Studio (works in every project):
                               registers the tools and installs the Rune Studio skill.
  rune-studio connect antigravity  Connect Antigravity the same way (settings file and skill).
  rune-studio connect --print  Show the settings to paste into any other MCP-capable agent.
  rune-studio skill            Install or refresh the Claude Code skill only (--remove deletes it).
  rune-studio disconnect <agent>  Remove Rune Studio from claude or antigravity (tools and skill).
  rune-studio enable           Allow outside agents to see and use this PC through Rune Studio.
  rune-studio disable          Turn that off again.
  rune-studio status           Show whether Rune Studio is running and what is allowed.
  rune-studio guide            Print the guide an agent reads.
  rune-studio mcp              Used by agents: speaks MCP on stdin/stdout and opens the app if needed.

You are always in control: press Stop in the app (Ctrl+Alt+Esc) and the agent loses control at once.
";

/// Runs the command line and returns the exit code.
pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Command::App => open_app(),
        Command::Mcp => run_mcp(),
        Command::Status => status(),
        Command::Enable => set_switch(true),
        Command::Disable => set_switch(false),
        Command::Connect(target) => connect(&target),
        Command::Skill(option) => skill_command(&option),
        Command::Disconnect(target) => disconnect(&target),
        Command::Guide => {
            say(GUIDE);
            0
        }
        Command::Help => {
            say(HELP);
            0
        }
        Command::Unknown(name) => {
            say(&format!("Unknown command \"{name}\".

{HELP}"));
            2
        }
    }
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

fn status() -> i32 {
    let running = door_is_open();
    say(&format!(
        "Rune Studio {}\n  App:               {}\n  Outside agents:    {}",
        env!("CARGO_PKG_VERSION"),
        if running { "running" } else { "not running (it opens when an agent connects, or run: rune-studio)" },
        if switch_is_on() { "allowed" } else { "not allowed (run: rune-studio enable)" },
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
        current.external_agent_since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
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
    std::env::current_exe().map(|path| path.display().to_string()).unwrap_or_else(|_| "rune-studio".into())
}

/// The windowed app, which sits next to this program.
fn app_exe() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?.with_file_name("rune-studio-app.exe");
    if exe.exists() { Ok(exe) } else { Err(format!("cannot find {} next to this program", exe.display())) }
}

/// `rune-studio` with no arguments: open the app (unless it is already running).
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

/// The agents `connect` and `disconnect` know how to set up.
const AGENTS: &str = "claude, antigravity";

fn unknown_agent(verb: &str, target: &str) -> i32 {
    if target.is_empty() {
        say(&format!("Say which agent: rune-studio {verb} <agent>   (agents: {AGENTS})"));
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
            say(&serde_json::to_string_pretty(&json!({ "mcpServers": { "rune-studio": { "command": exe, "args": ["mcp"] } } })).unwrap_or_default());
            0
        }
        "claude" => connect_claude(&exe),
        "antigravity" => connect_antigravity(&exe),
        other => unknown_agent("connect", other),
    }
}

/// Runs Claude Code's `claude` command, which may be a .cmd shim that only the command interpreter can start.
fn run_claude(arguments: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    match Process::new("claude").args(arguments).status() {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Process::new("cmd").arg("/C").arg("claude").args(arguments).status(),
        other => other,
    }
}

fn connect_claude(exe: &str) -> i32 {
    let _ = set_switch(true);
    match run_claude(&["mcp", "add", "--scope", "user", "rune-studio", "--", exe, "mcp"]) {
        Ok(status) if status.success() => {
            let skill_note = match install_skill() {
                Ok(dir) => format!(" It also installed the Rune Studio skill ({}).", dir.display()),
                Err(err) => format!(" (The skill could not be installed: {err}. Run: rune-studio skill)"),
            };
            say(&format!("Connected. Claude Code can now use Rune Studio in every project.{skill_note} Restart Claude Code, then ask it to do something on your PC."));
            0
        }
        _ => {
            say(&format!("Could not run Claude Code's \"claude\" command. Connect by hand with:

  claude mcp add --scope user rune-studio -- \"{exe}\" mcp
"));
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
    let changed = match agents::edit_file(&paths.config, |text| agents::add_server(text, exe).map(Some)) {
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
            skill_note = format!(" (The skill could not be installed in {}: {err})", dir.display());
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
            say(&format!("Installed the Rune Studio skill in {}. Restart Claude Code to pick it up.", dir.display()));
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
                    say("The skill was removed, but Claude Code's \"claude\" command did not remove the tools. Run: claude mcp remove --scope user rune-studio");
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
                    say("Rune Studio was removed from Antigravity (tools and skill). Refresh or restart Antigravity.");
                    0
                }
                Ok(false) => {
                    say("Rune Studio was not in Antigravity's settings. The skill was removed if it was there.");
                    0
                }
                Err(problem) => {
                    say(&format!("Could not update Antigravity's settings: {problem}"));
                    1
                }
            }
        }
        other => unknown_agent("disconnect", other),
    }
}

// --- mcp: stdio <-> local door -------------------------------------------------------------

fn rpc_error(id: &Value, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": message } })
}

/// Starts the windowed app if the door is closed, and waits until it is open.
fn ensure_app() -> Result<(), String> {
    if door_is_open() {
        return Ok(());
    }
    let mut launch = Process::new(app_exe()?);
    launch.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        launch.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    launch.spawn().map_err(|err| format!("could not start Rune Studio: {err}"))?;
    let deadline = Instant::now() + START_TIMEOUT;
    while Instant::now() < deadline {
        if door_is_open() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err("Rune Studio started but its door did not open. Is this the build with agent support?".into())
}

/// Sends one message to the app. `Ok(None)` means it was a notification and nothing comes back.
async fn forward(client: &reqwest::Client, message: &Value) -> Result<Option<Value>, String> {
    ensure_app()?;
    let secret = token::get_or_create()?;
    let response = client
        .post(server::url())
        .bearer_auth(secret)
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
            .and_then(|body| Some(format!("{}: {}", body["error"].as_str()?, body["message"].as_str()?)))
            .unwrap_or(text);
        return Err(detail);
    }
    serde_json::from_str(&text).map(Some).map_err(|err| err.to_string())
}

fn run_mcp() -> i32 {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
        return 1;
    };
    let client = reqwest::Client::new();
    let mut stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // One at a time keeps replies in the order the agent asked.
        let reply = match runtime.block_on(forward(&client, &message)) {
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
        assert_eq!(parse(&args(&["mcp"])), Command::Mcp);
        assert_eq!(parse(&args(&["status"])), Command::Status);
        assert_eq!(parse(&args(&["enable"])), Command::Enable);
        assert_eq!(parse(&args(&["disable"])), Command::Disable);
        assert_eq!(parse(&args(&["guide"])), Command::Guide);
        assert_eq!(parse(&args(&["skill"])), Command::Skill(String::new()));
        assert_eq!(parse(&args(&["skill", "--remove"])), Command::Skill("--remove".into()));
        assert_eq!(parse(&args(&["disconnect", "claude"])), Command::Disconnect("claude".into()));
        assert_eq!(parse(&args(&["--help"])), Command::Help);
        assert_eq!(parse(&args(&["connect", "claude"])), Command::Connect("claude".into()));
        assert_eq!(parse(&args(&["connect"])), Command::Connect(String::new()));
    }

    #[test]
    fn a_typo_is_an_error_not_a_second_app() {
        assert_eq!(parse(&args(&["statsu"])), Command::Unknown("statsu".into()));
    }

    #[test]
    fn the_help_text_mentions_every_command() {
        for word in ["connect claude", "connect antigravity", "disconnect", "skill", "enable", "disable", "status", "guide", "mcp"] {
            assert!(HELP.contains(word), "help does not mention {word}");
        }
    }
}
