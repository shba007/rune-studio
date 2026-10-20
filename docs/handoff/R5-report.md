# R5 report: agent identity, demo mode, idle reflection shell

Worked directly on `develop`, no branch, no PR, one commit per work package. **Nothing here was run on Windows, with an outside agent connected, in OBS or with a model server.** Every behaviour has a unit test unless the table says "live check"; the Windows build was only cross-checked (`cargo check --lib --tests --target x86_64-pc-windows-gnu` is clean). F1 was done first (its report was removed in the cleanup that closes this work; its fixes are in the history).

## 1. Numbers

|                                                       | Before (end of F1) | After |
| ----------------------------------------------------- | ------------------ | ----- |
| `cargo test --lib`                                    | 225                | 252   |
| `cargo test --lib --features agent-bridge`            | 264                | 293   |
| `cargo test --lib --features eval,agent-bridge`       | 297                | 326   |
| `bun run test:unit`                                   | 14                 | 30    |
| `node --test` (eval + m15)                            | 20                 | 20    |
| clippy (eval,agent-bridge), `cargo fmt --check`, lint | clean              | clean |

## 2. Brand research (what could and could not be found)

The handoff says not to invent hex values or download logos. The maintainer asked for research, so this is what a search and the Simple Icons data (npm `simple-icons` 16.35, fetched to a scratch folder and **not** added as a dependency) gave. Every value in `src/config/agents.ts` carries its source.

