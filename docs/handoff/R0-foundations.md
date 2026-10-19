# Handoff: Milestone R0, Foundations

For an AI coding agent. Read this whole file before touching code. The plan it comes from is `docs/ROADMAP.md` (milestone R0); how the code works today is `docs/ARCHITECTURE.md`. When this file and the code disagree about a fact, the code wins: record the difference in your report.

## 1. Goal

Make the existing agent reliable enough to build the flight recorder, verifier and reflection mode on top of it. R0 adds **no new user-facing features**. It fixes the foundations the audit found, adds CI that runs the Rust tests, and prepares a live wire-format check against rune-infer.

When R0 is done:
1. A stopped run can never be revived by starting the next one (per-run cancellation scope).
2. "Is a task running?" is answered in one place, and there is an `AgentState::Reflecting` that counts as **not** running.
3. A panic inside a run still ends the task cleanly (state reset, click-through restored, session folder closed).
4. A transient model error no longer kills a task.
5. Two outside-agent actions can never drive the mouse at once.
6. Stop cannot panic because of a poisoned lock.
7. `rune-studio` has single-instance behaviour and detects elevated foreground windows.
8. `model/decision.rs` accepts both documented response shapes, and a script exists that captures what a live rune-infer really accepts and returns.
9. CI runs `cargo test`.

## 2. Ground rules

- **Work only on what this file lists.** No drive-by refactors, renames, formatting sweeps or dependency bumps. If you find another bug, list it in the report, do not fix it.
- **Do not change which audio part or thinking parameter Studio sends** (`audio_url`, `chat_template_kwargs.enable_thinking` in `model/client.rs` and `voice/direct.rs`). The maintainer has not decided yet (see `docs/ROADMAP.md`, rune-infer expectation 1). WP8 only adds tolerance and an audit script.
- **Do not change safety rules** (`safety/rules.rs`) or the confirm timeout (120 s = No).
- **No new dependency** except those named in a work package. If you think you need another, stop and write it in the report under "Questions".
- Anything you cannot confirm from code, docs you actually opened, or a command you actually ran is **NOT CERTAIN**: say so in code comments only if it matters, and always in the report. Never guess a crate API or a Win32 signature; open the docs (`cargo doc`, docs.rs via the Context7 tool if you have it) or read the crate source in `~/.cargo/registry`.
- Branch from the current branch head: `git switch -c feat/r0-foundations`. Conventional commits, one commit per work package (`feat:`, `fix:`, `ci:`, `test:`, `docs:`). Do not force-push. Do not merge. Do not open a PR unless the maintainer asks.
- Toolchain: Rust `1.98.1` or newer (`rust-version` in `src-tauri/Cargo.toml`), Bun `1.4.3`. The real app and `system_tests` need **Windows 11**. Everything marked "Windows only" below must be tested there; if you do not have a Windows desktop, implement it, mark it "not run on Windows" in the report and do not claim it works.
- Keep the code style of the surrounding file (comment density, naming, long lines are accepted in this repo; do not run `cargo fmt` over existing files).

## 3. Start here (WP0): baseline

Run and record the outputs in the report before changing anything:

```bash
cd src-tauri
cargo test --lib                          # note: passed / failed / ignored counts
cargo test --lib --features agent-bridge  # same
cargo clippy --all-targets --features agent-bridge 2>&1 | tail -3   # warning count only
cd .. && bun install --frozen-lockfile && bun run build && bun run test:unit
```

If a test already fails, write down which one. Do not fix it unless a work package says so. These counts are the baseline the verifier compares against.

## 4. Work packages

Do them in this order; later ones depend on earlier ones. Each has acceptance criteria that the verifier will check literally.

### WP1: CI that runs the Rust tests

Files: new `.github/workflows/rust.yml`.

