# Handoff: F1, fixes from the field test of 2026-10-19

For an AI coding agent. Read all of it first. Source of truth for the bugs: `docs/field-test-report-2026-10-19.md` (read it fully; BUG numbers below refer to it). Current behaviour: `docs/ARCHITECTURE.md`. Where this file and the code disagree about a fact, the code wins: say so in your report.

## 1. Goal

Make R0 to R4 behave correctly on a real Windows machine. BUG-01 and BUG-03 are already fixed on `develop` (`9748ca1`, verified: both `locked()` pairs replaced by one guard, `tools` wrapped in `json!([..])`). Do **not** redo them; do add the regression tests listed in WP0. No new features.

## 2. Preconditions and rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop` then `git push origin develop`.
2. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline test numbers first (`cargo test --lib`, with `--features agent-bridge`, with `--features eval,agent-bridge`, `node --test scripts/eval/test`, `bun run test:unit`, `cargo clippy`, `cargo fmt --check`).
3. Every fix needs a test that fails before and passes after, unless it needs a live window or model; then say exactly how the maintainer checks it. **Never claim a live behaviour you did not observe.**
4. Do not change the model, prompts or thresholds beyond what a work package says. Do not touch R5+ ideas.
5. If a fix needs a decision that is the maintainer's (marked DECIDE), implement the default stated and list it in the report.

## 3. Work packages (in this order; stop and report after WP3 if anything blocks)

### WP0: repo hygiene and regression tests (BUG-01, BUG-03, BUG-18)
- `git rm -r --cached eval-out/` (commit `9748ca1` added 30 junk files under `eval-out/preflight-test/`), add `eval-out/` to `.gitignore`.
- Regression test for BUG-01: a lint-style test or a clippy config that rejects two `.locked()` calls in one expression is optional; at minimum a unit test that builds the `RunInfo` through the new helper. Preferred: move the "read model + settings snapshot under one guard" into one small function used by both `agent/mod.rs` and `bridge/mcp.rs`, and test it.
- Regression test for BUG-03: assert the analysis request body has `tools` as a JSON array whose first item has `"type":"function"`.
- BUG-18: make `archive::tests::only_expired_date_folders_below_sessions_are_removed` pass on Windows (compare `Path`s, not strings with `/`); exclude `temp/` from `bun run test:unit`; make `tauri:dev` (or its docs in the guide) build `rune-studio-cli`.

### WP1: the confirm card must not take the foreground (BUG-02, S1)
Read `safety/gate.rs::ask_user` and the confirm card window code. After the user answers, the approved action ran against Rune instead of the target. Required: the card must not become the foreground window or must hand focus back to the window that was foreground when it appeared (remember its `HWND` before showing the card, restore with `SetForegroundWindow` after the answer, and verify the target is foreground before executing). If the target window is gone, fail the action with a clear message, do not type into Rune. Test: a pure test for the "remember and restore" decision logic; live check described for the maintainer (approve a `press delete` on a file in Explorer; the file must go to the recycle bin).

### WP2: the stuck detector (BUG-04)
`agent/stuck.rs`: the 32x18 grey comparison misses small real changes (typing "hello rune" scored 0.04, a menu opening 0.5, threshold 1.5), so correct steps trigger "your actions are not working" and runs end after five. Required: a detector that does not fire when the action should have changed little (typing, key press, scroll within a document) or that compares a higher-resolution/localised region (for example the changed-pixel fraction or per-tile max difference rather than the mean). Keep stuck detection working for the real case (identical frames after clicks). Use the stored screenshots from the report's runs if available; otherwise synthetic frames. Report the before/after false-positive count on any frames you can use. Because this distorts the eval, note in the report that the baseline must be re-run after it.

