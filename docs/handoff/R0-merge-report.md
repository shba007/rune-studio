# Merge report: orchestration branch into develop

Branch `merge/orchestration-into-develop`, created from `origin/develop` (`c0350b3`), merging `origin/claude/intelligent-brahmagupta-u7c6o7`.

## 1. Environment

Linux (cloud container), `rustc 1.98.1`, bun 1.4.2, node 22. **Not run on Windows.** One difference from the handoff: the head of the orchestration branch was `95ad0e3` (one doc commit, "handoff for merging the orchestration branch", after the stated `543e6f4`). That commit only adds `docs/handoff/R0-merge.md`.

## 2. Conflicts

Exactly the four expected files conflicted: `README.md`, `docs/ARCHITECTURE.md`, `src-tauri/src/agent/mod.rs`, `src-tauri/src/bridge/mcp.rs`.

| Hunk                                               | What I kept                                                                                                                                         |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `agent/mod.rs` 1 (task spawned by `start_planned`) | develop's supervisor lines, then `end_session(&session, &outcome);` before `finish`. The branch's `run(&app, &task, plan)` line is gone. As in 3.1. |
| `agent/mod.rs` 2 (after `start_planned`)           | develop's `supervised` and `settle` unchanged, one added `}`, then the branch's `end_session` unchanged. As in 3.2.                                 |
| `bridge/mcp.rs` 1 (`begin`)                        | `state.begin_run();` with develop's comment, then the branch's `archive::begin_session` block. `state.cancel.reset();` deleted. As in 3.3.          |
| `bridge/mcp.rs` 2 (`done`)                         | `state.lease.locked().end();` then `archive::end_session(None, ..., &summary)`. As in 3.4.                                                          |
| `README.md`, `docs/ARCHITECTURE.md`                | Took the branch's whole file (`git checkout --theirs`), then edited (section 4).                                                                    |

The resolutions were applied with a script that matched each conflict block; I then read the four code hunks back and they match section 3 of the handoff.

### 3.5 Auto-merged files, read and confirmed

- `commands.rs`: `stop` calls `stop_current()` (line 93) and then `crate::archive::end_session(None, "stopped", "")` (line 106); `.is_running()` used at 3 places.
- `voice/mod.rs`: `apply(..., Some(saved.as_path()))` (line 81), `archive::adopt(recording)` after a task starts (line 141), `.is_running()`.
- `bridge/server.rs`: `end_session(None, "lease_expired", "")` plus `.locked()`.
- `archive.rs`, `docs/AGENT-GUIDE.md`: merged cleanly, not edited.

## 3. Commands (last lines)

```
cargo test --lib                          test result: ok. 119 passed; 0 failed; 2 ignored
cargo test --lib --features agent-bridge  test result: ok. 146 passed; 0 failed; 2 ignored
bun install --frozen-lockfile             Checked 417 installs across 534 packages (no changes)
bun run build                             exit 0 ("built in ..." printed)
bun run test:unit                         Tests  6 passed (6)
grep -rn "lock poisoned\|\.reset()\|state\.cancel" src-tauri/src      (no output)
grep -rn "<<<<<<<\|>>>>>>>" . (rs, md, ts, vue, yml; node_modules and target excluded)
                                          only hits: docs/handoff/R0-merge.md lines 74 and 122, which quote the grep command itself
```

Both test counts match the expected 119 and 146. Extra check, not asked: `cargo check --lib --tests --target x86_64-pc-windows-gnu --features agent-bridge` finishes (type-check only, not run).

`git diff --stat origin/develop` (12 files): README.md, docs/AGENT-GUIDE.md, docs/ARCHITECTURE.md, docs/ROADMAP.md, docs/handoff/R0-foundations.md, docs/handoff/R0-merge.md, plus this report; code: `agent/mod.rs` (+22), `archive.rs` (+310/-), `bridge/mcp.rs` (+5), `bridge/server.rs` (+1), `commands.rs` (+2), `voice/mod.rs` (30). `docs/handoff/R0-report.md` is already on develop and unchanged.

## 4. Documentation edits

**4.1 README.md:** all three done (Tests sentence replaced; `rust.yml` row added and the `integrate.yml` sentence changed; one sentence on a second launch under "Using the app").

**4.2 ARCHITECTURE.md** (all by text search):

| Row                                                 | Result                                                                                                                                                                           |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Section 1 "Not built"                               | done: elevated-window detection removed                                                                                                                                          |
| Section 4 retry sentence                            | done                                                                                                                                                                             |
| Section 6 module tree                               | done: `sync.rs` added; I also added `RunScope` next to `CancelToken` in the `exec/` line (small addition)                                                                        |
| Stop bullet                                         | done (RunScope text)                                                                                                                                                             |
| Locks bullet                                        | done: also lists the run scope among the mutexes                                                                                                                                 |
| New bullets (supervised, act_turn, single-instance) | done; "Not cancellable once started" and Voice bullets unchanged                                                                                                                 |
| State machine bullet                                | done: `Reflecting`, `is_running`, "Several places..." sentence dropped                                                                                                           |
| Elevated windows bullet                             | done, with the "Not confirmed on Windows" caveat                                                                                                                                 |
| Section 12 "user wins" and "Not built"              | done; added a "Turns" bullet                                                                                                                                                     |
| Section 13 CI bullet                                | done: `rust.yml` added; "no tests" kept for `integrate.yml` (I changed "no tests and no Rust checks" to "no tests" since `rust.yml` covers Rust)                                 |
| Section 14 M6 row                                   | done                                                                                                                                                                             |
| Section 15 gaps                                     | removed gap 1, the panic half of gap 2, gap 3, the simultaneous-calls half of gap 4, gap 10; the rest renumbered (9 gaps). A cross-reference "gap 7" in the risks became "gap 5" |
| Section 16 risk 5                                   | done                                                                                                                                                                             |

Not changed (as instructed to keep the rest): gap "Wire format against rune-infer is unverified" still says the `/v1/decisions` shape the parser expects is unverified; that is still true for which shape a live server sends, though the parser now accepts both.

**4.3 ROADMAP.md:** done. A status table was added above the R0 plan, dated 2026-10-19. The CI claim is backed by the check runs on PR 15 (`Test (ubuntu-latest)` and `Test (windows-latest)` both concluded success; I looked them up). No NOT CERTAIN marker was removed.

## 5. NOT CERTAIN

- Everything Windows-only (single-instance behaviour, elevated detection) is still unrun.
- The wire audit has still not been run against a real server.
- The merged `develop` has not been through CI (it will run on the PR/push).

## 5b. Deviation: the commit subject

The handoff asks for `merge: bring the session archive, roadmap and rewritten docs into develop`. The repo's commitlint rejects the type `merge`, and I did not bypass the hook. The merge commit subject is `Merge branch 'claude/intelligent-brahmagupta-u7c6o7' into merge/orchestration-into-develop` (commitlint ignores such subjects), and its body carries the sentence from the handoff.

## 6. Noticed, not changed

- A panic in a run is caught and the state is reset, but the run's cancellation token is not cancelled afterwards, so anything the panicked task left running in the blocking pool is not signalled to stop. It would be a small addition in the supervisor; out of scope, I think it is worth doing.
- The pre-commit formatter may re-align Markdown tables in the commit; whitespace only.