# Rune Studio — Use Cases Guide

> Consolidated from 12 core workflow use-cases and scenarios.

---

## Use Case 01-open-app-and-type

**Status:** Works.

## What it is

You give Rune a task in words. Rune opens the app, clicks where needed and types the text for you.

## How to use

1. Click the orb to open the pill.
2. Type a task in the box, for example `Open Notepad and write hello`.
3. Press Enter, and take your hands off the mouse and keyboard.
4. To stop at any moment, press Ctrl+Alt+Esc (the default Stop hotkey).

## What happens

Rune repeats a loop until the task is done:

1. It makes a short plan of one to three goals.
2. It takes a screenshot and reads the list of controls on the screen.
3. It picks one action, for example "open Notepad".
4. The safety gate checks the action. Low-risk actions go straight through.
5. The action runs, then Rune waits about 2 seconds for the screen to settle.
6. It goes back to step 2, for example to "type hello", until it reports done.

## What you will see

A frame around the screen, a short caption for each action, a ripple where it clicks, and the plan shown as arcs around the orb.

## How well it works

Measured on Windows 11: 55 seconds and 6 steps, and Notepad contained "hello". Two of the six steps were wasted because the stuck detector fired wrongly (fixed since, not yet re-measured). Stop took 420 ms. Per step: looking at the screen 0.3 s, acting 0.04 s, settling 1.8 s, the model about 4.5 s.

## Limits

- With thinking switched on it is about 4 times slower (runs of 93 to 169 s), so thinking is off by default.
- Rune does one action per model call, so long tasks take minutes.
- Some Store apps cannot be opened by name yet.
- Not tested: apps run as administrator, display scaling of 125% or 150%, a second monitor.

## Where it can grow

Faster steps, remembered tips per app (for example "Notepad restores old tabs"), more than one monitor.

## Features used

Text input, planning, screenshot and control list, the action loop, the safety gate, stuck detection, the Preview.

---

## Use Case 02-system-settings

**Status:** Works.

## What it is

Rune opens a built-in Windows app, such as Settings or Calculator, and operates it for you.

## How to use

1. Click the orb to open the pill.
2. Type one task in the box. Two examples: `Open Settings`, or `Open the calculator and compute 12 times 13`.
3. Press Enter, and take your hands off the mouse and keyboard.
4. To stop at any moment, press Ctrl+Alt+Esc.

## What happens

1. Rune makes a short plan.
2. It opens the app by name ("settings" is turned into the Windows Settings address, "calculator" into Calculator).
3. It takes a screenshot and reads the list of controls in the front window.
4. It clicks a control by its id from that list, which is exact, instead of guessing a position on the picture. It also uses key presses.
5. The safety gate checks each action, and Rune repeats from step 3 until the task is done.

## What you will see

The app opens and its buttons are pressed one by one, with a caption for each action and a frame around the screen.

## How well it works

Measured: both examples passed, and the end-of-task check said "verified" with correct evidence.

## Limits

- Before the F1 fixes, `Open Settings` and `Calculator` could not be opened by name (error 2). Fixed since, not yet seen working on Windows.
- A Windows admin-permission (UAC) prompt cannot be seen or answered by Rune.
- A locked or sleeping PC stops the task with "screen unavailable".

## Where it can grow

A list of known apps with their shortcuts, and direct tools (through rune-kit) that change a setting without clicking.

## Features used

App opening with aliases, control ids, key presses, the safety gate, the end-of-task check.

---

## Use Case 03-browse-the-web

**Status:** Partly works.

## What it is

Rune opens a web page in your default browser and works on it: closes a popup, clicks a small button, scrolls.

## How to use

1. Click the orb to open the pill.
2. Type a task that names the page, for example `Open example.com and close the cookie banner`.
3. Press Enter, and take your hands off the mouse and keyboard.
4. To stop at any moment, press Ctrl+Alt+Esc.
5. Optional: in Settings you can switch on "fast checks". Leave them off on a server that holds only one model (see Limits).

## What happens

1. Rune opens the address in the browser.
2. It takes a screenshot. Web pages show few controls to Windows, so Rune mostly clicks by position on a grid from 0 to 1000. It can ask for a zoomed picture of one region to aim better.
3. It picks one action, the safety gate checks it, and it runs.
4. Rune repeats from step 2 until the task is done or it is stuck.

## What you will see

The page opens, with a ripple for each click and a caption for each action.

## How well it works

