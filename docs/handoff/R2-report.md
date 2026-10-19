# R2 report: the evaluation harness

Worked directly on `develop` (you chose that instead of the `feat/r2-eval-harness` branch the handoff names), starting from `2e3cea5`. Five work commits plus this report. **Nothing was run end to end**: there is no Windows desktop, no model server and no PowerShell here. In other words R2 is "built, not measured".

## 1. Environment

Linux (cloud container), `rustc 1.98.1`, Node 22, bun 1.4.2. No `pwsh`, no Windows, no live rune-infer. A headless Chromium (pre-installed) was used to check the fixture pages.

## 2. Test counts

|                                                      | Baseline                        | Final                                                                     |
| ---------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------------- |
| `cargo test --lib`                                   | 151 passed, 0 failed, 2 ignored | 151 passed, 0 failed, 2 ignored (no `eval::` tests in a default build: 0) |
| `cargo test --lib --features agent-bridge`           | 183 passed, 0 failed, 2 ignored | 183 passed, 0 failed, 2 ignored                                           |
| `cargo test --lib --features eval,agent-bridge`      | n/a                             | 212 passed, 0 failed, 2 ignored (29 new `eval::` tests)                   |
| `node --test` on the report tests                    | n/a                             | 11 passed                                                                 |
| `bun run build` / `bun run test:unit`                | ok / 6 passed                   | ok / 6 passed                                                             |
| `cargo clippy --all-targets --features agent-bridge` | 18 warning lines                | 18 (also 18 with `eval,agent-bridge`)                                     |

Also: `cargo check --lib --tests --target x86_64-pc-windows-gnu --features eval,agent-bridge` finishes, so the Win32 code (`EnumWindows`, `GetWindowTextW`) type-checks against the real crate. `grep -c eval .github/workflows/deploy.yml` prints 0, and the release workflow builds with `--features agent-bridge` only.

`pwsh scripts/eval/run.ps1 -DryRun`: **not run** (no PowerShell here).

`git diff --stat 2e3cea5`: 34 files, all new except `Cargo.toml` (the `eval = []` feature), `lib.rs` (`mod eval` and `eval::start`, both under the feature), `safety/gate.rs` (below), `package.json` (`test:eval`), README, ARCHITECTURE and ROADMAP.

## 3. Work packages

### WP1: suite format and pure logic (done) `32097cd`

