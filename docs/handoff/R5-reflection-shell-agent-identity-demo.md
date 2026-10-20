# Handoff: Milestone R5, agent identity, demo mode and the idle reflection shell

For an AI coding agent. Read all of it first. Plan it comes from: `docs/ROADMAP.md` (R5, extended here). Current behaviour: `docs/ARCHITECTURE.md`. Where this file and the code disagree about a fact, the code wins: say so in the report.

## 1. Goal

Three parts, in this order (each is its own commit set; stop and report if one blocks):

- **R5a Agent identity.** When an outside agent (Claude Code, Antigravity, others) is in control, the user sees *which* agent: the orb's dots and progress ring take that agent's accent colour instead of black, the agent's mark appears inside the orb, and in the top floating bar the red pulsing dot is replaced by the agent's mark ("Antigravity is in control"). Unknown agents get a neutral look. The design philosophy stays: dot-matrix, monochrome-first, one accent at a time, red reserved for attention/stop.
- **R5b Demo mode.** Today Rune's windows are excluded from screen capture (`window::exclude_from_capture` -> `set_content_protected(true)`: pill, overlay, analysis window), so Windows Snipping Tool, Game Bar and OBS cannot record them. Add a setting that lets external recorders see Rune, without letting the model see Rune.
- **R5c Idle reflection shell.** Idle timer, a prompt with Deny, a `Reflecting` state and progress, pre-emption. The job itself is a stub (real jobs arrive in R6).

Decision already made for recording: **external recorder (OBS, Game Bar), not a built-in recorder.** A built-in one needs a video encoder, audio capture, codecs and storage handling, and the same capture-exclusion problem still applies to Rune's own windows. It is out of scope; do not build one.

## 2. Preconditions and rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop` then `git push origin develop`. F1 (`docs/handoff/F1-field-test-fixes.md`) should be done first; if its report is not there, say so and continue only with R5a/R5b if they do not touch the same files, else stop.
2. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (`cargo test --lib`, with `--features agent-bridge`, with `--features eval,agent-bridge`, `node --test scripts/eval/test`, `bun run test:unit`, clippy, fmt).
3. Every behaviour needs a test unless it needs a live window; then write exact steps for the maintainer. **Never claim a live behaviour you did not observe.**
4. Keep the UI language of the repo: Oxanium font, the `text-caption`/`text-label` scale in `src/style.css`, dot-matrix orb, `#141414` cards, `#f1f1ef` light, `#d71921` red only for attention and stop. Check `prefers-reduced-motion` like `GlyphOrb.vue` does.
5. Anything marked **NOT CERTAIN** must be measured or left to the maintainer (listed in the report), not assumed.

## 3. What is known and what is not

Known from the code:
- The top bar is in `src/components/ControlOverlay.vue` (template near line 481): a black `bg-black/85` pill with a pulsing `bg-[#d71921]` dot, text `{{ agentName ?? 'Rune' }} is in control`, an action chip, the hotkey label and a red stop button. `agentName` comes from the event `external_agent` (`{ agent: string | null }`, `events::ExternalAgent`, emitted in `bridge/mcp.rs::begin` and cleared in `bridge/server.rs::watch_for_idle` and elsewhere).
- The orb is `src/components/GlyphOrb.vue`: a canvas of an 11x11 dot grid; dots are `rgba(17,17,17,b)` and red `rgba(215,25,33,b)` only for `attention`/`failed`. `KIND` already maps the state `reflecting` to `idle`. Progress is `ProgressRing.vue` (strokes `#f1f1ef`, red `#d71921`). The orb lives in the pill window (`AgentPill.vue`), a *different* window from the overlay: both need to learn the agent identity.
- The agent name is **self-declared**: `rune_begin` takes `agent` (clipped to 40 chars, default "External agent"). Nothing authenticates it. `rune-studio-cli connect claude|antigravity` registers the MCP command with args `["mcp"]` (see `bridge/cli.rs` around lines 313 and 332). So any program holding the bearer token can claim to be "Claude".
- `AgentState::Reflecting` exists; nothing produces it. Idle watching patterns exist in `bridge::server::watch_for_idle` and the voice `resume_listening_soon`.
- The three windows are excluded in `overlay.rs:99`, `lib.rs:135` (pill) and `analysis/window.rs:28`. Screenshots for the model come from `observe/screenshot.rs` (`xcap`).

**NOT CERTAIN (do not assume):**
- The brand accent colours and marks of Claude, Antigravity and any other agent. The maintainer supplies, per agent, an accent hex and a glyph as a simple single-colour SVG path (or approves a dot-matrix drawing). Do not invent hex values and do not download logos. Until supplied, ship the registry with the entries present but `accent: null`, which renders as neutral. Whether using the official marks is permitted under each company's brand rules: the maintainer decides; note it in the report.
- Whether `xcap` can capture the screen while Rune's windows are *not* protected and then hide them cleanly. WP B2 is a measurement first.
- How other agents (Cursor, Cline, Codex, Gemini CLI, Copilot) name themselves. Only `claude` and `antigravity` are connected by `connect` today.
- Whether the model's behaviour changes when it can see Rune's pill in screenshots (it might read it, or click it). Measure with the R2 suite (WP B3).

