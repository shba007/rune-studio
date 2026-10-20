# Rune Studio use cases

What Rune Studio is good for today, which features each use case uses, how a run flows, what does not work yet, and where each one can grow.

**How to read the evidence.** Numbers come from the field test of 2026-10-19 (Windows 11, one 1920x1080 monitor at 100%, rune-infer 0.6.0 with one model loaded at a time, `gemma-4-12b-it`, a dev build) and from the F1 fix report. That test was run **before** the F1 fixes, so rows that depended on a fixed bug say so. Anything not measured is marked **NOT MEASURED**; anything planned is marked **PLANNED**. Behaviour is described from the code (`docs/ARCHITECTURE.md`); where a run is shown, it is a flow, not a recorded transcript, unless it says "recorded".

Status key: **Works** (observed), **Partial** (observed with known problems), **Untested**, **Planned**.

| #   | Use case                                  | Status                                      |
| --- | ----------------------------------------- | ------------------------------------------- |
| 1   | Open an app and type into it              | Works                                       |
| 2   | Change or read a system setting           | Works                                       |
| 3   | Browse a page, dismiss popups, small targets | Partial                                  |
| 4   | A risky action that waits for your Yes    | Partial (hand-back fix not observed)        |
| 5   | Hands-free voice tasks                    | Partial (mishearing)                        |
| 6   | An outside agent drives the PC            | Works (branding PLANNED, R5)                |
| 7   | "What is on my screen?" analysis          | Partial (boxes unusable, off by default)    |
| 8   | Review what happened (flight recorder)    | Works                                       |
| 9   | Measure quality (eval harness, verifier)  | Partial (baseline not yet taken)            |
| 10  | Record a demo                             | PLANNED (R5 demo mode)                      |
| 11  | Idle self-review                          | PLANNED (R5 shell, R6 content)              |

---

## 1. Open an app and type into it

**Example task:** "Open Notepad and write hello."

**Features used:** text or voice input, planning call, observe (screenshot plus UI Automation control list), one-action-per-turn loop, `open_app`, `type_text`, `done`, stuck detector, flight recorder, background verifier.

**Flow**

```
task -> plan (1 to 3 goals) -> [observe -> choose ONE action -> safety gate -> act -> settle 1.8 s] x N -> done
         |                                                                                        |
     orb ring shows goals                                                  session folder + trace rows written every step
```

Typical steps: open_app notepad -> (maybe wait) -> click the text area if it lacks focus -> type_text "hello" -> done. The pill shows the goal arcs; the overlay shows a ripple and a caption per action; Stop (hotkey Ctrl+Alt+Esc by default) ends it in under a second.

**Recorded (field test, T-0-2):** 55 s, 6 steps, Notepad contained "hello". Two of the six steps were needless: the stuck detector fired falsely (BUG-04, fixed in F1, false-positive rate not re-measured). Stop ended a run in 420 ms. Per step: observe median 312 ms, act median 36 ms, settle 1,800 ms (fixed), model call 4.5 s median with thinking off.

**Limitations**
- Thinking on is about 4 times slower (decide median 18 s, runs 93 to 169 s), so it is off by default.
- One action per model call, so a task is at least (steps x ~7 s). A 30-step task is minutes, not seconds.
- `open_app` of Store apps by name is not fully covered (F1 maps a few aliases). The verifier said "VS Code" for a correct Notepad screen (BUG-11; F1 adds the window title to its text, effect NOT MEASURED).
- Elevated (admin) windows: reading the process token works for the same user, but the end-to-end behaviour is **Untested** (T-R0-6).
- Scaling 125% and 150% and a second monitor are **Untested**.

**Upgrade scope:** faster steps (smaller settle, cached plans), parallel verification, notes about an app ("Notepad opens with a tab restored") fed back into the prompt (R6, R7), multi-monitor.

---

## 2. Change or read a system setting

**Example tasks:** "Open Settings", "Open the calculator and compute 12 times 13".

**Features used:** `open_app` aliases (`settings` -> `ms-settings:`, `calculator`), UI Automation `click_element` by id, key presses, verifier.