- Triggers: `pull_request` and `push` to `develop` and `main`.
- Jobs on `windows-latest` (the real target) and `ubuntu-latest` (install the same apt packages as `deploy.yml`'s "Install OS Dependencies" step).
- Steps: checkout (same action version as other workflows), Rust toolchain (same action as `deploy.yml`), cache, then `cargo test --lib` and `cargo test --lib --features agent-bridge` in `src-tauri`.
- `cargo clippy` and `cargo fmt --check`: run them, but with `continue-on-error: true` **unless** the baseline from WP0 shows zero warnings / a clean format, in which case make them blocking. Write which you chose and the counts in the report.
- Do not touch `integrate.yml`, `release.yml`, `deploy.yml`.

Acceptance: the workflow file is valid YAML (`actionlint` if available), the commands in it work locally, `cargo test --lib` is a blocking step.

### WP2: per-run cancellation scope (the core change)

Problem (verified in code): one shared `exec::CancelToken` in `AppState.cancel`; `agent::start_planned` (`agent/mod.rs:58`), `bridge/mcp.rs::begin` (`:292`) and `debug.rs` call `.reset()` on it, so a run that is still inside a non-cancellable section is un-cancelled when the next run starts. `commands::stop` (`commands.rs:92`) cancels the one flag.

Design (follow it unless you find a concrete problem, then explain in the report):
- In `exec/mod.rs` add `RunScope { id: u64, token: CancelToken }` (`Clone`), ids from a process-wide `AtomicU64`.
- `AppState` replaces `cancel: CancelToken` with `run: Mutex<RunScope>` (use the lock helper from WP7 once it exists; until then the existing `.expect` style is fine, WP7 sweeps it) plus a method `AppState::begin_run(&self) -> RunScope` that **cancels the previous scope's token, then installs a fresh scope** and returns it. `AppState::stop_current(&self)` cancels the current scope's token. There must be **no `reset()`** on a token any more (delete `CancelToken::reset`).
- Everything that used `state.cancel.clone()` gets the scope's token: `agent::run` (`agent/mod.rs:184`; `start_planned` creates the scope and passes it to `run`), `bridge/mcp.rs` (`begin` creates the scope when a lease starts; `act` at `:391` uses the current scope's token), `debug.rs:32` (its own scope per command).
- `commands::stop` calls `stop_current()`; it must still cancel even when the state is `Idle` (typing from debug commands relies on it) and still revoke the lease and close the session as it does now.
- Keep `CancelToken::{cancel, is_cancelled, check, sleep}` as they are.

Tests (in the module where the type lives, plain `#[test]`/`#[tokio::test]`, no Tauri window needed; factor `begin_run`/`stop_current` so they can be tested on a small struct, not only on `AppState`):
1. Starting run B cancels run A's token and A stays cancelled; B's token is not cancelled.
2. `stop_current` cancels only the current scope.
3. Scope ids strictly increase.

Acceptance: `grep -rn "\.reset()" src-tauri/src` shows no cancel-token resets; `grep -rn "state.cancel\|\.cancel\b" src-tauri/src` shows no use of the old field; tests above pass; the existing tests still pass.

### WP3: one "is a task running" answer, and `Reflecting`

Duplicates to replace (verified): `commands.rs:133`, `:163`, `:192`, `voice/mod.rs:44` (`running`), `bridge/mcp.rs:282`. Leave `overlay.rs::has_control` and `voice/direct.rs::describe` alone: they mean something different.

- `events.rs`: add `AgentState::Reflecting` (serde `snake_case` gives `"reflecting"`), and `impl AgentState { pub fn is_running(self) -> bool }` = `Planning | AwaitingConfirm | Acting | Observing`. `Reflecting` is **not** running. Replace the five matches.
- Frontend: add `'reflecting'` to `AgentState` in `src/types/protocol.ts`, a label in `STATE_LABELS` (`src/composables/useAgent.ts`), and make `GlyphOrb.vue` treat it like `idle` (no new animation yet; R5 designs it). `RUNNING` in `useAgent.ts` must not include it.
- Nothing produces `Reflecting` yet. That is intended.

Tests: `is_running` truth table for all 10 variants.
Acceptance: the five old `matches!` blocks are gone; `bun run build` (vue-tsc) passes; `cargo test --lib` passes.

### WP4: a panic in a run still ends the task

Problem: `agent::start_planned` spawns `run(...)` and drops the handle; a panic leaves the state at `Planning`/`Acting`, the pill click-through, and the session folder open.

