# Rune Studio: Architecture

A small Windows desktop app. You give it a voice or text command, and it controls the PC to carry it out: it looks at the screen, picks one action, performs it, checks the result, and repeats until the task is done.

All AI inference goes to **rune-infer**, a local OpenAI-compatible server (Postman collection: `code-infer/postman`). The app never talks to a cloud model directly.

## 1. Product decisions

| Decision   | Choice                                                                               | Notes                                                          |
| ---------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| OS         | Windows 11 only                                                                      | macOS/Linux later behind the native-layer traits               |
| Brain      | rune-infer, `qwen3.8-27b` (vision + tools)                                           | Our own function-calling protocol, no vendor computer-use tool |
| Voice      | Hands-free: click the mic once to start a listening session, click again to stop     | VAD cuts utterances; no push-to-talk                           |
| Autonomy   | Runs freely, confirms risky actions                                                  | Confirm by button or voice "yes"/"no"                          |
| Stack      | Tauri 2 (Rust core) + Vue 3 + Tailwind                                               | Rust owns capture, input, UIA, audio, the agent loop           |
| UI         | Minimal floating pill (Spotlight-like) that expands to a log                         |                                                                |
| Out of MVP | Memory, skills/macros, task-history DB, spoken replies (TTS), wake word, macOS/Linux |                                                                |

## 2. The core loop

```
input (voice/text) -> intent -> observe screen -> choose ONE action -> safety gate -> execute -> verify -> repeat
```

The model only **proposes** actions. Nothing reaches the OS except through the safety gate and the executor.

## 3. Components

```
UI shell (pill) <-events/commands-> Agent core (state machine, history, budget)
                                      |-- Input layer   (hotkey/mic -> VAD -> STT -> text)
                                      |-- Model client  (rune-infer: chat, STT, grounding)
                                      |-- Observer      (screenshot + UIA element list)
                                      |-- Safety gate   (risk -> allow / confirm / block)
                                      '-- Executor      (SendInput: mouse, keyboard, scroll)
```

1. **UI shell:** pill, live transcript, status, action log, confirm prompt. A pure view over events.
2. **Input layer:** global hotkeys, mic capture (16 kHz mono), Silero VAD utterance segmentation, STT. Typed input skips straight to the core.
3. **Agent core:** builds the prompt, calls the model, handles the result, enforces step and time budgets, detects stuck loops, supports steering and cancel.
4. **Model client:** the only code that knows rune-infer's wire format.
5. **Observer:** captures and downscales the screen, and reads a compact UIA element list.
6. **Executor:** performs actions in physical pixels. Owns coordinate mapping.
7. **Safety gate:** decides allow / confirm / block, and owns the kill switch.

## 4. rune-infer integration

| Need                            | Endpoint                                                                        |
| ------------------------------- | ------------------------------------------------------------------------------- |
| Decide next action              | `POST /v1/chat/completions` (vision model, `tools`, `tool_choice: "required"`)  |
| Speech to text                  | `POST /v1/audio/transcriptions` (multipart; 16 kHz mono WAV is the native path) |
| Locate a UI target (fallback)   | `POST /v1/images/grounding` (`{image, query}` -> `point`, `box_2d`)             |
| Read text with boxes (fallback) | `POST /v1/ocr`                                                                  |
| Health / models                 | `GET /health` (no auth), `GET /v1/models`                                       |

Facts confirmed with the backend:

- Base64 `image_url` data URIs and multiple images per request work. Body limit is 128 MB.
- `qwen3.8-27b` supports Vision + Tool Call. `tool_choice: "required"` is enforced.
- Coordinates: Qwen-native **0-1000 normalized**. Boxes are `[ymin, xmin, ymax, xmax]`; points are `[x, y]`. The Model client normalizes everything to `[x, y]` so the ordering never leaks.
- Auth: every `/v1/*` route needs `Authorization: Bearer <key>` when a key is configured; health probes are open.
- Context is 131k tokens. A 1080p screenshot costs about 2.7k tokens, so keep only the last 2-3 screenshots in history.
- Language is auto-detected when omitted from STT. The STT model is `parakeet-tdt-0.6b-v3` or `whisper-large-v3`.
- `audio8-asr-infinite` offers streaming STT with rolling VAD (possible later upgrade).

## 5. Action protocol

