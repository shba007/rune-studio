# Handoff: Milestone R6a, screen analysis as an overlay (no separate window)

For an AI coding agent. Read all of it first. Plan it comes from: `docs/ROADMAP.md` (R6, part R6a). Current behaviour: `docs/ARCHITECTURE.md` and `docs/use-cases/07-screen-analysis.md`. Where this file and the code disagree about a fact, the code wins: say so in the report. R6b (post-mortems and notes) has its own handoff later; do not start it.

## 1. Goal

Screen analysis stops being a separate window. It becomes part of the existing interface, in the same language as the "agent is in control" overlay:

- A **status bar** at the top of the screen (the same black pill as the control bar) shows that Rune is looking at the screen, what stage it is in, and a Cancel button.
- **Cards** appear **above the orb** in the pill window: the question, a short answer, and a compact list of findings with their evidence.
- Optionally, **numbered dot markers** are drawn on the real screen at the places the findings refer to.
- The analysis window (`analysis/window.rs`, `?view=analysis`, `AnalysisView.vue`, `AnalysisStage.vue`) is removed.

Design philosophy (Nothing-phone style, as the rest of the app): dot-matrix, monochrome, one accent at a time, Oxanium type with the `text-caption`/`text-label`/`text-status` scale from `src/style.css`, `#141414` cards with `rounded-[28px] border border-white/15`, `#f1f1ef` as the light colour, `#d71921` red **only** for errors and Stop. Compact: nothing larger than it needs to be, no shadows or gradients, no decorative animation. Reduced motion: no animation at all (use `usePreferredReducedMotion` like `GlyphOrb.vue`).

## 2. Preconditions and rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop`, then `git push origin develop`.
2. R5 (agent identity, demo mode, reflection shell) should be merged first because this work touches the same pill, overlay and window code. Check with `ls src/components/ReflectionCard.vue src/components/AgentMark.vue`. If R5 is not done, stop and say so.
3. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (`cargo test --lib`, with `--features agent-bridge`, with `--features eval,agent-bridge`, `node --test scripts/eval/test`, `bun run test:unit`, clippy, fmt).
4. Analysis logic is **not** rewritten: `analysis/run.rs`, `report.rs`, `state.rs`, the saved record and the CLI keep working. Only presentation changes, plus the small additions below.
5. Never claim a live behaviour you did not observe. The pill and overlay are capture-excluded, so they cannot be screenshotted; test through unit/component tests and write exact live steps for the maintainer.

## 3. What is known and what is not

Known from the code:
- The current feature: `commands.rs` has `start_analysis`/`ask_analysis`/`cancel_analysis`/`analysis_state`; `analysis/state.rs` holds the snapshot; `analysis/run.rs` captures once, makes one forced `report` tool call (up to 8 findings with boxes `[x0,y0,x1,y1]` in 0..1000), emits `analysis_updated`; `analysis/window.rs` builds the 1040x680 window; the webview parts are `AnalysisView.vue`, `AnalysisStage.vue`, `AnalysisFindings.vue`, `src/config/analysis.ts` and the composable `useAnalysis`.
- Task and analysis exclude each other (`commands.rs` refuses a task while `analysis::state::in_flight`, and the reverse).
- The pill window has modes (`window.rs::Mode`: Ambient, Open, Task, Settings) and fits its height to its content for Open and Task (`fit_height`), anchored at the bottom-right of the primary monitor's work area. Existing cards on the pill (`ConfirmCard.vue`, `ReflectionCard.vue` from R5, the task panel) are the pattern to follow.
- The control overlay (`overlay.rs`, `ControlOverlay.vue`) is a transparent, click-through, capture-excluded window per monitor with a canvas and the top bar. It is created when control starts.
- Box accuracy is poor (field test: 1 of 8 right), which is why boxes are off by default (`analysisBoxes`). Earlier point probing (M1.5) measured points at up to 93% hit for the best model, small targets 57 to 79%. Whether a **point** per finding is reliable in this report prompt is **NOT CERTAIN**: WP5 measures it.

**NOT CERTAIN (measure or leave to the maintainer):**
- Whether the overlay can be created on demand for analysis without flicker and without taking focus.
- Whether the pill window can grow upward by the needed height without jumping on a second monitor or with a taskbar on another edge.
- Marker position accuracy (WP5).
- The look: the maintainer approves the visual result on a real screen; list what you could not see.

