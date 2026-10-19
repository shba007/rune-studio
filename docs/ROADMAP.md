# Rune Studio roadmap: reflective personal agent + visual analysis

> Status: **plan, nothing here is built.** The current behaviour is in [`ARCHITECTURE.md`](ARCHITECTURE.md). Rules for this document: only facts read from the repositories named below, or decisions recorded here. Anything else is marked **NOT CERTAIN**. Durations and overhead numbers are estimates. Facts about the sibling projects were read from their code at the commits cloned on 2026-10-19 and not run.

## Goal and decisions

Rune Studio becomes a **reflective personal agent**: it records what it does, checks whether it worked, reflects while idle, and carries what it learns into later runs without making tasks noticeably slower. It also gains **visual analysis** (annotated image Q&A: labelled boxes, callouts and evidence linked to findings).

The four projects and who owns what:

| Project | Role (from its code/README) | Studio's relation |
|---|---|---|
| **rune-studio** | This app: the desktop agent | |
| **rune-infer** | Rust gateway with OpenAI-style APIs in front of local engines (llama-server, audio, encoder, ...) and cloud adapters | The model server; Studio is its HTTP client |
| **rune-kit** (`rune` CLI) | MCP runtime and package manager: installs WASM/native plugins and skills, runs `rune run --all` as one stdio MCP gateway | Planned: Studio spawns `rune` as an MCP client; kit owns MCP servers, skills, plugins and external binaries so Studio does not |
| **rune-tools** | Plugin registry and 14 plugins (filesystem, fetch, memory, git, browser, email, audio, video, image, slides, figma, time, sequential-thinking, one private email-template plugin) | Reached only through kit |

Decisions:
- Trajectory: reflective personal agent. Visual analysis: **general image Q&A with annotations**; v1 analyses the **current screen only**; no MCP tool for it yet.
- Idle = no task and no Rune interaction for 10 minutes. Reflection uses the same rune-infer server. A prompt in the pill offers **Deny**; a timeout starts reflection.
- Autonomy: reflection writes **notes and reports only**. Prompts and safety thresholds change only through approved diffs.
- Verifier runs at the end of a task only; overhead target **<= 10%** of task time. Traces go in **SQLite**.
- Model server: rune-infer, and rune-infer will change to match Studio's calls (see expectations). Voice stays enabled; `audio_url` support is to be implemented in rune-infer.
- Kit link: Studio is an MCP client of `rune`. Kit tools are offered to Studio's model **through the safety gate**.

## Ground facts

**Studio** (code at the time of writing):
- One async run per task; step = capture + UIA, chat call, gate, action, settle (`agent/mod.rs`). Recorded live run: 9 steps in 88 s (~10 s/step); **no timing exists for the current default `gemma-4-12b-it`**.
- Archive saves screenshots/audio with no run id; `history` is memory-only; `done{success}` is unverified self-report.
- One shared cancel flag; "task running" tested in 5 places; `Reflecting` state does not exist.
- The orb ring is driven by `plan_updated`; the overlay is click-through; the confirm card is clickable and its timeout means **No**.
- Studio has an MCP **server** (bridge) but **no MCP client**. No SQLite crate, no dialog plugin, no single-instance plugin in `Cargo.toml`. CI runs no Rust tests.
- Tool set is a fixed Rust enum (`agent/protocol.rs::Action`) with `tool_choice: "required"`; the gate takes an `Action`.