- In `agent/mod.rs` spawn the run as an inner task and await its `JoinHandle` from an outer supervising task; if the join result `is_panic()`, convert it to `Outcome::Failed("Something went wrong inside Rune: the task was stopped")`, log the panic message at error level, then run the same end path as a normal finish (`end_session`, `finish`). Also treat a cancelled join (task aborted) as `Outcome::Stopped`.
- Extract the supervising logic into a function that takes a future and returns `Outcome`, so it can be unit tested without an `AppHandle`.

Tests: a future that panics yields `Outcome::Failed(..)`; one that returns normally passes its outcome through.
Acceptance: tests pass; reading `start_planned` shows `end_session` and `finish` run on the panic path too.

### WP5: retry transient model errors

- In `model/client.rs` add `ChatClient::call_retrying(&self, request, on_thinking)` that wraps `call`. Retry only: `ModelError::Request` (connect/timeout/transport) and `ModelError::Status` with status 429, 502, 503 or 504. Never retry `Reply`, `NoToolCall`, or other 4xx/5xx. At most 3 attempts in total; wait 500 ms, then 1500 ms, each with up to 20% random jitter (no new crate: derive jitter from `SystemTime` nanos or use `getrandom`, which is already an optional dependency, only if you can use it without enabling the feature everywhere; otherwise use nanos).
- Cancellation: the waits must be plain `tokio::time::sleep` inside the future, because callers already race it against Stop with `unless_stopped` (dropping the future cancels it). Do not add a token parameter.
- Use it for the planning call (`make_plan`) and the decide call in `agent/mod.rs`. **Do not** use it in `voice/direct.rs` (utterance latency) or the decision model (`model/decision.rs` has its own cooldown).
- Log each retry at `warn` with attempt number and reason.

Tests: `is_retryable` for every `ModelError` variant and for statuses 400, 401, 404, 429, 500, 502, 503, 504; `backoff(attempt)` stays inside its jitter bounds; plus **one** test with a throwaway HTTP server (std `TcpListener` on `127.0.0.1:0` in a thread) that answers 503 once and then a valid chat completion, asserting `call_retrying` returns the tool call. If you need `tokio`'s `io-util` feature for it, add it as a **dev-dependency feature only** and say so in the report.
Acceptance: tests pass; `grep -n "call_retrying" src-tauri/src` shows exactly the two intended call sites plus its definition and tests.

### WP6: serialise outside-agent actions

Problem: `bridge/mcp.rs::act` has no mutual exclusion; two parallel `tools/call`s run two `spawn_blocking` input sequences, and `gate::ask_user` overwrites the single confirm slot.