## 4. Work packages

### WP1: overlay status bar and scan line
- The overlay becomes usable by analysis: create it (or show it) when an analysis starts, hide it when the analysis ends or is cancelled, without disturbing a running control session (they exclude each other, so the overlay has exactly one owner at a time: add an `OverlayOwner` enum `{ None, Control, Analysis }` in Rust; test the transitions as a pure state machine).
- `ControlOverlay.vue` gets a second mode. In analysis mode the **same top bar component** is shown with: the Rune mark (from R5) in the slot where the agent mark goes, text `SCREEN ANALYSIS`, a stage chip (`LOOKING` while capturing, `READING` while the model works, `DONE` for 1.5 s, `CANCELLED`, `FAILED`) with the elapsed seconds as `00:14` in label style, and a **Cancel** button in the same circular slot as the control bar's Stop. Cancel is red only on hover/press? No: use the same red circle as Stop for consistency, but it must be clearly the cancel of the analysis (tooltip and accessible name "Cancel analysis").
- The overlay is click-through, so Cancel cannot be clicked on it today. Look at how the control bar's stop button is made clickable (or whether it is not: read `overlay.rs` and the hotkey). If it is not clickable, then Cancel in analysis mode is the hotkey (Ctrl+Alt+Esc, which already cancels analysis) plus the Cancel button on the pill card (WP2); in that case show the hotkey label in the bar instead of a button. Report which you did and why.
- A single **scan line** sweeps top to bottom once when the capture happens: a row of dots (the 5.4 px grid of the orb, scaled to screen), `#f1f1ef` at 0.6 opacity over a 0.4 s sweep. It happens **after** the screenshot is taken (the overlay is capture-excluded anyway). Skipped under reduced motion.
- Tests: the owner state machine, the stage text/timer formatting, and a component test that analysis mode renders the bar without the agent-specific parts.

