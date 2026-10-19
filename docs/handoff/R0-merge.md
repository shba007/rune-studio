# Handoff: merge the orchestration branch into `develop` (R0 follow-up)

For an AI coding agent. Read all of it first. This is a **merge and documentation task**: do not write new features, do not refactor, do not "improve" anything the merge brings in.

## 1. Situation (verified facts)

- `develop` is at `c0350b3`: it contains milestone R0 (per-run cancellation scope, panic supervisor, retries, serialised bridge actions, lock helper, decision-shape tolerance, single-instance, elevated-window detection, `rust.yml` CI). Its docs are the **old, stale** README and `docs/ARCHITECTURE.md`.
- `claude/intelligent-brahmagupta-u7c6o7` (call it **BRANCH**, head `543e6f4`) is based on the older commit `188b422`, so it does **not** contain R0. It adds: the rewritten README and `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/handoff/*`, the per-session archive folders (`src-tauri/src/archive.rs` + callers), and `docs/AGENT-GUIDE.md` CLI-name fixes.
- A trial merge was done by the verifier in a throwaway worktree: **4 files conflict** (`README.md`, `docs/ARCHITECTURE.md`, `src-tauri/src/agent/mod.rs`, `src-tauri/src/bridge/mcp.rs`). With the resolutions in section 3 the result compiled and passed: `cargo test --lib` 119 passed, `cargo test --lib --features agent-bridge` 146 passed, 0 failed, 2 ignored (Rust 1.98.1, Linux). Your numbers must match, or you must explain the difference.

## 2. Setup

```bash
git fetch origin
git switch -c merge/orchestration-into-develop origin/develop
git merge --no-ff --no-commit origin/claude/intelligent-brahmagupta-u7c6o7
```

Expected: conflicts in exactly those 4 files. If git reports different conflicts, **stop** and write that in the report instead of improvising. Toolchain: Rust 1.98.1 or newer, Bun 1.4.x. Linux needs the apt packages listed in `.github/workflows/rust.yml`. No force-push, no PR, no pushing to `develop` or BRANCH: push only `merge/orchestration-into-develop`.

## 3. Resolutions (apply exactly)

### 3.1 `src-tauri/src/agent/mod.rs` hunk 1 (inside the task spawned by `start_planned`)

Result must read:

```rust
    tauri::async_runtime::spawn(async move {
        // The run is its own task, so a panic inside it cannot skip `finish`: the state would stay
        // at Planning/Acting, the pill would stay click-through and the voice session would hang.
        let work_app = app.clone();
        let outcome = supervised(async move { run(&work_app, &task, plan, scope.token).await }).await;
        end_session(&session, &outcome);
        finish(&app, outcome);
    });
```

Delete the line `let outcome = run(&app, &task, plan).await;` (from BRANCH): `run` now takes the scope token.

### 3.2 `agent/mod.rs` hunk 2 (functions after `start_planned`)

Keep **both** blocks: develop's `supervised` and `settle` (unchanged), then BRANCH's `end_session` (unchanged). Each side's closing `}` is shared in the conflict, so the result needs one extra `}` between them:

```rust
async fn settle(run: tokio::task::JoinHandle<Outcome>) -> Outcome {
    match run.await {
        /* develop's body, unchanged */
    }
}

/// Closes the task's session folder with how the run ended. ...
fn end_session(session: &str, outcome: &Outcome) {
    /* BRANCH's body, unchanged */
}
```

The order in 3.1 matters: the session closes after the supervisor returns, so a panic is recorded as `failed`.

### 3.3 `src-tauri/src/bridge/mcp.rs` hunk 1 (in `begin`)

```rust
    // A fresh cancellation scope for this lease; it also cancels whatever the last run left behind.
    state.begin_run();
    // This agent's turn at the wheel is one session folder (closed by rune_done, Stop or the idle limit).
    if let Err(err) = crate::archive::begin_session(&task, &agent) {
        log::error!("could not open the session folder: {err}");
    }
```

Delete BRANCH's `state.cancel.reset();` (the field no longer exists).

### 3.4 `bridge/mcp.rs` hunk 2 (in `done`)

```rust
    state.lease.locked().end();
    crate::archive::end_session(None, if success { "succeeded" } else { "reported_failure" }, &summary);
```

