# Rune Studio — Feature Todo

Status reviewed against the local Studio checkout on 2026-11-14 (Studio 0.13.0).
Checked items below indicate code is present; they do not by themselves certify a
released build or a successful end-to-end run with real models and MCP servers.

## Current integration progress and next cycles

### Implemented in the current checkout

- [x] Connect model traffic through Rune Infer; manage the Infer catalog and model roles through its authenticated management interface (`infer_catalog.rs`, `InferCatalogEditor.vue`).
- [x] Use Infer's effective Agent role for evaluations; optional roles are handled separately, including Visual Analysis roles.
- [x] Provide a model library for local discovery, download/import, role selection, removal, and load/unload controls. Download availability depends on the embedded Infer build; residency and catalog operations go through Infer.
- [x] Include optional in-process Kit startup and shutdown, with default loopback port 47601 and an available-port fallback (`embedded_kit.rs`). Builds without `embedded-kit` connect to a separately run Kit server.
- [x] Route MCP discovery and configuration through Kit, with selected-tool restrictions and optional bearer token storage in the system credential store.
- [x] Expose Kit-managed plugin settings, third-party HTTP MCP server configuration, registry/installed MCP views, and package install/update/remove controls. Kit owns the server, packages, and settings.
- [x] Rename the Flight Recorder UI and code to Preview, and use Visual Analysis for the supporting workflow. Historical `screen-insight` source identifiers remain in saved-record handling for compatibility.
- [x] Fix composer height calculation for the empty-card state and update the blue voice button icon; focused component coverage exists.
- [x] Reject unsupported historical settings and older/unversioned trace databases while preserving existing files, rather than upgrading them automatically.

### Open work, ordered as small reviewable cycles

- [x] Verify the current pinned Infer and Kit revisions in a packaged Windows build: embedded startup, port fallback, shutdown, onboarding, authenticated management, and a small computer-use task with a real model and MCP.
  - 2026-11-14: frontend suite passed (527 tests); feature-enabled Rust suite passed serially (722 tests, 31 ignored). A recording timing test failed under concurrent compilation and passed on rerun.
  - An optimized unsigned Windows NSIS package and a separate-identity verification package built with embedded Infer and Kit after the sandbox could not download the NSIS dependency.
  - The isolated packaged executable started embedded Kit on the default port and, when 47601 was occupied, on a fallback port. Its management endpoint rejected unauthenticated access and returned the isolated installed MCP with the test token. Studio's live Kit client discovered and called the real `rune-time` MCP successfully. Automated test processes were terminated; normal tray exit was checked separately by the owner.
  - An isolated embedded-Infer probe using the installed Holo 3.1 9B model returned a text response; the existing Qwen-oriented image-plus-required-tool assertion did not receive the expected tool call. The test used a temporary model-ID alias, so verify the actual model's advertised vision/tool capabilities before drawing a compatibility conclusion.
  - The owner reported that onboarding with local Holo and `rune-time`, the Calculator task, the UTC-time MCP task, and normal tray exit all passed in the separate-identity verification build.
- [ ] Investigate the isolated Holo 3.1 9B image-plus-required-tool probe: the Qwen-oriented test did not receive the expected tool call when its model ID was temporarily mapped to Holo. Verify Holo's advertised vision/tool support and reproduce with a model-appropriate test before attributing the failure.
- [x] Verify Kit package actions and third-party MCP discovery/calls against the public registry and local server fixtures; confirm update, backup, removal, restart, and error behavior.
  - 2026-11-14: an isolated packaged Kit installed a local `rune-time` MCP and a skill; duplicate MCP installation returned 409, skill removal succeeded, and remote configuration hid URL query data. After restart, Studio's live client discovered and called `rune-time`; MCP removal persisted after a further restart.
  - Kit's local HTTP integration test passed for remote tool and resource discovery/calls through two real gateways. Its modified-skill backup test passed.
  - Reproduced against the public registry: an isolated `rune-time` update check found version 0.4.0, but the packaged Kit returned HTTP 409 because it incorrectly treated `mcp` as a skill. The owner approved and pushed the Kit fix; its regression test, all 58 Kit library tests, and Kit lint passed. Studio now pins `bc268a5daa2fd2665bd073f9ccb36d5e7a105488`. The rebuilt verification executable updated the isolated MCP from 0.0.1 to 0.4.0, reported that restart was required, and successfully called the updated MCP after restart.
  - The packaged Kit registry-list endpoint initially returned HTTP 502 at its 10-second timeout. On the owner's requested retry, two requests succeeded (14 entries, including `rune-time` 0.4.0); a direct request also returned HTTP 200. The earlier failure appears transient, but its cause was not determined.