Measured: the "close a popup" and "click a small target" test tasks passed. In one partial test run only 4 of 13 real tasks passed, but two bugs distorted that run, so it is not the true pass rate. In an earlier probe, small buttons were hit 57 to 79 % of the time.

## Limits

- Small targets are the weak spot.
- Web addresses with a `?query` lost the query before the F1 fixes (fixed since, not yet seen working).
- Fast checks use a small second model. On a server that holds one model at a time, each check swaps models and costs 7 to 16 seconds, and its answers on real screens were unreliable. They are off by default.

## Where it can grow

Control pages through the page structure (rune-kit's browser tool) instead of pixels, and a server that keeps both models loaded.

## Features used

App opening with an address, click, scroll, zoomed screenshot, stuck detection, optional fast checks.

---

## Use Case 04-risky-actions

**Status:** Partly works. The main fix has not yet been seen working on Windows.

## What it is

Before a risky action (deleting, paying, sending) Rune stops and asks you to approve it.

## How to use

1. Optional: in Settings turn on "Ask before every action" if you want a question for nearly every action.
2. Click the orb to open the pill.
3. Type a task that includes a risky step, for example `Delete the file report-old.txt on the Desktop`.
4. Press Enter. When the card appears on the pill, click **Yes** to allow the action or **No** to refuse it. You can also say "yes" or "no".
5. To stop the whole task, press Ctrl+Alt+Esc.

## What happens

1. The model proposes an action and says how risky it thinks it is.
2. The safety gate decides from fixed rules, the model's own label and, if enabled, a second opinion from a small model.
3. If the action is risky, the card asks "Allow?".
4. Yes: Rune returns focus to the window that was in front, then runs the action. No, or no answer in 120 seconds: the action is refused and the model is told.

## What you will see

A card with the question and a countdown, then either the action or a refusal.

## How well it works

Measured: the card appears, No is respected, and strict mode asks for key presses, clicks and opening apps but not for scrolling or waiting.
A bug was found: after Yes, Rune itself became the front window, so the approved action hit Rune and not your file. The F1 fix returns focus and refuses the action if it cannot. This has not been seen working on Windows and should be the first thing re-tested.

## Limits

- The second-opinion check uses the probability of the chosen answer. "Click Pay now" scored 0.587, below the 0.8 limit, so that check alone would not ask.
- Not tested: three No answers in a row, the 2-minute timeout, answering by voice. A spoken "yes" never approves an approval-type prompt, by design.
- Text you type is stored in the record, including passwords.

## Where it can grow

Risk rules per app, hiding secrets in the record, a dry-run mode that only proposes actions.

## Features used

Safety gate, confirm card, strict mode, spoken yes and no.

---

## Use Case 05-voice

**Status:** Partly works. The model often mishears.

## What it is

You speak a task instead of typing it. Rune listens hands-free once you click the microphone.

## How to use

1. Connect a microphone, click the orb to open the pill, then click the microphone once.
2. Say the task, for example "Open Notepad and write hello".
3. Read your words on the pill. The task starts after 2 seconds. To cancel it in that time, press Ctrl+Alt+Esc.
4. While a task runs you can say a control word: "stop", "yes" or "no". These act at once.

## What happens

1. A detector cuts your speech into separate utterances.
2. The same model that reads the screen hears the audio (there is no separate speech-to-text model) and writes down the words.
3. A control word acts immediately. A task is shown on the pill and then starts like a typed task.

## What you will see

The orb follows your voice level, then your words appear on the pill.

## How well it works

Measured: the audio is accepted and heard, but understanding is poor. Out of 5 recordings a speech model got all 5 right, while this model returned "Open PowerPoint", "Open terminal" and nonsense. "Open Notepad and write hello" once started a Calculator task. Since then the F1 fixes added a "transcribe only" instruction, a length cap, rejection of repeated words, logging of every ignored result and the 2-second window (none seen working yet).

## Limits

- Only tested with speakers into the microphone. "Stop", "yes" and "no" with a real voice have not been tested.
- There is no Cancel button. Use the Stop hotkey or the tray menu.

## Where it can grow

A real speech-to-text endpoint on rune-infer (the biggest gain), a Cancel button, a wake word.

## Features used

Voice detector, audio sent to the model, the 2-second delay, control words.

---

## Use Case 06-outside-agents

**Status:** Works. Agent colours and icons are planned (milestone R5).

## What it is

An AI tool such as Claude Code or Antigravity uses your PC through Rune, with the same screen frame and the same safety gate as Rune's own agent.

## How to use

1. Connect the tool. In a terminal run `rune-studio-cli connect claude` (or `rune-studio-cli connect antigravity`), then restart that tool.
2. Allow outside agents: switch on **Allow outside agent** in Rune Studio's Settings, or run `rune-studio-cli enable`.
3. In the tool, ask for the task, for example `Use Rune Studio to open Notepad and type hello`.
4. Watch the screen. Approve or refuse any card that asks about a risky action.
5. To take control back, press Ctrl+Alt+Esc. The tool is locked out until you run `rune-studio-cli enable` again.

## What happens

1. The tool starts with `rune_begin`. A frame appears and the top bar says who is in control.
2. It sends a plan, then repeats: look (`rune_observe`) and act (click, type, keys, open an app, scroll, drag, wait). Each action returns a fresh screenshot.
3. Risky actions wait for your Yes.
4. It ends with `rune_done` and a summary.

## What you will see

The frame, a bar reading "<agent> is in control" with the current action, ripples on clicks and a red Stop button.

## How well it works

Measured: begin, actions, Stop, re-enabling and overlapping looks all behaved. Connecting Claude was not run, because it edits your Claude settings.

## Limits

- The agent's name is whatever it says it is. It is not proven, and every agent gets the same safety checks.
- Only one agent works at a time. A silent agent loses control after a while.
- The end-of-task quality check does not run for outside agents.

## Where it can grow

Milestone R5 gives each known agent its own accent colour and icon in the orb and the top bar. Later: a quality check for outside runs and permissions per agent.

## Features used

The agent bridge at 127.0.0.1:47600, the access token, the safety gate, the one-at-a-time lease, the screen frame.

---

## Use Case 07-screen-analysis

**Status:** Built, not seen on a real screen. The boxes it draws on your screen are always on, and they are often wrong until the probe is run (see Limits).

## What it is

Rune looks at your screen once and answers a question about it, with a list of findings and the evidence for each.

## How to use

1. Make sure no task is running, then click the orb to open the pill.
2. Click the scan button on the pill. A bar appears at the top of the screen and cards grow upward from the orb.
3. Optionally type a question in the first card, for example `Summarise what is on the screen and point out anything unusual`, and press Enter. An empty question means "describe it".
4. Read the answer, then open the findings one by one: each shows its description and the evidence, and its box on the screen goes to full strength. To cancel, press the red Cancel button on the card or Ctrl+Alt+Esc.
5. Escape collapses the open finding, then clears the selection, then closes the cards (which also clears the boxes).
6. To read a saved result later, run `rune-studio-cli analyses`, then `rune-studio-cli analysis <id>`.

## What happens

1. Rune takes one screenshot, and a scan line sweeps the screen once.
2. One model call returns up to 8 findings, each with a label, a description, evidence and, if it could, a box.
3. A guard drops boxes that are plainly unusable (tiny, huge, off-screen, duplicates, or a set that tiles the whole screen). Findings stay.
4. Dotted boxes with numbered badges are drawn on your real screen. If the screen changes (another window, or more than 5% of it moves) the boxes fade out for good and the cards say SCREEN CHANGED.
5. The result is saved in the record.

## What you will see

A black bar at the top (the same style as the control bar) with the stage and a timer, a dot skeleton while it works, then the answer (four lines, MORE for the rest) and a list of findings. The answer arrives after 15 to 49 seconds. Everything Rune draws is hidden from screenshots.

## How well it works

Measured: 14.5 s for 1 finding and 44 to 49 s for 8 (the limit is 60 s). A file with a fake key and password did not leak either value into the answer. It refuses to start while a task is running. Closing the window or pressing Stop cancels it cleanly.
Problem found: only 1 of 8 boxes was in the right place. Before the F1 fixes the whole feature failed every time (fixed since).

## Limits

- Current screen only. No files, no drag and drop, no voice or outside-agent trigger.
- After a cancel the server may keep working for a while (a rune-infer issue).
- **The boxes cannot be turned off, and their accuracy is unmeasured.** In the field test 1 of 8 was right. The guard removes boxes that are plainly implausible but cannot know a plausible one is wrong. The former probe runner has been retired.
- Boxes are drawn on the primary monitor only.

## Where it can grow

Better boxes by combining the server's grounding and text-reading tools, comparing two screens, files as input.

## Features used

One screenshot, a forced report tool, the screen overlay and the pill's cards, the saved record.

---

## Use Case 08-review-what-happened

**Status:** Works.

## What it is

Everything Rune does is recorded, so you can check later what happened and why.

## How to use

1. Run any task first.
2. To watch it, click **Review** on the card that appears when it ends, or see [12 Review a session](12-review-a-session.md).
3. In a terminal run `rune-studio-cli runs` to list recent tasks.
4. Pick an id from the list and run `rune-studio-cli run <id>` to see its steps, or `rune-studio-cli run <id> --calls` to also list every model attempt (retries included).
5. To look at the pictures, open the task's folder: `Documents\Rune Studio\sessions\<date>\<time>-<n>-<name>\`.

## What happens

1. Each task gets its own folder with screenshots, voice clips and a `session.json`.
2. A database (`trace.sqlite3`) holds every run and step: the prompt, the reply, timings, the safety decision, the result and the quality check, and every model call with its raw request and response (images and audio are replaced by their size).
3. A task that was cut off by a crash is listed as "(open)".

## What you will see

A table of runs marked succeeded, stopped, failed or reported_failure. For one run, each step with its timings and outcome.

## How well it works

Measured: all outcome kinds appear, folder names equal run ids, 561 reads during a live task had no failures, and a run takes about 0.15 MB (80 runs were 11 MB of files plus a 270 KB database).

## Limits

- The record cannot be switched off.
- Text you typed is stored, including a spoken password. The titles of open browser tabs are stored too. So are the raw model requests and replies (cut at 64 KB) and the session video.
- Old folders are deleted only if you set a retention period.
- Under heavy load the database may drop rows (not tested).
- The server's token counts are unreliable, so the evaluation estimates them.

## Where it can grow

Hiding secrets, export and delete, automatic post-mortems.

## Features used

Session folders, the SQLite trace, the command-line tool, the Preview.

---

## Use Case 09-measure-quality

**Status:** Partly works. There is no trustworthy baseline yet.

## What it is

Two tools. An evaluation suite runs 15 fixed tasks and checks the real result. A verifier looks at the final screen after a task and says whether the claimed success is true.

## How to use

Evaluation suite:

1. Build Rune Studio with the `eval` feature (it is not in releases).
2. Make sure the model server is running and no Notepad or Calculator is open.
3. Run `scripts/eval/run.ps1`.
4. Compare two runs with `node scripts/eval/report.mjs`.

Verifier:

1. Open Rune Studio's Settings.
2. Set `verifyMode` to `off`, `background` (the default) or `blocking`.
3. Run a task. A notice appears only when the check contradicts the claimed success. Verdicts are in `rune-studio-cli run <id>`.

## What happens

1. The suite sets up a fixture, runs the task, and a checker that does not trust the model decides pass or fail.
2. The report applies a gate: success may not drop by more than one run, and median time may not be more than 10% worse.
3. The verifier asks the model "does the final screen match the claim?" and records verified, contradicted or unsure.

## What you will see

A results folder and a comparison table for the suite. A short notice for the verifier, only on a contradiction.

## How well it works

Measured: one partial baseline (6 of 15 passed) that is not valid, because two bugs distorted it. Verifier on Notepad: 7 verified, 2 false contradictions, 1 inconsistent (it called Notepad "VS Code"; the F1 fixes now give it the real window title, effect not measured). Blocking mode took 3.5 s. Stop and a new task cancel a pending check.

## Limits

- The 45-run baseline, the three-mode verifier comparison and replay have not been run.
- Everything the F1 fixes changed must be re-measured on a machine with the model.

## Where it can grow

A committed baseline per model, a release gate in CI, replay of stored runs.

## Features used

The eval feature, checkers, the report script, the verifier.

---

## Use Case 10-record-a-demo

**Status:** Planned (milestone R5). Nothing here is built yet.

## What it is

Make a screen video of Rune working, using an ordinary recorder such as OBS or Windows Game Bar.

## How to use (when built)

1. Open Rune Studio's Settings and switch on **Show Rune in screen recordings**.
2. Start OBS with **Display Capture** (or start Game Bar).
3. Run a task in Rune.
4. Stop the recording.
5. Switch the setting off again when you are done.

## The problem today

Rune hides its pill and its screen overlay (frame, bar and analysis boxes) from all screen capture, so the model never reads its own interface. That also hides them from Snipping Tool, Game Bar and OBS.

## What happens (planned)

1. The setting switches off the hiding for the three Rune windows, so recorders see them.
2. Rune still keeps its windows out of the screenshots the model gets, by hiding or masking them. The method is chosen by measuring.
3. An evaluation run with the setting off and on must show no loss in success or speed.
4. A built-in recorder is not planned. It would need a video encoder, audio and storage, and the same hiding problem would remain.

## What you will see (planned)

The pill, frame, captions and ripples in your video.

## How well it works

Not measured. It is not built.

## Limits (expected)

- OBS window capture does not see the click-through frame, so use display capture.
- Exclusive-fullscreen apps may not record.

## Where it can grow

A recording-ready mode with a larger pill and a timer.

## Features used (planned)

A new setting, the capture-hiding switch for Rune's windows, the screenshot masking.

Specification: the "Agent identity, demo mode and the idle review" section of `docs/ARCHITECTURE.md`.

---

## Use Case 11-idle-reflection

**Status:** Planned (milestones R5 and R6). Nothing here is built yet.

## What it is

When you have not used Rune for a while, it offers to review its recent tasks and write notes about what went wrong.

## How to use (when built)

1. Open Rune Studio's Settings and switch on reflection. It is off by default.
2. Optional: set the idle time (default 10 minutes).
3. Leave the PC alone. When the time passes, the pill asks "Rune wants to review its recent tasks" with **Start** and **Deny**.
4. Click **Start**, click **Deny** to skip this round, or do nothing: with no answer in 30 seconds it starts.
5. Later, read the proposed notes and accept or reject each one.
6. To interrupt a review, start a task, press Ctrl+Alt+Esc or use the hotkey. It stops within a second.

## What happens (planned)

1. The pill shows the question. A calm "Reflecting" orb and a progress ring show the work.
2. In R5 the review is an empty shell. In R6 it studies failed or slow runs, finds the likely cause and proposes a pending note.
3. In R7, accepted notes are added to the prompt, and kept only if an A/B test shows no loss.

## What you will see (planned)

The question card, the calm orb, a progress ring, then a list of pending notes.

## How well it works

Not measured. Whether the notes actually help is unknown until R7 measures it.

## Limits (expected)

- rune-infer has no request priority, so a review already running can delay your next task until it ends.
- Notes and reports only. Prompts and safety limits change only through a change you approve.

## Where it can grow

Skills built from repeated successful runs, safety limits tuned from your Yes and No answers, choosing a model per task.

## Features used (planned)

The idle timer, the Reflecting state, the record of past runs, notes storage.

See the roadmap: `docs/ROADMAP.md` (R5 to R7).

---

## Use Case 12-review-a-session

**Status:** Built and tested in code. Never seen on a real Windows screen, and the video encoder has never run there.

## What it is

When a task ends, Rune offers to replay it: a video of the session, a timeline of every step, and the details behind each step (what the model was told, what it answered, how safe the action was judged, and every try of every model call, retries included).

## How to use

1. Run any task. When it ends, a small card appears above the orb with the task, how it ended and the time and step count.
2. Click **Review**. Click **Dismiss**, or wait 15 seconds: the dots on the card run out and it goes away.
3. Preview opens inside Workspace. Press **Space** to play or pause, **Left** and **Right** to go to the previous or next step, **J** and **K** to slow down or speed up.
4. Click or drag on the timeline to jump. Roll the mouse wheel over it to zoom in.
5. Press **D** (or click **Details**) to open the drawer. Press **T** to move through its tabs. Click **Raw** to see the stored JSON, and **Copy** to copy it.
6. Press **A** to hide or show the small overlay of what Rune did (the click ripple, the typed text, the keys).
7. Typed text is hidden. Click **Typed text: hidden** (or **Show** next to the arguments) only when nobody else can see the window.
8. Press **Esc** to close the drawer, then again to close the window.
9. To look at an older task, open **Workspace** from the tray icon's menu and pick it from task history.

## What happens

1. When the task ends, a watcher notices the finished run and shows the card. It is hidden while another task runs, and a confirmation or the idle-review question comes first.
2. After the task, once nothing else is running, Rune builds `session.mp4` in the session folder from the screenshots the model looked at, each held for the time it was current. This is a **storyboard**, not smooth motion.
3. The player reads the record (it can only read, never change it) and plays the video. The overlay is drawn from the recorded steps, not burned into the video.
4. If there is no video (an older task, or no encoder on this PC), the player shows each step's screenshot instead and plays them on a clock.

## What you will see

A window with the task and its outcome on top, the picture in the middle, play and speed controls, a track of dots where each step is as wide as it really lasted, one sentence about the current step, and, on demand, a side drawer with the tabs Step, Reasoning, Prompt, Safety, API, Timing, Tools and Files. A tab appears only when the record has something for it. The API tab lists every model attempt with its status, time, wait and outcome; open one to read the request and the reply. A deleted screenshot or a missing video gives a one-line message, not an error.

## How well it works

Not measured on Windows. Measured only in a headless Chromium with a generated WebM file: the Video.js player loaded, played and seeked in 25 to 60 ms per jump. That says nothing about the Windows webview, H.264, or the size of a real video.

## Limits

- The video is a storyboard of about one frame per step. Step times have one-second resolution, so the picture and the timeline agree to about a second.
- The video encoder is Windows Media Foundation. It has not been run, and may be missing on "N" editions of Windows.
- Model requests made by the optional decision model, by voice and by steering are not in the API tab. Kit tool calls and skills have no data yet.
- Images are not kept in the stored request (only their size); the step's screenshot is shown instead.
- The record, including the video and the model bodies, can show anything that was on your screen. The window is hidden from screen capture unless demo mode is on.
- `recordSessionVideo` and `offerReview` are in `settings.json` only (no Settings page yet).
- Full-motion recording (`recordSessionVideo: "motion"`, 2 frames per second) is off by default and its cost is not measured.

## Where it can grow

Export a session, notes on a step, a comparison of two runs, post-mortems written from the record (R6c).

## Features used

The SQLite trace (`runs`, `steps`, `events`, `calls`), session folders, the video encoder, the asset protocol, Video.js v10, the capture exclusion of Rune's windows.

---

## Overview

One page per use case. Each page stands alone and has the same sections in the same order: What it is, How to use (always numbered steps), What happens, What you will see, How well it works, Limits, Where it can grow, Features used.

| Page                                                       | What you do                             | Status                     |
| ---------------------------------------------------------- | --------------------------------------- | -------------------------- |
| [01 Open an app and type](01-open-app-and-type.md)         | "Open Notepad and write hello"          | Works                      |
| [02 System settings and calculator](02-system-settings.md) | "Open Settings", "12 times 13"          | Works                      |
| [03 Browse the web](03-browse-the-web.md)                  | "Close the cookie banner"               | Partly works               |
| [04 Risky actions need your Yes](04-risky-actions.md)      | "Delete that file"                      | Partly works               |
| [05 Talk to it](05-voice.md)                               | Say the task out loud                   | Partly works               |
| [06 Let another AI drive](06-outside-agents.md)            | Claude Code or Antigravity uses your PC | Works                      |
| [07 Ask about your screen](07-screen-analysis.md)          | "What is on my screen?"                 | Partly works               |
| [08 See what it did](08-review-what-happened.md)           | "Why did that fail?"                    | Works                      |
| [09 Measure its quality](09-measure-quality.md)            | "Did my change make it worse?"          | Partly works               |
| [10 Record a demo](10-record-a-demo.md)                    | Make a screen video                     | Planned (R5)               |
| [11 Idle self-review](11-idle-reflection.md)               | It reviews its own work when idle       | Planned (R5, R6)           |
| [12 Review a session](12-review-a-session.md)              | Watch what it did, step by step         | Built, not seen on Windows |

## How to read the evidence

- **Measured** numbers come from the field test of 2026-10-19: Windows 11, one 1920x1080 monitor at 100%, rune-infer 0.6.0 with one model loaded at a time, model `gemma-4-12b-it`, a development build.
- That test ran **before** the fixes in milestone F1. Where F1 changed something, the page says "fixed, not yet seen working on Windows".
- **Not measured** means nobody has tried it yet. **Planned** means it is not built.
- The step-by-step flows are explained from the code, not copied from a recording.

## Things true for every use case

- Rune picks **one action at a time**: look at the screen, choose an action, check it with the safety gate, do it, repeat.
- Works on **Windows 11** only.
- How good it is depends on the model that rune-infer serves.
- Everything Rune sees and hears is **stored on your disk**.
- You can press **Stop** (Ctrl+Alt+Esc by default) at any moment.

## Add a real example run

Run the task, then:

```
rune-studio-cli runs
rune-studio-cli run <id>
```

Paste the step table into the page with the run id and date. Do not edit the numbers.

---