### What the model sees each turn

1. System prompt: role, rules, coordinate convention, risk rules. Screen text is treated as data, never instructions.
2. The task, plus any later steering messages.
3. History: a short text log of past actions and results. Only the last 2-3 screenshots stay as images.
4. Current observation: a fresh screenshot (about 1280 px long edge, JPEG) and a compact UIA element list for the focused window (`e1 Button "Save" [412,880]`). Elements are in the same 0-1000 space as clicks.

### Tools (one per action; `tool_choice: "required"`)

| Tool         | Arguments                                                     |
| ------------ | ------------------------------------------------------------- |
| `click`      | `target` = `{element: "e14"}` or `{x, y}`; `button`; `double` |
| `type_text`  | `text`; optional `target` to focus first                      |
| `press_keys` | `keys`, e.g. `["ctrl","s"]`                                   |
| `scroll`     | `target`/`x,y`; `direction`; `amount`                         |
| `drag`       | `from`, `to`                                                  |
| `open_app`   | `name`                                                        |
| `wait`       | `ms` (capped)                                                 |
| `ask_user`   | `question` (pauses the task)                                  |
| `done`       | `summary`; `success`                                          |

Every action also carries `reason` (one sentence, shown in the log) and `risk` (`low|medium|high`).

### Rules of the loop

- **One action per turn.** Observe again after every action. Batching is a later optimization.
- **Settle before observing:** short delay or pixel-diff check, so screenshots are not mid-animation.
- **Click resolution:** `element` id -> UIA rectangle center; otherwise `{x,y}` 0-1000 -> pixels via the Observer's scale and monitor origin.
- **Stuck detection:** unchanged screen after N actions, or repeated identical action -> inject "no effect", then `ask_user`.
- **Budget:** max steps per task (default 30) and a hard timeout.
- **Safety:** final risk = `max(model_risk, rule_risk)`. Rules look at window title/process, typed text, key combos (`alt+f4`, `delete`), and target keywords (Send, Delete, Pay, Install). `high` pauses for confirmation.

## 6. Native (Rust) layer

```
src-tauri/src/
  lib.rs           Tauri setup, tray, global hotkeys, command registration
  settings.rs      settings file + keyring secret
  hotkeys.rs       summon/stop hotkey parsing and (re)registration
  backend.rs       rune-infer HTTP client, health/model check
  events.rs        typed events + commands shared with the UI
  window.rs        pill window: placement, ambient/open/settings sizing, click-through, capture exclusion
  agent/           loop, history, prompt, protocol            (M2+)
  model/           coords (done, M1); client                  (client: M2)
  observe/         screenshot (done, M1); uia                 (uia: M4)
  exec/            input + CancelToken (done, M1); apps       (apps: M2)
  debug.rs         dev-only commands for the eyes and hands   (debug builds only)
  system_tests.rs  #[ignore]d end-to-end tests against a real window
  safety/          gate, rules                                (M3)
  voice/           capture (mic + level, done), vad, session  (VAD and session: M5)
```

| Job           | Crate                          | Notes (verified against docs, 2026-10)                                                          |
| ------------- | ------------------------------ | ----------------------------------------------------------------------------------------------- |
| Screenshot    | `xcap` 0.9                     | `Monitor::all`, `capture_image`, per-monitor `x/y/width/height/scale_factor`                    |
| Input         | `enigo` 0.6                    | Absolute `move_mouse`; **physical pixels** on Windows; wrap behind our own trait                |
| UI Automation | `uiautomation` 0.25            | Use the **cache request API** plus depth/element caps; uncached walks are slow                  |
| Mic           | `cpal` 0.18                    |                                                                                                 |
| VAD           | `voice_activity_detector` 0.2  | Silero wrapper; API unverified, read docs.rs when implementing. Fallback: Silero ONNX via `ort` |
| HTTP          | `reqwest` 0.13                 |                                                                                                 |
| Secrets       | `keyring`                      | rune-infer API key lives in Windows Credential Manager                                          |
| Hotkeys       | `tauri-plugin-global-shortcut` | Pin the stable 2.x line matching Tauri 2.x                                                      |

Design points:

