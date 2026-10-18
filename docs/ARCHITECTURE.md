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
  agent/           loop, prompt, protocol (done, M2); history pruning, stuck detection (M4)
  model/           coords (done, M1); client (done, M2)
  observe/         screenshot (done, M1); uia                 (uia: M4)
  exec/            input + CancelToken (done, M1); apps (done, M2)
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
| Icon sizes                                     | `icon-sm` / `icon-md` utilities in `src/style.css`                      | Two roles: **sm** 12 px (trailing arrows, remove links; the one 4 px-grid size) and **md** 16 px (the default: icon-only buttons and beside a label). Never put a raw `size-*` on an icon. Filled glyphs carry more weight than outline ones, so keep them small; check new ones at real size.                                                                                                                       |
| Spacing grid                                   | Tailwind spacing, `layout.ts`, `window.rs`                              | **Everything snaps to an 8 px grid** (Tailwind units 2, 4, 6, 8, ...: `p-2`, `gap-2`, `size-10`). Where 8 cannot work, 4 px is allowed (type line heights of 20 px, radii of 20 and 28 px, 12 px icons). Never use half units (`gap-1.5`, `px-2.5`). Pages are 272 px wide, the orb 64 px, the window 344 wide and 184 (open) or 592 (settings) tall. The control overlay's dots sit on an 8 px pitch.               |
| Scrollbars                                     | global rules in `src/style.css`                                         | One slim (8 px, inset thumb), rounded, quiet scrollbar for every scroll area. Thumb colours are the `--scrollbar-thumb` and `--scrollbar-thumb-hover` variables; a light surface overrides them locally (the composer field does). Never style a scrollbar per component or leave the native one.                                                                                                                    |
| Browser preview                                | `@tauri-apps/api/mocks` (`mockIPC`)                                     | `src/dev/mockTauri.ts` answers the startup commands so the UI renders in a plain browser for design review. Dev only: it is not in the production bundle.                                                                                                                                                                                                                                                            |

unjs packages (`defu`, `consola`, `pathe`, `ofetch`, ...) are used only when a concrete need appears; none is needed yet, since all I/O lives in Rust.

## 7. UI: the pill