- [x] Resolve the Kit legacy-stdio conflict: on 2026-11-14 the owner confirmed the breaking-release decision to remove legacy stdio migration and backup. The current Kit settings reader rejects incompatible settings and preserves their file; no legacy migration or backup implementation was found.
- [x] Infer engine inventory: authenticated Infer inventory and a read-only Studio view use Infer's managed installation directory and explicitly configured engine paths. Studio pins Infer `c1c8abea3a04160d0769c7bcfc79169556449e6b`.
  - Infer tests passed with and without the TUI, its non-TUI library compiled, and library Clippy passed. Studio component, type, lint, authenticated HTTP client, and actual embedded Infer inventory tests passed.
- [x] Expose Infer-owned engine release checks in Studio. Infer shares the TUI's release lookup with its authenticated API and limits concurrent checks. Infer's focused tests, TUI-enabled Clippy, and non-TUI library check passed; Studio's four component tests and authenticated HTTP fixture passed. The pinned embedded Infer integration test checked authentication, unsupported engines, and a successful live Needle release lookup.
- [x] Expose engine installation through Infer. A background installation API and Studio status controls use Infer's managed home, including for standalone servers with an explicit config workspace; explicitly configured engine paths remain supported.
  - Infer's managed-path, authenticated API, and staged-install preservation tests passed, as did TUI Clippy and the non-TUI build. Studio's component tests and authenticated installation HTTP fixture passed. No real engine was installed during these fixture checks.
  - The first real Needle installation failed because the configured model repository contained no runtime executable. The owner approved Cactus's official runtime and requested its version and revision be kept in configuration. Infer's engine registry now contains the reviewed platform artifacts, immutable URLs, sizes, and checksums; an isolated Infer test installed the configured runtime and produced a real Needle tool call. The pinned Studio embedded-server test installed Needle through the API, checked its status and inventory, and rejected a duplicate installation. A separate isolated Whisper download reached extraction but exceeded the four-minute verification deadline; its temporary files were removed.