- `eval/spec.rs`: versioned suite JSON with `deny_unknown_fields`; `Config` (`fast_checks`, `show_thinking`, `model`, all optional); tasks with `setup`, `check`, `expect`, `teardown`, optional `timeout_secs`; setup/teardown kinds `make_dir`, `write_file`, `delete_path`, `kill_process`, `wait_ms`; checks `file_exists`, `dir_exists`, `file_contains`, `file_absent`, `any_window_title_contains`, `process_running`, `all`, plus `any` and `not`. Placeholders `{work}`, `{run}`, `{fixtures}`, and `{fixtures_url}`.
- Hard limits: `jail` (absolute, no `..`, inside the work or output folder, and no link that leads out where it can be told) and `kill_allowed` (case-insensitive allowlist of the five programs). `Task::confine` applies both to every path of the setup, teardown and checker and every kill of a resolved task. `steps.rs` checks the jail again when a step runs.
- `eval/check.rs`: `evaluate` over a `Machine` trait (windows and processes), `read_text` (UTF-8, UTF-8 with a mark, UTF-16), and `passed(task, check_passed, outcome)`.
- Tests: parse (valid, unknown field rejected at every level, unknown kind rejected, wrong version rejected, duplicates, bad repeats, bad ids); placeholder substitution; the jail rejects `..`, another drive, an absolute path outside the folder, a sibling folder with a similar name, and (on Unix) a link out; the allowlist (including `explorer.exe`, browsers, `notepad.exe /f`); each file checker against a temp dir; encodings; windows and processes through a fake; `all`/`any`/`not`; an unsupported machine is an error, never a quiet no; the pass rule for success and failure tasks.
- **Deviations (by design, small):** I added `any` and `not` (the handoff asks for an `any` for Calculator and a negation for `impossible-task`), `accept_outcomes` on a task, and `{fixtures_url}` (see WP2). Paths in the shipped suite use `\`; the Linux test normalises them to `/`.

### WP2: the 15 tasks and fixtures (done) `f4661de`

- `scripts/eval/suite.desktop-15.json` with the 15 fixed ids in order; eight fixture pages (`click`, `form`, `scroll`, `controls`, `popup`, `small`, `slider`, `missing`) that read `run` from the query and set `document.title` only on the success condition; `scripts/eval/README.md` (prerequisites, what the run does to the machine, the 15-task table, what is not certain).
- Verified in headless Chromium, including the negative cases (wrong list item, a half-done controls page, the banner still up, a slider short of the end): each success title appeared only on its success condition. Re-checked after the pre-commit formatter rewrote the pages. Rust tests keep the shipped suite valid, confined, and pointed at existing pages whose success title names the run.
- **Calculator:** the check is `any` of `process_running CalculatorApp.exe` and `process_running Calculator.exe`, and teardown closes both. I chose both names because I could not confirm which Windows 11 uses (the handoff marks it NOT CERTAIN; I believe `CalculatorApp.exe` but have not seen it).
- **Deviation:** the handoff writes the web prompts as `{fixtures}\click.html?run={run}`. A path with a query string is not a file Windows can open, so the prompts use `{fixtures_url}/click.html?run={run}`, a `file:///` address.
- **Deviation:** `delete-denied` accepts `failed` as well as `reported_failure`/`stopped`. After the harness says No three times the agent ends with outcome `failed` ("you said no to this several times"), which is the right behaviour here. `impossible-task` keeps the handoff's two outcomes.

### WP3: eval mode (done on paper) `11bec84`

- `eval/mod.rs`, `eval/steps.rs`, `eval/windows.rs`; `lib.rs` calls `eval::start` only under the feature. Behaviour as specified: preflight (exit code 2 and `preflight.txt`); in-memory overrides (never `settings.json`); `env.json` (model, settings snapshot, version, git commit via `RUNE_GIT_COMMIT` at build time, date, suite name and hash); per task and repeat setup, idle wait, `commands::begin_task`, a 500 ms poll that answers No to any question and calls `commands::stop` on timeout, the checker, teardown, and the run read back from the trace database by id; one `results.jsonl` line with the fields the handoff lists; `app.exit(code)` with 1 only if setup or teardown failed.
- `safety/gate.rs`: a read-only `pending_question` accessor and a `text` field on `Pending`, both only under the `eval` feature; asking and answering are unchanged.
- Pure functions tested: `poll_decision` (every state and the clock), `preflight`, `build_result` and the exact JSON line, `totals`, `file_url`, `parse_tasklist`, and the step executors on a temp dir (they refuse to leave the jail, to delete a root folder, to close another program, or to follow a link out).
- **Not tested:** `execute` and `run_one` (they need an `AppHandle` and a desktop).
- Adds no dependency and no `windows` feature (processes come from `tasklist`/`taskkill`; windows from `EnumWindows`, already enabled).

### WP4: runner and report (done; the runner is untested) `2e092cd`

