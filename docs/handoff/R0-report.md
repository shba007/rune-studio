# R0 report: Foundations

Branch `feat/r0-foundations`, branched from `188b422` (the head of `develop`). Eleven commits, one per work package. Nothing pushed, no PR opened.

## 1. Environment

|                   |                                                                                                                                                        |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| OS                | Linux 6.18 (cloud container). **No Windows desktop.**                                                                                                  |
| `rustc`           | `1.98.1` (installed with rustup; the container shipped 1.97.0, which `rust-version = "1.98.1"` refuses). Also added the clippy and rustfmt components. |
| `bun`             | 1.4.2 (the handoff says 1.4.3)                                                                                                                         |
| `node`            | v22.22.0                                                                                                                                               |
| Live rune-infer   | **Not available.**                                                                                                                                     |
| Extra OS packages | The apt list from `deploy.yml`, plus `gcc-mingw-w64-x86-64` to type-check the Windows target.                                                          |

## 2. Baseline (WP0) and final numbers

**At the starting commit, `cargo test --lib` does not compile on Linux** (both feature sets). `src/system_tests.rs` is `#[cfg(test)]` and uses the `windows` crate, which is only a dependency on Windows. The output is five `E0433: cannot find module or crate windows` errors. The numbers below are therefore _with_ the one-line gate added in WP1 (`#[cfg(all(test, windows))] mod system_tests;`). Without it the WP1 ubuntu job could never pass.

|                                                                         | Baseline (with the WP1 gate)    | Final                                                                 |
| ----------------------------------------------------------------------- | ------------------------------- | --------------------------------------------------------------------- |
| `cargo test --lib`                                                      | 95 passed, 0 failed, 2 ignored  | 113 passed, 0 failed, 2 ignored                                       |
| `cargo test --lib --features agent-bridge`                              | 122 passed, 0 failed, 2 ignored | 140 passed, 0 failed, 2 ignored                                       |
| `cargo clippy --all-targets --features agent-bridge`                    | 18 `warning` lines              | 18 `warning` lines (same as before; none new)                         |
| `cargo fmt --check`                                                     | 403 diffs                       | 428 diffs (the repo style uses long lines; I did not run `cargo fmt`) |
| `bun install --frozen-lockfile` / `bun run build` / `bun run test:unit` | ok / ok / 6 passed              | ok / ok / 6 passed                                                    |

`vue-tsc` could not be run on its own here (`typescript/lib/tsc` is missing from `node_modules`; `bun run build` is `vp build`, which passed). The only TypeScript edits are one added union member and two added `Record<AgentState, …>` entries. I did not confirm with a type checker that these are exhaustive.

## 3. Work packages

Hashes are in the branch log. Commands were run from `src-tauri` with `RUSTUP_TOOLCHAIN=1.98.1` unless stated.

### WP1: CI (done) `c51286e`

- Files: `.github/workflows/rust.yml` (new), `src-tauri/src/lib.rs` (the `system_tests` gate).
- Triggers, matrix (`windows-latest`, `ubuntu-latest`), apt packages, `actions/checkout@v7` and `dtolnay/rust-toolchain@stable` follow the existing workflows. `cargo test --lib` and `cargo test --lib --features agent-bridge` are blocking.
- **Clippy and fmt are `continue-on-error: true`**: baseline had 18 clippy warning lines and 403 fmt diffs, not clean.
- Cache is `Swatinem/rust-cache@v2`. It is a new third-party action not used elsewhere in the repo, and I did not check its current version. **NOT CERTAIN.**
- Checked: valid YAML (`python3 yaml.safe_load`); `actionlint` is not installed. The workflow has never run on GitHub. `dtolnay/rust-toolchain@stable` must resolve to >= 1.98.1 when it runs.
- Deviation: the `lib.rs` change is outside the files WP1 names. Reason above.

### WP2: per-run cancellation scope (done) `65dec10`