**rune-infer** (`/home/user/shba007/rune-infer`; read from code, not run):
- Endpoints include: `/v1/chat/completions`, `/v1/models`, `/v1/decisions`, `/v1/embeddings`, `/v1/ocr`, `/v1/images/{grounding, recognize, detections, captions, embeddings, ...}`, `/v1/audio/*`, `/v1/inference`, plus `/health`, `/livez`, `/readyz`. `/v1/responses` and Anthropic `/v1/messages` are disabled.
- Chat accepts `text`, `image_url`, `video_url` parts. **No `audio_url`/`input_audio` anywhere in `src`.** Studio's voice path sends `audio_url`.
- Thinking is read from `thinking`, then `chat_template_kwargs.thinking`, then `reasoning_effort`. `enable_thinking` is **not** read (Studio sends `chat_template_kwargs.enable_thinking`).
- `response_format` supports `json_object` and `json_schema` (strict needs `additionalProperties:false`). `usage` has only prompt/completion/total tokens (no image tokens, no `include_usage`); image tokens are estimated pre-flight.
- Client disconnect sets a cancel flag and stops generation (logged 499); whether this interrupts `llama-server` mid-prefill is **NOT CERTAIN**.
- `/v1/models` items carry a `capabilities` list (Chat, Tool Call, Vision, ...). No dedicated capabilities endpoint. `/health` has no queue depth. `llama-server` runs with `--parallel 1` by default; `max_loaded_models` default 1 (models swap). **No request priority anywhere.**
- `/v1/decisions` code path **does** pass images to the engine (`src/api/decisions.rs`). The maintainer's own observation (2026-10-19) was that images had no effect, so whether the engine uses them is **NOT CERTAIN**.
- `/v1/images/grounding` returns `point`, `point_pixels`, `box_2d` as **[ymin,xmin,ymax,xmax]** in 0-1000 (Studio's own regions are `[x0,y0,x1,y1]`; watch the order). The earlier M1.5 probe saw `[500,500]` every time, so output quality is **NOT CERTAIN**. `/v1/images/recognize` is a similarity matcher (`image, top_k, targets[]`), **not** a describe/detect endpoint. `/v1/ocr` takes `{document:{image_url|url}, model?}`. Request/response shapes of `detections` and `captions` were not read: **NOT CERTAIN**.
- Auth: bearer or `x-api-key`; off if no key configured.

**rune-kit** (`/home/user/shba007/rune-kit`):
- One `rune` binary; commands: `run [PLUGIN] [--all] [-p K=V]`, `install`, `uninstall`, `list`, `available`, `search`, `update [--check]`, `self-update`. Table output only, **no `--json`**.
- MCP over **stdio only** (protocol `2024-11-05`; tools, resources, prompts). Tools namespaced `<ns>__<tool>`. No HTTP API, no auth, no events/hooks/logging beyond `eprintln!`, no scheduler.
- Plugins: WASM in Extism, or native child processes over stdio JSON-RPC. Capabilities declared in `plugin.toml` (network hosts, filesystem root, allowed binaries). WASM enforcement exists; empty `network_hosts` + `allowed_hosts` allows `*`; **native plugins have no sandbox** in the code read.
- Skills: `SkillManager` installs markdown into `skills/<name>/`. Kit does not inject skills into any model's prompt; how Studio would consume them is **NOT CERTAIN**.
- `rune-kit-core` is a library (path dep only; crates.io publication **NOT CERTAIN**). Tests ~8; CI does not run `cargo test`.

**rune-tools** (`/home/user/shba007/rune-tools`): 14 plugins (5 WASM: filesystem, fetch, memory, sequential-thinking, time; 9 native: git, browser, email, audio, video, image, slides, figma, mhb-mconnect). `rune-browser` (needs `agent-browser >= 0.38.0`) has 17 tools incl. `browser_screenshot`; there is **no desktop-screen, OCR or vision plugin**. `rune-memory` is a JSON knowledge graph (no SQLite, no scheduling anywhere). Windows native builds exist. README/AGENTS list plugins that do not exist (print, scan, ssh, game-engine).

**oceanir.ai** (as fetched): depth-tiered location analysis (D1-D3) and API/MCP/CLI links; no mention of Orca, Manta or reflection. Orca/Manta details come only from a reference screenshot supplied by the maintainer.

## Milestones (difficulty L/M/H; feasibility from the facts above)

### R0 Foundations — M / high
- `RunScope` (run id + cancellation token) replacing the shared flag; one `is_task_running()`; `AgentState::Reflecting` counted as **not** running.
- Panic supervisor for `agent::run`; retry/backoff on transient model errors; serialise bridge actions.
- Single-instance (plugin choice **NOT CERTAIN**), elevated-window detection (Windows API **NOT CERTAIN**).
- CI: `cargo test`, `clippy`, `fmt --check` for Studio.
- **Wire-format audit against the real rune-infer**: record what actually happens for chat with image, tool_choice required, thinking on/off, and audio. Output: a table of "Studio sends / rune-infer accepts". Voice stays **enabled** and unchanged: `audio_url` is to be implemented in rune-infer, so Studio keeps sending it. For thinking, Studio additionally sends `thinking` next to `enable_thinking` (assumption: harmless to other servers, **NOT CERTAIN** until measured).
Exit: tests green in CI; stop/new-task race test; forced panic leaves the app usable; wire-format table committed.

### R1 Flight recorder — M / high
- SQLite (`rusqlite` bundled; builds on all CI targets **NOT CERTAIN**): `runs`, `steps`, `verdicts`, `notes`, `reflections`, `analyses`, **`tool_calls`** (kit calls, below). Files stay on disk; rows keep paths.
- Per-step: prompt, reply, tokens, latency, observation path, element-list hash, gate verdict, result, per-phase timings. Archive retention + `fsync` off the observation path.
Exit: every run (built-in or MCP) yields complete rows; browsable from UI/CLI.

### K1 Kit integration — M-H / medium
- Studio-side **MCP client** (stdio, JSON-RPC 2024-11-05) spawning `rune run --all`; `initialize`, `tools/list`, `tools/call`, `resources/*`, `prompts/*`. Needs `rune` on PATH or a configured path; **installing/updating kit and plugins stays on the `rune` CLI** (no `--json`, so Studio shows plain output or runs the command for the user; parsing tables is avoided).
- Dynamic tools: the per-turn tool list = fixed desktop actions + selected kit tools (names `<ns>__<tool>`). `agent/protocol.rs::Action` gets an `External { server_tool, args }` variant; `parse`, `describe`, `body::execute` and `rules::assess` must handle it.
- **Gate coverage (required):** every kit call goes through `safety::gate::check` before it runs. Risk rules need a per-tool policy table (e.g. `filesystem__write_file`, `edit_file`, `move_file`, `git_commit`, `send_email`, `reply_email` high; reads low), because kit native plugins are unsandboxed and `fetch`/`email` allow any host. Default for an unknown tool: **ask**.
- Record every call in `tool_calls` (kit has no logs of its own).
- Failure handling: kit child crash/hang, per-call timeout, and Stop cancelling in-flight calls. The native sidecar call is serialised per process in kit, so a long call blocks others (**NOT CERTAIN** of effect on the loop).
- Rollout: allow a curated read-only set first; widen after R2 shows tools help (the gate is the safeguard; the allowlist is only staging).
Exit: with kit installed, the model can call `filesystem__read_text_file`; a write call pauses for Yes; Stop cancels; calls appear in the trace.

### R2 Eval harness and baseline — M-H / medium
- ~15 fixed desktop tasks plus a few kit-tool tasks; records success, steps, time, tokens; fast checks on/off. Replay mode against stored episodes (needs Observer/Actor traits). Local PowerShell script first; a self-hosted Windows runner is **NOT CERTAIN**.
Exit: baseline for the current default model. Every later milestone is gated: success not lower, median task time within +10%.

### R3 Verifier — M / high
- One omni-model call at `done` (no thinking, small `max_tokens`, reuse the already captured frame): `verified | contradicted | unsure` + evidence, stored. Default **background** (0% perceived overhead); optional blocking mode (one more ~10 s call on a ~90 s run is ~11%, over budget, so off by default; estimate from one recorded run, **NOT CERTAIN** for the current model).
- Prefer `response_format: json_schema` on rune-infer (supported per code) over a forced tool call; choose after R0's wire audit.
Exit: agreement with human labels measured; overhead <= 10%; no success drop.

### R4 Visual analysis v1 — M-H / medium
- Input: **current screen only** in v1 (decision); files/drag-drop are out of scope.
- Pipeline options, to be settled by a probe on the synthetic scenes (`scripts/m15`): (a) chat with a forced `report` tool/JSON schema returning findings `{label, kind, box, confidence, description, evidence}`; (b) compose rune-infer's `/v1/images/grounding` (box), `/v1/ocr` (text) and `captions`/`detections` (shapes **NOT CERTAIN**). Normalise box order ([ymin,xmin,ymax,xmax] from grounding vs Studio's [x0,y0,x1,y1]).
- Kit helpers (optional): `rune-image__compare_images` for before/after diffs; `rune-filesystem__read_media_file` for files. Not required.
- Result view: new larger window (like overlays; capability file update), image with numbered boxes, leader lines, caption tags, evidence list linked to boxes. Visual design details **NOT CERTAIN** until a layout is chosen. Persist in `analyses`.
- Trigger: pill button only. **No `rune_analyze` MCP tool** for now (decision).
- Risk: box accuracy never measured (M1.5 measured points only).
Exit: probe reports box hit rate per path; the view renders findings with working evidence links.