## 4. Work packages

### R5a: agent identity

**A1. Registry and matching (Rust, pure, tested).** New `src-tauri/src/bridge/identity.rs`:
- `fn identify(declared: &str, attested: Option<&str>) -> AgentIdentity { id: String, name: String, known: bool, verified: bool }`. Known ids to start: `claude` (aliases: "claude", "claude code", "claude-code"), `antigravity` (aliases: "antigravity", "google antigravity"). Matching is case-insensitive on the trimmed string with punctuation and spaces normalised, exact alias match only (no substring: "Not Claude" must not match). Unknown -> `id: "external"`, `known: false`, name = the declared name as sent (clipped, control characters removed).
- `attested`: the stdio shim can be started with `--agent <id>` (A2). When present and known, it wins over the declared name; `verified` means "the registration said so", not cryptographic proof. Document that in the code comment and in `docs/AGENT-GUIDE.md`: the label is a courtesy, not a security boundary, and the safety gate treats all outside agents alike.
- Tests: aliases, case, punctuation, "Not Claude", empty, 400-character name, a name with newline/RTL override characters, attested vs declared conflict (attested wins; declared name kept as tooltip text).

**A2. Attestation through `connect`.** `rune-studio-cli connect claude` registers `... -- <exe> mcp --agent claude`; `connect antigravity` writes `"args": ["mcp", "--agent", "antigravity"]`. `parse` accepts `mcp [--agent <id>]`; the shim passes the id to the app with the bridge request (a header or a field you choose, document it). Existing registrations without `--agent` keep working (declared name only). `disconnect` removes both forms. Tests for `parse`, the JSON written, and the removal. Do not run `connect` against the real config in tests.

**A3. Event and settings.** Extend `ExternalAgent` to `{ agent: Option<String> /* display name */, id: Option<String>, known: bool, verified: bool }`; keep the old field so the TS side that reads `agent` keeps working. Emit it from `bridge/mcp.rs::begin`; clear it everywhere it is cleared today. Also expose it to the pill window (the pill must show the identity too: read how the pill learns that an outside agent is in control, e.g. via `agent_state`/`externalAgentSince`, and add the identity to the same path). Update `src/types/protocol.ts`.

**A4. Brand registry (frontend).** New `src/config/agents.ts`: `AGENT_BRANDS: Record<id, { name; accent: string | null; accentOnDark: string | null; glyph: string | null /* SVG path, 24x24, single colour */ }>`. Entries for `claude`, `antigravity`, and `external` (neutral: no accent, a generic "plug" glyph from the existing icon set). Values for accent/glyph are the maintainer's to supply (section 3). Contrast rule: the accent is used on a `#141414`/black surface, so a unit test checks each non-null `accentOnDark` has WCAG contrast >= 3:1 against `#141414` (UI-component threshold) and falls back to `#f1f1ef` otherwise. Test that every id the Rust side can emit exists in the registry.

**A5. Orb.** In `GlyphOrb.vue` add an optional prop `accent?: string | null` and `mark?: string | null` (glyph path). When `accent` is set and the kind is not `attention`/`failed`: lit dots use the accent colour instead of `rgba(17,17,17)`; the unlit dot tint stays neutral. Red stays red for `attention` and `failed`. The mark: drawn in the centre of the disc, about 14 px, single colour, **made of the same dot grid** if the glyph is simple enough to rasterise to the 11x11 grid (preferred, matches the dot-matrix language), otherwise as a small vector in the accent colour at 0.9 opacity above a dimmed dot field. No gradients, no glow, no new animation beyond the existing `acting` motion; the mark may breathe at the existing `wave` rate. While an outside agent is in control the pill's orb state is `acting`/`thinking` as today; only colours and the mark change. `ProgressRing.vue` strokes use the accent while an outside agent is in control (rim of the orb), red stays for attention. Reduced motion: no breathing.

**A6. Top bar.** In `ControlOverlay.vue`, replace the pulsing red dot with the agent's mark in a 16 px slot, same left padding, in the accent colour; keep the existing pulse as an opacity pulse of the mark only for the first 600 ms after control starts, then steady (a constant pulse next to a moving action chip is noisy). Unknown/neutral: keep a small `#f1f1ef` dot (not red; red now belongs to the stop button only). Text stays uppercase caption style: `ANTIGRAVITY IS IN CONTROL`. If `verified` is false and `known` is true, append nothing visible; show "declared by the agent" only in the tooltip/`title`. Keep the red stop button exactly as is. For Rune's own agent (no outside agent) nothing changes except the red dot becomes the Rune mark `rune-mark.svg` in `#f1f1ef` (DECIDE: default yes; say if the mark does not fit 16 px).

