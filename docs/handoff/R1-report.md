# R1 report: the flight recorder

Branch `feat/r1-flight-recorder`, from `origin/develop` (`353a1a9`, which has PR #17 and the R0.5 change: `input_audio` is in `model/client.rs`). Ten commits (nine work commits and this report). Pushed the branch only; no PR.

## 1. Environment

Linux (cloud container), `rustc 1.98.1`, bun 1.4.2. **No Windows desktop, no live model server, no macOS.** No task was run end to end, so I have **no measured per-phase timings and no real `trace.sqlite3` size**. I do not claim any.

## 2. Test counts

|                                                      | Baseline                        | Final                                   |
| ---------------------------------------------------- | ------------------------------- | --------------------------------------- |
| `cargo test --lib`                                   | 125 passed, 0 failed, 2 ignored | 151 passed, 0 failed, 2 ignored         |
| `cargo test --lib --features agent-bridge`           | 152 passed, 0 failed, 2 ignored | 183 passed, 0 failed, 2 ignored         |
| `bun run build` / `bun run test:unit`                | ok / 6 passed                   | ok / 6 passed                           |
| `cargo clippy --all-targets --features agent-bridge` | 18 warning lines                | 18 (my first pass had 24; I fixed mine) |

Also `cargo check --lib --tests --target x86_64-pc-windows-gnu --features agent-bridge` finishes (this compiles the bundled SQLite C code with mingw, so it is real evidence the dependency builds for Windows; it is not a run).

`git diff --stat origin/develop` (23 files): the new `trace/` module (4 files), `agent/mod.rs`, `bridge/{cli,mcp,server}.rs`, `commands.rs`, `events.rs`, `lib.rs`, `archive.rs`, `settings.rs`, `safety/{gate,mod}.rs`, `observe/screenshot.rs`, `Cargo.toml`/`Cargo.lock`, `src/types/protocol.ts`, `src/dev/mockTauri.ts`, README, ARCHITECTURE, ROADMAP. Files outside the ones section 4 names: `events.rs` (the `trace` field of `AppState`), `lib.rs` (start the recorder, startup retention), `commands.rs` (end the open run on Stop), `bridge/server.rs` (end it on idle expiry), `safety/mod.rs` (`Risk::label`), `settings.rs` (the setting), `src/dev/mockTauri.ts` (keeps the mock typed), `Cargo.lock`.

## 3. Schema (`trace/schema.rs`, version 1)

```sql
CREATE TABLE runs (
    id TEXT PRIMARY KEY,
    scope_id INTEGER,
    task TEXT,
    source TEXT,
    started_utc TEXT,
    ended_utc TEXT,
    outcome TEXT,
    summary TEXT,
    plan_json TEXT,
    model TEXT,
    app_version TEXT,
    settings_json TEXT,
    session_dir TEXT
);

CREATE TABLE steps (
    run_id TEXT NOT NULL REFERENCES runs(id),
    step INTEGER NOT NULL,
    started_utc TEXT,
    goal INTEGER,
    tool TEXT,
    args_json TEXT,
    reason TEXT,
    risk_stated TEXT,
    risk_final TEXT,
    risk_reasons_json TEXT,
    gate TEXT,
    result TEXT,
    error TEXT,
    observation_path TEXT,
    elements_hash TEXT,
    prompt_text TEXT,
    thinking_chars INTEGER,
    completion_tokens INTEGER,
    observe_ms INTEGER,
    screen_check_ms INTEGER,
    finish_check_ms INTEGER,
    decide_ms INTEGER,
    gate_ms INTEGER,
    act_ms INTEGER,
    settle_ms INTEGER,
    PRIMARY KEY (run_id, step)
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES runs(id),
    at_utc TEXT,
    kind TEXT,
    data_json TEXT
);

CREATE INDEX steps_run ON steps(run_id);
CREATE INDEX events_run ON events(run_id);
CREATE INDEX runs_started ON runs(started_utc);
```

Applied with `PRAGMA journal_mode=WAL; synchronous=NORMAL; foreign_keys=ON; busy_timeout=5000`; the version is `PRAGMA user_version = 1`.

## 4. Work packages

### WP1: dependency and the panic gap (done; CI part **NOT done**) `fd41c53`

- `rusqlite 0.40` with `default-features = false, features = ["bundled"]` (the crate's default features pull in a wasm-only dependency). Smoke test in `trace/tests.rs`.
- `stop_leftovers(token, outcome)` cancels the run's own token after `supervised` returns, whatever the outcome; unit tested (all three outcomes cancel; another token is untouched).
- **The "push and confirm the `Rust` workflow passes on both OSes before building on it" step was not done.** `rust.yml` triggers only on pull requests and pushes to `develop`/`main`, so pushing the branch starts nothing, and I did not open a PR because you did not ask for one and you asked earlier to stop the queued Rust runs. `rust.yml` is also still on `develop` (the removal branch `chore/remove-rust-workflow` is not merged). Evidence I have instead: Linux tests, the Windows cross build above. **macOS: not checked** (no Apple C toolchain here; the cc-rs build of `sqlite3.c` for `aarch64-apple-darwin` fails on Linux for that reason, so it says nothing about macOS itself).

### WP2: the trace store (done) `d768a45`

- `trace/{mod,schema,store}.rs` (+ `tests.rs`). One writer thread (`trace-writer`), bounded `sync_channel(1024)`, `try_send` only; a full or closed queue drops, counts (`Recorder::dropped`) and warns at most once a minute. Messages are batched (up to 64) into one transaction; a failed message does not stop the rest.
- Schema as in section 3. Opening a database with a **newer** `user_version` leaves it untouched and records nothing. An unopenable file gives a no-op recorder.
- API: `begin_run`, `step` (an upsert: a piece only overwrites what it carries, via `COALESCE`), `event`, `end_run` (only the first end counts), plus `set_plan`, `purge_sessions`, `open_run_id`, `end_open_run`. Blocking read API `store::{list_runs, get_run}` opens its own read-only connection.
- Tests (temp-dir databases): creation at version 1, idempotent open, newer version untouched, full round trip with exact values, a step in two pieces is one row, truncation to 2,000 / 8,000 / 2,000 characters, a full queue returns at once and counts (3 dropped of 5 at capacity 2), newest-first ordering and the limit, reading 12 times while 300 steps are being written (WAL), first end wins and a new run replaces an open one, a step for a run whose beginning was dropped, purge, a missing file is not created, ISO parsing.
- A real bug caught by the first test run: SQLite wants a table constraint after the column list, so my first `CREATE TABLE steps` failed. In production that would have meant a silent no-op recorder (logged at error), so the tests matter here.
- **Deviation:** the module lives in `trace/mod.rs` as asked, plus a `tests.rs` file; I added `set_plan`, `purge_sessions`, `open_run_id` and `end_open_run` to the API because the callers need them (plan JSON, retention, and Stop/idle/`rune_done` ending a run without an id).

### WP3: screenshot path (done) `5959d4c`

- `Frame.path: PathBuf` set from the value `archive::save` already returned; empty in unit tests. No other change.

### WP4: the built-in agent (done) `6ba77cc`

- `start_planned` calls `begin_run` (task, source "built-in", model, app version, settings snapshot of exactly four settings, session dir) and, after `supervised` returns, `end_run` with `outcome_label`, the one mapping `end_session` now also uses. A panic still ends the run as `failed`.
- `run` records the plan (`plan_json` and a `plan` event) and one `StepRow` per iteration. `StepRow` sends its `StepRecord` from `Drop`, so every `continue`, `return` and even a panic leaves a row; an iteration that ends with no result set counts as `cancelled` (all of the loop's early returns are Stop). Recorded: observation path, `elements_hash` (64-bit FNV-1a of the control list; the list text is not a column), prompt text, tool, arguments, reason, goal, stated risk, final risk and reasons, gate decision, token and thinking counts, and `observe_ms`, `screen_check_ms`, `finish_check_ms`, `decide_ms`, `gate_ms`, `act_ms`, `settle_ms`. Events: `plan`, `steer`, `invalid_answer`, `stuck_warning`, `elevated_warning`, `ask_user`, `note` (the decision model ended the task).
- `safety/gate.rs`: `check_detailed` returns `CheckResult {verdict, risk_final, reasons, decision}`; `check` is a wrapper returning `.verdict` (now unused, marked `allow(dead_code)`). The code paths, decisions and the `audit.log` line are unchanged; I did not add a test for `check` itself because it needs an `AppHandle`.
- No `.await` added to the loop; all recorder calls are `try_send`.
- Tests: outcome labels; a dropped `StepRow` reaches the database with the right result (`cancelled` default, `failed` with error).

### WP5: outside agents (done) `1bd44b5`

- `rune_begin` opens a run (source = agent name, same id as its session folder); each action tool leaves one step row (tool, arguments, reason, goal, stated and final risk, gate, result, `gate_ms`, `act_ms`, `settle_ms`, the screenshot returned with the reply); the plan is an event (also through `set_goals`); `rune_done`, Stop and the idle expiry call `end_open_run` with the labels the session folder gets. The recorder remembers the open run, so Stop and the idle watcher need no id.
- `act_turn` still has the same awaits as before. The `act` closure now measures act and settle times separately.
- **Not tested:** the bridge path needs an `AppHandle`; it is covered only by type-checking and by the unit tests of the recorder.

### WP6: CLI (done) `21514f2`

- `rune-studio-cli runs [--limit N]` and `run <id>`; pure formatting functions with exact-output tests (table, durations, step line, whole run) and parsing tests; `HELP` updated (the existing help-text test passes). A missing database prints a plain message and exits 0. I ran the real binary once against a database I seeded by hand with Python, and with no `APPDATA` (exit 1; see NOT CERTAIN): the table and report printed as the tests expect. That is not a real run.
- `docs/AGENT-GUIDE.md` untouched.

### WP7: retention (done) `c8317b2`

- `archiveKeepDays` (`u32`, default 0, camelCase in `settings.json` and `src/types/protocol.ts`; no UI). At startup, only if > 0, a background thread calls `archive::purge_expired`, which removes `Documents/Rune Studio/sessions/<date>/` folders older than the cutoff (a folder exactly `keep_days` old stays), skips links and everything that is not a `YYYY-MM-DD` folder directly below `sessions/`, logs at `info`, and returns the removed session folders' paths so `Recorder::purge_sessions` deletes the matching runs, steps and events.
- `expired_dates(names, today, keep_days)` as specified. Tests: 0 keeps all; the boundary day; month, year and leap-day boundaries; non-date names (including `2026-02-30`); and a temp-directory test proving only expired date folders below `sessions/` go (sibling `screenshots/`, `voice/`, `not-a-date/`, a loose file all survive). The per-screenshot `fsync` is untouched.

### WP8: docs (done) `c5871ea`

- ARCHITECTURE: new "The trace store" section; the record section (retention, `trace.sqlite3`); known gaps 4 and 7. README: data location, the two CLI commands, the setting, the privacy note, the module list. ROADMAP: R1 status table (built / not built / unverified). The pre-commit formatter re-aligned table whitespace in the Markdown files.

## 5. NOT CERTAIN

- `rusqlite` `bundled` on macOS (release workflow builds macOS): **not checked**. Windows: cross-compiled only. The `Rust` workflow on both OSes for this branch: **not run** (see WP1).
- The CLI finds the database through `APPDATA` (`settings::path_without_app`), which only exists on Windows. On Linux and macOS `rune-studio-cli runs` prints "Cannot tell where Rune Studio keeps its data" and exits 1, even though the app writes the file to its own app data folder there. I did not change `path_without_app`.
- Whether a read-only connection works while the app's writer holds a WAL on Windows (tested on Linux only; the test passes there).
- Per-phase timings and the database size on a real run: **not measured**.
- Whether `fsync` on the screenshot path is a real cost: the timings now exist to answer it, but nobody has looked.

## 6. Noticed, not changed

- `prompt_text` is the whole user part of the model request, which includes the **control list** (names and the text a field holds). The privacy note in the README says prompts are stored; if field contents matter, that text is where they land. `elements_hash` alone would not have been enough to compare lists later, so I kept the prompt as the handoff says.
- `args_json` for `type_text` holds the typed text (capped at 2,000 characters).
- A run's `plan_json` for a preset plan comes from `begin_run`; for a made plan from `set_plan`. If the queue is full both can be dropped; the row then has no plan.
- Step numbers in the bridge are the lease's counter, which increases on every tool call, so observe-only calls leave gaps in `steps.step`.
- `Msg` batches are applied in order, but nothing orders a late `end_run` after a `begin_run` of the next run beyond "the first end wins".
- Existing databases from before are not a concern (new file); the schema has no tables for the verifier or reflection yet (version 2 later).

## 7. Questions for the maintainer

1. May I open a PR so the `Rust` workflow can confirm the dependency on Windows and Ubuntu, or do you want `chore/remove-rust-workflow` merged first (then there is no CI to confirm it)?
2. Should the CLI also look in the platform data folder on Linux and macOS, so `runs` works there?
3. Do you want `prompt_text` to exclude the control list (smaller, less sensitive, but then the rows cannot show what the model saw)?