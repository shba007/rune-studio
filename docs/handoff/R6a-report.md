# R6a report: screen analysis as an overlay

Worked directly on `develop`, no branch, no PR, one commit per work package. **Nothing here was seen on a real screen**: there is no Windows desktop, no model server and the pill and overlay cannot be screenshotted (they are capture-excluded). Every behaviour has unit or component tests unless a row says "live check". R5 was merged first (`ReflectionCard.vue` and `AgentMark.vue` exist). R6b (post-mortems and notes) was not started, as the handoff says.

## 1. Numbers

|                                                 | Before (end of R5) | After |
| ----------------------------------------------- | ------------------ | ----- |
| `cargo test --lib`                              | 252                | 277   |
| `cargo test --lib --features agent-bridge`      | 293                | 318   |
| `cargo test --lib --features eval,agent-bridge` | 326                | 351   |
| `bun run test:unit`                             | 30                 | 52    |
| `node --test` (eval + m15)                      | 20                 | 21    |
| clippy (eval,agent-bridge), fmt, lint, build    | clean              | clean |

`cargo check --lib --tests --target x86_64-pc-windows-gnu --features eval,agent-bridge` is clean.

## 2. Work packages

### WP1: overlay bar and scan line (built; not seen)

- `overlay.rs`: `Owner { None, Control, Analysis }` and `step(owner, change)`, a pure state machine (tests): control always wins and only the owner releases the overlay; an analysis only takes a free one; control ending does not hide an analysis overlay. `overlay_mode` event and an `overlay_info` command (mode, whether this window is on the primary monitor, its physical size and scale).
- `OverlayBarShell.vue` is the shared bar shell. **Control mode now renders through it with identical classes** (a test pins them); `AnalysisBar.vue` is the analysis content: Rune's mark, `SCREEN ANALYSIS`, the stage chip, `mm:ss`, and the cancel hotkey.
- **Cancel on the bar is the hotkey, not a button**: the overlay is click-through (`set_ignore_cursor_events(true)`), and so is the control bar's Stop (it is drawn, not clickable). So Cancel is Ctrl+Alt+Esc (shown in the bar) plus the red Cancel button on the pill card.
- Scan line: one row of dots (8 px grid, the same pitch as the screen frame, not the orb's 5.4 px) at `#f1f1ef` 0.6 opacity sweeping top to bottom in 0.4 s, triggered when the status goes from capturing to asking (after the screenshot), skipped under reduced motion.
- The end word (`DONE`, `CANCELLED`, `FAILED`) stays for 1.5 s, then the bar goes; the boxes stay.

### WP2: cards above the orb (built; not seen)

- `Mode::Analysis` (Rust and TS), 344 wide, fits its content, **never taller than 70% of the work area** (`max_pill_height`, tested at 100/125/150%), scrolling inside beyond that. The cards open by themselves when an analysis starts and the pill returns to the composer when they are closed.
- `AnalysisCard.vue` (stack, header with stage and `SCREEN CHANGED` chip, question, error), `AnalysisAnswer.vue` (four lines, MORE or LESS, a dot skeleton in a travelling wave only inside `prefers-reduced-motion: no-preference`), `AnalysisFindingRow.vue` (40 px accordion row, number badge, label, kind icon, chevron; evidence as plain text, tested against HTML). The old `AnalysisView.vue`, `AnalysisStage.vue`, `AnalysisFindings.vue` and `layoutChips` are deleted (leader lines are out of scope).
- Escape: collapse the open row, then clear the selection, then close the cards (tested). Retry asks the failed question.
- **Orb:** while an analysis captures or asks the orb shows the `planning` (thinking) motion; before this the analysis did not touch the agent state, so the orb showed idle.
- Badge look: filled for the selected finding, a thin outline for the others, **dashed** for a finding with no position (the handoff used "hollow" for both; I needed them to differ).

### WP3: boxes on the real screen, always on (built; not seen)

- `analysisBoxes` removed from settings (Rust and TS), the Settings row and the mock; a test proves an old file that contains it still loads and it is no longer written back. No replacement toggle exists; a test shows only staleness changes what is drawn.
- `analysis/boxes.rs`: `usable_box([f32;4]) -> Result<Box4, Reject>` (malformed; more than 40 units outside the screen rejected, less clamped; swapped corners fixed; under 8 units; over 60% of the screen), and `check_set` (the later of two near-identical boxes; two or more boxes whose union covers 95% of the screen, tested with the field test's three thirds, three rows and a 2x2 grid; a few boxes with gaps pass). `guard` nulls rejected boxes, logs each reason and returns the count, stored in the record (`boxes_rejected`, **trace schema version 5**, older files read as 0).
- **Behaviour change:** `report::normalise_box` now goes through `usable_box`, so a box far outside the screen is dropped instead of clamped (the old clamp tests were rewritten).
- Overlay drawing (`AnalysisLayer.vue`): a dark 3 px 0.55 underlay under a 2 px dashed `#f1f1ef` rectangle, a 20 px badge with the number at the top-left corner, the selected finding at full strength and the others at 35%, a 40 ms staggered fade-in (none under reduced motion), a short pulse on the picked badge. `boxToCss` converts 0..1000 to CSS pixels from the overlay window's physical size and scale (tests at 100/125/150% and a monitor that is not at the desktop origin: only the window size counts). Markers appear on the **primary monitor only**.
- The cards and the markers select each other through the backend (`select_finding`, the snapshot's `selected`), not a leader line.

### WP4: keep boxes honest (built; not run)

- `analysis/stale.rs::decide` (tested with fixed pictures: same, a caret under 5%, over 5%, a different window, Rune in front, uncomparable pictures). A watcher runs every 1.5 s while boxes are on screen, compares `screenshot::live_signature()` (captures without writing to the archive, applies the demo-mode mask) with the grey picture taken at capture and the foreground window now with the one then; the first change sets `stale`, the boxes fade out and never return, the findings stay. This is the only way the boxes disappear (written in the code and the docs).
- **A weakness to check live:** the window baseline is the foreground window _at capture_, and pressing the pill's scan button makes the pill the foreground window, which `remember()` reports as `None`. Then the window rule is off and only the pixel rule (5%) decides. So "I clicked another window with identical pixels" is only caught if the baseline was a real window. Whether this is common in practice is unmeasured.
- The watcher captures the primary monitor every 1.5 s while boxes are shown; its cost was not measured.

### WP5: measuring box quality (built; **never run**)

- Probe conventions `analysis` (the app's system prompt and `report` tool, asked "Where is the <name> <role>?") and `analysis-point` (`report::tool_with(true)` plus a one-sentence addition to the prompt). The finding that names the element is scored like a `box` answer (IoU, centre inside, IoU >= 0.5, per size class); `boxes_rejected` counts findings whose raw box the app's guard drops; `report.mjs` prints a "Boxes the quality guard drops" table. **Approximation:** the app asks one general question, the probe asks per element, so the probe measures box quality for a named element with the app's prompt, not the app's behaviour on an open question.
- `report::REQUEST_POINTS = false` (a constant, not a setting) and an optional tolerant `point` per finding (clamped, wrong length dropped). Nothing switches it on and nothing draws a badge at a point: the path exists and is unused.
- **How to run:** `pwsh scripts/m15/run-probe.ps1 -Modes "gemma-4-12b-it:analysis,gemma-4-12b-it:analysis-point"`. **No accuracy is claimed. Until it is run, the boxes users see may be wrong, and they cannot be hidden.**

| Mode           | Asked     | Answered | Mean IoU | IoU >= 0.5 | Centre inside | Guard drops |
| -------------- | --------- | -------- | -------- | ---------- | ------------- | ----------- |
| analysis       | _to fill_ |          |          |            |               |             |
| analysis-point | _to fill_ |          |          |            |               |             |

### WP6: the window is gone, docs updated

`analysis/window.rs`, the `analysis` capability entry, `?view=analysis` routing and the image in the snapshot are removed (the saved screenshot path in the record stays; `rune-studio-cli analyses` and `analysis <id>` work). `docs/ARCHITECTURE.md`, `docs/use-cases/07-screen-analysis.md`, `README.md`, `docs/use-cases/10-record-a-demo.md` and `docs/ROADMAP.md` updated; the use case no longer mentions turning boxes on.

## 3. Live checks (exact steps)

1. Press the scan button with nothing running: the bar appears at the top (Rune mark, `SCREEN ANALYSIS`, `LOOKING` then `READING`, a timer), a dot row sweeps down once after the capture, the cards grow upward from the orb and show the dot skeleton, the orb turns the thinking motion.
2. When the answer arrives: four lines with MORE, the findings list, **dotted boxes with numbered badges on the screen** (they must always be there; a finding with no box shows a dashed badge in the list and nothing on screen). Open a finding: its box goes to full strength, the others dim.
3. Switch to another window, or open a menu that moves more than 5% of the screen: the boxes fade out within about 0.2 s and the header says `SCREEN CHANGED`. Try also: click another window **without** the picture changing much.
4. Cancel from the red button on the card, and with Ctrl+Alt+Esc; close with the x and with Escape (open row, selection, cards, in that order).
5. Start a task while the cards are open (refused) and an analysis while a task runs (refused); the overlay of a running task must look as it did.
6. A second monitor (markers only on the primary), 125% and 150% scaling (boxes must sit where the labels say), the pill growing upward with a taskbar on another edge.
7. Look at the result: bar, cards, badges, colours. Nothing of this was seen.

## 4. DECIDE items

- No clickable Cancel on the overlay (hotkey plus card button), because the overlay is click-through.
- Orb shows the thinking motion during an analysis.
- Unselected badge outlined, selected filled, no-position dashed.
- `box + point` implemented and **not** switched on.

## 5. NOT CERTAIN, status

| Item                                                                      | Status                                                                  |
| ------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Overlay created on demand without flicker or focus                        | not seen; it is shown with `SW_SHOWNOACTIVATE` like the control overlay |
| Pill growing upward on a second monitor or with a taskbar on another edge | not seen; the ceiling arithmetic is tested                              |
| Marker position accuracy                                                  | unmeasured (WP5 probe not run)                                          |
| The foreground-window rule when the pill was in front at capture          | see WP4                                                                 |
| Cost of the 1.5 s watcher                                                 | unmeasured                                                              |
| The look                                                                  | not seen                                                                |

## 6. Clean-up

The R5 report and the R6a spec were removed with this report; this is the only file left in `docs/handoff/`.