# Handoff: Milestone R1, the flight recorder

For an AI coding agent. Read all of it first. Plan it comes from: `docs/ROADMAP.md` (milestone R1). How the code works today: `docs/ARCHITECTURE.md`. Where this file and the code disagree about a fact, the code wins: say so in your report.

## 1. Goal

Today a task leaves screenshots and a `session.json` manifest, plus a few lines in `audit.log`. The step history lives only in memory and is dropped when the run ends. R1 makes every run, built-in or by an outside agent, leave a **complete, queryable record**: what was asked, each step (what the model saw, what it chose, what the gate decided, what happened, how long each phase took), and how the run ended. Later milestones (the verifier, reflection, evals) read this record; R1 only **writes and reads it**. R1 adds no behaviour that changes what the agent does.

Hard rules for the recorder: it must **never** slow a step noticeably, never block the agent loop, and **never fail a task**. If it cannot write, the task goes on and the failure is logged. (This is the opposite of the screenshot archive, which fails closed on purpose; do not change the archive's behaviour.)

## 2. Preconditions

1. PR #17 and the R0.5 branch must be **merged into `develop`**. Check: `git fetch origin && git merge-base --is-ancestor 2cd920c origin/develop && echo ok`. Also `grep -rn "input_audio" src-tauri/src/model/client.rs` must find the R0.5 change. If either fails, **stop** and tell the maintainer.
2. `git switch -c feat/r1-flight-recorder origin/develop`. One commit per work package. Push only that branch. No force-push, no PR unless asked, never delete branches.
3. Toolchain: Rust 1.98.1+, Bun 1.4.x; Linux needs the apt packages in `.github/workflows/rust.yml`.
4. Baseline numbers (put them in your report first): `cd src-tauri && cargo test --lib` and `cargo test --lib --features agent-bridge`, then `bun run build` and `bun run test:unit`.

## 3. Decisions already made (do not reopen)

- Store: **SQLite**, one file, in the **app data folder** (the folder that already holds `audit.log`, found with `app.path().app_data_dir()`; for the command line without an app, `%APPDATA%\dev.shirsendu-bairagi.rune-studio\`, the same place `settings::path_without_app` uses). File name `trace.sqlite3`. *(This differs from the earlier idea of putting it next to the screenshots: the CLI must be able to find it without the app.)*
- Screenshots and recordings stay files in the session folders; rows keep their **paths**.
- The run's id **is** the archive session id (`archive::begin_session` returns it, the folder name `<hhmmss>-<n>-<slug>`).
- Retention is **opt-in**: nothing may delete a user's files unless a setting says so.
- No UI in R1. A read-only command line view is in scope.

## 4. Work packages

### WP1: prove the dependency, and close the panic gap (small)

1. Add `rusqlite` with the `bundled` feature to `src-tauri/Cargo.toml` (choose a current version compatible with Rust 1.98.1; look it up, do not guess). Add a tiny test that opens an in-memory database, creates a table, inserts and reads a row. **Push the branch and confirm the `Rust` workflow passes on both `windows-latest` and `ubuntu-latest` before building on it** (use `mcp__github__actions_list` or the Actions page). Whether `bundled` builds on macOS (the release workflow builds macOS) is **NOT CERTAIN**: run `cargo check --target aarch64-apple-darwin` if you can, otherwise say "not checked on macOS".
2. Panic gap from R0: in `agent::start_planned` keep a clone of the run's token (`scope.token`) and, after `supervised(...)` returns (any outcome), call `.cancel()` on it, so anything the run left running (an input action already in flight) is told to stop. Extract it as `fn stop_leftovers(token: &CancelToken, outcome: &Outcome)` so it can be unit tested (a panicked outcome cancels the token; so does a normal one: tokens are per run).

Acceptance: CI green on both OSes with the dependency; test for `stop_leftovers`; no other behaviour change.

### WP2: the trace store (`src-tauri/src/trace/`, new module)

Layout: `trace/mod.rs` (public API, the recorder handle), `trace/schema.rs` (SQL, migrations), `trace/store.rs` (blocking reads and writes). Register `mod trace;` in `lib.rs`.

**Writer.** A dedicated `std::thread` owns the `rusqlite::Connection`. The rest of the app sends it small messages through a bounded `std::sync::mpsc::sync_channel` (capacity 1024) using **`try_send`**: if the channel is full or the thread is gone the message is dropped, a counter is incremented and a `warn` is logged at most once a minute. Nothing in the agent path may `await` or block on the database. Open the database with `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000`. Schema version in `PRAGMA user_version`; migrations run at open; opening a database whose version is **newer** than the code must not touch it (record nothing, log once).

**Schema (version 1).** Use exactly these columns unless you find a concrete problem (then explain in the report):

`runs`: `id TEXT PRIMARY KEY` (session id, or `norun-<scope id>-<unix seconds>` if the session folder could not be made), `scope_id INTEGER`, `task TEXT`, `source TEXT` ("built-in" or the outside agent's name), `started_utc TEXT`, `ended_utc TEXT`, `outcome TEXT` (same labels as the manifest: `succeeded`, `reported_failure`, `failed`, `stopped`, `lease_expired`, `replaced`), `summary TEXT`, `plan_json TEXT`, `model TEXT`, `app_version TEXT`, `settings_json TEXT` (a snapshot of `max_steps`, `show_thinking`, `ask_every_action`, `fast_checks`; **never** the API key, base URL, or hotkeys), `session_dir TEXT`.

`steps`: `run_id TEXT REFERENCES runs(id)`, `step INTEGER`, `PRIMARY KEY (run_id, step)`, `started_utc TEXT`, `goal INTEGER`, `tool TEXT`, `args_json TEXT` (capped at 2,000 characters), `reason TEXT`, `risk_stated TEXT`, `risk_final TEXT`, `risk_reasons_json TEXT`, `gate TEXT` (`allowed`, `approved`, `declined`, `no answer in time`, `stopped`, `none` for harmless actions), `result TEXT` (`ok`, `failed`, `cancelled`, `skipped`), `error TEXT`, `observation_path TEXT` (the screenshot file the model saw), `elements_hash TEXT` (hash of the control list text; no list text), `prompt_text TEXT` (the user part of the model request, capped at 8,000 characters; **never** the base64 image), `thinking_chars INTEGER`, `completion_tokens INTEGER`, and phase timings in milliseconds: `observe_ms`, `screen_check_ms`, `finish_check_ms`, `decide_ms`, `gate_ms`, `act_ms`, `settle_ms` (NULL when the phase did not run).

`events`: `id INTEGER PRIMARY KEY AUTOINCREMENT`, `run_id`, `at_utc`, `kind TEXT` (`plan`, `steer`, `invalid_answer`, `stuck_warning`, `elevated_warning`, `ask_user`, `note`), `data_json TEXT` (capped at 2,000 characters).

Indexes: `steps(run_id)`, `events(run_id)`, `runs(started_utc)`.

**API** (all non-blocking `&self` methods on a cheap-to-clone `Recorder`, stored in `AppState` as `trace: Recorder`, created in `lib.rs::setup` right after `archive::init`; a no-op recorder when opening failed): `begin_run(RunInfo)`, `step(StepRecord)` (an upsert: the loop may send a step in two pieces, e.g. decision first, result later), `event(run_id, kind, data)`, `end_run(run_id, outcome, summary)`. Keep the types plain structs. A separate **blocking** read API for tooling (`store::list_runs(limit)`, `store::get_run(id)` returning the run and its steps and events) that opens its own read-only connection.

Tests (temp-dir databases; no Tauri needed): migration from nothing creates version 1; opening twice is idempotent; a newer `user_version` is left untouched; run/step/event round trip; a step sent in two pieces ends up as one row; `step` and `event` truncate over-long text; with the channel full `step` returns at once, drops, and counts; `list_runs` ordering (newest first) and the limit; reading while the writer is writing (WAL) works.

### WP3: capture the screenshot path

`observe/screenshot.rs::capture_region` saves the JPEG with `archive::save` but throws the returned path away. Add `pub path: PathBuf` to `Frame` (empty in unit tests, like `archive::path_for` returns). `Frame` is built in one place only. No other behaviour change.

### WP4: record the built-in agent's runs (`agent/mod.rs`, `safety/gate.rs`)

1. `start_planned`: after `begin_session`, call `trace.begin_run(...)` (task, source "built-in", model, settings snapshot, app version, session dir). After `supervised` returns, call `end_run` with the **same outcome labels** `end_session` uses (extract the label mapping once and reuse it for both). A panic must still produce an `end_run` with outcome `failed`.
2. In `run`: record the plan (`plan_json` and a `plan` event), and per loop iteration build one `StepRecord`: observation path (`frame.path`), `elements_hash`, `prompt_text` (the `text` passed to the model), `decide_ms` (time around `call_retrying`), `observe_ms` (the capture + control-list `join!`), `screen_check_ms`, `finish_check_ms`, `completion_tokens`, `thinking_chars` (length of `reply.thinking`), tool and args (`reply.call.arguments` as JSON text), `reason`, `goal`, `risk_stated`; then the gate result, `act_ms`, `settle_ms`, `result`, `error`. Iterations that end without an action (invalid answer, declined action, `ask_user`, a plan-ending `done`) still produce a row with the right `result`/`gate`; record `steer`, `invalid_answer`, `stuck_warning`, `elevated_warning` as events when they happen.
3. `safety/gate.rs`: today `check` returns only a `Verdict` while the risk and reasons it computed go to `audit.log`. Add `check_detailed(...) -> CheckResult { verdict, risk_final, reasons, decision: &'static str }` and make `check` a thin wrapper returning `.verdict`. Do **not** change what `check` decides or what it writes to `audit.log`.
4. Timing is taken with `Instant`; do not add `.await` points or sleeps. The recorder calls are fire-and-forget.

### WP5: record outside agents' runs (`bridge/mcp.rs`)

`begin` -> `trace.begin_run` (source = the agent's name, task, plan if given); each action tool (`act_turn`) -> one `step` row (tool, args_json, reason, goal, risk, gate, result, `act_ms` and `gate_ms`, the screenshot path returned with the reply if available); `done`, Stop and the idle expiry -> `end_run` with the same labels as `archive::end_session` at those places. `rune_observe`, `rune_elements`, `rune_plan`, `rune_status` are not steps (record `plan` as an event). The recorder must not add any `.await` to a call's path.

### WP6: read-only command line view (feature `agent-bridge`, `bridge/cli.rs`)

`rune-studio-cli runs [--limit N]` (default 20) prints a table: id, started, source, outcome, steps, duration, task (cut to 50 characters). `rune-studio-cli run <id>` prints the run, then one line per step (`n tool result gate decide_ms act_ms reason`) and the events. If the database does not exist, say so plainly and exit 0. Update `HELP` (a test checks the help text mentions every command) and the `rune-studio-cli` section of `docs/AGENT-GUIDE.md` is **not** edited (agents do not use these). Tests: the table and step line formatting as pure functions with fixed input.

### WP7: opt-in retention (`archive.rs`, `settings.rs`)

Add a setting `archiveKeepDays: u32` (serde default **0 = keep everything**), in `Settings` (Rust) and `src/types/protocol.ts`. No UI. If it is > 0, once at startup on a background thread, delete session folders under `Documents/Rune Studio/sessions/<date>/` older than that many days, **only** folders whose date folder is older than the cutoff and **only** below the `sessions` folder; delete the matching `runs` (and their steps and events) from the trace database; never touch anything else; log what was removed at `info`. Pure function `fn expired_dates(names: &[String], today: &str, keep_days: u32) -> Vec<String>` with tests (0 keeps all; boundary day; ignores names that are not `YYYY-MM-DD`). **Do not** remove the per-screenshot `fsync` in this milestone: the new timings are there to measure it first.

### WP8: docs

`docs/ARCHITECTURE.md`: a new section "The trace store" (location, schema summary, writer thread and drop policy, what is and is not stored, retention, the CLI commands); update the record section (rows keep paths; id = session id) and the known-gaps list (R1 items done). `README.md`: data locations (`trace.sqlite3`), the two CLI commands, the new setting, and the privacy note: the database holds the task text, the model's prompts and the arguments of typed text, so it is as sensitive as the screenshots. `docs/ROADMAP.md`: R1 status table with built / not built / unverified. Keep the NOT CERTAIN markers that remain true.

## 5. Do NOT

- Change what the agent does, any safety rule or threshold, retries, the bridge's behaviour, or the audio/thinking request format (R0.5).
- Remove `sync_data()` from `archive::save`, or make the archive best-effort.
- Store the API key, the base URL, image data, or the bridge token anywhere in the database.
- Add a UI, a reflection job, or a verifier. Do not add dependencies other than `rusqlite` (and a time-formatting helper only if `archive.rs`'s existing ISO helper cannot be reused: prefer reuse).
- Run `cargo fmt` over existing files.

## 6. Checks to run and paste (last lines)

```bash
cd src-tauri
cargo test --lib                          # baseline + your new tests, 0 failed
cargo test --lib --features agent-bridge  # baseline + your new tests, 0 failed
cd .. && bun run build && bun run test:unit
git diff --stat origin/develop            # only files named in section 4
```
Show the final `CREATE TABLE` statements (from `schema.rs`). Report whether the `Rust` workflow passed on both OSes for your branch. If you have a Windows desktop and a live model server, run one real task, then paste `rune-studio-cli runs` and `rune-studio-cli run <id>` output and the size of `trace.sqlite3`; otherwise write "not run end to end". Do not claim measurements you did not make.

## 7. Report (required)

`docs/handoff/R1-report.md`: environment; baseline and final numbers; per work package status, commit hash, files, deviations; every NOT CERTAIN item (macOS build of `bundled`, anything about the CLI path on non-Windows, etc.); the per-phase timings you saw if you ran a real task; things noticed but not changed; questions for the maintainer. Conventional commit messages only (`feat(trace): ...`, `fix(agent): ...`, `docs: ...`). Push only your branch, then tell the maintainer you are done.

## 8. What the verifier will do (and will not)

The verifier writes no code. It will re-run section 6, read each work package's diff against its design, check that the recorder never awaits or blocks in the agent path (`try_send`, no `.lock()` held across awaits), check that the schema matches section 4, that no secret or image data can reach the database, that `check` still behaves identically, and that the tests assert exact values. After merge the maintainer should run one real task on Windows and inspect the database, because no unit test proves that.
