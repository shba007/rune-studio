# Rune Studio roadmap: reflective personal agent + visual analysis

> Status: **plan, nothing here is built.** The current behaviour is in [`ARCHITECTURE.md`](ARCHITECTURE.md). Rules for this document: only facts read from the repositories named below, or decisions recorded here. Anything else is marked **NOT CERTAIN**. Durations and overhead numbers are estimates. Facts about the sibling projects were read from their code at the commits cloned on 2026-10-19 and not run.

## Goal and decisions

Rune Studio becomes a **reflective personal agent**: it records what it does, checks whether it worked, reflects while idle, and carries what it learns into later runs without making tasks noticeably slower. It also gains **visual analysis** (annotated image Q&A: labelled boxes, callouts and evidence linked to findings).

The four projects and who owns what:

| Project                   | Role (from its code/README)                                                                                                                                                       | Studio's relation                                                                                                              |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **rune-studio**           | This app: the desktop agent                                                                                                                                                       |                                                                                                                                |
| **rune-infer**            | Rust gateway with OpenAI-style APIs in front of local engines (llama-server, audio, encoder, ...) and cloud adapters                                                              | The model server; Studio is its HTTP client                                                                                    |
| **rune-kit** (`rune` CLI) | MCP runtime and package manager: installs WASM/native plugins and skills, runs `rune run --all` as one stdio MCP gateway                                                          | Planned: Studio spawns `rune` as an MCP client; kit owns MCP servers, skills, plugins and external binaries so Studio does not |
| **rune-tools**            | Plugin registry and 14 plugins (filesystem, fetch, memory, git, browser, email, audio, video, image, slides, figma, time, sequential-thinking, one private email-template plugin) | Reached only through kit                                                                                                       |

Decisions:

- Trajectory: reflective personal agent. Visual analysis: **general image Q&A with annotations**; v1 analyses the **current screen only**; no MCP tool for it yet.
- Idle = no task and no Rune interaction for 10 minutes. Reflection uses the same rune-infer server. A prompt in the pill offers **Deny**; a timeout starts reflection.
- Autonomy: reflection writes **notes and reports only**. Prompts and safety thresholds change only through approved diffs.
- Verifier runs at the end of a task only; overhead target **<= 10%** of task time. Traces go in **SQLite**.
- Model server: rune-infer, and rune-infer will change to match Studio's calls (see expectations). Voice stays enabled; as of 2026-10-19 Studio sends audio as `input_audio` (the only form rune-infer reads) and rune-infer does not need to add `audio_url`.
- Kit link: Studio is an MCP client of `rune`. Kit tools are offered to Studio's model **through the safety gate**.

## Ground facts

**Studio** (code at the time of writing):

- One async run per task; step = capture + UIA, chat call, gate, action, settle (`agent/mod.rs`). Recorded live run: 9 steps in 88 s (~10 s/step); **no timing exists for the current default `gemma-4-12b-it`**.
- Archive: screenshots/audio are saved per session folder (one task run) with a `session.json` manifest (done in the app, see `docs/ARCHITECTURE.md`), but there is still no run id shared with a database, no per-step record, and `history` is memory-only; `done{success}` is unverified self-report.
- One shared cancel flag; "task running" tested in 5 places; `Reflecting` state does not exist.
- The orb ring is driven by `plan_updated`; the overlay is click-through; the confirm card is clickable and its timeout means **No**.
- Studio has an MCP **server** (bridge) but **no MCP client**. No SQLite crate, no dialog plugin, no single-instance plugin in `Cargo.toml`. CI runs no Rust tests.
- Tool set is a fixed Rust enum (`agent/protocol.rs::Action`) with `tool_choice: "required"`; the gate takes an `Action`.

**rune-infer** has two sources that disagree in places; both are recorded. _Source_ = the GitHub repo at commit `bd155a5` (2026-10-19), read, not run. _Spec_ = the Postman collection "Rune REST API" the maintainer supplied on 2026-10-19 as the latest API spec (115 files; examples only, not executed). Where they differ, which one the running server follows is **NOT CERTAIN** until a live call is made (R0).