- **Threading:** UIA is COM and needs a dedicated STA thread. One native worker thread owns UIA, capture and input; the async agent loop talks to it over channels.
- **Coordinates live in one place** (`coords.rs`). Executor and gate receive pixels; the UI receives 0-1000 plus pixels for overlays.
- **Self-click protection:** the pill is excluded from capture (content protection / `WDA_EXCLUDEFROMCAPTURE`) and is click-through while the agent acts.
- **State machine:** `Idle -> Listening -> Planning -> AwaitingConfirm -> Acting -> Observing -> Done | Failed | Stopped`. A voice utterance starts a task in `Idle` and becomes a steering message otherwise; "stop" cancels at once.
- **Elevated windows** cannot receive injected input (UIPI). Detect and tell the user; do not run as admin.
- **DPI:** the whole pipeline stays in physical pixels; the process is per-monitor DPI aware.

### Frontend conventions

Prefer maintained packages from the unjs / unplugin / VueUse / Vue ecosystem over hand-rolled code, when a real need exists.

| Need                                           | Use                                                                     | Notes                                                                                                                                                                                                                                                                                                                                                                                                                |
| ---------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fonts                                          | `unplugin-fonts` (Vite) with its Fontsource provider                    | Oxanium comes from `@fontsource-variable/oxanium`, so it is bundled and never fetched at runtime. `main.ts` imports the virtual `unfonts.css`.                                                                                                                                                                                                                                                                       |
| Icons (ready-made)                             | `unplugin-icons` (Vite) + `@iconify-json/ph`                            | **Phosphor Fill** is the only ready-made icon set, **filled everywhere**. State is carried by the container and colour (a dark circle, red while listening), never by swapping outline for filled. Only the icons used are bundled.                                                                                                                                                                                  |
| Icons (custom)                                 | `unplugin-icons` custom collection                                      | Hand-made SVGs live only in `src/assets/icons/` (one file per icon, 24x24, filled, `currentColor`) and load as `~icons/custom/<file-name>`. Never inline `<svg>` in a component or composable. See `src/assets/icons/README.md`.                                                                                                                                                                                     |
| Icon registry                                  | `src/icons/index.ts` + `<AppIcon>`                                      | The **only** file that names an icon set. It maps meanings (`mic`, `send`, `tune`, `back`, `refresh`, `ok`, `open`, `key`, `eye`, `link`, `shortcut`, `stop`, `trash`) to icons; components render `<AppIcon name="mic" size="lg" />`. Switching family or style is a one-file change. A lint rule (`no-restricted-imports` on `~icons/**`, off only inside `src/icons/`) fails the build on any direct icon import. |
| Animation loop, reduced motion, tab visibility | VueUse `useRafFn`, `usePreferredReducedMotion`, `useDocumentVisibility` | `GlyphOrb.vue`                                                                                                                                                                                                                                                                                                                                                                                                       |
| Timers, keys, focus                            | VueUse `useTimeoutFn`, `onKeyStroke`, `useFocus`                        | `useAgent.ts`, `AgentPill.vue`, `Composer.vue`                                                                                                                                                                                                                                                                                                                                                                       |
| Styling                                        | Tailwind v4 with the five-role type hierarchy                           | see Typography                                                                                                                                                                                                                                                                                                                                                                                                       |
| Icon sizes                                     | `icon-sm` / `icon-md` / `icon-lg` utilities in `src/style.css`          | Three roles like the type scale: **sm** 12 px (trailing arrows, remove links), **md** 14 px (beside a label: setting rows, save action), **lg** 16 px (icon-only buttons). Never put a raw `size-*` on an icon. Filled glyphs carry more weight than outline ones, so keep them small; check new ones at real size (link and keyboard were too dense at 14 px and were replaced by `link-simple` and `command`).     |
| Scrollbars                                     | global rules in `src/style.css`                                         | One slim (8 px, inset thumb), rounded, quiet scrollbar for every scroll area. Thumb colours are the `--scrollbar-thumb` and `--scrollbar-thumb-hover` variables; a light surface overrides them locally (the composer field does). Never style a scrollbar per component or leave the native one.                                                                                                                    |
| Browser preview                                | `@tauri-apps/api/mocks` (`mockIPC`)                                     | `src/dev/mockTauri.ts` answers the startup commands so the UI renders in a plain browser for design review. Dev only: it is not in the production bundle.                                                                                                                                                                                                                                                            |