### WP2: cards above the orb
- New pill mode `Mode::Analysis` in `window.rs` (and the matching TypeScript mode) sized by content like Task: grows **upward** from the orb, never taller than 70% of the work-area height, scrolls inside beyond that (reuse the app's scrollbar styles). Width the same as the other pill modes (344).
- Components (new, small, each one file): `AnalysisCard.vue` (the stack container), `AnalysisAnswer.vue`, `AnalysisFindingRow.vue`. Delete `AnalysisView.vue`, `AnalysisStage.vue`, `AnalysisFindings.vue` and their wiring when the new ones work.
- Layout, top to bottom (a card per block, 8 px between cards, all `#141414`):
  1. **Header card**: AppIcon + `SCREEN ANALYSIS` (caption), the stage text, and a close (x) button. Close = collapse the cards and clear markers; it also cancels a running analysis.
  2. **Question card**: the existing composer styled for analysis (placeholder `Ask about the screen`; empty means "describe it"). Enter sends. While busy it shows the question read-only with a Cancel button in the Stop circle style.
  3. **Answer card**: the model's answer, clamped to 4 lines with a `MORE` / `LESS` toggle. While waiting: a **dot skeleton**, three rows of dots pulsing in a travelling wave, the same motion language as the orb's thinking state. No spinner.
  4. **Findings list** (only when there are findings): one compact row each, 40 px high: a numbered dot badge (a filled `#f1f1ef` circle with the number in `#141414`, or an outlined one for unselected), the label in `text-status`, the kind icon at the right, a chevron. Tap a row to expand its description and evidence inline (accordion, one open at a time); the expanded row selects its marker on the overlay (WP3). Evidence strings are plain text (never HTML).
  5. **Error card**: red dot + the message + `RETRY` (asks the question that failed, as now) .
- A stale marker is not shown in the cards; instead a one-line chip `SCREEN CHANGED` (WP4) sits in the header.
- The orb uses the existing `thinking` motion while capturing/reading (the state mapping already counts analysis as busy; verify and say what the orb shows today).
- Keyboard: Escape collapses the expanded finding, then clears the selection, then closes the cards. Tab order follows the visual order. Every control has an accessible name.
- Tests (Vue component tests with the repo's runner): each card state (waiting, answer, findings, expanded, error), the accordion rule, Escape order, long text and 8 findings do not overflow (assert clamp classes and the scroll container), `MORE`/`LESS`, reduced motion removes animation classes.

### WP3: markers on the real screen (optional layer)
- Setting `analysisMarkers`: `off` (default), `points`, `boxes`. Replaces the boolean `analysisBoxes` (migrate: `analysisBoxes: true` becomes `boxes`; the settings loader must accept old files; test it). Settings row text: "Show markers on the screen".
- Overlay draws, in analysis mode only: for each finding with a position, a **numbered dot marker** (a 20 px circle, `#f1f1ef` fill, `#141414` number, with a 1 px `#141414` outer ring so it reads on light and dark backgrounds), and in `boxes` mode a dotted rectangle (2 px dashes in `#f1f1ef` over a 3 px `#141414` 0.55 opacity underlay). The selected finding is full strength; the others are at 35%. They fade in with a 40 ms stagger (skipped under reduced motion).
- Coordinates: findings use 0..1000 over the whole captured screenshot; the overlay canvas is in device pixels per monitor. One pure function `to_screen(point_or_box, monitor, scale)` in TypeScript (and the same logic checked in Rust if the monitor geometry is computed there) with tests at 100/125/150% and a monitor with a non-zero origin. The analysis always uses the primary monitor, as today: markers only on it.
- Markers and cards select each other: tapping a row selects the marker (a pulse of the marker, not a leader line). A leader line from marker to card is out of scope.
- Tests: coordinate conversion, selection state, markers hidden when mode is `off`.

### WP4: keep markers honest
The screen can change after the picture was taken. At capture store the foreground window handle and title, and a small grey thumbnail (reuse the comparison in `agent/stuck.rs`). Every 1.5 s while markers are visible compare the live screen to the thumbnail (cheap, off the UI thread). If the foreground window changed or more than 5% of the thumbnail moved, fade the markers out within 200 ms and show the `SCREEN CHANGED` chip in the header; the findings text stays. Markers never come back for that analysis. Pure tests for the decision (same, small change, window changed, big change) with fixed frames.

### WP5: measure points versus boxes (the maintainer runs it)
- Extend the existing probe (`probe_m15.rs`, `scripts/m15/`) with an `analysis` convention: ask the **report prompt exactly as the app sends it** on the synthetic scenes and score where each finding's point (or box centre) falls against the ground truth. Report hit rate per size class and per mode (`point only` request, `box only` request, `both`).
- Make the report schema accept an optional `point: [x, y]` per finding next to `box` (parsing tolerant: swapped corners, out-of-range values clamped, wrong length dropped), and a request variant per mode, selected by the `analysisMarkers` setting (`points` asks for points, `boxes` asks for boxes, `off` asks for neither, which also shortens the answer and the latency; **measure the latency difference**, do not assume).
- Do not claim accuracy. The report says how to run the probe (`pwsh scripts/m15/run-probe.ps1 -Modes "gemma-4-12b-it:analysis"`) and the table to fill in.

### WP6: remove the window, update docs
- Delete `analysis/window.rs`, the `analysis` window label/capability entry, `?view=analysis` routing in `src/main.ts`, and the image data URL from the analysis snapshot sent to the webview (the saved screenshot path in the record stays). Keep `rune-studio-cli analyses` and `analysis <id>` working.
- Update `docs/ARCHITECTURE.md` (windows list, analysis section), `docs/use-cases/07-screen-analysis.md` (How to use, What you will see, Limits), `README.md` (the analysis feature line), and `docs/ROADMAP.md` (tick R6a). Update `docs/AGENT-GUIDE.md` only if it mentions the window.
- Remove dead code and unused strings; clippy and fmt clean.

## 5. Do not

No separate window of any kind. No new dependencies. No change to the model prompt beyond WP5's point/box variants. No leader lines, no drag, no resizing of cards, no new colours. Do not change how tasks or the control overlay look (only the shared bar component may be reused, with no visible change to control mode; add a test). Do not add a hotkey, voice trigger or MCP tool for analysis.

## 6. Report

`docs/handoff/R6a-report.md`: baseline and final test numbers; per work package what changed, tests, and exact live steps for the maintainer (press the analysis button: bar appears, scan line, cards grow upward, answer, findings accordion, markers in each mode, SCREEN CHANGED after switching windows, Cancel from card and hotkey, a second monitor, 125% and 150% scaling); every DECIDE with the default taken; every NOT CERTAIN with what you measured or left open; what you could not see on a real screen.
