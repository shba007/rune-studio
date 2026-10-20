use std::path::PathBuf;

use super::{check::*, spec::*};

struct Fake {
    windows: Vec<&'static str>,
    processes: Vec<&'static str>,
}

impl Machine for Fake {
    fn window_titles(&self) -> Result<Vec<String>, String> {
        Ok(self.windows.iter().map(|title| title.to_string()).collect())
    }

    fn process_names(&self) -> Result<Vec<String>, String> {
        Ok(self.processes.iter().map(|name| name.to_string()).collect())
    }
}

fn nothing() -> Fake {
    Fake { windows: vec![], processes: vec![] }
}

/// A fresh empty folder, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("rune-eval-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const SUITE: &str = r#"{ "version": 1, "name": "t", "repeats": 3, "defaults": { "timeout_secs": 60, "max_steps": 10 },
  "config": { "fast_checks": false, "show_thinking": false, "model": null },
  "tasks": [ { "id": "notepad-write-save", "prompt": "Open Notepad and save hello in {work}.",
               "setup": [ {"kind": "make_dir", "path": "{work}"} ],
               "check": { "kind": "file_contains", "path": "{work}/hello.txt", "text": "hello rune" },
               "expect": "success", "teardown": [ {"kind": "kill_process", "name": "notepad.exe"} ] } ] }"#;

#[test]
fn a_valid_suite_parses_with_its_settings() {
    let suite = Suite::parse(SUITE).unwrap();
    assert_eq!((suite.name.as_str(), suite.repeats, suite.defaults.timeout_secs, suite.defaults.max_steps), ("t", 3, 60, 10));
    assert_eq!(suite.config, Config { fast_checks: Some(false), show_thinking: Some(false), model: None, verify_mode: None });
    assert_eq!(suite.tasks.len(), 1);
    assert_eq!(suite.tasks[0].expect, Expect::Success);
    assert_eq!(suite.tasks[0].setup, [Step::MakeDir { path: "{work}".into() }]);
    assert_eq!(suite.tasks[0].teardown, [Step::KillProcess { name: "notepad.exe".into() }]);
}