- [x] Expose Infer-owned engine updates using shared Infer TUI/API eligibility and dispatch, authenticated background jobs, managed-home paths, and Studio controls. The pinned embedded-server test installed Needle, checked an already-current update through Studio's client, verified the binary remained unchanged, and removed the test home. Managed-update failure and file-preservation fixtures and Studio's component/client tests passed.
- [ ] Verify an actual engine replacement with a newer release end to end in an isolated environment; the update cycle's live verification covered an already-current engine.
- [x] Move model candidates and resource recommendations into Infer's authenticated API. Infer prepares a fresh local configuration before onboarding choices without overwriting existing files, returns all installer-supported registry models for the library, and limits onboarding candidates to advertised computer-use capabilities. Infer setup, preservation, candidate API, isolated fresh-server and lint checks passed; Studio onboarding, settings, authenticated client and pinned embedded-server tests passed. The revised onboarding UI has not been manually checked in a packaged desktop build.
- [x] Move model downloads and imports into Infer: download jobs, status, cancellation, folder scanning and verified import staging run through its authenticated API. Infer serializes API model and engine operations, shares the model-directory lock with TUI installs during staging, and preserves conflicting files. Infer's import, authentication, configuration-path and concurrency fixtures, lint and CPU build passed; Studio's client and focused UI checks passed. Studio pins `35953b8d2c5998ee23abe48a1bfe702483235ffe`; its embedded status, scan and cancellation integration test passed against that revision.
- [x] Route Settings model discovery, downloads, cancellation and role selection to the active Infer connection, while keeping onboarding on embedded Infer. The connection ID and revision protect against stale library actions. Focused onboarding/settings tests (11), Vue type checking and feature-enabled Rust checking passed; a packaged desktop check of this change has not been run.
- [x] Add Settings model import through the active Infer server. Users can enter a server-side folder path, and embedded Infer also offers the local folder picker. Folder scans and reviewed import requests use the active connection and revision. Thirteen focused onboarding/settings tests, Vue type checking and feature-enabled Rust checking passed; a packaged desktop check has not been run.
- [x] Remove Studio-owned model policy: backend discovery now shows every Infer-reported model; new settings no longer choose Agent or Decision model IDs, and unset roles no longer fall back to the Agent model. Runtime Agent and Analysis paths require Infer roles, optional role operations use their configured Infer role, and queued task traces no longer record a model from stale Studio settings. The serial feature-enabled library suite passed (650 tests, 33 ignored), followed by focused discovery and default-role tests after the final changes.
- [x] Expose cancellation of Infer engine operations in Studio. Authenticated cancellation signals the matching install/update job, reports that it is waiting for the current step, and keeps the job reserved until it ends. Infer's API authentication/job tests, staged-install and update cancellation/preservation fixtures, Clippy and CPU build passed; Studio's UI tests, type check and authenticated HTTP client fixture passed. Studio pins Infer `040c01df948b0f2c5e2442095db67297cd0ff595`; its embedded endpoint check confirmed authentication and rejection when no matching job exists. A real in-progress cancellation was not tested in the packaged desktop.
- [x] Expose read-only Kit sandbox workspaces and path approvals through Kit's management API. Inspection does not create sandbox files and rejects malformed inventory. Kit's 59-test library suite, focused sandbox tests and Clippy passed; Studio's Settings Tools tests, type check and authenticated HTTP fixture passed. Studio pins Kit `f36ba4c108fd1b298b7c1f45a36fce0f3f037e70`; the isolated embedded endpoint check verified authentication and that viewing an empty inventory creates no sandbox directory.
- [ ] Expose read-only Kit staged changes and recovery backups through Kit's management API. The inventory request uses the transaction lock and does not create transaction directories, prune backups, apply changes or restore files; Studio displays paths and hashes without file contents. Kit's 60-test library suite and Clippy passed, as did Studio's seven Settings Tools tests, type check and authenticated client fixture. Awaiting owner push of Kit and a pinned isolated embedded endpoint check.
- [ ] Add explicitly confirmed Kit apply and restore actions. The local Kit API binds actions to the exact reviewed item and rechecks staged, original and backup data; Studio requires a separate confirmation. Kit's 61-test library suite and Clippy passed, including stale-review and external-edit checks, and Studio's seven Settings Tools tests and type check passed. The earlier read-only Kit changes are staged and these action changes are unstaged; both need the owner-pushed Kit revision before the isolated embedded end-to-end check can run.
- [ ] Review remaining Infer and Kit feature coverage, including model-management controls for separately run Infer servers.
- [ ] Inventory other duplicated functionality across Studio, Infer, Kit, and Tools and present specific ownership choices for review before moving or removing code.

## 1. Desktop UI & Window Architecture