**Flow:** same loop as 1. The control list from UI Automation lets the model click by id (exact) instead of guessing a point, which is why small buttons in native apps work.

**Recorded (field test):** Calculator and Settings tasks passed and the verifier returned `verified` with correct evidence (T-R3-1).

**Limitations:** before F1, `open_app "settings"` and `"Calculator"` failed with error 2 (BUG-09; fixed in code, **not observed**). Anything behind a UAC prompt cannot be done (the secure desktop is not capturable). A locked or sleeping PC returns `screen_unavailable`.

**Upgrade scope:** a library of known apps and their shortcuts (R6 notes), kit tools for deterministic settings changes instead of clicking (K1).

---

## 3. Browse a page, dismiss popups, small targets

**Example tasks:** "Open example.com and close the cookie banner", "Click the tiny X".

**Features used:** `open_app` with a URL, `click`, `scroll`, zoomed `observe`, fast checks (optional), stuck detection.

**Recorded (field test):** the popup-dismiss and small-target eval tasks passed their checkers (4 of 13 real tasks passed in the one partial baseline; that run was distorted by BUG-04 and BUG-05, so do not read it as the true pass rate). **Browser pages expose few UI Automation controls**, so clicks are by coordinate (0..1000 grid), where small targets are the weak spot (earlier probe: small targets 57 to 79% hit).

**Limitations**
- URLs with `?query` were dropped before F1 (BUG-05; fixed, not observed).
- Fast checks (a small decision model that answers yes/no about the screen) are **off by default**: on a one-model server each decision call evicts the main model and costs 7 to 16 s (BUG-06). The decision answers about real screens were unreliable (A-7).
- The model's own risk label can be wrong; see use case 4.

**Upgrade scope:** a browser surface through rune-kit's `rune-browser` (DOM-level, K1/R9) instead of pixels; a server that keeps both models loaded.

---

## 4. A risky action that waits for your Yes

**Example task:** "Delete the file report-old.txt on the Desktop."