Delete BRANCH's `state.lease.lock().expect("lease lock poisoned").end();`.

### 3.5 Files that merge by themselves: check, do not edit

`commands.rs` (Stop must call `stop_current()` and then `crate::archive::end_session(None, "stopped", "")`), `voice/mod.rs` (`apply(..., Some(saved.as_path()))`, `adopt` after a task starts, and `.is_running()`), `bridge/server.rs` (`end_session(None, "lease_expired", "")` plus `.locked()`), `archive.rs`, `docs/AGENT-GUIDE.md`. Read each merged file around those lines and confirm in the report.

### 3.6 After the code is resolved, this must hold

`grep -rn "lock poisoned\|\.reset()\|state\.cancel" src-tauri/src` prints nothing; `grep -rn "<<<<<<<\|>>>>>>>" .` prints nothing (except none in code, docs or scripts).

## 4. Documentation resolutions

### 4.1 `README.md`

Take **BRANCH's** whole file (`git checkout --theirs README.md` in this merge direction, then check it is the rewritten one: it starts with the logo block and has "How it works"). Then edit:

1. In "Tests", replace the line `CI does **not** currently run \`cargo test\`, \`clippy\` or \`fmt\` (see below).`with: CI runs`cargo test --lib`, with and without `--features agent-bridge`, on Windows and Ubuntu (`.github/workflows/rust.yml`); clippy and `cargo fmt --check` run too but do not fail the build yet.
2. In "Build and release pipeline", add a `rust.yml` row (triggers: pull requests, push to `develop` and `main`; runs the tests above on `windows-latest` and `ubuntu-latest`, clippy and fmt advisory). On the `integrate.yml` row replace "**No tests and no Rust checks.**" with "No tests; the Rust workflow covers Rust".
3. Under "Using the app" add one sentence: launching Rune Studio a second time shows the existing pill instead of starting another copy.

### 4.2 `docs/ARCHITECTURE.md`

Take **BRANCH's** whole file, then port develop's facts. Use `git show origin/develop:docs/ARCHITECTURE.md` and `git diff 188b422 origin/develop -- docs/ARCHITECTURE.md` as the source of wording; do not copy its table whitespace changes. Edit these places (find them by text, line numbers will differ):

| Find (BRANCH text)                                                                                    | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Section 1 table row "Not built" listing "elevated-window detection"                                   | Remove that item.                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Section 4: "a transport or HTTP error fails the run at once (no retry)"                               | Transient errors (transport, HTTP 429, 502, 503, 504) are retried up to 3 times for the planning and decide calls (500 ms then 1500 ms plus up to 20% jitter); other errors fail the run.                                                                                                                                                                                                                                                                         |
| Section 6 module tree                                                                                 | Add `sync.rs` (lock helper: a poisoned Mutex is logged and used, never a panic).                                                                                                                                                                                                                                                                                                                                                                                  |
| Section 6 "Concurrency and cancellation, as built": the bullet "Stop sets one shared `AtomicBool`..." | Replace with `exec::RunScope { id, token }`: `AppState.run` holds the current one; `begin_run` cancels the previous scope and installs a fresh one; `stop_current` cancels the current one from any state; no token is ever un-cancelled (`CancelToken::reset` is gone). Async waits still poll it every 25 ms, sync code every 10 ms.                                                                                                                            |
| Same section, bullet "Locks: ... Lock errors are `expect`ed"                                          | `sync::LockExt::locked()` logs once and uses a poisoned lock; no `expect` remains.                                                                                                                                                                                                                                                                                                                                                                                |
| Same section                                                                                          | Add bullets: the run is spawned under `supervised`, so a panic becomes `Outcome::Failed("Something went wrong inside Rune: the task was stopped")` and `finish` (and the session close) still run; outside-agent actions take turns through `AppState.act_turn` and a waiting call is released by Stop; single-instance plugin registered first, a second launch shows the pill. Keep "Not cancellable once started" and "Voice is a cpal thread..." as they are. |
| Section 6 "State machine" bullet                                                                      | Add `Reflecting` (nothing produces it yet; it does not count as running) and `AgentState::is_running` as the one "is a task running" answer; drop "Several places repeat the test".                                                                                                                                                                                                                                                                               |
| Section 6 "Elevated windows" bullet                                                                   | `safety::foreground::is_elevated` reads the front window's token; when it is elevated and we are not, the model is warned and the run ends after two such looks in a row; outside agents get a text note on `rune_observe` and action replies. **Not confirmed on Windows**: Windows may refuse the token of a higher-level process, in which case nothing is detected.                                                                                           |
| Section 12 "The user wins": "Stop cancels the shared token"                                           | "...the current run's token".                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Section 12 "Not built": "concurrency control between simultaneous tool calls"                         | Remove that phrase and add "Actions take turns" under the lease/door bullets.                                                                                                                                                                                                                                                                                                                                                                                     |
| Section 13 CI bullet                                                                                  | Add `rust.yml` (tests on Windows and Ubuntu; clippy/fmt advisory) and keep "`integrate.yml` runs no tests".                                                                                                                                                                                                                                                                                                                                                       |
| Section 14 status table, row M6 "Not built: single-instance, ..., elevated-window detection"          | Remove those two (built, untested on Windows).                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Section 15 Known gaps                                                                                 | Remove or shorten: gap 1 (shared flag), the "panic not caught" half of gap 2, gap 3 (no retry), the "simultaneous tool calls" half of gap 4, gap 10 (CI). Keep the rest.                                                                                                                                                                                                                                                                                          |
| Section 16 risk 5 "Elevated windows: not detected."                                                   | Detected for the front window since R0; untested on Windows, see above.                                                                                                                                                                                                                                                                                                                                                                                           |