Design language: Nothing-phone inspired. Monochrome, a circular dot-matrix glyph, one red accent (needs attention, recording, Stop), **Oxanium as the only typeface in the project** (open licence, bundled locally via `@fontsource-variable/oxanium`; Tailwind's `font-sans` and `font-mono` both resolve to it, so no other font can appear; no Nothing fonts or logos). Text follows a fixed five-role hierarchy (see Typography below). Frameless, transparent, always-on-top, skip-taskbar window, **anchored to the bottom-right of the primary monitor's work area** (16 px margin, above the taskbar) and draggable (position remembered from M6). Vue devtools are disabled (the plugin is removed from the Vite config).

**Logo.** The mark is the orb itself: a 9x9 dot-matrix disc on a dark circle with a lit ring, a bright scanning head and a bright centre, in the monochrome palette. `scripts/make-logo.mjs` draws it (`public/logo.svg`, full colour; `src/assets/icons/rune-mark.svg`, single colour for the UI), and `bun run tauri icon public/logo.svg` makes the whole icon set (exe, tray, installer, taskbar), then the unused mobile icon folders are deleted. The same SVG is the favicon and the README logo; the single-colour mark is registered as the `logo` icon and shows at the foot of Settings. Change the logo by editing the script and re-running those two commands.

A single **bot glyph** (an 11x11 dot orb on a white disc: **64 px in total**, a 56 px disc inside a 4 px dark rim) is the whole app at rest. When opened, the **composer card** grows to its right while the orb stays at the left, bottom-aligned with the card.

```
AMBIENT    (orb)                                  64 px, the resting form, bottom-right corner

OPEN                  +----------------------------+
            (orb)     | +------------------------+ |
                      | | Type or say a command  | |   white field, 3 lines, Enter sends
                      | | [tune]       [mic] [↑] | |   settings left; mic + send together, same look
                      | +------------------------+ |
                      | • Ready                    |   status bar: backend dot, state or step reason
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

Modes: **ambient** (orb only), **open** (orb + composer) and **settings**. The card opens on the summon hotkey and when the agent needs the user (`ask_user`, high-risk confirm). It stays open until dismissed (`Esc`, the summon hotkey again, or clicking the orb while idle). Clicking the orb while the agent runs is Stop. Every page beside the orb (composer, settings, future task views) is exactly **272 px** wide (`PAGE_WIDTH` in `src/config/layout.ts`). The composer is one connected frame: the white field and the status bar in one frame, so nothing floats. There are no suggestion rows: they were removed for a cleaner look. Mic and send are twin 32 px dark circles on the right (the mic turns red while listening; while a task runs the send button turns into a red, icon-only Stop button, so there is no separate Stop control); settings is a ghost button on the left. The backend chip became a dot in the status bar (green, amber or red; hover for the details and model name).

**Voice reaction.** The mic button toggles hands-free listening: Rust opens the default input device (cpal, on its own thread), computes an RMS level about 30 times a second, maps -55..-10 dBFS to 0..1 and emits `audio_level`. The orb's listening bloom follows that level (fast attack, slow decay); at silence only the centre breathes, so a dead mic looks different from a live one. If nothing is heard for 4 s the status bar says so. Level capture was pulled forward from M5; segmentation (VAD) and transcription remain M5.

**Typography.** Five roles, defined once as design tokens in `src/style.css`. The default Tailwind size scale is removed (`--text-*: initial`), so a raw size such as `text-sm` or `text-[13px]` produces nothing; always use a role class.

| Role    | Class          | Size / line | Weight | Tracking | Used for                                  |
| ------- | -------------- | ----------- | ------ | -------- | ----------------------------------------- |
| Title   | `text-title`   | 18 / 24     | 600    | 0.02em   | Panel headings                            |
| Body    | `text-body`    | 14 / 20     | 400    | none     | Composer input, form fields, reading text |
| Status  | `text-status`  | 12 / 16     | 500    | 0.03em   | The live state line in the status bar     |
| Label   | `text-label`   | 11 / 16     | 500    | 0.04em   | Buttons, field labels, badges             |
| Caption | `text-caption` | 10 / 14     | 600    | 0.10em   | Section headings; pair with `uppercase`   |

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

**Window mechanics.** The window resizes in discrete steps (ambient 64x64, open 344x184, settings 344x592; width = 64 orb + 8 gap + 272 page) **keeping its bottom-right corner fixed**, so it grows up and to the left from wherever it sits, clamped to the work area. Because a transparent window still catches mouse clicks and clips what does not fit, the order is the same for every pair of modes (ambient < open < settings): when the target needs a bigger window, the window grows first and the page animates in afterwards; when it needs a smaller one, the page animates out first and the window shrinks last (after `COLLAPSE_MS`, longer than the page cross-fade). Pages switch with an out-in cross-fade, so only one is ever on screen and the fixed page width means nothing reflows. Verified natively: the window grows within about 40 ms of the click and shrinks about 370 ms after going back. Windows clamps a window's configured size to a minimum width, so the exact size is set explicitly at startup.

**M6 additions.** Hover or click on the glyph opens a small ticker list in the label and status roles: one line per step (`click · Save`, risk, result) plus the last-screenshot thumbnail.

- **Hotkeys:** `Alt+Q` opens the card (and shows the window if hidden); pressing it again collapses to the orb. `Ctrl+Alt+Esc` is the global **Stop**. Both are settings (`toggleHotkey`, `stopHotkey`) and re-register on save. `Ctrl+Alt+Space` was rejected (taken by the Claude desktop app); `Alt+Tab` cannot be registered (reserved by Windows); `Ctrl+Space` would break editor autocomplete and IME toggles.
- **Click-through while acting:** during a real agent run the window ignores the mouse in the Acting state, so it can never block a click meant for another app. Stop is then the hotkey, the voice word "stop", or the tray menu's **Stop** (which works even while the window is click-through and does not depend on the hotkey being free). The dev-only state switcher in Settings > Developer uses a preview path that never enables click-through, so forcing "acting" by hand cannot lock the window. Opening that section scrolls its buttons into view.

### Rust <-> Vue contract

Commands (UI -> Rust): `start_task(text)`, `steer(text)`, `toggle_listening()`, `stop()`, `confirm(id, approved)`, `set_mode("ambient"|"open"|"settings")`, `get_settings()`, `save_settings(...)`, `set_api_key(key)`, `check_backend()`.

Events (Rust -> UI): `state_changed(state)`, `summon`, `open_settings`, `audio_level(0..1)`, `transcript(text, final)`, `step_started{id, tool, reason, risk, target}`, `step_finished{id, ok, error?}`, `screenshot{id, thumb}`, `confirm_request{id, text, risk}`, `task_done{summary, success}`, `error{message}`.

### M2 notes: the first loop

Built: `agent/{mod,protocol,prompt}.rs`, `model/client.rs`, `exec/apps.rs` and the `start_task` command. Each turn the loop captures the primary monitor (1280 px JPEG), sends **one image plus a text history** (the last 12 steps as `click (500, 400) -> done`; older screenshots are not kept, so prefill stays small) and requires exactly one tool call (`tool_choice: "required"`, `reasoning_effort: "none"`, `temperature 0`). Tools in M2: `click`, `type_text`, `press_keys`, `open_app`, `wait`, `done`. `open_app` uses `ShellExecuteW` with the name as a single argument (no shell, so nothing can chain a second command). The default vision model is now `ternary-bonsai-2-27b` (the M1.5 result); settings saved earlier keep their old value.

Behaviour: Stop aborts even an in-flight model request (the request future is dropped). An unusable answer (no tool call, bad arguments, a click outside 0 to 1000) is reported back to the model as a failed step, and the run fails after 3 in a row. The step limit (default 30) fails the run. The status line shows the model's one-sentence reason for the current step, then the final summary. The control overlay shows while acting or observing, and shows each action visually without clutter: one white caption under the control pill (icon + `CLICK`, `TYPE hello`, `KEYS CTRL + S`, `OPEN notepad`, `WAIT 1.0 s`), replaced by the next step, and for clicks a ripple of dots spreading from the exact point that was clicked (two pulses for a double click). Both live only in the capture-excluded overlay, so the model never sees them. The overlay is **one pill**: who is in control, the current action, and the stop hint with a red stop icon like the composer's. The action's icon is the _device_ it uses, from `src/config/actions.ts` (a new tool needs one line there): mouse for anything with the cursor, keyboard for typing and key presses, a monitor for looking at the screen, a video camera for screen recording, a microphone for listening, headphones for recording or playing sound (video, mic and headphones are ready for when those tools exist). A screenshot is announced by an `observe_started` event (the model loop sends it every step, the bridge's `rune_observe` sends it with the zoom region) and flashes over exactly the captured area: dot corner brackets, a scan line of dots sweeping down, and a brief light wash. The real cursor no longer jumps: `Input::glide_to` moves it to a click target with an ease-in-out curve (120 ms plus 0.25 ms per pixel, capped at 450 ms, Stop-aware, ending exactly on the target; verified live by sampling the cursor, which showed slow start, fast middle, soft landing). `step_started.duration_ms` carries the glide time so the click ripple lands when the cursor arrives, and for `type_text` the typing time, so the caption types the text out letter by letter with a blinking caret, showing only the newest 24 characters of long text. `step_started` carries `detail`, `point` (physical pixels), `double` and `button` for this.

**Live result (2026-10-18):** "Open Notepad and write hello" ran end to end in about 1 minute and 3 steps (open_app, type_text, done) with `ternary-bonsai-2-27b`; Notepad contained exactly "hello" and the status line ended on the model's summary. Stop during a run was covered by unit tests and the M1 system tests but not re-run live.

Not in M2 by design: risk gating and confirmation (M3), element ids and stuck detection (M4), steering and voice (M5). Until M3 the agent clicks and types whatever the model decides, so only run it on tasks you would run yourself.

### External agent bridge (feature `agent-bridge`, built in M2.5)

**Question:** can an outside agent use Rune Studio's eyes and hands directly, skipping the local model? **Yes.** Everything an agent needs already sits behind two internal calls: `screenshot::capture` (eyes) and `agent::execute` (hands, with `visual()` for the on-screen caption and ripple). Today the loop in `agent/mod.rs` calls the model between them; the bridge lets an external caller take that place. Nothing else changes: the overlay, step captions, click ripple, Stop (hotkey, tray, button), the state machine and the default (model-driven) mode all stay as they are. The full agent-facing contract is in `docs/AGENT_GUIDE.md`.

**Switch.** A Cargo feature, off by default: `bun run tauri dev -- --features agent-bridge`. Normal builds contain no server and no way in. Even in a bridge build the user must turn on Settings > External agent, which shows a random token (stored in the credential manager).

**Transport.** An MCP server over Streamable HTTP at `http://127.0.0.1:47600/mcp`, because MCP is what agents such as Claude Code already speak (`claude mcp add --transport http ...`). A plain JSON mirror of the same handlers can be added later for non-MCP callers. Rejected: stdio (the app is a long-running GUI, so a spawned child would need a second channel back to it) and exposing Tauri commands (not reachable from outside).

**Tools** mirror the internal `Action` enum one to one, so there is a single execution path: `rune_begin`, `rune_observe` (optional zoom `region`), `rune_click`, `rune_type_text`, `rune_press_keys`, `rune_open_app`, `rune_wait`, `rune_status`, `rune_done`, `rune_guide`. Every action returns a settled screenshot and a plain-words text line, so the agent needs no separate observe. Same 0 to 1000 coordinates, same 1280 px JPEG, same `reason` shown in the pill and caption. `scroll`, `drag` and UI Automation element ids arrive with M4 and join the bridge for free.

**Control and safety.**

- _Who may drive:_ one lease at a time, held by either the internal agent or one external agent (`start_task` and `rune_begin` refuse each other with `busy`). The lease ends on `rune_done`, on Stop, or after 5 minutes without a call.
- _The user wins:_ Stop cancels the shared token, revokes the lease, and every later call returns `stopped_by_user` until the user re-arms. Click-through and the capture-excluded overlay behave exactly as in model mode; the control pill reads the external agent's name.
- _The door:_ 127.0.0.1 only, bearer token on every request, requests with a browser `Origin` header are refused (blocks web pages and DNS rebinding), small request size limit, calls rate-limited, every action written to an audit log.
- _Gating:_ the M3 safety gate applies to external actions too (risky actions pause for the user's yes; `declined_by_user` comes back). **Do not ship the bridge before M3**; until then it is a developer-only build.
- _What the agent cannot do:_ nothing beyond what the app's own agent can: no shell, no file access, no settings changes. Prompt-injection guidance for the agent is in the guide, and the app never feeds screen text to anything as instructions.

**As built.** `agent/body.rs` holds what both drivers share (`announce`, `execute`, `settle_time`, `finish_step`), and `screenshot::capture_region` adds the zoom crop. `bridge/lease.rs` (always compiled, pure and tested) decides who may drive: one lease, revoked by Stop until the user turns the switch off and on, expiring after 300 s of silence. `bridge/server.rs` (axum, feature only) is the door: 127.0.0.1:47600, `Host` and `Origin` checks, bearer token (`bridge/token.rs`: 32 random bytes in the credential manager, constant-time compare), 256 KB body limit, switch checked on every call. `bridge/mcp.rs` is a small hand-written JSON-RPC handler (initialize, ping, tools/list, tools/call) whose action tools call `agent::protocol::parse` and `agent::body`, so validation and behaviour are identical to the built-in loop. Unit tests assert that the guide mentions every tool and every error code. Settings > External agent shows the switch and a copy button for the connect command; the control pill shows the agent's name. Not built: rate limiting and a full audit log (one log line per begin exists); both are needed before any public release.

**The `rune-studio` command (no developer tools on the user's PC).** In agent builds the program is two executables in one folder: `rune-studio-app.exe` (the windowed Tauri app, `mainBinaryName` in tauri.conf.json) and `rune-studio.exe` (a console program, `src/bin/rune-studio.rs`, built only with the feature). They are separate because Windows starts a windowed program without a console, so a terminal would not wait for it or show its output. The command is plain Rust in `bridge/cli.rs`: `rune-studio` (open the app), `mcp` (an stdio-to-local-door bridge that starts the app if needed and reads the token straight from the credential manager, so no secret touches a file), `connect claude` (runs `claude mcp add --scope user rune-studio -- <path> mcp`, and enables the switch), `connect --print`, `enable`, `disable`, `status`, `guide`. The switch lives in settings.json and the door re-reads it on every request, so `enable` and `disable` act on a running app at once. **Packaging the guide for agents.** `docs/AGENT_GUIDE.md` stays the single source. `bridge/skill.rs` splits it by its numbered sections into a Claude Code skill (a short `SKILL.md` with name and trigger description, plus `tools.md`, `recipes.md`, `errors.md` read on demand) and `connect claude` installs it to `~/.claude/skills/rune-studio` next to the MCP registration; `skill` and `disconnect claude` refresh and remove it. A test fails if a guide section is placed in no file or two, or if a section-number reference sneaks back in. For clients without skills the MCP server also serves the guide as the `rune://guide` resource and a `rune_start` prompt. `connect antigravity` (`bridge/agents.rs`) merges the server into Antigravity's global `~/.gemini/config/mcp_config.json` without disturbing other servers or settings (a `.bak` of the old file, refusing files that are not valid JSON) and installs the skill to `~/.gemini/config/skills/rune-studio`; `disconnect` undoes both. An unknown agent name now lists the supported ones instead of a vague prompt. Later: a Claude Code plugin bundling the MCP config and skill, and more `connect` targets (Cursor, VS Code, Codex), which are each a settings path plus this same merge. User-facing steps are in `docs/SETUP.md`; the agent-facing contract in `docs/AGENT_GUIDE.md` deliberately says nothing about building or internals. Developers build with `bun run tauri build --no-bundle --features agent-bridge`.

## 8. Milestones

| #            | Milestone                                                                                                                     | Demo                                                                | Done when                                                                          |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| M0           | Shell and plumbing: pill window, hotkeys, settings + keyring, rune-infer health/model check, typed event skeleton             | Alt+Q shows the pill with "backend OK"                              | Works on the dev monitor setup; backend status is real                             |
| M1 (done)    | Eyes and hands, no AI: screenshot, downscale, `coords`, SendInput, global Stop                                                | Debug command clicks a normalized point and types in Notepad        | Clicks land within a few pixels at 100/125/150% DPI and on a second monitor        |
| M1.5 (done)  | Grounding probe: ~30 saved screenshots -> `qwen3.8-27b` "click X"                                                             | Hit rate and latency numbers                                        | We know if raw coordinates suffice or UIA/grounding must come earlier              |
| M2           | Vertical slice: model client, `click`/`type_text`/`press_keys`/`open_app`/`done`, loop, step limit, text input                | "Open Notepad and write hello"                                      | End-to-end works; Stop interrupts at any moment                                    |
| M2.5 (built) | External agent bridge behind the `agent-bridge` feature: MCP over local HTTP, same eyes and hands, lease, token               | An outside agent opens Notepad and writes hello through Rune Studio | Model mode unchanged; Stop revokes the lease; works with Claude Code as the client |
| M3           | Safety gate and confirmation (buttons)                                                                                        | "Delete this file" pauses for a yes                                 | High-risk actions cannot run without a yes                                         |
| M4           | Full tools and UIA: `scroll`, `drag`, `wait`, `ask_user`, element ids, stuck detection, history pruning                       | Copy text from a browser into Notepad                               | UIA apps use element ids; no-op loops are detected                                 |
| M5           | Voice: mic level (done), VAD, STT, listening toggle, steering, voice yes/no/stop                                              | Click the mic once, talk through a task                             | Utterances cut cleanly; speaking mid-task steers                                   |
| M6           | UI polish: glyph hover/click ticker list (thumbnail), tray, remembered position, error states                                 | The full orb + composer flow                                        | Every Step 7 state is real                                                         |
| M7           | Hardening and packaging: eval suite of ~15 tasks (success rate, steps, latency, tokens), elevated-window detection, installer | Report card + installer                                             | Success rate measured; installs on a clean machine                                 |

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

### M1.5 grounding probe: interim findings

**Tooling (built).** `scripts/m15/make-scenes.mjs` renders 10 synthetic GUI scenes (settings, editor, dialog, web page, file manager, each light and dark) at 1920x1080 with headless Chrome and reads every target's exact position from the DOM, so ground truth is exact and no live desktop is needed. `pwsh scripts/m15/run-probe.ps1` runs the Rust probe (`probe_m15.rs`, which encodes screenshots through the same `observe::screenshot::prepare` the app uses) against rune-infer and `report.mjs` summarises hit rate, near-miss rate and latency. Modes are `model:convention[@edge]` with conventions `norm` (0-1000), `brief` (norm plus "do not explain"), `px` and `ground` (the `/v1/images/grounding` endpoint).

**What is trustworthy so far.** The full 4-variant run was cut short when the user stopped rune-infer, so its hit-rate table is not a result (most requests failed to connect). Only 14 answers in it are valid, plus two 6-target pilots:

| Finding                                                         | Evidence                                                                                                                        | Confidence                                              |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `qwen3.8-27b` clicks accurately with 0-1000 coordinates         | 10 of 11 valid clicks inside the element                                                                                        | low (n=11)                                              |
| `ornith-1.5-9b` is much weaker                                  | 3 of 6                                                                                                                          | very low (n=6)                                          |
| Pixel coordinates are worse than 0-1000 for the 27B             | 1 of 3 against 5 of 5                                                                                                           | very low                                                |
| `/v1/images/grounding` returned `[500, 500]` every time         | 3 of 3, up to 1000 px off                                                                                                       | clear: looks like a fallback default, not a measurement |
| 27B is far too slow for the loop                                | server log: image prefill 47 tok/s (median 23.7 s for about 940 tokens), generation 9.7 tok/s; about 35 s per click, up to 50 s | solid (server-measured)                                 |
| 9B is fast                                                      | prefill 1183 tok/s (0.8 s), generation 42 tok/s; about 4 s per click                                                            | solid                                                   |
| `ternary-bonsai-2-27b` generates much faster than `qwen3.8-27b` | 46.8 vs 9.7 tok/s in the log                                                                                                    | untested for accuracy                                   |

A 1280x720 screenshot is about 880 image tokens (about 940 with the prompt), fewer than the 2.7k I had budgeted.

**Backend asks that came out of this:**

1. Find out why `qwen3.8-27b` prefill is about 25x slower than the 9B (47 vs 1183 tok/s). It is served as IQ3_S with a q4 KV cache and all layers on a 16 GB GPU (15.9 GB used), so memory pressure or contention is the likely cause.
2. Let a request turn thinking off (forward `chat_template_kwargs` such as `enable_thinking`, or add a reasoning budget): the model spends about 100 tokens reasoning before each click.
3. `/v1/images/grounding` returns `[500, 500]` (the image centre); check its parsing and fallback.
4. Prompt-cache reuse between questions about the same screenshot is not working (log: "non-consecutive token position", similarity 0.42), so the image is re-processed every time.
5. `usage.prompt_tokens` does not count image tokens.

### M1.5 results (final)

120 + 80 requests over 10 synthetic scenes, 40 targets, normalized 0..1000 coordinates through a `click` tool, `temperature 0`. "Hit" = inside the element; "near" = within 12 px. Thinking off means `reasoning_effort: "none"`.

| Model / setting                                | Hit     | Hit or near | Median latency | Small targets (<24 px) |
| ---------------------------------------------- | ------- | ----------- | -------------- | ---------------------- |
| `ternary-bonsai-2-27b`, thinking off           | **93%** | 100%        | 9.7 s          | 79%                    |
| `qwen3.8-27b`, thinking off                    | 85%     | 98%         | 20 s           | 57%                    |
| `ornith-1.5-9b`, thinking on                   | 73%     | 85%         | 4.0 s          | 71%                    |
| `ornith-1.5-9b`, thinking off                  | 58%     | 85%         | 1.8 s          | 71%                    |
| `qwen3.8-27b`, thinking off, 768 px screenshot | 63%     | 90%         | 5.1 s          | 29%                    |

**Findings.** (1) Raw model coordinates are good enough for medium and large targets: the best 27B hits 100% of them. Small icon buttons are the weak spot for every model (57 to 79%). (2) `ternary-bonsai-2-27b` is the best single choice: most accurate and about twice as fast as qwen 27B. (3) Thinking off helps latency a lot on the 9B but latency on the 27B is dominated by image prefill, not reasoning. (4) Downscaling to 768 px costs a lot of accuracy, especially on small targets, so keep 1280 px. (5) Latency is noisy because the GPU is shared; numbers are medians.

**Decision.** Use model coordinates as the default grounding (ternary-bonsai first, qwen as fallback), with thinking off. Keep **UI Automation (M4) as the precision path for small targets**: pull element-id clicking forward as an upgrade rather than a prerequisite. Add a verify step (screenshot after each click) in M2 so a miss is caught within one step. Backend asks (prefill speed, prompt-cache reuse, grounding endpoint, usage tokens) stay open.

## 9. Risks

1. **Local-model click accuracy** is the biggest unknown. M1.5 measures it before the loop is built; UIA and `/v1/images/grounding` are the fallbacks.
2. **Latency per step:** each turn ships about 2.7k image tokens to a 27B model. Levers: smaller screenshots, element-only turns, fewer images in history.
3. **Prompt injection from on-screen text:** the system prompt treats it as data, and the safety gate checks actions, not intent.
4. **DPI / multi-monitor bugs:** isolated by the no-AI M1 test.
5. **Elevated windows:** detect and report in M7.

## 10. Backend follow-ups (non-blocking)

- Streaming STT (`audio8-asr-infinite`) exposed through the API, for lower voice latency.
- `qwen3.8-27b` latency numbers for a ~1280 px screenshot, to set the step budget.