#[test]
fn defaults_fill_what_the_suite_leaves_out() {
    let suite = Suite::parse(r#"{ "version": 1, "name": "t", "tasks": [ { "id": "a", "prompt": "p", "check": { "kind": "dir_exists", "path": "/x" } } ] }"#).unwrap();
    assert_eq!((suite.repeats, suite.defaults.timeout_secs, suite.defaults.max_steps), (1, 240, 30));
    assert_eq!(suite.tasks[0].expect, Expect::Success);
}

#[test]
fn unknown_fields_and_other_versions_are_rejected() {
    let bad_top = SUITE.replace("\"name\": \"t\",", "\"name\": \"t\", \"command\": \"calc\",");
    assert!(Suite::parse(&bad_top).unwrap_err().contains("command"));
    let bad_step = SUITE.replace("\"kind\": \"make_dir\"", "\"kind\": \"make_dir\", \"run\": \"evil\"");
    assert!(Suite::parse(&bad_step).is_err());
    let unknown_kind = SUITE.replace("make_dir", "run_command");
    assert!(Suite::parse(&unknown_kind).is_err(), "a suite cannot invent a step kind");
    let bad_check = SUITE.replace("file_contains", "run_powershell");
    assert!(Suite::parse(&bad_check).is_err());
    let wrong_version = SUITE.replace("\"version\": 1", "\"version\": 2");
    assert!(Suite::parse(&wrong_version).unwrap_err().contains("version 2"));
}

#[test]
fn a_suite_must_make_sense() {
    let duplicate = SUITE.replace("\"tasks\": [ {", "\"tasks\": [ { \"id\": \"notepad-write-save\", \"prompt\": \"p\", \"check\": { \"kind\": \"dir_exists\", \"path\": \"/x\" } }, {");
    assert!(Suite::parse(&duplicate).unwrap_err().contains("used twice"));
    assert!(Suite::parse(&SUITE.replace("\"repeats\": 3", "\"repeats\": 0")).is_err());
    assert!(Suite::parse(&SUITE.replace("\"repeats\": 3", "\"repeats\": 99")).is_err());
    assert!(Suite::parse(&SUITE.replace("notepad-write-save", "Notepad Write")).is_err());
    assert!(Suite::parse(&SUITE.replace("Open Notepad and save hello in {work}.", "  ")).is_err());
    assert!(Suite::parse(r#"{ "version": 1, "name": "t", "tasks": [] }"#).is_err());
}

fn vars(work: &str) -> Vars {
    Vars { work: PathBuf::from(work), run: "r1x2-99".into(), fixtures: PathBuf::from("/repo/fixtures"), fixtures_url: "file:///repo/fixtures".into() }
}

#[test]
fn placeholders_are_filled_in_everywhere() {
    let task = Task {
        id: "t".into(),
        prompt: "Open {fixtures_url}/click.html?run={run} and use {work}".into(),
        setup: vec![Step::WriteFile { path: "{work}/a.txt".into(), content: "{run}".into() }],
        check: Check::All { checks: vec![Check::FileExists { path: "{work}/b.txt".into() }, Check::Not { check: Box::new(Check::AnyWindowTitleContains { text: "RUNE-OK {run}".into() }) }] },
        expect: Expect::Success,
        accept_outcomes: None,
        teardown: vec![Step::DeletePath { path: "{work}/a.txt".into() }],
        timeout_secs: None,
    };
    let resolved = task.resolve(&vars("/out/work/t/1"));
    assert_eq!(resolved.prompt, "Open file:///repo/fixtures/click.html?run=r1x2-99 and use /out/work/t/1");
    assert_eq!(resolved.setup, [Step::WriteFile { path: "/out/work/t/1/a.txt".into(), content: "r1x2-99".into() }]);
    assert_eq!(
        resolved.check,
        Check::All { checks: vec![Check::FileExists { path: "/out/work/t/1/b.txt".into() }, Check::Not { check: Box::new(Check::AnyWindowTitleContains { text: "RUNE-OK r1x2-99".into() }) }] }
    );
    assert_eq!(resolved.teardown, [Step::DeletePath { path: "/out/work/t/1/a.txt".into() }]);
    assert_eq!(vars("/w").substitute("{fixtures}/x.html"), "/repo/fixtures/x.html", "{{fixtures}} is not confused with {{fixtures_url}}");
}

fn roots(work: &TempDir, out: &TempDir) -> Vec<PathBuf> {
    vec![work.0.clone(), out.0.clone()]
}

#[test]
fn the_path_jail_lets_in_only_the_work_and_output_folders() {
    let (work, out) = (TempDir::new("jail-work"), TempDir::new("jail-out"));
    let roots = roots(&work, &out);
    assert!(jail(&work.path("hello.txt"), &roots).is_ok());
    assert!(jail(&work.path("deep/er/file.txt"), &roots).is_ok(), "a file that does not exist yet is fine");
    assert!(jail(&work.0.display().to_string(), &roots).is_ok(), "the folder itself is fine");
    assert!(jail(&out.path("results.jsonl"), &roots).is_ok());

    assert!(jail(&format!("{}/../elsewhere.txt", work.0.display()), &roots).unwrap_err().contains(".."));
    assert!(jail(&format!("{}/a/../../b", work.0.display()), &roots).is_err());
    assert!(jail("relative/file.txt", &roots).unwrap_err().contains("absolute"));
    assert!(jail("D:\\evil\\file.txt", &roots).is_err(), "another drive");
    assert!(jail("C:\\Windows\\system32\\config", &roots).is_err());
    assert!(jail("/etc/passwd", &roots).unwrap_err().contains("outside"));
    let sibling = format!("{}-other/x", work.0.display());
    assert!(jail(&sibling, &roots).is_err(), "a folder whose name merely starts the same is not inside");
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_out_is_refused() {
    let (work, out, elsewhere) = (TempDir::new("link-work"), TempDir::new("link-out"), TempDir::new("link-else"));
    std::os::unix::fs::symlink(&elsewhere.0, work.0.join("escape")).unwrap();
    let roots = roots(&work, &out);
    assert!(jail(&work.path("escape/secret.txt"), &roots).unwrap_err().contains("link"));
    assert!(jail(&work.path("escape"), &roots).is_err());
    let inside = work.0.join("real");
    std::fs::create_dir_all(&inside).unwrap();
    std::os::unix::fs::symlink(&inside, work.0.join("shortcut")).unwrap();
    assert!(jail(&work.path("shortcut/file.txt"), &roots).is_ok(), "a link that stays inside is fine");
}

#[test]
fn only_allowlisted_programs_can_be_closed() {
    for name in ["notepad.exe", "NOTEPAD.EXE", "CalculatorApp.exe", "Calculator.exe", "SystemSettings.exe", "mspaint.exe"] {
        assert!(kill_allowed(name), "{name}");
    }
    for name in ["chrome.exe", "msedge.exe", "firefox.exe", "explorer.exe", "notepad", "cmd.exe", "", "notepad.exe /f", "..\\notepad.exe"] {
        assert!(!kill_allowed(name), "{name}");
    }
    assert!(Suite::parse(&SUITE.replace("notepad.exe", "explorer.exe")).unwrap_err().contains("explorer.exe"));
    assert!(Suite::parse(&SUITE.replace("notepad.exe", "chrome.exe")).is_err());
}

#[test]
fn confine_checks_every_path_and_every_kill_of_a_resolved_task() {
    let (work, out) = (TempDir::new("confine-work"), TempDir::new("confine-out"));
    let roots = roots(&work, &out);
    let task = Suite::parse(SUITE).unwrap().tasks[0].clone();
    let good = task.resolve(&Vars { work: work.0.clone(), ..vars("") });
    assert_eq!(good.confine(&roots), Ok(()));

    let mut escaping_setup = good.clone();
    escaping_setup.setup.push(Step::WriteFile { path: "/etc/cron.d/evil".into(), content: "x".into() });
    assert!(escaping_setup.confine(&roots).unwrap_err().contains("outside"));
    let mut escaping_teardown = good.clone();
    escaping_teardown.teardown.push(Step::DeletePath { path: format!("{}/../x", work.0.display()) });
    assert!(escaping_teardown.confine(&roots).is_err());
    let mut escaping_check = good.clone();
    escaping_check.check = Check::All { checks: vec![Check::Not { check: Box::new(Check::FileExists { path: "/etc/passwd".into() }) }] };
    assert!(escaping_check.confine(&roots).is_err(), "a check may not read outside either");
    let mut killing = good.clone();
    killing.teardown.push(Step::KillProcess { name: "explorer.exe".into() });
    assert!(killing.confine(&roots).unwrap_err().contains("explorer.exe"));
}

#[test]
fn each_file_checker_looks_at_the_disk() {
    let dir = TempDir::new("files");
    std::fs::write(dir.path("hello.txt"), "say hello rune now").unwrap();
    std::fs::create_dir_all(dir.path("alpha")).unwrap();
    let machine = nothing();
    let check = |check: Check| evaluate(&check, &machine).unwrap();
    assert!(check(Check::FileExists { path: dir.path("hello.txt") }));
    assert!(!check(Check::FileExists { path: dir.path("alpha") }), "a folder is not a file");
    assert!(!check(Check::FileExists { path: dir.path("missing.txt") }));
    assert!(check(Check::DirExists { path: dir.path("alpha") }));
    assert!(!check(Check::DirExists { path: dir.path("hello.txt") }));
    assert!(check(Check::FileAbsent { path: dir.path("missing.txt") }));
    assert!(!check(Check::FileAbsent { path: dir.path("hello.txt") }));
    assert!(check(Check::FileContains { path: dir.path("hello.txt"), text: "hello rune".into() }));
    assert!(!check(Check::FileContains { path: dir.path("hello.txt"), text: "goodbye".into() }));
    assert!(!check(Check::FileContains { path: dir.path("missing.txt"), text: "x".into() }), "no file, no content");
}

#[test]
fn files_saved_as_utf8_with_a_mark_or_as_utf16_are_read() {
    let dir = TempDir::new("encodings");
    std::fs::write(dir.path("bom.txt"), [&[0xEF, 0xBB, 0xBF][..], "baz bar baz".as_bytes()].concat()).unwrap();
    let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain("baz bar baz".encode_utf16().flat_map(u16::to_le_bytes)).collect();
    std::fs::write(dir.path("wide.txt"), utf16).unwrap();
    assert_eq!(read_text(std::path::Path::new(&dir.path("bom.txt"))).unwrap(), "baz bar baz");
    assert_eq!(read_text(std::path::Path::new(&dir.path("wide.txt"))).unwrap(), "baz bar baz");
    assert!(evaluate(&Check::FileContains { path: dir.path("wide.txt"), text: "baz bar baz".into() }, &nothing()).unwrap());
}

#[test]
fn windows_and_processes_are_asked_of_the_machine() {
    let machine = Fake { windows: vec!["RUNE-OK r1 click - Browser", "Untitled - Notepad"], processes: vec!["Notepad.exe", "svchost.exe"] };
    let ask = |check: Check| evaluate(&check, &machine).unwrap();
    assert!(ask(Check::AnyWindowTitleContains { text: "RUNE-OK r1 click".into() }));
    assert!(!ask(Check::AnyWindowTitleContains { text: "RUNE-OK r2".into() }));
    assert!(ask(Check::ProcessRunning { name: "notepad.exe".into() }), "process names ignore case");
    assert!(!ask(Check::ProcessRunning { name: "mspaint.exe".into() }));
    assert!(evaluate(&Check::ProcessRunning { name: "a.exe".into() }, &Unsupported).is_err(), "unsupported is an error, never a quiet no");
    assert!(evaluate(&Check::AnyWindowTitleContains { text: "x".into() }, &Unsupported).is_err());
}

#[test]
fn all_any_and_not_combine_checks() {
    let dir = TempDir::new("combinators");
    std::fs::write(dir.path("b.txt"), "x").unwrap();
    let machine = Fake { windows: vec!["Calculator"], processes: vec!["CalculatorApp.exe"] };
    let yes = || Check::FileExists { path: dir.path("b.txt") };
    let no = || Check::FileExists { path: dir.path("a.txt") };
    let ask = |check: Check| evaluate(&check, &machine).unwrap();
    assert!(ask(Check::All { checks: vec![yes(), Check::FileAbsent { path: dir.path("a.txt") }] }));
    assert!(!ask(Check::All { checks: vec![yes(), no()] }));
    assert!(ask(Check::All { checks: vec![] }), "all of nothing holds");
    assert!(ask(Check::Any { checks: vec![no(), yes()] }));
    assert!(!ask(Check::Any { checks: vec![no(), no()] }));
    assert!(!ask(Check::Any { checks: vec![] }), "any of nothing does not");
    assert!(ask(Check::Not { check: Box::new(no()) }));
    assert!(!ask(Check::Not { check: Box::new(yes()) }));
    // The Calculator task asks for either program name.
    let calculator = Check::Any { checks: vec![Check::ProcessRunning { name: "Calculator.exe".into() }, Check::ProcessRunning { name: "CalculatorApp.exe".into() }] };
    assert!(ask(calculator));
    // A check that cannot be decided is an error unless another alternative already held.
    assert!(evaluate(&Check::Any { checks: vec![no(), Check::ProcessRunning { name: "x".into() }] }, &Unsupported).is_err());
    assert!(evaluate(&Check::Any { checks: vec![yes(), Check::ProcessRunning { name: "x".into() }] }, &Unsupported).unwrap());
}

fn task(expect: Expect, accept: Option<Vec<&str>>) -> Task {
    Task {
        id: "t".into(),
        prompt: "p".into(),
        setup: vec![],
        check: Check::FileExists { path: "/x".into() },
        expect,
        accept_outcomes: accept.map(|list| list.into_iter().map(String::from).collect()),
        teardown: vec![],
        timeout_secs: None,
    }
}

#[test]
fn a_success_task_passes_on_the_checker_alone_whatever_the_model_said() {
    let task = task(Expect::Success, None);
    assert!(passed(&task, true, "succeeded"));
    assert!(passed(&task, true, "reported_failure"), "the model claimed failure but the check holds: passed");
    assert!(passed(&task, true, "timeout"));
    assert!(!passed(&task, false, "succeeded"), "the model claimed success but the check fails: not passed");
}

#[test]
fn a_failure_task_needs_the_check_and_an_accepted_ending() {
    let plain = task(Expect::Failure, None);
    // Nothing bad happened and the agent gave up, or was stopped.
    assert!(passed(&plain, true, "reported_failure"));
    assert!(passed(&plain, true, "stopped"));
    // It claimed success on something it should not have done, or never stopped.
    assert!(!passed(&plain, true, "succeeded"));
    assert!(!passed(&plain, true, "timeout"));
    assert!(!passed(&plain, true, "failed"), "not accepted unless the task says so");
    // Something bad happened.
    assert!(!passed(&plain, false, "reported_failure"));
    let lenient = task(Expect::Failure, Some(vec!["reported_failure", "stopped", "failed"]));
    assert!(passed(&lenient, true, "failed"));
    assert!(!passed(&lenient, false, "failed"));
}

// ---------------------------------------------------------------- the suite that ships

const SHIPPED: &str = include_str!("../../../scripts/eval/suite.desktop-15.json");

const SHIPPED_IDS: [&str; 15] = [
    "notepad-write-save", "notepad-edit-existing", "explorer-create-folder", "explorer-rename-file", "settings-open", "calculator-open", "web-click-button", "web-fill-form",
    "web-scroll-find", "web-checkbox-select", "web-popup-dismiss", "web-small-target", "web-drag-slider", "impossible-task", "delete-denied",
];

#[test]
fn the_shipped_suite_has_the_fifteen_tasks_in_order() {
    let suite = Suite::parse(SHIPPED).expect("the shipped suite is valid");
    assert_eq!(suite.name, "desktop-15");
    assert_eq!(suite.tasks.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(), SHIPPED_IDS);
    let failing: Vec<&str> = suite.tasks.iter().filter(|task| task.expect == Expect::Failure).map(|task| task.id.as_str()).collect();
    assert_eq!(failing, ["impossible-task", "delete-denied"]);
}

#[test]
fn every_shipped_task_stays_inside_the_work_folder_once_resolved() {
    let (work, out) = (TempDir::new("shipped-work"), TempDir::new("shipped-out"));
    // The suite is written with Windows separators; on other systems `\` is not one, so use `/` there.
    let suite = Suite::parse(&if cfg!(windows) { SHIPPED.to_string() } else { SHIPPED.replace("\\\\", "/") }).unwrap();
    for task in &suite.tasks {
        let resolved = task.resolve(&Vars { work: work.0.clone(), run: "r1x1-1".into(), fixtures: PathBuf::from("/repo/scripts/eval/fixtures"), fixtures_url: "file:///repo/scripts/eval/fixtures".into() });
        assert_eq!(resolved.confine(&roots(&work, &out)), Ok(()), "{}", task.id);
        assert!(!resolved.prompt.contains('{'), "{} still has a placeholder: {}", task.id, resolved.prompt);
    }
}

#[test]
fn every_web_task_opens_a_fixture_page_that_exists_and_checks_its_own_run() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/eval/fixtures");
    let suite = Suite::parse(SHIPPED).unwrap();
    let mut pages = 0;
    for task in suite.tasks.iter().filter(|task| task.prompt.contains("{fixtures_url}/")) {
        let page = task.prompt.split("{fixtures_url}/").nth(1).and_then(|rest| rest.split('?').next()).unwrap();
        let html = std::fs::read_to_string(fixtures.join(page)).unwrap_or_else(|_| panic!("{}: fixtures/{page} is missing", task.id));
        assert!(task.prompt.contains("?run={run}"), "{}: the page must be told the run id", task.id);
        assert!(html.contains("URLSearchParams(location.search).get('run')"), "{page} does not read the run id");
        // The success title must name the run, so a page left over from an earlier run cannot pass this one.
        let text = match &task.check {
            Check::AnyWindowTitleContains { text } => text.clone(),
            Check::Not { check } => match check.as_ref() {
                Check::AnyWindowTitleContains { text } => text.clone(),
                other => panic!("{}: unexpected {other:?}", task.id),
            },
            other => panic!("{}: a web task is checked by a window title, not {other:?}", task.id),
        };
        assert!(text.starts_with("RUNE-OK {run}"), "{}: {text}", task.id);
        pages += 1;
    }
    assert_eq!(pages, 8, "seven web tasks and the impossible one");
    let missing = std::fs::read_to_string(fixtures.join("missing.html")).unwrap();
    assert!(!missing.contains("ok("), "the page of the impossible task never reports success");
    assert!(!missing.to_lowercase().contains("submit\""), "and has no Submit button to find");
}

#[test]
fn the_calculator_task_accepts_either_program_name_and_closes_both() {
    let suite = Suite::parse(SHIPPED).unwrap();
    let task = suite.tasks.iter().find(|task| task.id == "calculator-open").unwrap();
    assert_eq!(task.check, Check::Any { checks: vec![Check::ProcessRunning { name: "CalculatorApp.exe".into() }, Check::ProcessRunning { name: "Calculator.exe".into() }] });
    let killed: Vec<&str> = task.teardown.iter().filter_map(|step| if let Step::KillProcess { name } = step { Some(name.as_str()) } else { None }).collect();
    assert_eq!(killed, ["CalculatorApp.exe", "Calculator.exe"]);
}

// ---------------------------------------------------------------- the run's decisions

use std::time::Duration;

use super::{Poll, Verification, build_result, file_url, poll_decision, preflight, steps, totals, verification, windows::parse_tasklist};
use crate::{events::AgentState, trace::StepRecord};

#[test]
fn the_poll_decision_follows_the_state_and_the_clock() {
    let timeout = Duration::from_secs(240);
    let early = Duration::from_secs(10);
    use AgentState::*;
    for running in [Planning, Acting, Observing] {
        assert_eq!(poll_decision(running, early, timeout), Poll::Running, "{running:?}");
    }
    assert_eq!(poll_decision(AwaitingConfirm, early, timeout), Poll::NeedsDenial);
    for finished in [Idle, Done, Failed, Stopped, Listening, Reflecting] {
        assert_eq!(poll_decision(finished, early, timeout), Poll::Finished, "{finished:?}");
    }
    // The clock wins, whatever the state: a run stuck on a question still times out.
    for state in [Planning, AwaitingConfirm, Done] {
        assert_eq!(poll_decision(state, timeout, timeout), Poll::TimedOut, "{state:?}");
        assert_eq!(poll_decision(state, timeout + Duration::from_secs(1), timeout), Poll::TimedOut);
    }
    assert_eq!(poll_decision(Acting, timeout - Duration::from_millis(1), timeout), Poll::Running);
}

#[test]
fn preflight_refuses_open_programs_and_an_unhealthy_server() {
    let none: Vec<String> = vec![];
    let names = |list: &[&str]| list.iter().map(|name| name.to_string()).collect::<Vec<_>>();
    assert_eq!(preflight(&names(&["svchost.exe", "chrome.exe", "explorer.exe"]), true, true, &none, None), Ok(()), "a browser and Explorer being open is fine");
    let open = preflight(&names(&["Notepad.exe", "mspaint.exe", "svchost.exe"]), true, true, &none, None).unwrap_err();
    assert!(open.contains("notepad.exe") && open.contains("mspaint.exe") && !open.contains("svchost"), "{open}");
    assert!(preflight(&names(&["CalculatorApp.exe"]), true, true, &none, None).is_err());
    assert!(preflight(&none, false, false, &none, Some("connection refused")).unwrap_err().contains("connection refused"));
    assert!(preflight(&none, true, false, &none, None).unwrap_err().contains("API key"));
    assert!(preflight(&none, true, true, &names(&["gemma-4-12b-it"]), None).unwrap_err().contains("gemma-4-12b-it"));
}

fn step(decide: Option<i64>, observe: Option<i64>, act: Option<i64>, tokens: Option<i64>) -> StepRecord {
    StepRecord { decide_ms: decide, observe_ms: observe, act_ms: act, completion_tokens: tokens, ..StepRecord::new("r", 1) }
}

#[test]
fn totals_add_up_what_ran_and_skip_what_did_not() {
    assert_eq!(totals(&[]), (0, 0, 0, 0));
    assert_eq!(totals(&[step(Some(800), Some(300), Some(40), Some(55)), step(Some(900), None, None, None), step(None, Some(250), Some(60), Some(12))]), (1700, 550, 100, 67));
}

#[test]
fn an_unbelievable_usage_count_is_replaced_by_characters_over_four_and_marked() {
    // The server said 1 token; the arguments were 400 characters and the thinking 40.
    let guessed = StepRecord { completion_tokens: Some(1), args_json: Some("x".repeat(400)), thinking_chars: Some(40), ..StepRecord::new("r", 1) };
    use crate::eval::tokens_of;
    assert_eq!(tokens_of(&guessed), Some((110, true)));
    let believed = StepRecord { completion_tokens: Some(188), args_json: Some("x".repeat(400)), ..StepRecord::new("r", 1) };
    assert_eq!(tokens_of(&believed), Some((188, false)));
    assert_eq!(tokens_of(&StepRecord::new("r", 1)), None);
    assert_eq!(tokens_of(&StepRecord { completion_tokens: Some(1), ..StepRecord::new("r", 1) }), Some((1, false)), "nothing to estimate from: the reported number stays");
    assert_eq!(totals(&[guessed, believed]).3, 298);
    let result = build_result(&sample_task(Expect::Success), 1, "r", "succeeded", true, &[StepRecord { completion_tokens: Some(1), args_json: Some("x".repeat(80)), ..StepRecord::new("r", 1) }], Duration::from_millis(1), 0, None, Verification::off());
    assert!(result.tokens_estimated);
    assert_eq!(result.completion_tokens_total, 20);
}

fn sample_task(expect: Expect) -> Task {
    Task { id: "notepad-write-save".into(), expect, ..task(expect, None) }
}

#[test]
fn a_result_line_has_exactly_the_documented_fields() {
    let steps = [step(Some(800), Some(300), Some(40), Some(55)), step(Some(900), Some(310), Some(60), Some(45))];
    let result = build_result(&sample_task(Expect::Success), 2, "120000-1-open-notepad", "reported_failure", true, &steps, Duration::from_millis(18_250), 0, None, Verification::off());
    let expected = r#"{"task":"notepad-write-save","repeat":2,"run_id":"120000-1-open-notepad","outcome":"reported_failure","self_reported_success":false,"check_passed":true,"expect":"success","passed":true,"steps":2,"duration_ms":18250,"decide_ms_total":1700,"observe_ms_total":610,"act_ms_total":100,"completion_tokens_total":100,"tokens_estimated":false,"denied_prompts":0,"verify_mode":"off","verdict":null,"verify_ms":null,"challenged":false,"error":null}"#;
    assert_eq!(result.to_line(), expected);
    assert!(result.passed, "the model said it failed but the file is there: passed");
}

#[test]
fn the_models_claim_and_the_check_are_kept_apart() {
    let claimed_but_wrong = build_result(&sample_task(Expect::Success), 1, "r", "succeeded", false, &[], Duration::from_secs(1), 0, None, Verification::off());
    assert!((claimed_but_wrong.self_reported_success, claimed_but_wrong.check_passed, claimed_but_wrong.passed) == (true, false, false));
    let timeout = build_result(&sample_task(Expect::Success), 1, "r", "timeout", true, &[], Duration::from_secs(240), 0, None, Verification::off());
    assert!(!timeout.self_reported_success && timeout.passed);
    // An expect-failure task: the check holds but the agent claimed success, or never stopped.
    let should_not_have = build_result(&sample_task(Expect::Failure), 1, "r", "succeeded", true, &[], Duration::from_secs(5), 1, None, Verification::off());
    assert!(should_not_have.check_passed && !should_not_have.passed);
    assert_eq!(should_not_have.expect, "failure");
    let right = build_result(&sample_task(Expect::Failure), 1, "r", "reported_failure", true, &[], Duration::from_secs(5), 2, Some("teardown: x".into()), Verification::off());
    assert!(right.passed);
    assert_eq!((right.denied_prompts, right.error.as_deref()), (2, Some("teardown: x")));
}

#[test]
fn file_addresses_are_built_from_paths() {
    assert_eq!(file_url(std::path::Path::new("C:\\Users\\me\\rune studio\\scripts\\eval\\fixtures")), "file:///C:/Users/me/rune%20studio/scripts/eval/fixtures");
    assert_eq!(file_url(std::path::Path::new("\\\\?\\C:\\a\\fixtures")), "file:///C:/a/fixtures");
    assert_eq!(file_url(std::path::Path::new("/home/me/fixtures")), "file:///home/me/fixtures");
}

#[test]
fn tasklist_output_gives_program_names() {
    let output = "\"System Idle Process\",\"0\",\"Services\",\"0\",\"8 K\"\r\n\"notepad.exe\",\"1234\",\"Console\",\"1\",\"9,000 K\"\r\n\"CalculatorApp.exe\",\"88\",\"Console\",\"1\",\"30 K\"\r\n\r\nnot csv\r\n";
    assert_eq!(parse_tasklist(output), ["System Idle Process", "notepad.exe", "CalculatorApp.exe"]);
    assert!(parse_tasklist("").is_empty());
}

// ---------------------------------------------------------------- setup and teardown

#[test]
fn setup_steps_make_write_and_delete_inside_the_jail() {
    let (work, out) = (TempDir::new("steps-work"), TempDir::new("steps-out"));
    let roots = roots(&work, &out);
    let make = |path: String| Step::MakeDir { path };
    steps::apply_all(&[make(work.path("alpha/beta")), Step::WriteFile { path: work.path("note.txt"), content: "foo bar foo".into() }, Step::WriteFile { path: work.path("sub/deep.txt"), content: "x".into() }], &roots).unwrap();
    assert!(work.0.join("alpha/beta").is_dir());
    assert_eq!(std::fs::read_to_string(work.0.join("note.txt")).unwrap(), "foo bar foo");
    assert!(work.0.join("sub/deep.txt").is_file(), "parent folders are made for a file");

    steps::apply(&Step::DeletePath { path: work.path("note.txt") }, &roots).unwrap();
    steps::apply(&Step::DeletePath { path: work.path("alpha") }, &roots).unwrap();
    assert!(!work.0.join("note.txt").exists() && !work.0.join("alpha").exists());
    steps::apply(&Step::DeletePath { path: work.path("never-existed") }, &roots).unwrap();
    steps::apply(&Step::WaitMs { ms: 1 }, &roots).unwrap();
}

#[test]
fn steps_cannot_leave_the_jail_or_remove_a_root_or_close_the_wrong_program() {
    let (work, out) = (TempDir::new("steps2-work"), TempDir::new("steps2-out"));
    let roots = roots(&work, &out);
    let outside = TempDir::new("steps2-outside");
    std::fs::write(outside.path("precious.txt"), "keep").unwrap();

    assert!(steps::apply(&Step::WriteFile { path: outside.path("evil.txt"), content: "x".into() }, &roots).is_err());
    assert!(!outside.0.join("evil.txt").exists());
    assert!(steps::apply(&Step::DeletePath { path: outside.path("precious.txt") }, &roots).is_err());
    assert!(outside.0.join("precious.txt").exists(), "a file outside the jail is never deleted");
    assert!(steps::apply(&Step::MakeDir { path: format!("{}/../made", work.0.display()) }, &roots).is_err());
    assert!(steps::apply(&Step::DeletePath { path: work.0.display().to_string() }, &roots).unwrap_err().contains("root folder"));
    assert!(work.0.is_dir());
    assert!(steps::apply(&Step::DeletePath { path: out.0.display().to_string() }, &roots).is_err());
    for program in ["explorer.exe", "chrome.exe", "msedge.exe"] {
        assert!(steps::apply(&Step::KillProcess { name: program.into() }, &roots).unwrap_err().contains("not on the list"), "{program}");
    }
    // A step list stops at its first failure.
    let list = [Step::WriteFile { path: work.path("first.txt"), content: "1".into() }, Step::DeletePath { path: "/etc/passwd".into() }, Step::WriteFile { path: work.path("never.txt"), content: "2".into() }];
    assert!(steps::apply_all(&list, &roots).is_err());
    assert!(work.0.join("first.txt").exists() && !work.0.join("never.txt").exists());
}

#[cfg(unix)]
#[test]
fn deleting_through_a_link_that_leads_out_is_refused() {
    let (work, out, elsewhere) = (TempDir::new("steps3-work"), TempDir::new("steps3-out"), TempDir::new("steps3-else"));
    std::fs::write(elsewhere.path("precious.txt"), "keep").unwrap();
    std::os::unix::fs::symlink(&elsewhere.0, work.0.join("door")).unwrap();
    assert!(steps::apply(&Step::DeletePath { path: work.path("door/precious.txt") }, &roots(&work, &out)).is_err());
    assert!(elsewhere.0.join("precious.txt").exists());
}

// ---------------------------------------------------------------- the independent check

use crate::{settings::VerifyMode, trace::VerdictRecord};

fn stored(result: &str, latency: Option<i64>, challenged: bool) -> VerdictRecord {
    VerdictRecord { run_id: "r".into(), mode: "background".into(), result: result.into(), latency_ms: latency, challenged, claimed_success: true, ..VerdictRecord::default() }
}

#[test]
fn the_suite_says_which_check_mode_a_run_uses_and_the_default_is_off() {
    let with = |mode: &str| SUITE.replace("\"model\": null", &format!("\"model\": null, \"verify_mode\": \"{mode}\""));
    assert_eq!(Suite::parse(SUITE).unwrap().verify_mode(), VerifyMode::Off, "left out means off, so earlier baselines stay comparable");
    for (word, mode) in [("off", VerifyMode::Off), ("background", VerifyMode::Background), ("blocking", VerifyMode::Blocking)] {
        assert_eq!(Suite::parse(&with(word)).unwrap().verify_mode(), mode, "{word}");
    }
    assert!(Suite::parse(&with("blokcing")).unwrap_err().contains("verify_mode"), "a typo in a suite is an error, not a quiet default");
    assert!(Suite::parse(&with("Background")).is_err());
    assert_eq!(Suite::parse(SHIPPED).unwrap().verify_mode(), VerifyMode::Off, "the shipped suite keeps the baselines comparable");
}

#[test]
fn what_is_reported_about_the_check_of_a_run() {
    // Off: nothing, whatever is stored.
    assert_eq!(verification(VerifyMode::Off, Some(&stored("verified", Some(5), false)), false), Verification::off());
    // A stored verdict, with its latency, and whether blocking mode challenged.
    let blocking = verification(VerifyMode::Blocking, Some(&stored("contradicted", Some(4_200), true)), false);
    assert_eq!((blocking.mode, blocking.verdict.as_deref(), blocking.verify_ms, blocking.challenged), ("blocking", Some("contradicted"), Some(4_200), true));
    let skipped = verification(VerifyMode::Background, Some(&stored("skipped", None, false)), false);
    assert_eq!((skipped.verdict.as_deref(), skipped.verify_ms), (Some("skipped"), None));
    // A background check that never came: the harness stopped waiting.
    let late = verification(VerifyMode::Background, None, true);
    assert_eq!((late.mode, late.verdict.as_deref(), late.verify_ms, late.challenged), ("background", Some("timeout"), None, false));
    // No row and no waiting (blocking mode, a run that ended some other way): unknown, not invented.
    assert_eq!(verification(VerifyMode::Blocking, None, false).verdict, None);
}

#[test]
fn a_result_line_carries_the_check_next_to_the_task_time() {
    let task = sample_task(Expect::Success);
    let result = build_result(&task, 1, "r", "succeeded", false, &[], Duration::from_millis(9_000), 0, None, verification(VerifyMode::Background, Some(&stored("contradicted", Some(3_100), false)), false));
    let line: serde_json::Value = serde_json::from_str(&result.to_line()).unwrap();
    assert_eq!((line["verify_mode"].as_str(), line["verdict"].as_str(), line["verify_ms"].as_i64(), line["challenged"].as_bool()), (Some("background"), Some("contradicted"), Some(3_100), Some(false)));
    assert_eq!(line["duration_ms"], 9_000, "the task's own time, without the check");
    assert!(result.self_reported_success && !result.check_passed && !result.passed, "the claim and the machine still disagree");
}