### WP3: `open_app` (BUG-05, BUG-09)
`exec/apps.rs`: (a) a `file:///...?run=<id>` target loses its query string; pass URLs to the shell in a way that keeps them, or open them through the default browser with the full URL. (b) `open_app "notepad"` may resolve to an extensionless file on PATH; prefer registered apps and `.exe` over extensionless files, never treat an extensionless file as an app. (c) `settings`, `Calculator` fail with error 2: add the aliases `calc`, `calculator`, `settings` -> `ms-settings:`. Tests: pure resolution tests with a fake PATH listing.

### WP4: fast checks with a one-model server (BUG-06, BUG-07)
- BUG-06: with one loaded model, a decision call evicts the omni model and costs 7 to 16 s. Required: fast checks default to **off**, and when enabled the settings text must say a single-model server will swap models. The default decision model must match what rune-infer 0.6.0 serves (`liquid-d1-3b`; confirm against `/v1/models` in the field report) and `decisionUrl` must have a sensible default (the same server). When a decision call times out, remember it for the minute as now, but do not count the swap time as the task's. DECIDE: keep the feature at all? Default: keep, off by default.
- BUG-07: `agent/quick.rs` risk check reads `confidence`; the picked option's probability is `probabilities[decision]`. Use that. Update tests with the report's numbers ("click Pay now": probability 0.587, confidence 0.38).

### WP5: voice mishearing (BUG-08)
The omni model hears the audio and often returns the wrong words (a spoken "Open Notepad and write hello" started a Calculator task); an empty system prompt loops. Required without a new model: (a) always send a short system prompt that says "transcribe only"; (b) cap `max_tokens` for the transcript; (c) reject transcripts that repeat a token more than 3 times; (d) log every ignored result with its reason (BUG-19). DECIDE: should the task start require showing the transcript in the pill for 2 s with Cancel? Default: yes for tasks (not for control words). Do not add a speech-to-text model.

### WP6: small items
- BUG-10: tolerate malformed tool arguments: try strict parse, then one conservative repair (strip a trailing junk after the last balanced brace); if still invalid, send one corrective message to the model instead of failing the step. Never guess values.
- BUG-11: the verifier says "VS Code" for Notepad. Add the foreground window title and process name to the verifier's user text, as facts. Re-measure by the maintainer.
- BUG-12: `completion_tokens` is 1 with thinking off. Treat server `usage` as unreliable: store it but mark it; compute the tokens-per-step shown in the eval from an estimate (characters/4) when usage is below 2. Say so in the docs.
- BUG-13: after Stop or closing the analysis window the server keeps generating (next plan call took 75.7 s). Required: on cancel, close the HTTP connection for real (drop the response body, not just stop awaiting). If rune-infer does not abort, say that in the report: it belongs to the backend.
- BUG-14: `status` must check that the process answering on 47600 is Rune (e.g. the bearer token is accepted), and report "port in use by another program" otherwise.
- BUG-15: do not change the model. Hide the numbered boxes in the analysis view unless a setting `analysisBoxes` is on (default off), keep the findings list and evidence. DECIDE: default off, because 1 of 8 boxes was right.
- BUG-16 to BUG-19: the confirm countdown must count; versions: one source for the version (CLI, eval env, trace) from `CARGO_PKG_VERSION` matching Settings (`0.0.11`); fix the CLI help names; remove the dead code clippy reports; run `cargo fmt`; `rune-studio-cli run <unknown id>` exits non-zero; the `PostMessage ... Invalid window handle` message when the analysis window closes (find which call, avoid it); `/readyz` 404: note for the backend.

## 4. Do not

Do not commit the partial baseline `eval-out/field-off/`. Do not change the eval gate. Do not touch `docs/ROADMAP.md` except to tick what you finished. Do not add dependencies without saying why.

## 5. Report

Write `docs/handoff/F1-report.md`: baseline numbers before and after, per work package: what changed, the test that proves it, what only the maintainer can check live (exact steps), anything you chose differently from this file and why, and every DECIDE with the default taken. List every BUG with status: fixed / mitigated / not fixed (why).