- [x] Floating, frameless, click-through, always-on-top desktop pill UI (Tauri v2 + Vue 3 + Tailwind CSS)
- [x] Multi-monitor transparent click-through control overlay (`ControlOverlay`)
- [x] Per-monitor DPI awareness and negative coordinate origin normalization
- [x] Frameless window summoning and collapse hotkey (`Alt+Q`, configurable `toggleHotkey`)
- [x] Emergency stop hotkey (`Ctrl+Alt+Esc`, configurable `stopHotkey`)
- [x] Single-instance mutex preventing duplicate app instances
- [x] Native window blur effect (`useNativeBlur`) and rounded glass aesthetic
- [x] Screen capture exclusion for Studio windows (with opt-in `demoMode` for screen recordings)
- [x] Tray icon menu with quick Stop, Show, and Exit actions
- [x] Automatic update check against signed GitHub releases via Tauri updater

## 2. Perception & Screen Observation

- [x] Primary monitor screenshot capture and downscaling pipeline (JPEG via `xcap`)
- [x] Windows UI Automation control inspection (`exec::uia`, up to 60 controls within 1.5s budget)
- [x] Normalized model coordinate space (`0..1000`) across full desktop area
- [x] Screen visual analysis mode with numbered bounding boxes drawn on real screen
- [x] Native video recording tools (`rune_record_screen`, `rune_start_recording`, `rune_stop_recording`, `rune_list_capture_targets`)
- [x] Visual grounding acceptance probe against Infer vision models (`grounding_probe.rs`)
- [ ] Cross-platform window and UI Automation inspection for macOS (Accessibility API) and Linux (AT-SPI)
- [ ] Calibrated visual analysis bounding box coordinates evaluation

## 3. Action Execution & Input Simulation

- [x] Virtual mouse gliding and cursor step interpolation (`enigo`)
- [x] Real mouse and keyboard actions: `click`, `click_element` (by UIA ID), `type_text`, `press_keys`, `scroll`, `drag`, `open_app`, `wait`, `ask_user`, `done`
- [x] Single-action-per-model-turn safety loop with Stop checks between keystrokes and scroll notches
- [x] Multi-tiered safety gate combining deterministic local regex/path rules and model risk ratings
- [x] Stuck detection: unchanged screen warning after 2 actions, abort after 5 actions
- [x] Independent task verification mode (`background`, `blocking`, `off`) with final screen check
- [ ] Audio response playback and direct TTS playback inside Studio

## 4. Outside Agent MCP Bridge

- [x] In-process MCP server over stdio (`rune-studio-cli mcp`) and HTTP on `127.0.0.1:47600`
- [x] Mutual exclusion lease locking (single agent control with 5-minute inactivity timeout)
- [x] 21 exposed MCP tools:
  - [x] Navigation & control: `rune_guide`, `rune_begin`, `rune_plan`, `rune_status`, `rune_done`
  - [x] Perception: `rune_observe`, `rune_elements`, `rune_analyze`, `rune_wait_for_request`
  - [x] Actions: `rune_click`, `rune_click_element`, `rune_type_text`, `rune_press_keys`, `rune_scroll`, `rune_drag`, `rune_open_app`, `rune_wait`
  - [x] Screen recording: `rune_list_capture_targets`, `rune_start_recording`, `rune_stop_recording`, `rune_record_screen`
- [x] Console CLI tool (`rune-studio-cli`):
  - [x] `status`, `enable`, `disable` (permissions & state management)
  - [x] `connect`, `disconnect` (auto-configuration for Claude Code and Antigravity)
  - [x] `runs`, `run` (historical trace log inspection from SQLite)
  - [x] `analyses`, `analysis` (historical visual analysis inspection)
  - [x] `reflections` (historical post-run review records)
  - [x] `life` (export, audit, delete commands)

## 5. Playground: Multi-Modal Operations

