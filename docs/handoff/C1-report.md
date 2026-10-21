# C1 closeout report — 2026-10-21

## Scope and checkout

Implemented C1-1 through C1-4 on the existing `develop` checkout, initially at `0cb76b5`. No K1, R6c or L1 work, no safety-gate changes, and no edits inside `src-tauri/src/life/`.

At entry, the checkout already contained changes in `life/mod.rs`, untracked `life/policy.rs`, and L0 additions in `model/client.rs` and `model/capabilities.rs`. These were preserved. The client changes overlap C1, so the work is left uncommitted for review rather than including another workstream's changes in C1 commits. No pull, rebase or push was performed on this mixed working tree. The requested one-commit-per-package publishing workflow remains outstanding.

Toolchain observed: Rust 1.98.1, Bun 1.4.3, Node 24.22.1. No live model-server or desktop interaction was performed. Fake HTTP servers ran on loopback during tests.

## Baseline and final checks

| Check                                              | Baseline                                                                               | Final                                                                                                                            |
| -------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test --lib`                                 | Compilation failed: 14 Request constructors missing pre-existing L0 `data_class` field | 388 passed, 0 failed, 10 ignored                                                                                                 |
| `cargo test --lib --features agent-bridge`         | Same 14 compilation errors                                                             | 428 passed, 0 failed, 10 ignored                                                                                                 |
| `cargo test --lib --features eval,agent-bridge`    | Same 14 compilation errors                                                             | 460 passed, 0 failed, 10 ignored                                                                                                 |
| `node --test "scripts/eval/test/*.test.mjs"`       | 18 passed                                                                              | 18 passed                                                                                                                        |
| `bun run test:unit`                                | 152 passed, 19 files                                                                   | 153 passed, 20 files                                                                                                             |
| `cargo clippy --lib --all-features -- -D warnings` | Failed on existing incomplete L0 constructors                                          | Non-strict final `cargo clippy --lib --features eval,agent-bridge` succeeds with 30 warnings; strict warnings check is not clean |
| `cargo fmt --check`                                | Failed in pre-existing L0 policy/client formatting                                     | Only untouched `life/policy.rs` differs; all changed C1 Rust files formatted                                                     |
| `bun run build`                                    | Not measured                                                                           | Passed, 552 modules                                                                                                              |
| `bunx vue-tsc --noEmit`                            | Not measured                                                                           | Tool startup fails: installed TypeScript does not export `./lib/tsc` for vue-tsc; no typecheck result                            |
| `git diff --check`                                 | Not measured                                                                           | Passed                                                                                                                           |

Sandbox execution initially prevented frontend temporary-file access and Rust loopback/keyring/filesystem tests. Tests were rerun outside the sandbox with tool approval. The final test counts refer to those runs. Ignored Rust tests remain ignored; no live behavior is inferred from their names.

## C1-1: model contract

- Real swap headers take priority over legacy fallback names in `model/swap.rs`: `x-model-resident: false`, `x-model-load-duration-ms`, `x-model-evicted`. Cold-load headers invoke the existing loading callback; the duration is recorded even when the cold-load signal is `Loading`.
- `409` plus `error: "swap_refused"` or an error object with type/code `swap_refused` produces a non-retryable skip. Decision calls log the reason without entering the failure cooldown. Verifier calls record `calls.outcome = skipped`, and the verdict records a skip with its reason rather than an error.
- Decision and verifier clients send `x-no-swap: true` when `skip_swap_for_optional_roles` is true. The cached non-resident early-skip optimization remains. False permits normal swapping.
- `x-model-served` takes precedence over the body model for both JSON and streamed replies; the body model remains in the recorded response. Mismatches remain warnings. `x-model-locality` is stored alongside the served model in `calls` and exposed in the Flight Player API tab.
- Added schema v8 for `calls.locality`; earlier migrations were not edited. Tests cover v7 migration, preservation of historical calls, new locality round-trip, and older Flight Player queries using NULL for absent columns.
- `thinking_control` accepts `thinking`/`none`, booleans and yes/no. Explicit `none` suppresses thinking switches even when a bundled/user profile would otherwise enable them. Existing optional resident/status/VRAM/load parsing remains; added roles/aliases and optional server memory counters. Explicit zero and missing memory values remain distinct. Server memory is exposed through backend status as `memory`.
- Added `Request.background`: sends `x-rune-priority: background`; background 429 yields `Deferred { retry_after_secs }`, is recorded as deferred, and is not retried as a foreground failure. The caller can schedule the next attempt after the supplied delay. No background job starts using it in C1.
- Added `data_class: None` at existing constructors to compile against the pre-existing L0 client field; no policy classification changes.

Tests: real-header precedence and refusal shapes; thinking forms and optional metadata; fake HTTP header inspection, skip/defer outcomes with one attempt, `Retry-After: 7`, decision-route refusal, header/body model disagreement and locality; verifier verdict skip semantics; migration and locality persistence; Flight Drawer attribution display. Existing legacy-header tests remain.

## C1-2: settings controls

Added `SettingsReview.vue` within General → Recording, using existing Rune row styles. Offer review choices: always, failures, never. Session video choices: off, motion. Copy states motion records at 2 frames per second and its cost is unmeasured. Component coverage loads saved values and changes every option; the existing settings draft/save watcher persists the changes.

## C1-3: evidence-based roadmap

Added a current table with separate built, unit tested and seen-on-machine columns; marked old snapshots historical. Corrected the blanket “nothing built”, R1 pending-merge and R6b never-observed wording. Added the supplied Field test 2026-10-19 summary verbatim, with emphasis on fixes not being re-observed. Evidence includes existing M1/R6b reports and recovered field/F1/R5/R6a reports in Git history. Headless fixture playback is explicitly distinguished from Windows desktop playback.

| Milestone                            | Built  | Unit tested | Seen on a real machine               | Evidence and remaining gaps                                                                                                                                                                                     |
| ------------------------------------ | ------ | ----------- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R0 Foundations                       | yes    | yes         | partly, 2026-10-19 Windows           | Field report at `0531bec^:docs/field-test-report-2026-10-19.md`; race, crash, retry, single-instance and outside-agent checks passed; elevation partial; wire audit restored in [wire-audit.md](wire-audit.md). |
| R1 Flight recorder                   | yes    | yes         | partly, 2026-10-19 Windows           | Same field report: passed live; queue pressure not run. Code is on develop, rather than awaiting merge.                                                                                                         |
| R2 Eval harness and baseline         | partly | yes         | partly, 2026-10-19 Windows           | Same field report: partial run, invalid baseline; harness and fixture checkers built, valid baseline still absent. Replay and kit tasks deferred.                                                               |
| R3 Verifier                          | yes    | yes         | partly, 2026-10-19 Windows           | Same field report: verifier made false contradictions on Notepad. Agreement and overhead exit criteria remain unmeasured.                                                                                       |
| R4 Visual analysis v1                | yes    | yes         | partly, 2026-10-19 Windows           | Same field report: boxes 1 of 8 right. Live use does not establish usable grounding accuracy.                                                                                                                   |
| F1 Field-test fixes                  | yes    | yes         | no, fixes not re-observed            | `0531bec` and historical `docs/handoff/F1-report.md`; 18 reported bugs fixed, no follow-up live verification.                                                                                                   |
| R5 Identity, demo, idle shell        | yes    | yes         | no                                   | `9aa1c40` historical R5 report; identity `b4dfc69`, demo `806bb54`; idle review is a stub. No live evidence found.                                                                                              |
| R6a Analysis overlay                 | yes    | yes         | no                                   | `9171b31` historical R6a report, implementation `13c24f1`, `c3f0aee`; R4 field results predate this overlay.                                                                                                    |
| R6b Session review and Flight Player | yes    | yes         | partly, 2026-10-20 headless Chromium | [R6b report](handoff/R6b-report.md): fixture playback and seeking observed in Chromium; Windows encoder, WebView2 H.264 and asset protocol not observed.                                                        |
| M1 Model interchangeability          | yes    | yes         | no                                   | [M1 report](handoff/M1-report.md), `7fd7d5b`: unit/component/fake-server fixtures; no live model server. C1 corrects the reported contract.                                                                     |
| L0 Life data foundation              | partly | partly      | no                                   | Commits `7e60e06`, `217947a`, `0cb76b5` implement store, consent and encryption. Local-only policy work was uncommitted at C1 start; C1 does not audit or modify that module. No live evidence found.           |
| K1 Kit integration                   | no     | no          | no                                   | Planned; C1 does not start it.                                                                                                                                                                                  |
| K2 Studio catalog                    | no     | no          | no                                   | Deferred; rune-tools blueprints remain drafts.                                                                                                                                                                  |
| R6c Post-mortems and notes           | no     | no          | no                                   | Planned; C1 does not start it.                                                                                                                                                                                  |
| L1 Life jobs                         | no     | no          | no                                   | Planned; C1 does not start it.                                                                                                                                                                                  |
| R7 Notes in the loop                 | no     | no          | no                                   | Planned; no implementation evidence found.                                                                                                                                                                      |
| R8 Trust                             | partly | partly      | no                                   | Archive retention exists in R1; L0 provides consent, retention, export and encryption for life data. Broader archive controls and kit trust remain planned.                                                     |
| R9 Universal abstractions            | no     | no          | no                                   | Planned; replay and surface traits remain deferred.                                                                                                                                                             |

## C1-4: wire audit

Recovered the actual 2026-10-19 output from `9748ca1:docs/wire-audit-2026-10-19.md` (removed by `0531bec`) into `docs/wire-audit.md`. The historical output is preserved verbatim. Added provenance, exact PowerShell command, and a separate pending table for the current headers and each relevant route. The audit script does not yet implement these new header/concurrency probes; its existing output alone cannot validate them. No current results were invented.

## Live checks for the maintainer

1. Run the command in `docs/wire-audit.md` against the current rune-infer and retain its dated output. Fill the pending current-contract checks separately, including chat image, forced tool choice, thinking off/on and input_audio.
2. Configure two local models. Trigger a call to the non-resident one, observe the loading state, then inspect the API attempt's canonical served model, locality and cold-load duration. Compare requested model, header and recorded body model.
3. With optional no-swap enabled, target a non-resident decision model. Confirm the request includes `x-no-swap: true`, server returns `409` with `swap_refused`, and Studio skips without retry or failure cooldown. Repeat for the verifier and confirm both call and verdict show skipped with the reason. Disable the option and confirm the header is absent.
4. In Flight Player → API, confirm served-model mismatch warning and locality on both JSON and thinking/streamed calls; open an older session as well.
5. Keep a foreground request active; send a new short background request with `x-rune-priority: background`. Confirm 429 and Retry-After, then schedule a later attempt after the delay. A background request already running is not pre-empted according to the supplied contract; keep future background jobs short and serial.
6. Change review/video settings, reopen Settings and check persistence. Observe review behavior and motion recording on Windows; measure CPU, disk and task latency before claiming a cost.

## NOT CERTAIN and remaining limits

- The reported rune-infer/kit/tools contract commits were supplied, not independently checked in sibling repositories or run end to end here. Actual servers take precedence.
- Swap/loading UI and Flight Player behavior with a live server, WebView2 H.264, Windows video encoding and the cost of motion remain unobserved.
- Server usage is server-reported; accurate image/audio/cached token accounting, forced tool-call reliability, speech output and pointing conventions remain unmeasured.
- Attribution headers on transcription/decisions/OCR are reported absent; transcription latency/model residency and the existence/shape of any OCR route are unknown. C1's chat attribution does not prove these routes are safe for tagged data; L0 owns that policy.
- The restored wire output describes the historical server, not the current one. New no-swap/priority and route-attribution checks require maintainer execution.
- The installed vue-tsc/TypeScript compatibility issue prevents a standalone typecheck. Production bundling and component tests pass, but do not replace that check.
- L0 policy formatting and existing clippy warnings remain outside C1's scope. The full working tree includes uncommitted L0 changes; commit/pull/rebase/push must preserve that work.
- Kit annotation/rules forwarding and real read-only behavior remain unconfirmed and are not exercised by C1. Draft A2UI blueprints were not read or rendered.