- `exec::RunScope { id, token }` with ids from a process-wide `AtomicU64`. `begin_run` cancels the previous token, then installs a fresh scope. `stop_current` cancels the current one. `CancelToken::reset` is deleted.
- `AppState.cancel` became `run: Mutex<RunScope>`, with `AppState::{begin_run, stop_current, current_scope}` delegating to free functions in `exec` so they are testable on a bare `Mutex<RunScope>`.
- `agent::start_planned`, `bridge::mcp::begin` and each `debug.rs` command start a scope. `act` uses the current scope's token. `commands::stop` calls `stop_current()` from any state and still revokes the lease.
- Tests: `starting_run_b_cancels_run_a_for_good`, `stop_cancels_only_the_current_scope`, `scope_ids_strictly_increase`. The old input test used `reset`; it now uses a fresh token for its second assertion (`exec/input.rs`, a needed edit).
- Greps: `grep -rn "\.reset()" src-tauri/src` and `grep -rn "state.cancel" src-tauri/src` return nothing.
- Side effect: a debug command (debug builds only) now cancels a task that is running, because it starts its own scope.

### WP3: `is_running` and `Reflecting` (done) `3d4f954`

- `AgentState::Reflecting` and `AgentState::is_running` replace the five hand-copied matches (`commands.rs` x3, `voice/mod.rs`, `bridge/mcp.rs`). `overlay::has_control` and `direct::describe` are untouched.
- Frontend: `'reflecting'` in the union, `STATE_LABELS` entry "Reflecting", `GlyphOrb` maps it to `idle`. `RUNNING` in `useAgent.ts` is unchanged.
- Test: truth table for all 10 variants, plus `"reflecting"` serialisation.

### WP4: panic supervisor (done) `842b992`

- `supervised(future) -> Outcome` spawns the run as its own task: a panic becomes `Outcome::Failed("Something went wrong inside Rune: the task was stopped")` (message logged at error), an abort becomes `Outcome::Stopped`. `settle(JoinHandle)` is split out so the abort path is testable.
- **Difference from the handoff:** there is no `end_session` and no session folder in this codebase. The end path is `finish`, which resets the state and click-through (via `set_agent_state`) and resumes listening. Reading `start_planned` shows `finish` runs after the supervised future in every case.
- Tests: panic, pass-through of all three outcomes, aborted handle.

### WP5: retry (done) `7e948f7`

- `is_retryable`, `backoff`, `ChatClient::call_retrying` in `model/client.rs`; 3 attempts, waits 500 ms then 1500 ms plus 0-20% jitter from clock nanoseconds (no new crate). `warn` log on each retry. No token parameter.
- Call sites: `make_plan` and the decide call in `agent/mod.rs` only (the grep shows exactly these two plus definition and tests). Not used in `voice/direct.rs` or `decision.rs`.
- `git diff 188b422 -- src-tauri/src/model/client.rs` has no removed lines: no request-format change. `voice/direct.rs` is untouched.
- Tests: `is_retryable` for every variant and statuses 400/401/404/429/500/502/503/504; `backoff` bounds; throwaway `TcpListener` server answering 503 then a valid completion. I added a second server test (a 400 is not retried, one request served) beyond the single test asked for. No `tokio` feature change was needed (std sockets in a thread). The tests use `reqwest::Client::builder().no_proxy()` because the container sets proxy variables.

### WP6: serialised outside-agent actions (done) `6d40588`

- `AppState.act_turn: tokio::sync::Mutex<()>` (compiled only with `agent-bridge`). I put it on `AppState` rather than in the lease holder because the lease lives behind a std mutex and a tokio guard must be held across `.await`.
- `act` takes the turn through `exec::lock_unless_stopped` (polls the scope token every 25 ms; returns `Cancelled` immediately if already cancelled), then calls `act_turn` (the old body). The guard lives until the reply is built. Waiters return `LeaseError::StoppedByUser`. No new error code; `the_guide_documents_every_error_code` passes unchanged.
- Deviation: the lock is taken **before** the lease touch, so step numbers follow the order actions actually run. A caller without a lease waits for the turn before getting `not_begun`.
- Not locked: `rune_observe`, `rune_elements`, `rune_status`, `rune_plan`, `rune_done`.
- Tests: four tasks never overlap; a waiter is released by Stop while the first holder holds the lock; an already-cancelled token never takes a free lock.

### WP7: lock helper (done) `9634c63`

- `src-tauri/src/sync.rs`: `LockExt::locked()` logs once at error level and returns `into_inner()`. All 45 `.lock().expect("… poisoned")` sites use it.
- `grep -rn "poisoned" src-tauri/src` returns only lines inside `sync.rs`. `grep -rn "lock poisoned"` returns nothing.
- Test: a thread panics holding the mutex; `locked()` still returns and can write the data.
- `model/decision.rs::FAILED_AT` uses `.lock().ok()` (no `expect`), so I left it. A poisoned lock there just means "not cooling down".