- [x] Dedicated Playground window (`PlaygroundWindow.vue`) with navigation and responsive sidebar
- [x] Multi-source capture picker (`PlaygroundImagePicker.vue`, `CaptureTargetPicker.vue`) for screens, individual windows, and image files
- [x] Side-by-side and split-slider comparison viewer (`PlaygroundCompare.vue`)
- [x] Interactive visual result bounding box viewer (`PlaygroundVisualResult.vue`)
- [x] Audio waveform player widget (`PlaygroundSpeech.vue`)
- [x] Cancellable execution pipeline (`PlaygroundRuns`, `playground_cancel_run`)
- [x] Session run history and persistence (`usePlaygroundHistory`)
- [x] Visual Analysis model roles configuration panel with catalog sync (`PlaygroundSettingsPanel.vue`)
- [x] Active Image Operations:
  - [x] **Ask (`ask`)**: visual question answering
  - [x] **Caption (`caption`)**: concise and detailed vision captioning via `playground_caption`
  - [x] **Point (`point`)**: visual coordinate pointing
  - [x] **Detect objects (`detect`)**: bounding box detection via `playground_visual_branch`
  - [x] **Segment objects (`instance-segment`)**: instance masks via `playground_visual_branch`
  - [x] **Detect oriented objects (`oriented-detect`)**: rotated bounding boxes via `playground_visual_branch`
  - [x] **Segment scene (`segment`)**: semantic scene maps via `playground_visual_branch`
  - [x] **Pose (`pose`)**: skeletal keypoints via `playground_visual_branch`
  - [x] **Depth (`depth`)**: relative depth maps via `playground_depth`
  - [x] **Develop RAW (`develop-raw`)**: 16-bit PNG RAW development via `portfolio_develop_raw`
  - [x] **Create (`create`)**: text-to-image generation via `playground_create`
  - [x] **Edit (`edit`)**: instruction-based image editing via `playground_edit`
  - [x] **Variations (`variation`)**: composition variation generation via `playground_variation`
  - [x] **Reference (`reference`)**: dual-image style & subject synthesis via `playground_reference`
  - [x] **Upscale (`upscale`)**: 2× and 4× super-resolution upscaling via `playground_upscale`
- [x] Active Audio Operations:
  - [x] **Create Voice (`tts`)**: text-to-speech audio synthesis via `playground_speech`
  - [x] **Transcribe (`transcribe`)**: speech-to-text WAV transcription via `playground_transcribe`
  - [x] **Translate speech (`translate-speech`)**: speech translation via `playground_translate_speech`
- [ ] Deferred Operations (marked `later` in UI configuration):
  - [ ] **Transform (`transform`)**: prompt-based image redrawing
  - [ ] **Inpaint (`inpaint`)**: interactive mask painting and regional regeneration
  - [ ] **Extend (`extend`)**: canvas outpainting and edge generation
  - [ ] **Clone Voice (`clone`)**: zero-shot voice cloning from audio sample
  - [ ] **Audio Edit (`audio-edit`)**: word replacement in existing voice clip
  - [ ] **Voice Convert (`convert`)**: voice timbre conversion
  - [ ] **Music (`music`)**: song and sound effect generation
  - [ ] **Separate Audio (`separate`)**: vocal and instrumental track separation
  - [ ] **Video Create (`video-create`)**: text-to-video generation

## 6. Playground: Multi-Step Flows & Graph Visualizer

- [x] Dual-view rendering: interactive App view and visual DAG Graph view (`PlaygroundFlowLayout`, `PlaygroundFlowNode`, `PlaygroundFlowConnection`)
- [x] Flow: **Image to Voice (`image-voice`)** chaining image captioning and voice synthesis
- [x] Flow: **Visual Analysis (`visual-analysis`)** with perception planning and multi-branch execution
- [x] Flow: **Portfolio Studio (`portfolio-studio`)** with RAW development, look estimation, portrait matting, and composite export

## 7. Scheduled Tasks, Alarms & Intake Engine

- [x] Natural language scheduling interpretation ("remind me in 10 minutes", "at 4pm", etc.)
- [x] In-process alarm and scheduled task runner (`schedule.rs`, `alarms.rs`)
- [x] Intake discovery engine (`intake.rs`, `intake_discovery.rs`, `intake_domain.rs`, `intake_pathways.rs`, `intake_subdomains.rs`)
- [x] Goal domains catalog (`goal_domains.json`, `goal_domains.rs`)
- [x] Task calendar and proposal management (`task_calendar.rs`, `task_proposals.rs`, `workspace_tasks.rs`)

