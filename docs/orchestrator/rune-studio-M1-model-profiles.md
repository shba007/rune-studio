# Handoff: Milestone M1, interchangeable models (profiles, roles, swap-awareness, per-model baselines)

For an AI coding agent working in the **rune-studio** repository. Read all of it first. Current behaviour: `docs/ARCHITECTURE.md`. Where this file and the code disagree about a fact, the code wins: say so in the report. Related (other repositories, not yours to change): the rune-infer handoff requests INF-7 (structured capabilities), INF-15 (local or cloud), INF-16 (served-model identity), INF-17 (support matrix). Studio must work **before** those exist and use them when they do.

## 1. Goal

The owner will use models interchangeably, for example **MiniCPM-o 4.5**, **Bonsai 2 27B**, `gemma-4-12b-it` and others, with rune-infer's hot swap. Studio today assumes one model (`settings.model`, plus `decision_model`). After M1:

- Studio knows, per model, what it can do and how to talk to it (a **profile**), and never hard-codes a model's quirks in the agent loop.
- The user chooses a model **per role**: agent, verifier, analysis, voice, decision (and `advisor`, reserved for the life track).
- A model swap is a normal, visible, slow event, not a failure.
- Every call and every run records which model served it, and evaluation results are compared **per model**.
- A new, unknown model can be **calibrated** in minutes by a built-in probe instead of by editing code.