**A7. Tests and checks.** Component tests (the repo's `bun run test:unit`) for: accent applied, red preserved for attention, neutral fallback, mark slot present, text for a known and an unknown name. A static check that no brand value is hard-coded outside `src/config/agents.ts`.

### R5b: demo mode

**B1. Setting.** `demoMode: bool` (default false), Settings row in the "Developer" section, label "Show Rune in screen recordings", help text: "Lets OBS, Game Bar and Snipping Tool capture Rune's windows. Rune hides itself from its own model either way." Changing it applies immediately to open windows (`set_content_protected(!demo)` on pill, overlay, analysis window); new windows read the setting at creation. One helper in `window.rs` is the only place that calls `set_content_protected`.

**B2. Keep Rune out of the model's screenshots when demo mode is on.** The model must never see Rune's UI, because it could read or click it. First measure (write a small test binary or reuse `probe_m15`): with the windows unprotected, does `xcap` capture them? (yes, expected.) Then implement the least invasive way that works and report the choice and why:
  1. Preferred: before each agent capture, hide the Rune windows with `ShowWindow(SW_HIDE)`/`SetWindowPos` ... NOT a visible flicker >1 frame; **measure the flicker**, or
  2. Mask: paint the Rune windows' rectangles (outer position and size, DPI-corrected, from the window APIs) neutral grey in the JPEG after capture, and for the overlay mask only the top bar and the screen-edge frame strips, or
  3. Capture a single window via Windows Graphics Capture excluding Rune (only if `xcap` supports it; NOT CERTAIN).
  Pure functions (rect scaling to the downscaled frame, mask application, DPI scaling at 100/125/150%) with numeric tests. The mask must not shift coordinates: the frame stays the same size.

**B3. Proof it does not hurt.** Run the R2 suite (`scripts/eval/run.ps1`) with demo mode off and on, at least 3 runs each, report success rate and median duration. Gate: same as R2 (success not lower by more than one run, median within +10%). The maintainer runs it (needs the live model); in your report give the exact commands. Do not claim a result.

**B4. Docs.** A short section in `docs/AGENT-GUIDE.md`-adjacent user docs (README "Recording a demo"): turn on demo mode, start OBS (Window Capture does not see the click-through overlay; use Display Capture), what the model sees, and that Game Bar needs the overlay not to be exclusive-fullscreen. Mark any step you could not test as untested.

### R5c: idle reflection shell

(From the roadmap; the job is a stub.)
- **Settings:** `reflectionEnabled` (default **false**; DECIDE: false until R6 gives it a real job), `idleMinutes` (default 10, range 1 to 120).
- **Idle definition:** no task running (own or outside agent), no voice utterance, no pill interaction, no analysis window open, for `idleMinutes`. One `IdleClock` struct with an injectable clock; the watcher task in a new `reflect/` module. Pure tests with a fake clock: resets on each input kind, does not count time while a task runs, does not fire while the lease is held.
- **Prompt:** on the pill card (not the click-through overlay), text "Rune wants to review its recent tasks.", two buttons Start and **Deny**, a 30 s countdown that visibly counts down (BUG-16 shows the confirm card countdown is stuck at 0:00: do not copy that code; test the countdown). Timeout starts reflection (DECIDE: default yes, as you decided). Deny skips this cycle; the next prompt after another `idleMinutes` of idle. The prompt must not steal focus from the window the user is working in (same fix as F1 WP1).
- **Reflecting state:** `Reflecting` produced for the first time. Orb: a new calm motion in the monochrome language (a slow rotating arc of dots; no new colour; accent colours do not apply); label "Reflecting". New event `reflection_progress { stage: string, done: u32, total: u32 }` drives the ring (the ring currently reads `plan_updated`; keep both, never at the same time).
- **Pre-emption:** a task start (typed, voice, hotkey), Stop, or an outside agent's `rune_begin` cancels reflection through its own `RunScope` within one second, ends the state, and the new task proceeds normally. The reflection job never takes the `act_turn` mutex, never touches the screen and never runs an action. Model calls from reflection (none in R5; the stub waits and reports progress) will be short and one at a time.
- **Stub job:** `reflect::run(scope, report)` that steps through three fake stages with 1 s sleeps and writes one row to a new trace table `reflections (id, started_utc, ended_utc, outcome, stages)` (migration v4 after R4's v3; check `VERSION` first). `rune-studio-cli reflections` lists them.
- Tests: idle clock, prompt state machine (shown, denied, timed out, started, pre-empted), pre-emption timing under a fake clock, migration v4, CLI listing.

## 5. Do not

No built-in screen recorder. No new dependencies unless named here (an SVG parser is not needed: glyphs are path strings or pre-rasterised grids). No model prompts changed. No notes or post-mortems (R6). No hard-coded brand colours outside the registry. Do not weaken the capture exclusion when `demoMode` is off.

## 6. Report

`docs/handoff/R5-report.md`: baseline and final test numbers; per work package what changed, tests, and what only the maintainer can check live with exact steps (an outside agent connected as Claude and as Antigravity: orb colour and mark, the top bar, Stop; OBS Display Capture in demo mode; reflection prompt, Deny, timeout, pre-emption). List every DECIDE with the default taken and every NOT CERTAIN with what you measured or left open. State the B2 method you chose with the measured flicker or mask error.
