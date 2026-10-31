# Handoff: C1, close-out of M1, R5, R6b and the stale status text

For an AI coding agent working in the **rune-studio** repository. Read all of it first, plus `rune-studio-contract-addendum.md` (the real names reported by rune-infer, rune-kit and rune-tools; give both files together). This is **not** a new feature: it closes gaps the orchestrator found while auditing the milestones on 2026-10-21 (Studio `develop` at `0cb76b5`). Where this file and the code disagree about a fact, the code wins: say so in the report. Do not start K1, R6c or L1. The **L0 agent** works in parallel on `src-tauri/src/life/`; do not touch that module.

## Rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop`, then `git push origin develop`.
2. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (`cargo test --lib`, with `--features agent-bridge`, with `--features eval,agent-bridge`, `node --test scripts/eval/test`, `bun run test:unit`, clippy, fmt) and again at the end.
3. Never claim a live behaviour you did not observe; you have no Windows desktop and no model server. Test with fakes and fixtures that follow the shapes in the addendum.

## Work packages

### C1-1: M1 swap and capability contract (real names)

Found in the code (`model/swap.rs`, `model/capabilities.rs`): M1 was built before rune-infer's real contract was known and guessed names.

- **Swap signals.** `detect_swap_signal` looks for `x-rune-infer-loading`, `x-rune-model-loading`, `x-model-loading`, `x-loading-model` and `x-rune-infer-swapped`, `x-rune-model-swapped`, `x-model-swap-ms`, `x-swap-duration-ms`. rune-infer's real signals (reported, spot-checked in its code) are: response headers `x-model-resident` (`false` means a cold load happened), `x-model-load-duration-ms`, `x-model-evicted`, `x-model-served`, `x-model-locality`; and the refusal `409` with error type and code `swap_refused` when the request carried `x-no-swap: true` and the model was not resident. Rewrite the detection to use the real names, keep the old ones as a fallback list (one function), and add tests with fixtures that follow the real shapes (including the JSON error body shape `{"error": {"message", "type": "swap_refused", ...}}` as rune-infer builds it: check `src/api/chat/mod.rs` in the rune-infer repository only if you have it; otherwise accept both a string and an object under `error`).
- **`x-no-swap`.** M1's setting `skip_swap_for_optional_roles` (default true) must **send `x-no-swap: true`** on decision and verifier requests, and treat the `409 swap_refused` answer as "skipped, with a recorded reason" (not as a failure and not retried). Test with a fake server.
- **Served model.** Also read `x-model-served` (and `x-model-locality`) from the response headers and record them next to the body's `model` (trace `served_model`, plus a `locality` column or field in `calls` if there is room in the existing v7 migration; otherwise a v8 migration; never edit earlier ones). Mismatch between the requested and served model remains a warning.
- **`thinking_control`.** The real value is the string `"thinking"` (the model has a thinking switch) or `"none"` (no switch; do not send thinking switches). Today `parse_tri_state` turns both into Unknown. Map `"thinking"` to Yes and `"none"` to No, keep booleans and `yes`/`no` working, and make the request builder send **no** thinking switches when the value is No. Tests for all four forms.
- **Other fields.** Parse `roles`, `aliases`, `resident`, `status` (`available`, `loaded`, `remote`), `vram_mb_estimate`, `load_ms_estimate` and the server-level `vram_total_bytes`, `vram_free_bytes`, `vram_used_bytes` if M1 does not already; a missing value is unknown, never zero.
- **Background priority.** Any call that is not part of the user's task (the idle review when it gets a real job, later life jobs) must send `x-rune-priority: background` and treat `429` with `Retry-After` as "try later", not as an error. Add the capability to the client now (a request option) and test it; nothing needs to use it yet.

### C1-2: R6b settings controls

The R6b report says `offerReview` and `recordSessionVideo` have no Settings control (`settings.json` only). Add them to the existing settings window in the Rune style: `offerReview` (always, failures, never) and `recordSessionVideo` (off, motion) with one honest line each (motion records the screen at 2 frames per second and its cost is unmeasured). Component tests.

### C1-3: stale status text

`docs/ROADMAP.md` still shows 2026-10-19 status tables that predate the field test and later work (for example R0 says the wire audit was never run and single-instance was never run on Windows; R1 says "never run end to end"; R2, R3, R4 and R5 say "not run"). Bring every milestone status up to date from evidence you can cite (commits, the existing reports, `docs/handoff/M1-report.md`, `docs/handoff/R6b-report.md`): for each, three separate words: **built**, **unit tested**, **seen on a real machine** (yes, no, partly, and on what date if known). Do not claim "seen" for anything you cannot cite. The 2026-10-19 field test is not in the repository; the orchestrator's summary is: R0 race, crash, retry, single-instance and outside-agent checks passed live; elevation partial; R1 passed live (queue pressure not run); R2 partial and invalid baseline; R3 verifier made false contradictions on Notepad; R4 boxes 1 of 8 right; the confirm-focus bug (BUG-02) and 17 others were found and then fixed in F1 **without being re-observed**. Record that sentence in the roadmap under a "Field test 2026-10-19" heading.

### C1-4: wire-format table

`scripts/wire-audit.mjs` exists and was run in the field test, but the table was never committed (R0's exit criterion). If a table or output exists anywhere in the repository history, restore it as `docs/wire-audit.md`; if not, add to `docs/wire-audit.md` the exact command and a "to be filled by the maintainer" table with the rows rune-infer's real contract now implies (chat with image, forced tool choice, thinking off/on, `input_audio`, `x-no-swap`, `x-rune-priority`, model attribution headers on each route). Do not invent results.

## Do not

No new features, no change to the safety gate, no K1 work, nothing in `src-tauri/src/life/`.

## Report

`docs/handoff/C1-report.md`: baseline and final numbers; per work package what changed and its tests; the real header and field names you now rely on and any you could not confirm; the updated milestone status table; live steps for the maintainer (one swap with two models and the loading state; a decision call against a non-resident model returns `swap_refused` and is skipped; the served model appears in the Flight Player's API tab); every NOT CERTAIN.