- Add one `tokio::sync::Mutex<()>` (field on `AppState`, or inside the lease holder; your choice, explain it) and hold its guard across the **whole** `act`: gate check, announce, execute and settle. Calls **wait their turn** (do not refuse; do not add a new error code, the guide's error list is tested). While waiting, a Stop must still release the waiter: race the lock acquisition against the scope token (reuse the 25 ms polling pattern from `agent::unless_stopped` or a small helper) and return the existing `LeaseError::StoppedByUser` reply.
- Do not hold the lock in `rune_observe`, `rune_elements`, `rune_status`, `rune_plan`, `rune_done` (read-only or bookkeeping).

Tests: a helper that takes the lock and a cancel token: (1) two acquisitions are strictly sequential, (2) a waiter returns `Cancelled` when the token is cancelled while the first holder still holds the lock.
Acceptance: tests pass; reading `act` shows the guard is held until the reply is built; the bridge guide test (`the_guide_documents_every_error_code`) still passes unchanged.

### WP7: locks cannot take Stop down

Problem: 45 sites do `.lock().expect("… poisoned")`; one panic while a lock is held makes every later call, including `commands::stop`, panic.

- Add a tiny helper (new file `src-tauri/src/sync.rs`, or in `error.rs`) that locks and, on `PoisonError`, logs once at error level and returns `into_inner()`. Do **not** add `parking_lot`.
- Replace every `.lock().expect("... poisoned")` in `src-tauri/src` (list them with `grep -rn "lock poisoned" src-tauri/src`).
- `commands::stop` must have no remaining panic path.

Acceptance: `grep -rn "poisoned" src-tauri/src` returns only the helper's own message; a unit test poisons a `Mutex` (panic in a thread holding it) and shows the helper still returns the data; `cargo test --lib` passes.

### WP8: wire-format tolerance and the audit script (no behaviour change to requests)

(a) `model/decision.rs` must accept **both** documented `/v1/decisions` answer shapes:
- source shape (current): yes/no `{ "decision": bool, "probability": f }`, choice `{ "decision": "x", "confidence": f, "probabilities": {...} }`, score `{ "level", "score", "probabilities" }`;
- spec shape (Postman): `{ "type": "noul"|"choice"|"score", "value": ..., "confidence": f }` where a `noul` value is the string `"yes"`/`"no"` (accept a JSON bool too), a `choice` value is a string, a `score` value is a number.
For the spec shape, treat a yes/no answer as "yes with probability = `confidence`" when `value` is yes (and not-yes otherwise). Whether `confidence` really means P(value) is **NOT CERTAIN**: leave a one-line comment saying so. Keep `confident_yes` and `choice` signatures. Log once (info) which shape was seen.
Tests: use these fixtures verbatim: spec `noul` `{"type":"noul","value":"yes","confidence":0.89}`, spec choice `{"type":"choice","value":"canary_rollout","confidence":0.94}`, source `{"decision":true,"probability":0.96}`, source choice `{"decision":"high","confidence":0.88,"probabilities":{"low":0.02,"medium":0.1,"high":0.88}}`; thresholds respected; missing answer is "not sure", never "no". The existing tests in the file must still pass.

(b) New `scripts/wire-audit.mjs` (Node/Bun, no dependencies). It reads `RUNE_BASE_URL` (default `http://127.0.0.1:8080`), `RUNE_API_KEY`, `RUNE_MODEL` (omni, default `gemma-4-12b-it`), `RUNE_DECISION_MODEL` (default `liquid-d1-3b`) and prints/writes a Markdown table to `docs/wire-audit-<YYYY-MM-DD>.md` with, for each request: what was sent (the part/parameter name), HTTP status, whether the server accepted it, and the raw response (truncated to 1,500 characters, base64 stripped). Requests:
1. `GET /health`, `GET /readyz`, `GET /v1/models`, `GET /v1/models/{model}`.
2. Chat with a generated PNG, one tool, `tool_choice: "required"` and again with `"auto"`.
3. Thinking switch, three variants against the same prompt: `chat_template_kwargs.enable_thinking: false`, `thinking: {"type":"disabled"}`, `reasoning_effort: "none"`; and the "on" counterparts. Record whether `reasoning_content` appears.
4. Audio, two variants with a generated one-second 16 kHz mono WAV: Studio's `audio_url` part with a data URI, and the spec's `input_audio {data, format:"wav"}`.
5. `POST /v1/decisions` with a generated solid-red PNG and the question "Is this image mostly red?" (type `noul`), and the same text without the image; record the **raw** answer object so the real shape is visible.
6. `POST /v1/images/grounding` with a generated image containing one clearly separate square (draw it in the PNG) and query "square": record `point`, `point_pixels`, `box_2d`, `box_normalized` so units and axis order can be read.
PNG/WAV generation must be self-contained (use `zlib` and a CRC32 you write). The script must never print the API key. If no server is reachable it must exit 1 with a clear message.
**If you do not have a live rune-infer, do not fabricate output.** Commit the script, run `node scripts/wire-audit.mjs` against a mock only to prove it executes (say so), and write "NOT RUN against a real server" in the report.

Acceptance: decision tests pass; `node --check scripts/wire-audit.mjs` passes; the report states honestly whether the audit ran live.

### WP9: single instance (desktop only)

- Add the official Tauri 2 single-instance plugin. **NOT CERTAIN**: the exact crate name/version for Tauri 2.12 (`tauri-plugin-single-instance` is the expected name). Confirm it on docs.rs / the plugin's repository before adding; use a version compatible with `tauri = "2.12"`; add it only under `[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]` like the updater.
- Register it first in the builder (`lib.rs::run`). The callback for a second launch calls `window::summon(app)` so the existing pill opens.
- `rune-studio-cli`'s `ensure_app` spawns the app only when the local door is closed; confirm by reading `bridge/cli.rs` that a second launch exiting immediately does not break it, and state the conclusion in the report.

Acceptance: builds; on Windows, launching `rune-studio.exe` twice leaves one process and the second launch shows/focuses the pill (manual check; say if not run).

### WP10: detect elevated foreground windows (Windows)

- In `safety/foreground.rs` add `pub fn is_elevated() -> Option<bool>` for the foreground window's process (`None` when it cannot be read; non-Windows returns `None`). Use the process token elevation query (`OpenProcessToken` + `GetTokenInformation` with `TokenElevation`). **NOT CERTAIN**: the exact `windows` 0.62 module paths and Cargo features needed (likely `Win32_Security`); read the crate source or docs, then add only the missing features to `Cargo.toml`. Close every handle you open.
- Also add `fn app_is_elevated() -> bool` the same way for our own process, computed once. Only warn when the foreground is elevated and we are not.
- Agent loop (`agent/mod.rs`): when the foreground window is elevated, add one line to the prompt warning (the existing `warning` string: "The front window runs as administrator: clicks and typing are ignored. Use ask_user or finish with done and success=false.") and count consecutive looks with an elevated foreground; after **2** in a row end the run with `Outcome::Failed("I can't control this window because it runs as administrator.")`. Put the streak logic in a small pure struct so it can be tested.
- Outside agents: in `bridge/mcp.rs::act` and `observe` replies add the same one-line note when the foreground is elevated (text only; do not change any error code).

Tests: the pure streak struct (0, 1, 2 elevated looks; reset on a normal look). Manual on Windows: open an elevated Notepad (Run as administrator), run a task, expect the warning then the clear failure.
Acceptance: tests pass; manual result recorded or "not run on Windows".

### WP11: docs

Update, minimally and truthfully:
- `docs/ARCHITECTURE.md`: section 6 "Concurrency and cancellation, as built" (RunScope, supervisor, retry, serialised bridge actions, lock helper, `Reflecting`); section 12 (actions are serialised; elevated note); section 14/15 (remove the gaps you actually fixed: 1, 2 in part, 3, 4 in part; keep the rest); section 13 (CI now runs Rust tests).
- `docs/ROADMAP.md`: mark R0 sub-items done / not done with one line each; keep NOT CERTAIN markers that are still true.
- `README.md`: the "CI does not currently run `cargo test`" sentence and the CI table row.
Do not edit `docs/AGENT-GUIDE.md` unless you changed a tool or error code (you should not have).

## 5. Definition of done

All of the following, evidenced in the report:
1. WP1-WP11 each "done", "partly" (say what is missing) or "not done" (say why).
2. `cargo test --lib` and `cargo test --lib --features agent-bridge` pass; test counts before and after.
3. `bun run build` and `bun run test:unit` pass.
4. `grep -rn "lock poisoned" src-tauri/src` and `grep -rn "\.reset()" src-tauri/src` show what WP2 and WP7 require.
5. No change outside the files these packages name, except tests and docs; list every file touched (`git diff --stat` against the starting commit).
6. Nothing claimed that you did not run.

## 6. Report (required)

Write `docs/handoff/R0-report.md` with:
1. Environment: OS, `rustc --version`, `bun --version`, whether a Windows desktop and a live rune-infer were available.
2. Baseline (WP0) and final numbers: tests passed/failed/ignored for both feature sets, clippy warning count, bun build/test result.
3. Per work package: status, commit hash, files touched, the exact commands you ran and their result (paste the last lines), deviations from this design and why.
4. "NOT CERTAIN" list: every item you could not confirm (plugin name/version, Win32 features, `confidence` meaning, wire-audit not run live, etc.).
5. "Found but not fixed": other bugs or risks you saw.
6. "Questions for the maintainer".
Then tell the maintainer you are done. **The verifier will review your work against section 4's acceptance criteria and will not modify code.**

## 7. What the verifier will do

Re-run the commands in section 5, read the diff against each work package's design and acceptance lines, grep for the forbidden leftovers, check that no request-format change slipped into `model/client.rs` or `voice/direct.rs`, check the tests assert what they claim (not just that they exist), and compare your report with what the code actually does. Anything unproven is reported as unproven.