### WP8: wire-format tolerance and audit script (done, **audit NOT RUN against a real server**) `5c5cfbc`

- (a) `model/decision.rs` reads the source shape and the spec shape. Spec `noul` yes with `confidence` = probability (one-line comment says this is **NOT CERTAIN**). A JSON bool value is also accepted. The shape seen is logged once at info. `confident_yes` and `choice` keep their signatures. Missing, partial or unknown answers are "not sure", never "no" and never a pick. The four fixtures from the handoff are used verbatim. I did not add a `score` accessor (nothing uses it); score answers parse as neither a yes nor a choice.
- (b) `scripts/wire-audit.mjs`: no dependencies, own CRC32 and PNG/WAV generation, all requests from the handoff (discovery, tool_choice required/auto, three thinking spellings on and off, `audio_url` vs `input_audio`, decisions with and without image, grounding on a 320x240 picture with a 60x60 square at (200, 60)). The key is only sent as a header and scrubbed from output; base64 stripped; raw responses cut to 1,500 characters; exit 1 when no server answers. The "on" counterparts I chose are `enable_thinking: true`, `thinking: {"type":"enabled"}`, `reasoning_effort: "high"`; the last two values are my guess (**NOT CERTAIN**). The grounding body `{model, image, query}` is copied from `probe_m15.rs`.
- Verification I actually did: `node --check`; ran it against a small mock server (not committed; the output file was deleted, nothing is in `docs/`) to prove it executes, writes the table and scrubs the key; ran it with nothing listening (exit 1, clear message); decoded the generated PNG with PIL (320x240, square pixels where expected) and the WAV with `wave` (mono, 16 kHz, 16000 frames).
- **No `docs/wire-audit-*.md` exists. The wire audit was NOT RUN against a real server.**
- The repo's pre-commit linter (`vp lint`) forced a few rewrites of the script (template-literal types, `void main()`, fetch options); behaviour re-checked against the mock after.

### WP9: single instance (done; manual check **not run**) `9cef8b3`

- Crate: `tauri-plugin-single-instance`. crates.io lists `3.0.0-alpha.3` as latest; I used `2.5` (resolved to `2.5.4`) for Tauri 2, read its `init` signature in the crate source. Added under the non-mobile target block next to the updater; `Cargo.lock` gains only that crate. Registered first in `lib.rs::run`; the callback calls `window::summon(app)`.
- `rune-studio-cli`'s `ensure_app` (`bridge/cli.rs:376`) only spawns the app when `door_is_open()` is false and then polls the door until it opens or times out; it ignores the child's exit status. A second instance that exits at once therefore does not break it. If the app is still starting and a second spawn happens, the second exits and the CLI keeps waiting for the first instance's door (better than before).
- Builds on Linux (`cargo build --features agent-bridge`). On Windows: "launching twice leaves one process" was **not run**. The plugin's docs say the second instance never reaches `run`; I did not check the Windows named-mutex path myself.

### WP10: elevated foreground (done on paper; **not run on Windows**) `fd36a66`