- `scripts/eval/run.ps1`: `-Suite`, `-Out`, `-Repeats`, `-FastChecks on|off|suite`, `-NoBuild`, `-DryRun`. It validates the suite, prints the plan, builds with `--features eval,agent-bridge`, copies the suite and the fixture pages into the output folder (the app reads fixtures from next to the suite file), runs the app with the two variables set, prints `preflight.txt` on a refusal, and runs `report.mjs`. **It has never been executed.** PowerShell was not available, so syntax and behaviour are unchecked.
- `scripts/eval/report.mjs` (Node, no dependencies; first file is the run, `--compare` the baseline): per task and overall (runs, passes, rate, median steps and duration, median decide time per step, tokens), the disagreement tables, and the gate. Exit code 1 on FAIL, 2 on bad arguments.
- Tests (`scripts/eval/test/`): medians for odd and even counts, aggregation by hand, the disagreement split, and the gate on equal runs, one run lower (14 vs 13 of 15: pass), a real drop (11 of 15: fail), 12% slower (fail), and exactly 10% slower (pass), plus the command line exit codes.
- **Deviation:** `package.json` has `test:eval` as `node --test "scripts/eval/test/*.test.mjs"`. Node 22 treats `node --test scripts/eval/test` as a file path and fails.
- **Deviation:** the disagreement table compares `self_reported_success` with `check_passed` only for tasks that should succeed. For an `expect: failure` task `check_passed` means "nothing bad happened", so those runs get a third list: the model claiming success on a task that should not succeed.

### WP5: CI and docs (docs done, CI not done on your instruction) `bb23b0a`

- No CI job: you chose to skip it because the Rust workflow is gone. The handoff's reference to `.github/workflows/rust.yml` is stale. The README and ARCHITECTURE list the commands to run locally.
- ARCHITECTURE has a new "The evaluation harness" section and a line under "Build, test and release"; README has an "Evaluating changes" subsection; ROADMAP has the R2 status table and the move of replay to a later milestone; `docs/evals/README.md` explains how to make, commit and use a baseline.

## 4. NOT CERTAIN

- Everything that needs Windows: `EnumWindows` titles, `tasklist` parsing on a real machine, `taskkill`, the preflight, and the whole run. Type-checked only.
- `run.ps1`: never run; PowerShell 7 behaviour of `ConvertTo-Json` on the suite copy (arrays, escaping) is unchecked.
- Default browser behaviour with `file://` pages and query strings, whether the browser puts the page title in its window title, and that the page stays in the foreground long enough to be seen.
- The Calculator process name (both are accepted).
- Task 5 (`settings-open`) passes on any visible window whose title contains `Settings`.
- Whether the agent can actually do each of the 15 tasks; that is the point of the baseline.
- That `begin_task` followed by `open_run_id` always finds the new run: `begin_run` on the recorder is synchronous, so it should.

## 5. Notes for the baseline run

- Close Notepad, Calculator, Settings and Paint first (the run refuses otherwise), set a default browser, close Rune Studio, start rune-infer.
- Expect roughly 45 runs (15 tasks x 3 repeats); if every run timed out it would be 3 hours (240 s each), a normal run is a few minutes.
- The run leaves the fixture pages open in your browser, and `delete-denied` leaves its work folders under the output folder.
- Run it twice (`-FastChecks off`, then `on`), look at the report before committing, and use `docs/evals/README.md` for the file names.
- If many runs show an `error` about setup or teardown, fix that first; those are harness problems, not agent failures.

## 6. Noticed, not changed

- The `Rust` workflow being removed means none of the new tests run in CI; they are listed as commands only.
- The pending question the harness answers is whichever is on screen; if two ever overlapped the harness would answer the current one only. They do not today.
- The R2 handoff `docs/handoff/R2-eval-harness.md` is still in the repository: I did not remove it before you have looked at this work. R3 is next.
- `results.jsonl` `duration_ms` is measured by the harness from starting the task to its end; the trace's own timestamps have one-second resolution.

## 7. Questions for the maintainer

1. Should the first baseline use `-Repeats 3` (45 runs, about 2 percentage points per run) or more, since one run is the gate's tolerance?
2. Do you want `failed` accepted for `impossible-task` too (it can end `failed` if the screen stops changing)? I kept the handoff's two outcomes there.
3. May I remove `docs/handoff/R2-eval-harness.md` now, as with the earlier completed handoffs?