**Features used:** safety gate (rules, the model's stated risk, optional decision-model opinion), confirm card on the pill, voice yes/no, 120 s timeout = No, "Ask before every action" strict mode.

**Flow**

```
model proposes press delete (risk: low/high)  ->  gate assesses  ->  card: "Allow?  Yes / No"  ->
   Yes: focus returns to the window that was in front, action runs     No / 120 s: action refused, the model is told
```

**Recorded (field test):** the card appears (even when the model says risk low), No is honoured, strict mode asks for key/click/open_app and not for scroll/wait. **Before F1, an approved action ran against Rune itself** because answering the card made Rune the foreground window (BUG-02). F1 remembers the window in front, returns focus after Yes and **denies** the action if it cannot (never types into Rune). **This is not yet observed on Windows**: it is the first thing to re-test.

**Limitations**
- The decision-model risk check uses the picked option's probability; "click Pay now" scored 0.587, still under the 0.8 threshold, so it is **not** escalated by that check (a threshold decision is open).
- The 3-No ending, the 2-minute timeout and the spoken yes/no are **Untested**. A spoken yes never approves an "approval"-type prompt, by design.
- Typed secrets are stored in the record (step args and prompts), see use case 8.

**Upgrade scope:** per-app risk policy from notes (R6), redaction of secrets in the record (R8), a dry-run mode that proposes but never acts (R8).

---

## 5. Hands-free voice tasks

**Example:** click the mic once, say "Open Notepad and write hello".

**Features used:** energy-based voice detector, the omni model hears the audio (`input_audio` WAV), transcript shown on the pill, 2 s delay before a spoken task starts, control words (stop, yes, no) act at once.

**Recorded:** a whole WAV as `input_audio` is accepted and heard (A-5). **Understanding is poor**: of 5 saved recordings a speech model got all 5 right, the omni model returned "Open PowerPoint", "Open terminal" and nonsense; a spoken "Open Notepad and write hello" started a Calculator task (BUG-08).

**F1 mitigation (not observed):** a "transcribe only" system prompt, a 256-token cap, repeat-word rejection, every ignored result logged, and a 2 s window to press Stop before a spoken task starts.

**Limitations:** the test used speakers into the microphone; echo cancellation cut recordings to about 0.94 s, so "stop", yes and no **could not be tested with a human voice** (T-R05-3). There is no Cancel button, only the Stop hotkey or tray.

**Upgrade scope:** a dedicated speech-to-text endpoint on rune-infer (the largest improvement), a Cancel button on the transcript, wake word.

---

## 6. An outside agent drives the PC

**Example:** in Claude Code or Antigravity: "Use Rune Studio to open Notepad and type hello."

**Features used:** the MCP bridge at `127.0.0.1:47600` (bearer token in the keyring), `rune-studio-cli connect claude|antigravity`, the tool set `rune_begin`, `rune_plan`, `rune_observe`, `rune_elements`, `rune_click`, `rune_click_element`, `rune_type_text`, `rune_press_keys`, `rune_open_app`, `rune_scroll`, `rune_drag`, `rune_wait`, `rune_status`, `rune_done`; the same safety gate; a lease (one agent at a time) that expires if the agent goes silent; Stop locks the agent out until `enable`.

**Flow**

```
rune_begin(task, agent) -> frame around the screen + top bar "<AGENT> IS IN CONTROL" -> rune_plan -> [rune_observe / action -> new screenshot] x N
   -> rune_done(summary)           user can press Stop at any time: every later call returns stopped_by_user
```

**Recorded (field test, T-R0-5):** begin, actions, Stop gives `stopped_by_user` on every later call, `enable` re-arms, parallel observes did not overlap. The same lock-up that froze built-in tasks also froze `rune_begin` (BUG-01, fixed). `connect claude` was **not run** (it edits the Claude config).

**Limitations**
- The agent's name is whatever it declares; it is not authenticated. The label is a courtesy, not a security boundary, and every outside agent goes through the same gate.
- The verifier does not run for outside agents (T-R3-9). Only `claude` and `antigravity` have a `connect` command; other agents need the printed config (`connect --print`).
- Rune's own windows are hidden from capture, so the outside agent's screenshots never contain them.

**Upgrade scope (R5a, PLANNED):** each known agent gets its accent colour and mark in the orb, ring and top bar, with `connect` attesting the agent id. Next: verifier for outside runs, a per-agent permission level.

---

## 7. "What is on my screen?" analysis

**Example:** press the analysis button on the pill, ask "Summarise what is on the screen and point out anything unusual."

**Features used:** one capture, one model call with a forced `report` tool (up to 8 findings with labels, descriptions, evidence and boxes), a separate analysis window (hidden from capture), stored result (`rune-studio-cli analyses`).

**Recorded (field test):** the window opens in 0.56 s; 14.5 s for 1 finding, 44 to 49 s for 8 (limit 60 s). A file containing a fake key and password: neither value appeared in the answer (T-R4-7). Refused while a task is running, and Stop or closing the window cancels cleanly.

**Limitations**
- **Boxes are unusable**: 1 of 8 was right (BUG-15). Boxes are **off by default**; the findings list and evidence stay.
- Before F1, analysis failed every time (BUG-03, fixed). The server may keep generating after a cancel (BUG-13); the client side is proven to close the connection, the backend side is not.
- Current screen only: no file or drag-and-drop input; no `rune_analyze` MCP tool or voice trigger (by decision).
- The box probe (T-R4-1) was **not run**, so there is no measured accuracy per path.

**Upgrade scope:** compose the server's grounding and OCR endpoints for boxes and text, a before/after diff, files as input.

---

## 8. Review what happened (flight recorder)

**Example:** "Why did yesterday's task fail?"

**Features used:** per-task folders in `Documents/Rune Studio/sessions/<date>/<time>-<n>-<slug>/` (screenshots, voice, `session.json`), a SQLite trace (`runs`, `steps`, `events`, `verdicts`, `analyses`), `rune-studio-cli runs | run <id> | analyses | analysis <id>`.

**Flow**
```
rune-studio-cli runs                list runs: succeeded, stopped, failed, reported_failure, (open)
rune-studio-cli run <id>            steps with prompt, reply, timings, gate verdict, result, verifier verdict
```
**Recorded (field test):** all outcome kinds appear; session folder names equal run ids; 561 CLI reads during a live task with 0 failures; about 0.15 MB per run (80 runs = 11 MB of files and a 270 KB database plus a 4.1 MB WAL). A crash mid-task leaves the run as `(open)`.

**Limitations:** the record cannot be switched off. Typed text, including a password the user dictated, is stored in step args and prompts, and the control list includes every open browser tab title (T-R1-5). Retention is opt-in (`archiveKeepDays`). The write queue drops rows under heavy pressure (the pressure test was not run). Server `usage` token counts are unreliable (BUG-12; the eval estimates from characters).

**Upgrade scope:** redaction and export/delete (R8), a UI browser for runs, post-mortems built from this data (R6).

---

## 9. Measure quality (eval harness and verifier)

**Example:** "Did this change make the agent worse?"

**Features used:** the `eval` Cargo feature (not in releases), a 15-task suite with checkers independent of the model's claim, `scripts/eval/run.ps1`, `scripts/eval/report.mjs` with a gate (success not lower by more than one run, median duration within 10%); the verifier (`verifyMode` off, **background** default, blocking) that checks a claimed success against the last screenshot.

**Recorded (field test):** one partial baseline, fast checks off: 6 of 15 passed, 4 of 13 real tasks, median 45 s and 7 steps. **Not a baseline**: the stuck detector (BUG-04) and the `open_app` query bug (BUG-05) distorted it and the checkers disagreed with run outcomes in places (T-R2-5). Verifier: 7 verified, 2 false contradictions, 1 inconsistent on Notepad (BUG-11); blocking mode returned `verified` in 3.5 s; Stop and a new task cancel a pending check.

**Limitations:** the full 45-run baseline, the three-mode verifier suite and the replay mode are **NOT RUN**. All of F1 changed what is measured, so the baseline must be taken fresh on a machine with the model.

**Upgrade scope:** a committed baseline per model, CI-gated releases on the eval, a replay mode against stored episodes.

---

## 10. Record a demo (PLANNED, R5b)

**Today:** the pill, overlay and analysis window are excluded from all screen capture on purpose (so the model never reads its own UI), which also makes Snipping Tool, Game Bar and OBS unable to see them.

**Plan:** use an external recorder (OBS or Game Bar). A **Demo mode** setting (default off) lets recorders see Rune's windows while Rune hides or masks them from the model's own screenshots. The handoff requires measuring the chosen method (flicker or mask error) and an eval run proving demo mode does not hurt success or speed. A built-in recorder is deliberately not planned.

**Limitations (expected):** OBS window capture does not see the click-through overlay; use display capture. Exclusive-fullscreen apps may not record. All of this is **Untested** until R5 lands.

---

## 11. Idle self-review (PLANNED, R5c then R6)

**Plan:** after 10 idle minutes the pill asks "Rune wants to review its recent tasks" with Start and Deny (30 s, then it starts). A `Reflecting` state with a calm orb motion shows progress; any task, hotkey, Stop or outside agent cancels it within a second. R5 ships a stub job; R6 adds post-mortems (cause category, evidence steps, a proposed note) and **pending notes** you accept; R7 feeds accepted notes into the prompt and keeps them only if an A/B on the eval shows no loss. Off by default until R6.

**Limitations (known in advance):** rune-infer has no request priority and a default of one parallel request, so a reflection call already running **will delay** the next task until it ends. Autonomy is notes and reports only; prompts and safety thresholds change only through an approved diff.

---

## What is common to all of them

- One action per model turn, coordinates 0..1000, every action through the safety gate.
- Windows 11 only for capture, input and UI Automation.
- Quality depends on the model served by rune-infer; a one-model server swaps models for any second model call (decisions, voice, verifier), which is why those paths are slow or off by default.
- Everything the agent sees and hears is stored on disk.

## Capturing a real example run for this document

Replace any flow above with a recorded one: run the task, then

```
rune-studio-cli runs
rune-studio-cli run <id>        # steps, prompts, timings, gate verdicts, verifier verdict
```

and paste the step table with the run id and date. Do not edit numbers.
