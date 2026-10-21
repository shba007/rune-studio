# Milestone status audit, 2026-10-21

For the owner. Written by the Rune Studio orchestrator before any new milestone starts. It answers three questions for every milestone: **is it complete, if not what blocks it, and who clears the block.**

## How "complete" is judged

A milestone is **complete** only when its exit criterion has evidence. I use three separate words, because "built" has been mistaken for "done" before:

- **Built:** the code is on `develop`.
- **Tested:** automated tests pass (Studio's, re-run by me where noted).
- **Seen:** observed working on a real Windows machine with a real model server. **The only such observation is the field test of 2026-10-19, which ran before the F1 fixes.** Nothing built after it, and none of the F1 fixes, has been seen on a real machine.

Sources: the git history on `develop` (Studio at `0cb76b5`), `docs/handoff/M1-report.md`, `docs/handoff/R6b-report.md`, `docs/ROADMAP.md`, and code spot-checks. The ROADMAP status tables are stale (they predate the field test), so I did not rely on them.

## The one finding that matters most

**Five milestones of work (F1, R5, R6a, R6b, M1) sit on top of one field test that predates all of it.** Each report says "never seen on a real screen". The cost of finding a bug grows with every unobserved layer: BUG-01 (a deadlock at every task start) shipped unnoticed through R0 to R4 for exactly this reason. The recommendation at the end is to **stop adding milestones until a second field test passes**.

## Status by milestone

| Milestone | Exit criterion | Built | Tested | Seen | Verdict | What blocks completion | Who clears it |
|---|---|---|---|---|---|---|---|
| **R0** Foundations | Race test, forced panic, wire-format table committed | yes | yes | Mostly (race, crash, retry, single instance, outside agent passed live) | **Complete except two items** | Wire-format table never committed; elevated-window run (needs a UAC prompt) not done; confirm-focus fix (BUG-02) unobserved | Close-out agent (table), maintainer (elevated run, BUG-02 check) |
| **R1** Flight recorder | Every run yields complete rows; browsable | yes | yes | **Yes** (7 of 9 live checks passed; queue-pressure not run, growth partial) | **Complete** | Nothing blocking. Typed secrets are stored in the record: accepted, handled by L0/R8 | none |
| **R2** Eval harness | A baseline for the default model | yes | yes | Partly (a partial, invalid run) | **Not complete** | No valid baseline. The one partial run was distorted by two bugs (stuck detector, `open_app`) now fixed but unobserved; baselines must now be taken **per model** (M1) | Maintainer runs it on Windows with a live rune-infer; decide the first model |
| **R3** Verifier | Overhead under 10%, success not lower, agreement measured | yes | yes | Partly | **Not complete** | Nothing measured. Notepad gave 2 false contradictions of 4 decisive verdicts before F1; the mitigation is unobserved; the three-mode suite was never run | Maintainer (suite in off, background, blocking modes) |
| **R4** Visual analysis | Box hit rates measured | superseded by R6a | yes | Window opened (pre-overlay) | **Superseded; its measurement is open** | The box probe (T-R4-1) was never run; boxes were 1 of 8 right | Maintainer (probe) |
| **F1** Field-test fixes | The 19 bugs gone | yes | yes | **No** | **Built, unverified** | Every fix unobserved. Highest risk: BUG-02 (approved action ran against Rune, not the target) and BUG-04 (stuck detector) | Maintainer or tester (round 2) |
| **R5** Identity, demo mode, idle shell | Idle, prompt, reflect, pre-empt in one second | yes | yes | **No** | **Built, unverified** | Brand values for Antigravity, ChatGPT and Codex missing (owner input); demo-mode flicker and mask error unmeasured; idle review never seen; a running background request is **not** pre-empted by rune-infer (IF-1), so the future reflection job cannot be safe yet | Owner (palette and marks), maintainer (live), rune-infer agent |
| **R6a** Analysis overlay | Overlay, cards, boxes always on | yes | yes | **No** | **Built, unverified** | The overlay, pill growth (multi-monitor) and final look never seen; box accuracy unmeasured and boxes cannot be turned off | Maintainer (live look, probe) |
| **R6b** Video, review prompt, Flight Player | Review opens a player with the video | yes | yes (headless Chromium only) | **No** | **Built, unverified, one major risk** | The Windows H.264 encoder **has never run**; MP4 in the Tauri webview untested; size and cost of `motion` mode unmeasured; no Settings controls for `offerReview` and `recordSessionVideo` | Maintainer (encoder, MP4), close-out agent (controls) |
| **M1** Interchangeable models | Profiles, roles, swaps, per-model baselines | yes | yes (360 lib tests per its report; re-run below) | **No** | **Built, needs a contract fix** | **Contract drift (found in code):** swap detection uses guessed header names; rune-infer's real signals are `x-model-resident`, `x-model-load-duration-ms`, `x-model-evicted` and `409 swap_refused`. `x-no-swap` is never sent. `thinking_control` arrives as the string `"thinking"` or `"none"` and Studio parses both as Unknown. Probe and baselines never run | Close-out agent (C1-1); maintainer (probe, baselines) |
| **L0** Life foundations | Store, consent, local-only, wellbeing | WP1 to WP3 | partly | n/a | **In progress, not complete** | WP4 (local-only enforcement), WP5 (redaction in logs), WP6 (wellbeing) and WP7 (docs) not started; no report. The central safety rule, local-only, is the missing piece | L0 agent (already running) |
| **K1** Kit client | Kit tools through the gate | no | n/a | n/a | **Not started** | Real end-to-end check blocked by rune-tools (TF-1) and rune-kit (KF-1, KF-7); can be built against a fake server | After M1 is closed; see order below |

## Other projects (reported, not Studio's)

| Project | Reported | Verified by me | Open |
|---|---|---|---|
| **rune-kit** | KIT-1, 2, 3, 6, 13 built, 23 tests | 23 tests pass (re-run); code spot-checked | Annotation spelling (`readOnly` vs the spec's `readOnlyHint`), no release, KF-7 |
| **rune-infer** | 17 requests accepted, 121 tests | **119 passed, 1 failed** on a machine without a GPU; headers, `swap_refused`, priority refusal and `/readyz` found in code | Pre-emption of a running background request is not implemented (IF-1); model headers on chat routes only (IF-2); support matrix and speech route half answered; no release (v0.7.0 targeted) |
| **rune-tools** | TOOL-1 built across 140 tools, 228 tests | Not re-run; classification code found | **Does not reach kit or Studio** (it emits `rules`, kit reads `annotations`); blueprints written early with invented tokens; no release |

## What is blocking, grouped by who can clear it

**1. An agent can do now (no hardware needed)**
- C1 close-out (`rune-studio-C1-closeout.md`): the M1 contract fix, the two R6b settings controls, refresh the stale ROADMAP statuses, commit or scaffold the wire-format table.
- L0 WP4 to WP7 (the L0 agent, already running).
- The three follow-up handoffs for rune-infer, rune-kit and rune-tools.

**2. Only you (or a tester on Windows with a live model) can do**
- **Field test round 2.** Priority order, because each step can invalidate the next:
  1. Task start works at all and the confirm card (BUG-01 and BUG-02): approve a delete in Explorer; it must reach the file, not Rune.
  2. The stuck detector and `open_app` (BUG-04, BUG-05, BUG-09): a plain "open Notepad and write hello" in under about 60 s with no needless steps.
  3. R5 on screen: an outside agent in control shows its accent and mark; demo mode in OBS; idle review prompt, Deny, pre-emption.
  4. R6a on screen: the status bar, cards above the orb, boxes, on one monitor and on two.
  5. R6b: the review card, the player, **an MP4 actually produced and played** (the encoder), seeking.
  6. M1: two models in two roles, one swap, the loading state.
  7. Then the baseline (R2) and the verifier suite (R3), and the box probe (R4).
- Elevated Notepad, your own voice for stop, yes and no, 125 percent and 150 percent scaling, a second monitor, lock screen, UAC and sleep.

**3. Needs your decision or input**
- Brand colours and marks for Antigravity, ChatGPT and Codex (R5).
- Which model is the first baseline.
- Whether to accept Video.js 10.0.1 pinned (the agent chose it correctly; 10.1.0 was under two weeks old).

## Recommended order (and why)

1. **Now, in parallel:** C1 close-out agent; L0 agent continues; rune-tools follow-up (TF-1 unblocks K1); rune-infer follow-up (IF-1 unblocks the idle review's real job); rune-kit follow-up.
2. **Gate G1: field test round 2** after C1 lands. Nothing new starts until G1 passes: no K1, no R6c, no L1.
3. **After G1:** take the R2 baseline and the R3 measurement; fix what round 2 finds; then K1 (against a real kit with declared tools), then L0 completion review, then L1.
4. R6c, R7 and the rest follow the original order.

If you want to move faster at some risk, the only safe parallel work is L0 (it has no screen) and the other projects' follow-ups. Building K1 or L1 on top of unobserved code is the repeat of BUG-01.

## Re-run of Studio's tests

I re-ran `cargo test --lib` (Rust 1.98.1, Linux, no GPU, no OS keyring) on `develop` at `0cb76b5`: **376 passed, 1 failed, 3 ignored.** The M1 report states 360 passed at its end; the extra tests are L0's. I did not re-run the feature-flag runs, the Vitest suite, clippy or fmt.

**The one failure is an L0 test:** `life::store::tests::key_loss_and_start_fresh_recovery` fails with `keyring access error: No default store has been set`. The test needs a real OS keyring, which this machine does not have, so it is probably an environment difference (Windows Credential Manager exists on the maintainer's machine). It is still a defect in the test: a test must not depend on the machine's keyring, and CI or a headless machine will fail it. **Tell the L0 agent:** make the key store injectable and use an in-memory one in tests, and say what Studio does when no keyring is available at all (it must refuse to enable sensitive classes with a clear message, not fail later).
