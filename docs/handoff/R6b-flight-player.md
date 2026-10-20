# Handoff: Milestone R6b, session video, review prompt and the Flight Player

For an AI coding agent. Read all of it first. Plan it comes from: `docs/ROADMAP.md` (R6b). Current behaviour: `docs/ARCHITECTURE.md`, `docs/use-cases/08-review-what-happened.md`. Where this file and the code disagree about a fact, the code wins: say so in the report. R6a (analysis overlay) is already merged (see `docs/handoff/R6a-report.md`); R6c (post-mortems and notes) comes after this and will link into the player.

## 1. Goal

When a task ends (success or failure, own agent or outside agent), the pill offers **Review**. Review opens the **Flight Player**: a separate, minimal window that replays the session from what the flight recorder captured:

- a **video** of the session in a **Video.js v10** player,
- a **timeline** of steps and events under it,
- a **details drawer** that goes as deep as the user wants (reasoning, prompt, gate decision, API requests and responses with retries, tool calls, timings, files), closed by default so the first view stays calm.

## 2. What exists today (verified in the code) and what does not

| The user asked for | Today |
| --- | --- |
| Session step count | Derivable: rows in `steps`. Not stored as a number. |
| Session duration | Derivable: `runs.started_utc`, `runs.ended_utc`. |
| Each step's duration | Yes, as phases: `observe_ms`, `screen_check_ms`, `finish_check_ms`, `decide_ms`, `gate_ms`, `act_ms`, `settle_ms`. |
| Retries | **No.** `model/client.rs::call_retrying` retries (3 tries, waits 500 ms and 1500 ms) but nothing is stored: not the attempts, the waits or the errors. |
| A video of the session | **No.** There is no video encoder or video file anywhere. Only one JPEG per step (`steps.observation_path`) in the session folder. |
| Review prompt after a task | **No.** |
| A player | **No.** The CLI (`rune-studio-cli run <id>`) prints steps. |
| Reasoning | **Partly.** `steps.thinking_chars` (a count) only; the text is not stored. |
| API request and response | **No.** Only `prompt_text` (the user message), tool, args, `completion_tokens` (unreliable per the field test). |
| MCP tool calls | Outside-agent `rune_*` calls are recorded as steps (`source` = the agent). Kit tool calls do not exist yet (K1 is not built). |
| Skill use | **Does not exist.** Studio has no skill concept; rune-kit does not inject skills into any model (roadmap, NOT CERTAIN how it would be consumed). The track must exist in the player only when data exists. |
| Events | Yes, a generic `events` table (`plan`, `steer`, `stuck_warning`, `invalid_answer`, `ask_user`, `note`, `elevated_warning`, ...). |

## 3. Preconditions and rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop`, then `git push origin develop`.
2. R6a is merged (`src/components/AnalysisCard.vue` exists); read `docs/handoff/R6a-report.md` for its card and window patterns before you start.
3. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (all test commands, clippy, fmt).
4. Trace schema is at `VERSION = 5`. Add migration **v6** (never edit v1 to v5); a database from a newer build is left untouched as today.
5. Recording must never slow or break a task: writes stay on the existing writer thread with `try_send` (drop under pressure, count drops), the video is made **after** the task ends, and any failure in recording or encoding is logged and never changes the task outcome.
6. Never claim a live behaviour you did not observe. The pill and the player cannot be screenshotted (capture-excluded), so test with unit and component tests and write exact live steps for the maintainer.
7. Design: the repo's Nothing-style language (Oxanium, dot-matrix details, `#141414` surfaces, `#f1f1ef` light, `#d71921` only for failure and errors, `text-caption/label/status` scale, no gradients or shadows, reduced motion respected).

## 4. What is NOT CERTAIN (measure or ask, do not assume)