unjs packages (`defu`, `consola`, `pathe`, `ofetch`, ...) are used only when a concrete need appears; none is needed yet, since all I/O lives in Rust.

## 7. UI: the pill

Design language: Nothing-phone inspired. Monochrome, a circular dot-matrix glyph, one red accent (needs attention, recording, Stop), **Oxanium as the only typeface in the project** (open licence, bundled locally via `@fontsource-variable/oxanium`; Tailwind's `font-sans` and `font-mono` both resolve to it, so no other font can appear; no Nothing fonts or logos). Text follows a fixed five-role hierarchy (see Typography below). Frameless, transparent, always-on-top, skip-taskbar window, **anchored to the bottom-right of the primary monitor's work area** (16 px margin, above the taskbar) and draggable (position remembered from M6). Vue devtools are disabled (the plugin is removed from the Vite config).

A single **bot glyph** (an 11x11 dot orb on a white disc: **64 px in total**, a 56 px disc inside a 4 px dark rim) is the whole app at rest. When opened, the **composer card** grows to its right while the orb stays at the left, bottom-aligned with the card.

```
AMBIENT    (orb)                                  64 px, the resting form, bottom-right corner

OPEN                  +----------------------------+
                      | Open Notepad             ↗ |   suggestions: rows of the same frame,
                      | Open Calculator          ↗ |   folded away once you type
            (orb)     | +------------------------+ |
                      | | Type or say a command  | |   white field, 2 lines, Enter sends
                      | | [tune]       [mic] [↑] | |   settings left; mic + send together, same look
                      | +------------------------+ |
                      | • Ready            [Stop]  |   status bar: backend dot, state, Stop
                      +----------------------------+

SETTINGS              +----------------------------+
                      | ←  Settings            ⟳   |   back, title + live backend status, recheck
                      |    • Backend OK · 12 ms    |
            (orb)     | CONNECTION                 |
                      | (link) Base URL            |   grouped cards, one icon row per setting
                      | (key)  API key             |
                      | MODELS     (eye) Vision ...|
                      | SHORTCUTS  (kbd) Open ...  |
                      | [ ✓  Save and check ]      |
                      +----------------------------+

ATTENTION  composer card; the status bar reads "Needs your OK  [Yes] [No]" (voice yes/no from M5)
```

Modes: **ambient** (orb only), **open** (orb + composer) and **settings**. The card opens on the summon hotkey and when the agent needs the user (`ask_user`, high-risk confirm). It stays open until dismissed (`Esc`, the summon hotkey again, or clicking the orb while idle). Clicking the orb while the agent runs is Stop. Every page beside the orb (composer, settings, future task views) is exactly **276 px** wide (`PAGE_WIDTH` in `src/config/layout.ts`). The composer is one connected frame: one connected frame holding the suggestion rows, the white field and the status bar, so nothing floats. Mic and send are twin 36 px dark circles on the right (the mic turns red while listening); settings is a ghost button on the left. Suggestions show only while the field is empty and the agent is idle, and fold away smoothly once you type. The backend chip became a dot in the status bar (green, amber or red; hover for the details and model name).

**Voice reaction.** The mic button toggles hands-free listening: Rust opens the default input device (cpal, on its own thread), computes an RMS level about 30 times a second, maps -55..-10 dBFS to 0..1 and emits `audio_level`. The orb's listening bloom follows that level (fast attack, slow decay); at silence only the centre breathes, so a dead mic looks different from a live one. If nothing is heard for 4 s the status bar says so. Level capture was pulled forward from M5; segmentation (VAD) and transcription remain M5.

**Typography.** Five roles, defined once as design tokens in `src/style.css`. The default Tailwind size scale is removed (`--text-*: initial`), so a raw size such as `text-sm` or `text-[13px]` produces nothing; always use a role class.

| Role    | Class          | Size / line | Weight | Tracking | Used for                                       |
| ------- | -------------- | ----------- | ------ | -------- | ---------------------------------------------- |
| Title   | `text-title`   | 18 / 24     | 600    | 0.02em   | Panel headings                                 |
| Body    | `text-body`    | 14 / 20     | 400    | none     | Composer input, form fields, reading text      |
| Status  | `text-status`  | 12 / 16     | 500    | 0.03em   | The live state line in the status bar          |
| Label   | `text-label`   | 11 / 16     | 500    | 0.04em   | Suggestion rows, buttons, field labels, badges |
| Caption | `text-caption` | 10 / 14     | 600    | 0.10em   | Section headings; pair with `uppercase`        |

Glyph states (distinct by motion and shape, never colour alone; static frames under `prefers-reduced-motion`):

| State                | Dot-matrix behaviour                                         |
| -------------------- | ------------------------------------------------------------ |
| Idle / stopped       | Dim ring, slow breath                                        |
| Listening            | Bloom that follows the live microphone level (`audio_level`) |
| Thinking / observing | Comet orbiting the ring                                      |
| Acting               | One bright "cursor" dot roaming, click ripple                |
| Needs attention      | Red `!`, blinking (the only red)                             |
| Done                 | Check mark draws in dot by dot, then fades to idle           |
| Failed               | Red `x`                                                      |

A small dot on the disc's corner shows backend problems (amber/red) and is absent when everything is fine.

**Window mechanics.** The window resizes in discrete steps (ambient 64x64, open 348x244, settings 348x572; width = 64 orb + 8 gap + 276 page) **keeping its bottom-right corner fixed**, so it grows up and to the left from wherever it sits, clamped to the work area. Because a transparent window still catches mouse clicks and clips what does not fit, the order is the same for every pair of modes (ambient < open < settings): when the target needs a bigger window, the window grows first and the page animates in afterwards; when it needs a smaller one, the page animates out first and the window shrinks last (after `COLLAPSE_MS`, longer than the page cross-fade). Pages switch with an out-in cross-fade, so only one is ever on screen and the fixed page width means nothing reflows. Verified natively: the window grows within about 40 ms of the click and shrinks about 370 ms after going back. Windows clamps a window's configured size to a minimum width, so the exact size is set explicitly at startup.

**M6 additions.** Hover or click on the glyph opens a small ticker list in the label and status roles: one line per step (`click · Save`, risk, result) plus the last-screenshot thumbnail.

- **Hotkeys:** `Alt+Q` opens the card (and shows the window if hidden); pressing it again collapses to the orb. `Ctrl+Alt+Esc` is the global **Stop**. Both are settings (`toggleHotkey`, `stopHotkey`) and re-register on save. `Ctrl+Alt+Space` was rejected (taken by the Claude desktop app); `Alt+Tab` cannot be registered (reserved by Windows); `Ctrl+Space` would break editor autocomplete and IME toggles.
- **Click-through while acting:** during a real agent run the window ignores the mouse in the Acting state, so it can never block a click meant for another app. Stop is then the hotkey, the voice word "stop", or the tray menu's **Stop** (which works even while the window is click-through and does not depend on the hotkey being free). The dev-only state switcher in Settings > Developer uses a preview path that never enables click-through, so forcing "acting" by hand cannot lock the window. Opening that section scrolls its buttons into view.

### Rust <-> Vue contract

Commands (UI -> Rust): `start_task(text)`, `steer(text)`, `toggle_listening()`, `stop()`, `confirm(id, approved)`, `set_mode("ambient"|"open"|"settings")`, `get_settings()`, `save_settings(...)`, `set_api_key(key)`, `check_backend()`.

Events (Rust -> UI): `state_changed(state)`, `summon`, `open_settings`, `audio_level(0..1)`, `transcript(text, final)`, `step_started{id, tool, reason, risk, target}`, `step_finished{id, ok, error?}`, `screenshot{id, thumb}`, `confirm_request{id, text, risk}`, `task_done{summary, success}`, `error{message}`.

## 8. Milestones

| #         | Milestone                                                                                                                     | Demo                                                         | Done when                                                                   |
| --------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------- |
| M0        | Shell and plumbing: pill window, hotkeys, settings + keyring, rune-infer health/model check, typed event skeleton             | Alt+Q shows the pill with "backend OK"                       | Works on the dev monitor setup; backend status is real                      |
| M1 (done) | Eyes and hands, no AI: screenshot, downscale, `coords`, SendInput, global Stop                                                | Debug command clicks a normalized point and types in Notepad | Clicks land within a few pixels at 100/125/150% DPI and on a second monitor |
| M1.5      | Grounding probe: ~30 saved screenshots -> `qwen3.8-27b` "click X"                                                             | Hit rate and latency numbers                                 | We know if raw coordinates suffice or UIA/grounding must come earlier       |
| M2        | Vertical slice: model client, `click`/`type_text`/`press_keys`/`open_app`/`done`, loop, step limit, text input                | "Open Notepad and write hello"                               | End-to-end works; Stop interrupts at any moment                             |
| M3        | Safety gate and confirmation (buttons)                                                                                        | "Delete this file" pauses for a yes                          | High-risk actions cannot run without a yes                                  |
| M4        | Full tools and UIA: `scroll`, `drag`, `wait`, `ask_user`, element ids, stuck detection, history pruning                       | Copy text from a browser into Notepad                        | UIA apps use element ids; no-op loops are detected                          |
| M5        | Voice: mic level (done), VAD, STT, listening toggle, steering, voice yes/no/stop                                              | Click the mic once, talk through a task                      | Utterances cut cleanly; speaking mid-task steers                            |
| M6        | UI polish: glyph hover/click ticker list (thumbnail), tray, remembered position, error states                                 | The full orb + composer flow                                 | Every Step 7 state is real                                                  |
| M7        | Hardening and packaging: eval suite of ~15 tasks (success rate, steps, latency, tokens), elevated-window detection, installer | Report card + installer                                      | Success rate measured; installs on a clean machine                          |

### M1 results: eyes and hands

What exists: `model/coords.rs` (the only place that converts model space to pixels), `observe/screenshot.rs` (capture, downscale to 1280 px, JPEG), `exec/input.rs` (absolute mouse, cancellable typing, key combos) with a `CancelToken` that Stop sets, and dev-only commands plus a "Eyes and hands" panel in Settings > Developer.

**Verified on a real desktop** (1920x1080, 100% DPI, one monitor):

| Check                               | Result                                                                                             |
| ----------------------------------- | -------------------------------------------------------------------------------------------------- |
| Cursor lands on the requested pixel | 0 px error over 10 probe points, including corners (1919, 1079)                                    |
| Left, right and double click        | arrive at the exact pixel, correct button and click count                                          |
| Typing                              | ASCII, accents and CJK arrive intact; Ctrl+A combos work; no modifier left held                    |
| Stop                                | interrupts typing within about 250 ms (50 of 960 characters typed)                                 |
| Capture mapping                     | a model-space point lands on the right pixel of the target window, after downscaling               |
| Capture latency                     | about 70 ms in a release build, about 300 ms in a dev build (1080p to 1280x720 JPEG, about 105 KB) |
| Through the real UI                 | Capture, Click, Type and the Stop hotkey all work end to end from Settings > Developer             |
| DPI awareness                       | the app process is per-monitor aware                                                               |

**Not verified natively, and how it is covered instead:** 125% and 150% scaling and a second monitor need hardware this PC does not have. They are covered by unit tests on the mapping (physical sizes at 125/150%, a monitor left of and above the primary one with negative origins, every pixel round-tripping, clamping) plus the per-monitor awareness check. Run the native checks on a scaled or multi-monitor machine before relying on them.

How to run: `cargo test --lib` for the unit tests; `pwsh scripts/m1-system-test.ps1` for the end-to-end tests (it moves the real mouse and types for a few seconds against its own target window, and the typing tests refuse to run unless that window has focus).

Findings worth keeping: unoptimised image code made dev-build capture about 1 s, so the dev profile optimises this crate lightly (about 350 ms); WebView2 exposes its controls through UI Automation, so the real app can be driven and inspected without pixel coordinates; and a button that re-renders itself makes UIA `Invoke` throw even though it fired, so judge by the effect.

## 9. Risks

1. **Local-model click accuracy** is the biggest unknown. M1.5 measures it before the loop is built; UIA and `/v1/images/grounding` are the fallbacks.
2. **Latency per step:** each turn ships about 2.7k image tokens to a 27B model. Levers: smaller screenshots, element-only turns, fewer images in history.
3. **Prompt injection from on-screen text:** the system prompt treats it as data, and the safety gate checks actions, not intent.
4. **DPI / multi-monitor bugs:** isolated by the no-AI M1 test.
5. **Elevated windows:** detect and report in M7.

## 10. Backend follow-ups (non-blocking)

- Streaming STT (`audio8-asr-infinite`) exposed through the API, for lower voice latency.
- `qwen3.8-27b` latency numbers for a ~1280 px screenshot, to set the step budget.