- `Cargo.toml`: added feature `Win32_Security` to the `windows` dependency (confirmed in `windows-0.62.2` source that `OpenProcessToken` needs it; `GetTokenInformation`, `TOKEN_ELEVATION`, `TokenElevation`, `TOKEN_QUERY` are in `Win32::Security`).
- `safety/foreground.rs`: `is_elevated() -> Option<bool>` (foreground window's process, `OpenProcess` with limited-query rights, then token elevation; every handle closed), private `app_is_elevated()` computed once, and `blocked_by_elevation()` = foreground elevated and we are not. Non-Windows: `None` / `false`.
- Agent loop: adds the exact warning text to the prompt warning and ends with `Outcome::Failed("I can't control this window because it runs as administrator.")` after 2 elevated looks in a row (`stuck::ElevatedWatch`, pure, tested: 0, 1, 2 looks and reset). Outside agents: a text note appended to `rune_observe` and to successful action replies; no error code changed.
- Verification: `cargo check --lib --tests --target x86_64-pc-windows-gnu --features agent-bridge` compiles (so the Win32 calls type-check against the real crate). They have not been run.
- **Risk (NOT CERTAIN):** Windows may refuse `OpenProcessToken` on a process at a higher integrity level when we are not elevated. If so, `is_elevated()` returns `None` for exactly the case we care about, and nothing is flagged. The handoff says to return `None` on read failure, so I did. I log the failure at debug level so the Windows manual test shows which it is. I did not guess "access denied means elevated".
- Manual test (elevated Notepad, expect the warning then the failure): **not run on Windows**.

### WP11: docs (partly) `d800336`

- `docs/ARCHITECTURE.md`: new "Concurrency and cancellation, as built" subsection in section 6, plus edits to the module tree, state machine, elevated-windows bullet, the bridge Stop sentence and risk 5. `README.md`: one paragraph describing the Rust workflow.
- **Differences from the handoff:** `docs/ROADMAP.md` does not exist, so nothing was marked there and I did not invent a roadmap. `ARCHITECTURE.md` has 10 sections, not 15, so the handoff's section 12-15 references do not match; I edited the nearest real places. The README has no sentence "CI does not currently run `cargo test`" and no CI table row, so I added a paragraph instead.
- The pre-commit formatter re-aligned two Markdown tables in `ARCHITECTURE.md` (whitespace only; the diff is larger than my edits).
- `docs/AGENT-GUIDE.md` untouched (no tool or error code changed).

## 4. NOT CERTAIN list

1. Whether `OpenProcessToken` works on an elevated foreground process from a non-elevated Rune Studio (WP10). Nothing here has run on Windows.
2. `Swatinem/rust-cache@v2` as the cache action, and that `dtolnay/rust-toolchain@stable` is >= 1.98.1 at CI time (WP1). The workflow has not run on GitHub.
3. Whether `confidence` in the spec decision shape is the probability of `value` (WP8).
4. Which thinking-switch spelling, audio part and decision shape the real server accepts: the audit script exists but was never run live. Which audio part and thinking parameter Studio sends is unchanged.
5. The "on" values of `thinking` / `reasoning_effort` in the audit script (WP8).
6. Single-instance behaviour on Windows (WP9) and that no TypeScript type check exhaustively covers `'reflecting'` (section 2).
7. The `rune-studio.exe` build for Windows was only type-checked with `cargo check`, never linked or run.

## 5. Found but not fixed

- `cargo test --lib` did not compile on Linux before WP1 (the one change I made, see above). `probe_m15.rs` and `system_tests.rs` are the only Windows-only test code; `probe_m15` already compiles on Linux.
- Baseline `dead_code` warnings: `agent::start`, `uia::MAX_ELEMENTS`, `voice::act`, `voice::intent::normalise` and its constants. Not touched.
- `debug.rs` commands now each begin a run, which cancels any running task (dev builds only).
- In `act`, a waiting call does not refresh the lease's idle timer; it can only expire if a queued wait exceeds the idle limit (300 s) while the holder is silent. Realistically bounded by the 120 s confirm timeout.
- `Reflecting` is new on both sides but nothing produces it. `ControlOverlay.vue` listens to `state_changed` with the `AgentState` type; I did not read it for exhaustive handling of a new state.
- `NoToolCall` / `Reply` model errors are not retried (as specified), so a server that returns a truncated body once still fails the task.
- Hooks: husky ran `vp lint`/`vp fmt` on staged files for every commit; the commits are not signed off by a Windows run.

## 6. Questions for the maintainer

1. Should clippy and `cargo fmt --check` become blocking? That needs one cleanup pass (18 clippy warning lines, 428 fmt diffs).
2. `docs/ROADMAP.md` is missing from the repo. Is it on another branch, or should R0 be written into it?
3. The handoff mentions `end_session`, a session folder and a 15-section architecture doc that this branch does not have. Is the handoff written against a later branch?
4. Run `node scripts/wire-audit.mjs` against your rune-infer and commit the output? That settles `audio_url` vs `input_audio`, the thinking switch and the decision shape.
5. For WP10, if the Windows manual test shows `is_elevated()` returns `None` for an elevated Notepad, do you want "token unreadable" to be treated as elevated for the foreground window, or a different detection (for example comparing integrity levels)?

Done. The verifier can review against section 4 of the handoff.