| Agent         | Accent    | Mark      | Source / note                                                                                                                                                                                                                                              |
| ------------- | --------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Claude        | `#D97757` | yes       | Simple Icons "Claude". Anthropic's [trademark guidelines](https://www.anthropic.com/legal/trademark-guidelines) require prior approval of any use of its marks and forbid changing colour, font or proportion. **Your call**; set `glyph: null` to drop it |
| Antigravity   | none      | none      | No official palette or mark found, and it is not in Simple Icons. Renders neutral. (The only logo page the search found was a different product.)                                                                                                          |
| OpenCode      | `#000000` | yes       | Simple Icons "OpenCode". Black on the dark surfaces fails 3:1, so it falls back to the neutral light colour there                                                                                                                                          |
| ChatGPT       | none      | none      | OpenAI publishes [brand guidelines](https://openai.com/brand/) but no palette, and forbids altering its logo (no added colours on the Blossom); not in Simple Icons. Neutral. The widely quoted ChatGPT green was **not** used: no official source         |
| Codex         | none      | none      | Same as ChatGPT                                                                                                                                                                                                                                            |
| Gemini        | `#8E75B2` | yes       | Simple Icons "Google Gemini"                                                                                                                                                                                                                               |
| Cursor        | `#000000` | yes       | Simple Icons (neutral on dark)                                                                                                                                                                                                                             |
| Cline         | `#18181B` | yes       | Simple Icons (neutral on dark)                                                                                                                                                                                                                             |
| Copilot       | `#000000` | yes       | Simple Icons "GitHub Copilot" (neutral on dark)                                                                                                                                                                                                            |
| Windsurf      | `#0B100F` | yes       | Simple Icons (neutral on dark)                                                                                                                                                                                                                             |
| anything else | none      | plug icon | neutral                                                                                                                                                                                                                                                    |

Simple Icons is CC0 as data, but says plainly that the marks remain the trademarks of their owners and that users must seek the permissions their use needs. **I did not clear any brand's rules.** If you want only colours and no marks, set every `glyph` to `null` (one file). Filling Antigravity, ChatGPT or Codex later is a three-line edit each.

Only `claude` and `antigravity` can be connected with `connect` (the others' config formats are not known here); the rest are matched by their declared name, or by starting the shim as `rune-studio-cli mcp --agent <id>` (`connect --print` still prints the plain form).

## 3. Work packages

### R5a: identity (built; never seen on screen)

- **A1** `bridge/identity.rs`: exact-alias matching after normalising case and punctuation, control and bidi characters removed, 40-character clip. Tests: aliases, case, "Not Claude" and other substrings, empty, 400 characters, newline and RTL override, attested vs declared (attested wins, declared kept), unknown attested id ignored.
- **A2** `mcp --agent <id>` parses; `connect claude` registers it, `connect antigravity` writes `["mcp","--agent","antigravity"]`; the shim sends header `x-rune-agent`; the server drops any `_attested` argument an agent sends itself, so only the header can set it (tested). `disconnect` is unchanged: it removes the server by name, so both forms go. Existing registrations without `--agent` keep working.
- **A3** `external_agent` event now `{ agent, id, known, verified, declared }`, and a new command `external_agent` for windows that open late; `events::set_external` is the one place that sets and clears it. The pill window did not listen to the event before; it does now.
- **A4** `src/config/agents.ts` with provenance per entry; tests: every backend id has an entry and nothing else (also read from `identity.rs`), WCAG 3:1 for every `accentOnDark` (else the neutral light colour), unknown ids neutral, glyphs are plain path data, **no brand hex outside the registry**.
- **A5/A6** the orb's lit dots and the ring use the accent (the orb's accent on its light disc, the ring's on the dark rim), the mark is one flat vector over the dimmed dots, red is untouched, reduced motion adds nothing; the top bar shows the mark instead of the red dot, pulsing for 600 ms only, text `<NAME> IS IN CONTROL`, "Declared by the agent" only in the tooltip, Rune's own mark when no outside agent is in control (DECIDE: yes). **Deviation:** the mark is a vector over dimmed dots, not rasterised to the 11x11 grid (the handoff's fallback), because the brand paths are not simple.
- **A7** component tests for the mark and the identity reader (no canvas test: jsdom has no canvas; the dot colours are the one thing not covered).

**Live check:** connect Claude (`rune-studio-cli connect claude`, restart it), run a task: the orb dots and ring should be orange, the Claude mark in the middle, the top bar `CLAUDE IS IN CONTROL`, Stop still red. Then `rune_begin` with `agent: "Acme Bot"`: neutral, name shown as sent. Check the dots are legible on the light disc (the orange is about 2.9:1 there).

### R5b: demo mode (built; not run on Windows)

- `demoMode` (default off), a **Recording** toggle in Settings. **Deviation:** not in the Developer section, which only exists in development builds.
- `window::protect` is the only caller of `set_content_protected` (pill, overlay and analysis window all use it) and `apply_demo_mode` applies a change to open windows at once.
- **B2 method chosen: masking** (option 2 of the handoff), because hiding windows per capture cannot be measured here and capture-by-window needs APIs `xcap` may not have. `observe::mask`: the visible windows' outer rectangles, and for the overlay only the top bar and the four edge strips, are painted grey in the full-resolution capture **before** crop and downscale, so the frame size and every coordinate are unchanged. Pure functions with numeric tests (clipping, monitors left of the primary, 100/125/150% scales, centred bar, the middle of the screen untouched). **The overlay pieces are estimates read from the stylesheet (top 20 to 68 logical px, 1000 px wide, 36 px edges incl. margin), not measured**; I did not measure a flicker or a mask error and claim neither.
- **B3 (needs you):** `scripts/eval/run.ps1` with demo mode off and on, at least 3 runs each; the gate is R2's. No result is claimed.
- **B4:** README "Recording a demo", with everything untested marked as such.

**Live check:** turn it on, start OBS Display Capture: the pill and the overlay's bar must be visible in the recording; ask for a screen analysis in the same state: the screenshot must show grey where the pill, the analysis window and the overlay bar are, nothing else.

### R5c: idle reflection shell (built; not seen running)

- Settings `reflectionEnabled` (default **false**, DECIDE as written) and `idleMinutes` (default 10, clamped 1 to 120; **no UI field for the minutes**, edit `settings.json`).
- `reflect/idle.rs` (injectable clock: fires once per stretch; every input kind resets it; busy time does not count and the clock restarts when it ends; a denied prompt waits a full period), `reflect/prompt.rs` (shown, denied, timed out = start, started, pre-empted, finished), `reflect/job.rs` (three stages of 1 s that report progress; cancel noticed in about 10 ms, tested; never touches the screen, the model or `act_turn`).
- Busy means a task running, a held lease, an analysis in flight, the analysis window open, or the microphone open (the last is my addition: a user who left the microphone on is there).
- Prompt on the pill card (not the overlay): "Rune wants to review its recent tasks.", Deny (focused first) and Start, a countdown **computed from the deadline on every tick** (the confirm card bug, fixed separately, is not copied) with tests of the arithmetic. The pill is shown with `SW_SHOWNOACTIVATE`, so it does not take the keyboard. Timeout starts reflection (DECIDE: yes).
- `Reflecting` is produced for the first time: a slow rotating arc, neutral (an agent accent never applies), and `reflection_progress` drives the ring instead of the plan (the plan is cleared when a review starts).
- Pre-emption: `AppState::begin_run` (typed, voice and hotkey tasks, outside `rune_begin`) and `stop_current` cancel the job's token. The handoff says "its own `RunScope`"; I used its own `CancelToken` in `AppState.reflect` because a `RunScope` is what `begin_run` replaces, and reflection must not be replaceable by itself.
- Trace schema **version 4**: table `reflections`, migrated from 1, 2 and 3; `rune-studio-cli reflections [--limit N]`.

**Live check:** set `reflectionEnabled: true` and `idleMinutes: 1` in `settings.json`, leave the PC alone for a minute: the pill opens with the card and counts down; Deny, wait another minute for the next card; Start (or wait 30 s): the orb turns the calm arc and the ring fills in three steps; start a task by typing, by voice and from an outside agent in turn: reflection must end at once (`rune-studio-cli reflections` shows `preempted`). Confirm the card did not take the keyboard from the window you were in.

## 4. DECIDE items

- Rune mark in the top bar when Rune itself drives: **yes** (not seen at 16 px).
- `reflectionEnabled` default **false**; timeout starts reflection **yes**.
- Demo mode setting placed under **Recording**, not Developer.

## 5. NOT CERTAIN, status

| Item                                                                      | Status                                                               |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Brand colours and marks                                                   | researched above; three agents have none; rights not cleared         |
| Whether `xcap` captures unprotected windows and the best way to hide them | not measured; masking chosen                                         |
| Whether the model changes behaviour when Rune is masked grey              | not measured (B3)                                                    |
| How Cursor, Cline, Codex, Gemini CLI, Copilot name themselves             | unknown: matched by exact alias only; `--agent` is the reliable path |
| Orb legibility with the orange dots on the light disc                     | not seen                                                             |

## 6. Clean-up

The F1 report and the R5 spec were removed with this report's commit; this report is the only file left in `docs/handoff/`.