- Endpoints include: `/v1/chat/completions`, `/v1/models`, `/v1/decisions`, `/v1/embeddings`, `/v1/ocr`, `/v1/images/{grounding, recognize, detections, captions, embeddings, ...}`, `/v1/audio/*`, `/v1/inference`, plus `/health`, `/livez`, `/readyz`. `/v1/responses` and Anthropic `/v1/messages` are disabled.
- Audio in chat. _Source_: `text`, `image_url`, `video_url` parts only; no `audio_url` or `input_audio` in `src`. _Spec_: a part `{"type":"input_audio","input_audio":{"data":<base64>,"format":"wav"|"mp3"}}` (other formats return 400 `unsupported_audio_format`), mixed with `image_url` and `tools` in one message. Studio sends `{"type":"audio_url","audio_url":{"url":"data:audio/wav;base64,..."}}`, which **neither** source documents. _Resolved 2026-10-19: the running server reads only `input_audio`; Studio now sends it (expectation 1)._
- Thinking. _Source_: read from `thinking`, then `chat_template_kwargs.thinking`, then `reasoning_effort`; `enable_thinking` is not read. _Spec_: `reasoning_effort: "high"` plus `"thinking": {"type":"enabled","budget_tokens":2048}`; reply carries `message.reasoning_content`. Studio sends `chat_template_kwargs.enable_thinking`, which neither documents. _Resolved 2026-10-19: `thinking: {"type":"disabled"}` and `reasoning_effort: "none"` are honoured, `enable_thinking` is not; Studio now sends `thinking` (expectation 1)._
- `response_format` supports `json_object` and `json_schema` (strict needs `additionalProperties:false`). `usage` has only prompt/completion/total tokens (no image-token field); image tokens are estimated pre-flight. _Spec_ shows `stream_options: {"include_usage": true}` for streaming; _source_ has no `include_usage` (**NOT CERTAIN** which holds). _Spec_ examples use `tool_choice: "auto"`; Studio's `"required"` is supported per source (**NOT CERTAIN** per spec).
- Client disconnect sets a cancel flag and stops generation (logged 499); whether this interrupts `llama-server` mid-prefill is **NOT CERTAIN**.
- `/v1/models` items carry a `capabilities` list (Chat, Tool Call, Vision, ...). No dedicated capabilities endpoint. `/health` has no queue depth. `llama-server` runs with `--parallel 1` by default; `max_loaded_models` default 1 (models swap). **No request priority anywhere.**
- `/v1/decisions`: the maintainer states the decision engine supports images; the source decodes `images` and passes them to the engine (`src/api/decisions.rs`) and the spec has a multimodal example (`state: {text, images[]}`). **The response shape differs:** _Spec_: `answers.<name> = {"type", "value", "confidence"}` with `noul` value `"yes"`/`"no"`, `choice` value a string, `score` value a number; _source_ (`DecisionAnswer` in `src/types.rs`): `noul {decision: bool, probability}`, `choice {decision: string, confidence, probabilities}`, `score {level, score, probabilities}`. Studio's parser (`model/decision.rs`) matches the **source** shape. If the running server follows the spec, `confident_yes` can never be true and every fast check would silently answer "no", which would explain the earlier "images have no effect" observation; this is a **hypothesis, NOT CERTAIN** until a live call shows which shape is returned. Default decision model in the spec's environment is `liquid-d1-3b`; Studio's default is `d1-3b-q8-0`.
- Image endpoints (_Spec_ examples; request in, response out):
  - `/v1/images/grounding`: `{model, image, query}` -> `point` as **0-1 fractions** (example `[0.85, 0.75]`), `point_pixels`, `box_2d` (example `[740, 830, 796, 910]`, which looks like **pixels**, order **NOT CERTAIN**), `box_normalized` (example `[0.72, 0.81, 0.78, 0.89]`, order **NOT CERTAIN**), `confidence`. This contradicts my earlier note that boxes are 0-1000. The earlier M1.5 probe saw `[500, 500]` every time, so output quality is **NOT CERTAIN**.
  - `/v1/images/detections`: `{model, image, prompt, categories[], confidence_threshold, iou_threshold, features[boxes|masks|keypoints|attributes]}` -> `data[]` of `{label, confidence, box{x,y,width,height} (fractions; whether x,y is the corner or centre is NOT CERTAIN), bbox_pixels{x_min,y_min,...}}`.
  - `/v1/images/captions`: `{model, image, detail, max_tokens}` -> `{caption, usage}`.
  - `/v1/images/recognize`: a similarity matcher (`{image, top_k, targets[{label, text|collection}]}` -> `data[{label, similarity}]`), **not** a describe/detect endpoint.
  - `/v1/ocr`: `{model, document{type, image_url}, features[text|selection_marks], response_format}` -> `text` and `pages[{dimensions, lines[{text, bbox}]}]`.
- `/health` (_Spec_): `{status, service, version, backend, loaded_models[]}`; `/readyz`: `{status, models_configured, models_loaded[], models_loading[]}`. `GET /v1/models/{id}` (_Spec_) lists capabilities such as `Chat, Vision, Audio, Video, Omni, Tool Call`; the example omni model is `gemma-4-12b-it`, the same as Studio's default.
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

**Status as of 2026-10-19 (merged into `develop`):**