### 4.3 `docs/ROADMAP.md` (exists only on BRANCH; no conflict)

In milestone R0, mark each sub-item **built / not built / built but unverified**: run scope, `is_running`+`Reflecting`, panic supervisor, retry, serialised actions, lock helper, decision-shape tolerance: _built, tests pass_. CI: _built; the Rust workflow passed on Windows and Ubuntu for the feature branch_. wire-audit script: _built, never run against a real rune-infer_. Single-instance and elevated detection: _built, not run on Windows_. Add the date. Do not delete any NOT CERTAIN marker that is still true.

## 5. Do NOT

- Change anything in `src-tauri/src` beyond sections 3.1-3.4. No clippy fixes, no formatting, no renames.
- Change which audio part or thinking parameter Studio sends (still undecided).
- Add the "cancel the run's token after a panic" improvement: it is **out of scope** (the maintainer has not asked for it). List it under "Found but not fixed" if you agree it is worth doing.
- Touch `docs/handoff/R0-foundations.md` or `docs/handoff/R0-report.md` except to leave them as they merge.

## 6. Checks to run and paste (last lines of output)

```bash
cd src-tauri
cargo test --lib                          # expect 119 passed, 0 failed, 2 ignored
cargo test --lib --features agent-bridge  # expect 146 passed, 0 failed, 2 ignored
cd .. && bun install --frozen-lockfile && bun run build && bun run test:unit
grep -rn "lock poisoned\|\.reset()\|state\.cancel" src-tauri/src
grep -rn "<<<<<<<\|>>>>>>>" . --include=*.rs --include=*.md --include=*.ts --include=*.vue --include=*.yml
git diff --stat origin/develop   # list of files touched by the merge
```

If you have Windows, also run the Windows `cargo test --lib` and say so. If you do not, say "not run on Windows".

## 7. Report (required)

Write `docs/handoff/R0-merge-report.md` on the merge branch: environment; every conflict hunk and exactly what you kept; each command above with the final lines of output; the list of doc edits from section 4 with "done / not done / differs because"; anything NOT CERTAIN; anything you noticed but did not change. Commit the merge as one merge commit (`merge: bring the session archive, roadmap and rewritten docs into develop`), push only `merge/orchestration-into-develop`, then tell the maintainer you are done.

## 8. What the verifier will do (and will not)

The verifier does not write code. It will re-run section 6, diff the merge result against `origin/develop` and BRANCH to confirm only the hunks in section 3 were hand-resolved, check the four code hunks character by character against section 3, read each doc table row in 4.2 against the final text, and check that the report claims nothing that was not run.