- **Video.js v10 status.** Searched on 2026-10-20: the official docs describe v10 as in beta; the install page shows `@videojs/react@10.1.0`; Vue has **no dedicated package**: it uses `@videojs/html` (web components) and needs the Video.js tags listed in Vue's `compilerOptions.isCustomElement` (list the specific tags, no wildcard), with player state read from a template ref after mount. Mux's post says Vue composables are planned; shipped status unknown. Before writing code: check npm for the current `@videojs/html` version, pin an exact version at least two weeks old, read its install guide and the Vue guide (`https://videojs.org/docs/framework/html/how-to/use-videojs-with-vue`), and write the pinned version and the reason in the report. If Video.js v10 cannot be made to work in this Vue/Vite/Tauri setup, stop and report; do not silently substitute another player.
- **Local files in the Tauri webview.** Whether the asset protocol serves `session.mp4` with byte-range requests so seeking works, and the CSP/scope needed. Measure with a test file.
- **Windows H.264 encoder.** Media Foundation's H.264 encoder is part of Windows 11 but is missing on "N" editions without the Media Feature Pack. Whether the `windows` crate bindings for `IMFSinkWriter` are enough (the repo already depends on `windows 0.62` with some features), or a different encoder is needed, is **NOT CERTAIN**: WP2 starts with a spike. No new binary dependency (such as bundling ffmpeg) without writing the case in the report and asking.
- CPU, disk and battery cost of continuous capture (WP2b).
- Whether WebView2 decodes the encoded file on every Windows 11 machine (H.264 baseline/main in MP4 is the safest; say what you chose).

## 5. Work packages

### WP1: record what is missing (schema v6)
- New table `calls` (one row per model or tool **attempt**): `id` integer pk, `run_id`, `step` (nullable for plan/verify/analysis), `kind` (`plan`, `decide`, `verify`, `analysis`, `decision`, `voice`, `steer`), `attempt` (1-based), `started_utc`, `duration_ms`, `wait_before_ms` (the backoff that preceded this attempt), `status` (HTTP code or null), `outcome` (`ok`, `retried`, `failed`, `cancelled`, `timeout`), `error`, `request_json`, `response_json`, `truncated` (0/1), `model`, `prompt_tokens`, `completion_tokens` (as the server said; keep marked unreliable).
- `call_retrying` reports every attempt through a small callback/trait so `model/client.rs` stays free of trace code; the agent loop, verifier, analysis, decisions and voice pass the recorder. Tested with a fake server: one 503 then 200 gives two rows (attempt 1 `retried` with `wait_before_ms` 0, attempt 2 `ok` with the wait).
- **Redaction and size (mandatory).** Images (`data:` URLs, base64 audio) are replaced in `request_json` by a reference to the stored file path or `"[image n bytes]"`; the `Authorization` header and any API key are never stored; each body is capped at 64 KB and `truncated` set. Tests with a body that contains a data URL, a key-shaped header and a 200 KB prompt.
- Reasoning: store the model's reasoning text (when the reply carries it) in `response_json` as received and add `steps.reasoning_chars` consistently with `thinking_chars`; do not duplicate large text elsewhere.
- Step counters on `runs`: `step_count`, `retry_count`, `duration_ms` filled when the run ends (so the review card and the CLI do not recompute). Backfill nothing; older runs compute them on read.
- Retention: `calls` and the video follow the existing `archiveKeepDays` rules (delete with the session).
- Tests: migration v5 -> v6, attempt rows, redaction, truncation, counters, retention.