| Item                                              | Status                                                                                                                                               |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `RunScope`, `is_running`, `Reflecting`            | built, tests pass                                                                                                                                    |
| Panic supervisor for `agent::run`                 | built, tests pass                                                                                                                                    |
| Retry/backoff on transient model errors           | built, tests pass                                                                                                                                    |
| Serialised bridge actions                         | built, tests pass                                                                                                                                    |
| Lock helper (`sync::LockExt`)                     | built, tests pass                                                                                                                                    |
| `model/decision.rs` accepts both response shapes  | built, tests pass (which shape a live server sends is not known)                                                                                     |
| CI                                                | built: `rust.yml` ran `cargo test --lib` (both feature sets) green on Windows and Ubuntu for the feature branch (PR 15); clippy and fmt are advisory |
| Wire-format audit script `scripts/wire-audit.mjs` | built, **never run against a real rune-infer**; no table committed yet, so the exit criterion for the audit is open                                  |
| Single-instance                                   | built, **not run on Windows**                                                                                                                        |
| Elevated-window detection                         | built, **not run on Windows**; Windows may refuse the token of a higher-level process (**NOT CERTAIN**)                                              |
| Stop/new-task race test                           | built as unit tests on `RunScope`; no end-to-end test                                                                                                |
| Forced-panic test                                 | built as a unit test on the supervisor; not tried in the running app                                                                                 |

The original plan:

- `RunScope` (run id + cancellation token) replacing the shared flag; one `is_task_running()`; `AgentState::Reflecting` counted as **not** running.
- Panic supervisor for `agent::run`; retry/backoff on transient model errors; serialise bridge actions.
- Single-instance (plugin choice **NOT CERTAIN**), elevated-window detection (Windows API **NOT CERTAIN**).
- CI: `cargo test`, `clippy`, `fmt --check` for Studio.
- **Wire-format audit against the real rune-infer**: record what actually happens for chat with image, tool_choice required, thinking on/off, and audio. Output: a table of "Studio sends / rune-infer accepts". Voice stays **enabled**. Decision pending: the earlier plan was that rune-infer would add `audio_url` (Studio's current part); the Spec instead documents `input_audio {data, format}`. Which side changes is **an open question for the maintainer**; until answered, the audit only records what a live server does with each form. For thinking, the audit tries `thinking` / `reasoning_effort` alongside `enable_thinking` (assumption: harmless to other servers, **NOT CERTAIN** until measured). The audit also records the real `/v1/decisions` response shape and makes `model/decision.rs` accept both the Spec and Source shapes.
  Exit: tests green in CI; stop/new-task race test; forced panic leaves the app usable; wire-format table committed.

### R1 Flight recorder — M / high

- SQLite (`rusqlite` bundled; builds on all CI targets **NOT CERTAIN**): `runs`, `steps`, `verdicts`, `notes`, `reflections`, `analyses`, **`tool_calls`** (kit calls, below). Files stay on disk; rows keep paths.
- Per-step: prompt, reply, tokens, latency, observation path, element-list hash, gate verdict, result, per-phase timings. Archive retention + `fsync` off the observation path. (The per-session folder and `session.json` already exist; the run id in the database should be the session's `id`.)
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

1. ~~Settle the audio part and the thinking switch.~~ **Resolved 2026-10-19:** Studio moved to `input_audio {data, format: "wav"}` (rune-infer commit `9a34e50`, unreleased at the time of writing) and `thinking: {"type": "disabled" | "enabled"}`; `enable_thinking` is still sent but rune-infer does not read it.
2. Capability discovery: per-model vision/audio/tools/thinking/max image size/context in one place (today a free-text `capabilities` list on `/v1/models`).
3. Request priority or busy signal (header, `429/503 + Retry-After`, or queue depth in `/health`), so reflection yields.
4. `usage` with image tokens and `stream_options.include_usage`.
5. Confirm that disconnect aborts `llama-server` prefill; document it.
6. ~~Make the `/v1/decisions` response match its published spec.~~ **Resolved 2026-10-19:** the server returns `{decision, probability}` / `{decision, confidence, probabilities}` / `{level, score, probabilities}`; the Postman `{type, value, confidence}` is legacy, and images are used. Still wanted: a test with an image whose answer is obviously yes.
7. Specify coordinate conventions in the Postman examples: grounding `box_2d` units and order, detection `box` corner vs centre. Confirm whether a describe-with-boxes route exists or chat + `json_schema` is the intended path.
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

**For the rune-infer maintainer (all NOT CERTAIN, raised 2026-10-19):**

- What is the default thinking when no parameter is sent? A pasted reply with none contained `reasoning_content`, so it may be on by default, which would add hidden thinking tokens to every step.
- What does a choice answer's `confidence` mean? It was 0.1374 for a pick whose probability was 0.4249.
- `usage` token counts looked unreliable (prompt 786 = completion 786 for an audio call; prompt 1, completion 1 for the reply "Four").
- Does `thinking: {"type":"enabled"}` need `budget_tokens`?
- For WAV, should `input_audio.data` include the header, or is it raw PCM? (Studio sends the whole WAV file.)
- Does a request disconnect abort prefill? (Expectation 5.)

- Reflection prompt timeout: 30 s. After Deny: re-prompt after another 10 idle minutes.
- Orb colour for reflecting: monochrome with distinct motion (default) or a tint.
- Accepted notes enter prompts automatically (default) or after a second confirmation; mirror to `rune-memory`: no (default).
- Where evals run: local script (default).