## 2. Rules

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop`, then `git push origin develop`.
2. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (`cargo test --lib`, with `--features agent-bridge`, with `--features eval,agent-bridge`, `node --test scripts/eval/test`, `bun run test:unit`, clippy, fmt) and again at the end.
3. Never claim a live behaviour you did not observe. You have no model server: test with fakes and recorded fixtures, and write exact live steps for the maintainer.
4. **Do not write anything specific to MiniCPM-o 4.5 or Bonsai 2 27B in code.** Facts about them (from secondary and vendor sources, 2026-10-21, **not verified**): MiniCPM-o 4.5 is a 9B omni model (vision, speech in and out, full-duplex streaming; Apache 2.0; a GGUF exists); Bonsai 2 27B is a ternary 27B model, about 5.95 GB, 262,144-token context, reportedly needing PrismML's fork or MLX loader. They exist here only to show that models differ; the profile mechanism must handle them without code changes. Any default profile data you ship must be for a model whose behaviour was **measured** (only `gemma-4-12b-it` was, in the field test); every other model starts as "unknown, conservative".
5. Backward compatible: an existing `settings.json` (with `model`, `decisionModel`, `decisionUrl`) must keep working and migrate without loss. Test it with fixtures of old files.
6. Everything model-related stays in `src-tauri/src/model/` and settings; the agent loop asks for a role and gets a client plus a profile.

## 3. What is known and what is not

Known from the code (check it): `settings.rs` has one `model`, `decision_model`, `decision_url`, `fast_checks`, `show_thinking`; `model/client.rs` builds the chat body (sending both `thinking: {"type": ...}` and `chat_template_kwargs.enable_thinking`), retries 3 times; voice sends the whole WAV as `input_audio` to the chat model; `model/coords.rs` uses the `0..1000` convention for points and boxes; the verifier and the screen analysis use the chat model; the field test measured `gemma-4-12b-it` only (thinking off: decide median 4.5 s; on: 18 s; model swaps cost 7 to 16 s).

**NOT CERTAIN (measure or leave to the maintainer):** how each candidate model expresses coordinates (pixels, 0..1, 0..1000, other) and how well it points; whether it honours `thinking` and in which form; forced tool-call and JSON reliability; maximum useful image size; whether MiniCPM-o 4.5's audio hearing is better than the chat model's for Studio's voice path; whether rune-infer will provide structured capabilities or swap headers (INF-7, INF-16, INF-4) and under what names.

## 4. Work packages

### WP1: capability client (tolerant)

- Read `/v1/models`. Today's capabilities are a free-text list; INF-7 will add structured fields (`modalities_in`, `modalities_out`, `tool_calls`, `thinking_control`, `max_image_edge_px`, `context_tokens`, `locality`, `resident`, `vram_mb_estimate`, `load_ms_estimate`, `status`). Parse **both**, treating every unknown or missing field as **unknown** (a three-state value: yes, no, unknown), never as no. Unknown fields from a newer server are ignored, not errors.
- Cache with a short time-to-live; refresh when the user opens Settings and on a failed call; never block a task on it; a server without `/v1/models` works.
- Tests with fixtures: today's shape, the future structured shape, a partial one, garbage, an empty list.

### WP2: model profiles

- A `ModelProfile` with: `thinking` (how to switch: a list of request styles to send, or `none`), `audio_in` (yes/no/unknown, format), `images` (max edge, whether a screenshot at the app's size is fine), `pointing` (`norm1000`, `pixels`, `norm1`, `unknown`), `tool_choice_required` (yes/no/unknown), `max_tokens` defaults per role, `locality` (`local`, `cloud`, `unknown`).
- **Data, not code:** defaults live in a bundled data file (JSON) keyed by an exact model id or a documented pattern; the user can override in settings; precedence is user, then measured defaults, then server capabilities, then conservative. Only models measured by the project may be in the bundled file (today: `gemma-4-12b-it`).
- A conservative profile for unknown models: thinking off by sending every known style, `pointing` unknown (the agent then prefers `click_element` by UI Automation id and asks for a zoomed look instead of trusting a point), tool choice not assumed.
- Coordinate adapters: one pure module converts a model's answer in its `pointing` convention to Studio's `0..1000`. Tests with fixed numbers for each convention, a non-square screen, and 100/125/150% scaling.
- The request builder reads the profile (no `if model.contains(...)` anywhere). Test that a profile changes the request body exactly as described.

### WP3: roles

- Settings gain `models: { agent, verifier, analysis, voice, decision, advisor }`, each a model id (empty means "same as agent"). Migration: old `model` becomes `models.agent`, old `decisionModel` becomes `models.decision`; both old keys still read, and are written back only once, so a downgrade is not broken (test with fixtures).
- The agent loop, verifier, screen analysis, voice and decision code ask the settings for **their role** and get a client plus its profile. Remove direct reads of `settings.model` elsewhere.
- Settings UI (the existing settings window, in its style): one compact row per role with a model picker fed from `/v1/models`, a dot-style capability line (image in, audio in, speech out, tools, thinking; unknown shown as unknown) and a plain warning when a role's model lacks what the role needs (for example voice with a model that has no audio input, agent with no image input). A model that is **not local** shows a clear `CLOUD` tag.
- Tests: migration, role resolution, warnings (component tests).

### WP4: swap-aware calls

- A model swap is expected. When rune-infer says (a header, a status or a `loading` state; the names are **NOT CERTAIN** until INF-4 and INF-16, so isolate them behind one small function) that a model is loading or was swapped, extend the call's wait, show a calm "loading model" state in the pill (the dot arc; no red, no error), and record the swap time apart from the model's thinking time in the trace (`calls` table).
- Without such signals: a configurable allowance for the **first call after idle** (default generous, measured from recorded call durations, not guessed), and never give up a call that is still progressing because of a short timer (the field test: a 4 s client timeout abandoned a decision call while the swap went on and made tasks slower).
- A role call whose model is **not resident** and whose role is optional (decision, verifier) may be skipped, with a recorded reason, when the user chose "do not swap for optional roles" (setting, default on). Tests with a fake server that delays, swaps and refuses.
- Record the **served model** per call and per step (the response's `model`; INF-16 will make it reliable). If it differs from the requested one, log a warning and show it in the Flight Player's API tab (that data already exists in the `calls` table; add the field if missing).

### WP5: per-model evaluation baselines

- The evaluation harness (`scripts/eval/`, `eval` feature) stores the model id and the resolved profile in its environment file and in every results line.
- `report.mjs` compares only runs of the **same model and profile** for the existing gate (success not lower by more than one run, median duration within 10%); comparing different models prints a **model comparison table** instead (success, median time, steps, estimated tokens, boxes quality if present) and never a pass or fail.
- A `baselines/<model>/` convention in the docs, no baseline data committed (the maintainer produces it).
- Tests with fixture result files for two models.

### WP6: the calibration probe for a new model

- Extend the existing probe (`probe_m15.rs`, `scripts/m15/`) with a **profile mode**: for the model chosen on the command line it runs small synthetic scenes and writes a **suggested profile** (JSON): whether thinking is switched by each request style (compare durations and presence of reasoning), whether forced tool calls return valid arguments (count malformed over N tries), the pointing convention that best explains its answers on scenes with known targets (try the candidate conventions and report which has the best hit rate), a safe maximum image edge by trying two sizes.
- The probe **suggests**; the user approves by copying the profile into settings (a "Use this profile" action in Settings when a probe file exists is a nice-to-have, not required).
- Do not run it (no model); tests for the pure scoring functions with fixed inputs; exact commands in the report: `pwsh scripts/m15/run-probe.ps1 -Modes "<model>:profile"`.

### WP7: voice provider seam (small)

- Introduce a `Transcriber` interface with one implementation today (the chat model of role `voice`, as now). Another implementation (a dedicated speech route, INF-5) can be added later without touching the voice code. If the role's profile says audio input is **no**, voice is disabled with a clear message instead of failing.
- No other voice change here.

### WP8: docs and tests

- `docs/ARCHITECTURE.md` (models, roles, profiles), `README.md` (Settings), `docs/use-cases/` pages that name the model or its limits (state that results are per model), and a short `docs/models.md` explaining profiles, roles, the probe and how to add a model (user-facing, plain language).
- Final test numbers; clippy and fmt clean.

## 5. Do not

No model-specific code or strings; no bundled profile for an unmeasured model; no new dependency without writing the case; no change to prompts beyond what a profile requires; no change to the safety gate; do not remove the old settings keys in this milestone; do not weaken the retry behaviour.

## 6. Report

`docs/handoff/M1-report.md`: baseline and final numbers; per work package what changed and its tests; live steps for the maintainer (pick two different models for two roles, swap mid-session and see the loading state; an old settings file; the probe command for a new model; the per-model report); each choice you made where this document gave options; every NOT CERTAIN with what you measured or left open; and whether INF-7, INF-15 and INF-16 data was available to test against (say plainly if you could only use fixtures).