### WP2a: the session video (default, always on)
- After a session ends (own agent, outside agent, stopped or failed), a background job builds `session.mp4` in the session folder from the **real screenshots** of the session (each step's observation, plus the last frame after the final action if one was captured). Every frame is held on screen for its **real elapsed time** (taken from the step timestamps), so the video's length equals the session's length. 1280-wide at most, H.264, a very low frame rate with held frames; expected size is small (measure and report MB per minute).
- This is a storyboard of the moments the model looked at the screen (about one frame per 7 s), **not smooth motion**. Say that in the UI ("Frames from each step") and in the docs.
- Encoder spike first (see section 4). If no encoder is available, no video is written, the run is marked `video: unavailable` in `session.json`, and the player falls back to the still-frame viewer in WP5 (this fallback is required anyway, it also serves runs from before this milestone).
- Never encode while a task is running; wait for idle, cancel on a new task start, resume later. One job at a time.
- `session.json` gains `video: {file, frames, seconds, kind: "storyboard"}`.

### WP2b: full-motion recording (opt-in, measurement first)
- Setting `recordSessionVideo`: `off` (default) or `motion`. When `motion`, capture the primary monitor at 2 frames per second for the whole session and encode the same way. Honest default is off because it records everything on screen between steps (more than the archive records today) and costs CPU and disk.
- Do the spike and **measure** on the maintainer's machine settings: extra CPU %, disk MB/min, and whether the agent's own observe time (median 312 ms in the field test) gets worse. Gate: observe median must not get more than 10% slower and the eval gate (success not lower, median task time within +10%) must pass with `motion` on. Give the maintainer the exact commands; claim nothing.
- DECIDE: default `off`. Say if you recommend changing it.

### WP3: the review prompt
- When a session ends, the pill shows a compact card above the orb (same pattern as `ReflectionCard.vue`): the task text clipped to one line, an outcome chip (SUCCEEDED / FAILED / STOPPED), `0:55 . 6 STEPS`, and two buttons: **Review** and **Dismiss**. It disappears by itself after 15 s (a thin dot countdown, no text), never covers a confirm card, and is replaced by the next card if one arrives (priority: confirm > reflection prompt > review).
- Setting `offerReview` (default on), with options `always`, `failures` and `never` (DECIDE: default `always`, as requested). Not shown for a run that had zero steps.
- Works for outside-agent sessions too. Never shows while another task runs.
- Tests: card states, timer, priority, setting.

### WP4: data access for the player (Rust, read-only)
- Commands: `flight_runs(limit, before)` (list with summary), `flight_run(id)` (header: task, source, model, outcome, summary, verdict, counts, duration, paths), `flight_steps(id)` (steps with phase timings), `flight_events(id)`, `flight_calls(id, step?)` (paged, bodies loaded on demand), `flight_file(id, path)` (only paths inside the run's session folder; reject `..`, absolute paths and symlinks escaping the folder; tested). All read-only on a separate connection (WAL allows it).
- Exposed to the CLI too: `rune-studio-cli run <id> --calls` prints the attempts (parity, and it is how you test WP1 without a window).

### WP5: the Flight Player window
Window: label `flight`, opened by Review (and from the pill menu "Open Flight Player" for older runs; a small run picker inside). Normal decorated window, 1180x760, minimum 880x560, resizable; created on demand, reused if open. **Protected from capture** with the existing `window::protect` (it shows the user's screens, so the agent must never see it; demo mode makes it recordable like the other windows).

Layout (default view; everything else is one click or key away):
1. **Header** (one line): task text, outcome chip, source (Rune or the outside agent's name), date, `DURATION 0:55`, `STEPS 6`, `RETRIES 1`, model name. Verifier verdict as a small chip with its evidence on hover.
2. **Stage**: the Video.js v10 player (`@videojs/html`), custom skin in the app's language: `#141414`, dot-matrix scrubber (the progress bar is a row of dots that fill), caption-style time, play/pause, speed 0.5x/1x/2x/4x, step previous/next, fullscreen. Over the video, a light **action layer** driven by the trace, not burned in: a ripple at the click point, the typed text as a caption, the key combination, a small step badge `STEP 3 / 6 . CLICK`. Toggle `A` hides the layer. If there is no video the stage shows the step's screenshot with the same layer and prev/next (the fallback).
3. **Timeline** under the stage: one track. Each step is a segment whose width is its real duration, drawn as a row of dots, with the phases (observe, decide, gate, act, settle) as dot shades and the action icon above; markers for events: retry tick, stuck warning, steer, ask-user question (with the answer), invalid answer, verdict. Errors in red, nothing else red. Click or drag to seek; the video and the timeline always stay in sync (the player's time maps to a step and back through the step timestamps; a pure function with tests, including held frames, a first step at t = 0 and a session shorter than 1 s). Zoom with the wheel; the mini-map is the whole run.
4. **Caption line** under the timeline: one sentence for the current step ("Clicked Save: the dialog was open"), from the step's `reason` and `result`. Nothing else is shown by default.
5. **Details drawer** (right side, closed by default; `D` or the Details button, slides in over 220 ms, none under reduced motion). Tabs, each a calm list with a `RAW` toggle that shows the JSON with a Copy button:
   - **Step**: action, arguments, reason, result, error, goal, screenshots before and after.
   - **Reasoning**: the model's reasoning text for the step (or "not recorded" for older runs), and the answer.
   - **Prompt**: the exact user message and the control list sent.
   - **Safety**: stated risk, final risk, reasons, the gate verdict, the question and answer if asked.
   - **API**: one row per attempt (kind, attempt n, status, duration, wait before, outcome), expand for request and response bodies (loaded on demand, truncated flag shown, images shown as thumbnails from the files, never as base64).
   - **Timing**: phase bars for the step and a run-level breakdown.
   - **Tools**: outside-agent `rune_*` calls for the step; kit tool calls and skill use appear **only when rows exist** (K1 and skills are not built; do not show empty tabs or invented data).
   - **Files**: the screenshots, voice clips and `session.json` with "Reveal in Explorer".
   Typed text (`type_text` arguments and anything stored from them) is **hidden by default** behind a `SHOW` control in every view (DECIDE: default hidden; the video's action layer follows the same rule), because the window may be shown to others.
6. **Keyboard:** Space play/pause; Left/Right previous/next step; J/K slower/faster; D details; A action layer; T tab cycle; Esc closes the drawer, then the window. Every control has an accessible name; the timeline is operable by keyboard.
7. **Empty and error states**: no database, an old run without calls, a missing video file, a deleted screenshot, a still-open run (shown as LIVE with no end), a corrupt file; each with a one-line message, never a crash.

Tests: component tests for each state; the time-step mapping; the typed-text hiding; the drawer tabs showing only tabs that have data; `flight_file` path safety; a run with 200 steps stays smooth (virtualise the timeline and call lists; assert only visible rows render).

### WP6: docs and CLI
- `docs/ARCHITECTURE.md` (windows, the `calls` table, the video), `README.md` (feature line, privacy: the video and API bodies are stored), `docs/use-cases/08-review-what-happened.md` (update) and a new **`docs/use-cases/12-review-a-session.md`** in the same format as the others (self-contained, numbered How to use), `docs/use-cases/README.md` index, `docs/ROADMAP.md` (tick R6b).
- Privacy text: the record now includes raw requests and responses (with images and keys removed) and a video made of the screenshots; retention applies.

## 6. Do not

No new window besides the player. No cloud upload or sharing. No editing of the record from the player. No second video player library. No bundled binaries without asking. No change to how tasks run or to task speed (the eval gate applies). Do not store secrets: keys, tokens and image/audio base64 never go into `calls`.

## 7. Report

`docs/handoff/R6b-report.md`: baseline and final test numbers; the pinned Video.js version and how it was wired into Vue; the encoder spike result and the file size per minute; per work package what changed, the tests and the exact live steps for the maintainer (finish a task: the card; Review: video plays and seeks, the timeline follows, the drawer tabs, a task with a retry shows two attempts, an outside-agent run, an old run without a video, hidden typed text, `motion` mode measurements); every DECIDE and its default; every NOT CERTAIN with what was measured or left open; what you could not see on a real screen.