## 8. Personalities & Identity

- [x] Personalities system (`personalities.rs`) with custom system prompts
- [x] Personality symbols & badges: `companion`, `fitness`, `finance`, `learning`, `creativity`, `focus`
- [x] Dynamic personality switching and avatar representation

## 9. Storage, Tracing & Life Architecture

- [x] Full session recording and SQLite audit tracing (`trace.sqlite3`)
- [x] Post-task video generation and timeline step review
- [x] AES-GCM encrypted life data storage foundation (`src-tauri/src/life/crypto.rs`)
- [x] Life record schema and privacy policy enforcement (`life/policy.rs`, `life/record.rs`, `life/store.rs`)
- [ ] Automatic reflection and continuous life-data learning loops (currently stored foundation only)
- [ ] Review remaining advanced settings controls (including `maxSteps` and `archiveKeepDays`); model role configuration is exposed through the Infer catalog and Visual Analysis settings.

## 10. Platform Support & System Roadmap

- [x] Windows validated desktop platform
- [ ] macOS full computer-use and application lifecycle parity
- [ ] Linux (Wayland / X11) full computer-use and application lifecycle parity
- [x] Built-in bounded screen recording without requiring external OBS / Game Bar tools (`recording.rs`); production behavior still needs real-device verification.

## 11. Direction (see Goal.md)

- [ ] Live step timeline: render the timeline during a run from the `step_started` / `step_finished` / `plan_updated` events already emitted, using the same rows as the history view
- [ ] Tool call rows in the timeline: tool, target, before/after, risk, and undo where Kit staged the change
- [ ] Tools-first ladder: Kit tool, then UI Automation element, then pixels; allow background tasks to use write tools behind the safety gate
- [ ] Conversational turn alongside goal/task/step, with a spoken reply through Infer
- [ ] Headless Studio runtime: agent loop, board and scheduler run without the window; clients submit work and receive the event stream (home network first, tunnel later)
- [ ] Read memory from Rune Lake into prompts and write results back
- [ ] Move the life store to Rune Lake with its encryption, consent, retention and audit rules
- [ ] Move personalities to rune-agent as sub-agents; Studio selects and runs them
- [ ] Playground as the test bench: add a Tools tab (form from a Kit tool's input schema, result rendered from its output schema), tool steps in flows, and isolated runs that write no memory or task history
- [ ] Schema-driven result rendering shared by the Playground Tools tab and the workbench; no model-generated layouts
- [ ] Workbench window: step list with the model used per step, and the produced content where decisions are made
- [ ] Decisions with several options, one suggested, replacing yes/no confirmation
- [ ] Offer Infer's model operations to the agent as model-call steps, called directly on Infer
- [ ] Per-step model choice: tag each planned step's need, map to an Infer model id, group steps sharing a model
- [ ] Plan states whether an orchestrator with sub-agents is used; orchestrator runs non-conflicting sub-agent work in parallel; live and playback view per sub-agent
- [ ] Scheduled proposal pass over standing goals and read-only tools into the decision inbox (default level: suggest)
- [ ] Use only the embedded Infer for models; remote PC and cloud are reached through it
- [ ] Move the workflow registry and the Portfolio flow to rune-agent; RAW development becomes a Rune Tools plugin
- [ ] Linux computer-use parity; Android as a thin client
- [ ] Clarify step before planning: detect missing or conflicting details and ask direct questions with options and a suggestion
- [ ] Growing plans: admit steps added during a run, mark them as added, apply the safety gate
- [ ] Dependency record between step inputs and outputs, so feedback redoes only what depends on the changed part
- [ ] Feedback on any part of a result in the workbench, producing a new draft and keeping earlier ones
- [ ] Shared materials for a goal with provenance (source and licence if fetched, model if generated); keep sensitive data out of outbound requests