### R5 Idle detection + reflection shell — M / high
- 10-min idle timer (patterns: `resume_listening_soon`, `bridge::server::watch_for_idle`); prompt on the pill card (not the click-through overlay bubble) with **Deny**; auto-start on timeout (**length NOT CERTAIN, proposing 30 s**); Deny skips this cycle (**re-prompt policy NOT CERTAIN**).
- `Reflecting` state: new orb motion; ring shows a new `reflection_progress {stage, done, total}` event. Orb colour **NOT CERTAIN** (design allows red only for attention).
- Pre-emption: a task start, hotkey or Stop cancels reflection via its own `RunScope`. rune-infer has no priority and `--parallel 1` by default, so a reflection call already running **will delay** the next user task until it ends (**NOT CERTAIN** how long; mitigation: short calls, one at a time, abort on pre-empt; real fix is a server-side priority, listed below).
Exit: with a stub job, idle -> prompt -> reflecting -> task start pre-empts within a second; Deny and timeout work.

### R6 Post-mortems and notes — H / medium
- Candidates: failed, stuck, declined, verifier-contradicted, slow outliers (+ failed kit tool calls). One structured call per episode: cause category (grounding miss, stale element, popup/blocker, invalid answer, stuck, declined, elevated window, kit tool error), evidence step ids, proposed note, confidence.
- Output: reports and **pending** notes scoped by app (foreground process; Windows only). Notes live in Studio's SQLite. Mirroring accepted notes into `rune-memory` (kit) so other agents share them is an option, **NOT CERTAIN** it is wanted; its JSON-graph format is not designed for this.
- Accepted notes feed prompts; pending ones do not (default, **NOT CERTAIN** it is the preferred behaviour).
- Image token cost cannot be measured (server `usage` lacks it): cap images per session.
Exit: on seeded failing episodes, reports name the right category at a hand-measured rate.

