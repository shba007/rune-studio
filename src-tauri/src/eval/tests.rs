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
    assert_eq!(suite.config, Config { fast_checks: Some(false), show_thinking: Some(false), model: None });
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