### R7 Notes in the loop + proof — M / medium
- "Notes for this app" block in `prompt::user_message` (capped). A/B notes on/off on R2. Keep only if success is not lower and time within +10%. Outcome **NOT CERTAIN**.

### R8 Trust — M / medium (parallel)
- Redaction/retention/export/delete for archive and trace store; dry-run "record and propose, never act" replay; sandbox/VM feasibility **NOT CERTAIN**; tamper-evident hash chain optional, **NOT CERTAIN** it is worth it. Kit plugin trust: show which plugins/capabilities are installed and their declared hosts/binaries in Settings (data comes from `plugin.toml`; read path through kit **NOT CERTAIN**).

### R9 Universal abstractions — H / low-medium (later)
- Provider/capability traits; Observer/Actor/ElementSource traits (started in R2). Second surface: **`rune-browser` via kit already gives a DOM/accessibility-level browser surface** (needs `agent-browser`), so the first "universal" test can be browser tasks through kit instead of new Studio code. Multi-monitor needs hardware this PC lacks. Skills/macros, safety calibration, model routing, prompt tuning stay deferred.

## Expectations of other repos (none verified to be planned)

**rune-infer**
1. Accept `thinking` **and** `enable_thinking` in `chat_template_kwargs`, and accept `audio_url`/`input_audio` content parts (or tell Studio the supported audio route).
2. Capability discovery: per-model vision/audio/tools/thinking/max image size/context in one place (today a free-text `capabilities` list on `/v1/models`).
3. Request priority or busy signal (header, `429/503 + Retry-After`, or queue depth in `/health`), so reflection yields.
4. `usage` with image tokens and `stream_options.include_usage`.
5. Confirm that disconnect aborts `llama-server` prefill; document it.
6. `/v1/decisions` honouring images in the engine; a test showing an obviously-yes image answers yes.
7. Documented request/response for `detections`, `captions`, `ocr`, and a describe-with-boxes route (or confirm chat + json_schema is the intended path); grounding quality notes.
8. Real backend in `/health` (it currently reports a hardcoded string).

**rune-kit**
1. `--json` output for `list`, `available`, `search`, `install`, `update --check`.
2. A way to learn tool capabilities/risk (e.g. expose `plugin.toml` `[capabilities]` through MCP or the CLI).
3. Optional HTTP/streamable transport, and structured logs/events for tool calls.
4. Decide the `network_hosts = *` default and native-plugin sandboxing.

**rune-tools**
1. A desktop OCR/vision-capable plugin is absent; decide whether it belongs there or in rune-infer.
2. Fix README/AGENTS lists of plugins that do not exist.

## Open questions (defaults noted)
- Reflection prompt timeout: 30 s. After Deny: re-prompt after another 10 idle minutes.
- Orb colour for reflecting: monochrome with distinct motion (default) or a tint.
- Accepted notes enter prompts automatically (default) or after a second confirmation; mirror to `rune-memory`: no (default).
- Where evals run: local script (default).
