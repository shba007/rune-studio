# M1 report: interchangeable models (profiles, roles, swap-awareness, per-model baselines)

Worked directly on `develop`, one commit per work package, pushed to `origin develop` after each. **No model server was run live**: Studio was verified using unit, component, fake server, and recorded fixtures. Exact live steps for the maintainer are documented below.

---

## 1. Test Numbers

All test suites were recorded at the start of M1 (baseline) and again at the end of M1. All test counts increased; zero tests broken or regressed.

| Command                                             | Baseline (Before M1)            | Final (After M1)                                  | Delta       |
| :-------------------------------------------------- | :------------------------------ | :------------------------------------------------ | :---------- |
| `cargo test --lib`                                  | 319 passed, 0 failed, 9 ignored | 360 passed, 0 failed, 10 ignored                  | +41 passed  |
| `cargo test --features agent-bridge`                | 358 passed, 0 failed, 9 ignored | 399 passed, 0 failed, 10 ignored                  | +41 passed  |
| `cargo test --features eval,agent-bridge`           | 389 passed, 0 failed, 9 ignored | 431 passed, 0 failed, 10 ignored                  | +42 passed  |
| `node --test scripts/eval/test/report.test.mjs`     | 15 passed, 0 failed             | 18 passed, 0 failed                               | +3 passed   |
| `bun run test:unit` (Vitest)                        | 142 passed, 0 failed (17 files) | 148 passed, 0 failed (18 files)                   | +6 passed   |
| `cargo clippy --manifest-path src-tauri/Cargo.toml` | 10 warnings                     | 1 pre-existing warning (`lease.rs`), 0 on M1 code | -9 warnings |
| `cargo fmt --check`                                 | Clean                           | Clean                                             | Clean       |
| `bun run lint` (`vp lint`)                          | Clean                           | Clean (0 warnings, 0 errors across 95 files)      | Clean       |

---

## 2. Work Packages Delivered

### WP1: Tolerant capability client (`feat(model): tolerant capabilities parser and cache (WP1)`)

- Read `/v1/models` handling both legacy free-text lists and structured INF-7 fields (`modalities_in`, `modalities_out`, `tool_calls`, `thinking_control`, `max_image_edge_px`, `context_tokens`, `locality`, `resident`, `vram_mb_estimate`, `load_ms_estimate`, `status`).
- Added `TriState` (`Yes`, `No`, `Unknown`) and `Locality` (`Local`, `Cloud`, `Unknown`); unknown fields are safely treated as unknown, never as `No`. Unknown future keys from newer servers are ignored.
- Implemented `CapabilityCache` with a 30s TTL, refreshed on Settings open and call failure.
- Unit tests with 5 fixture shapes: current free-text, future structured, partial, garbage, empty list.

### WP2: Model profiles and coordinate adapters (`feat(model): model profiles and coordinate adapters (WP2)`)

- Introduced `ModelProfile` with `thinking` (`ThinkingSwitch`: `none` or list of styles `thinking_type`, `chat_template_kwargs`, `budget_tokens`), `audio_in` (`TriState`, format), `images` (`max_edge_px`, `screenshot_at_app_size`), `pointing` (`Norm1000`, `Pixels`, `Norm1`, `Unknown`), `tool_choice_required` (`TriState`), `max_tokens` per role, `locality` (`Local`, `Cloud`, `Unknown`).
- **Data, not code**: Defaults live in bundled JSON (`src-tauri/src/model/profiles.json`). Precedence: user settings override > measured bundled defaults > server capabilities > conservative defaults. **Bundled defaults contain only `gemma-4-12b-it`** (measured in field test).
- Zero code written specific to MiniCPM-o 4.5 or Bonsai 2 27B.
- Conservative profile for unknown models: sends all known thinking switch styles, pointing treated as unknown, tool calling unforced.
- Pure coordinate adapter module `src-tauri/src/model/coords.rs` (`adapt_point_to_norm1000`, `adapt_box_to_norm1000`) tested with fixed numbers, non-square displays, and 100%, 125%, 150% scaling.
- Request builder reads profile directly with no `if model.contains(...)` anywhere.

### WP3: Roles and role assignment UI (`feat(settings): model roles and settings UI (WP3)`)

- Added `models: { agent, verifier, analysis, voice, decision, advisor }` to `Settings`.
- **Bidirectional migration**: Legacy `model` migrates to `models.agent`; legacy `decisionModel` migrates to `models.decision`. Both old keys remain readable and are synchronized so older Studio versions do not break on downgrade. Tested with old fixture files.
- Refactored agent loop, verifier, screen analysis, voice, and decision code to request their role client and profile. Removed direct reads of `settings.model` elsewhere.
- Added `ModelRoleRow.vue` component to Settings UI: compact row per role, `/v1/models` dropdown, dot capability line (`imageIn`, `audioIn`, `speechOut`, `toolCalls`, `thinking`), capability warnings (e.g. voice without audio input, agent without vision), and prominent `CLOUD` tags for non-local models.

### WP4: Swap-aware calls and loading states (`feat(model): swap-aware client and loading states (WP4)`)

- Isolated swap detection behind `model::swap::detect_swap_signal`, recognizing headers (`x-rune-infer-loading`, `x-rune-infer-swapped`) and HTTP 503 retry responses.
- Implemented calm UI loading state: `AgentState::LoadingModel` animates the pill dot-arc in calm neutral blue/cyan (no red, no failure).
- Schema V7 migration (`trace.sqlite3`): `calls` gains `served_model TEXT` and `swap_ms INTEGER`; `steps` gains `served_model TEXT`.
- Client retries up to 15 times with backoff while model is loading, tracking `swap_ms` separately from inference duration.
- Generous configurable allowance for first call after idle (`first_call_idle_allowance_secs`, default 30s) prevents premature timeout stalls.
- Configurable option `skip_swap_for_optional_roles` (default `true`) skips non-resident decision and verifier calls.
- Served model tracked per call; warnings logged on mismatch and highlighted in Flight Player's API tab.

### WP5: Per-model evaluation baselines (`feat(eval): per-model baselines and cross-model comparison (WP5)`)

- Evaluation harness records `model` and resolved `profile` in `env.json` and every line of `results.jsonl`.
- `report.mjs` only executes the gate check (success rate not lower by >1 run, duration within +10%) when comparing runs of the **same model and profile**.
- Comparing different models outputs an informational **Model Comparison Table** (success rate, median duration, steps, tokens, and box quality if present) and never emits a PASS or FAIL (exit code 0).
- Documented `baselines/<model>/` directory storage convention in `docs/evals/README.md`; no baseline data committed.
- Tested with fixture result files for two models (`model-alpha.jsonl` and `model-beta.jsonl`).

### WP6: Calibration probe for new models (`feat(probe): model calibration probe for profile generation (WP6)`)

- Extended probe (`src-tauri/src/model/probe.rs`, `src-tauri/src/probe_m15.rs`, `scripts/m15/run-probe.ps1`) with `-Profile` mode.
- Pure scoring functions with comprehensive unit tests:
  - `score_pointing_conventions`: Tests candidate conventions against known synthetic targets and scores hit rates.
  - `score_thinking_styles`: Compares durations and reasoning presence across styles to determine which successfully disable thinking.
  - `score_tool_choice`: Evaluates tool-call argument validity across trials.
  - `score_max_image_edge`: Evaluates 1280 px vs 1920 px resolution success and latency.
- Generates a suggested `ModelProfile` JSON at `temp/m15/<model>-profile.json`.

### WP7: Voice provider seam (`feat(voice): transcriber seam and audio-in capability check (WP7)`)

- Introduced `Transcriber` trait (`src-tauri/src/voice/seam.rs`).
- Shipped existing implementation as `ChatModelAudio`.
- Prepared extensibility point `DedicatedTranscriber` (`Whisper`, `Parakeet`, `LocalEngine`) without hardcoding external engines.
- Cleanly disables voice with a clear human-readable sentence if the voice model profile has `audio_in == No`:
  - `toggle_listening` rejects immediately.
  - Recorded audio is dropped, mic closes, and UI emits `"Voice is disabled: model '<name>' does not support audio input."`
- Unit tests: fake transcriber, clean disablement check, unchanged 44-byte WAV RIFF headers.

### WP8: Documentation, full verification, and report (`docs: architecture updates and M1 milestone handoff report (WP8)`)

- Updated `docs/ARCHITECTURE.md` with section 4 on models, profiles, roles, and swaps.
- Created standalone reference `docs/models.md` detailing profiles, precedence, roles, coordinate adapters, probe calibration, and baseline storage.
- Re-verified all test commands against baselines.

---

## 3. Related Repository Features (Simulated with Fixtures)

The following features from other repositories were simulated and verified with unit test fixtures:

- **INF-7 (Structured capabilities)**: Tolerant parser accepts free-text capability strings as well as structured JSON capability payloads. Unknown fields are handled gracefully.
- **INF-15 (Locality)**: Server capabilities map `locality` (`local`, `cloud`, `unknown`); cloud models display `CLOUD` tags in Settings.
- **INF-16 (Served-model identity)**: Wire client extracts `response.model` and records it as `served_model` in SQLite trace and Flight Player.
- **INF-17 (Support matrix)**: Documented in `docs/models.md` and prepared for future backend consumption.

---

## 4. Live Steps for the Maintainer

When testing live with a running `rune-infer` instance:

### Step 1: Verify Roles and Settings UI

1. Launch Rune Studio: `bun run tauri:dev` (or build debug binary).
2. Open Settings (`Ctrl+,` or tray menu -> **Settings**).
3. Under **Model roles**, verify the 6 roles: `Agent`, `Verifier`, `Analysis`, `Voice`, `Decision`, `Advisor`.
4. Check that the dropdown for each role lists the models served by rune-infer (`GET /v1/models`).
5. Select a non-audio model for `Voice` (e.g. text/code model): observe the dot capability line and the warning: _"This model does not support audio input. Voice will not work."_
6. Attempt to toggle microphone (`Ctrl+Shift+Space` or mic icon): observe that listening is cleanly prevented with the notice message.

### Step 2: Observe Hot Swaps in Action

1. Set `Agent` to Model A (e.g. `gemma-4-12b-it`).
2. Set `Verifier` to Model B (if available) or trigger a model swap.
3. Start a task. When the verifier or another role is invoked:
   - Observe the calm dot-arc animation in the pill indicating **Loading model** (no red error state).
   - Once loaded, observe the task continuing smoothly.
4. Open Flight Player (`Open Flight Player` from tray menu) -> select the completed run -> inspect the **API** tab:
   - Verify `Served Model` matches requested model.
   - Verify `Swap Time` is recorded separately from inference duration.

### Step 3: Run the Calibration Probe on a New Model

1. Ensure rune-infer is running and serves a new candidate model.
2. Run the probe in profile mode:
   ```powershell
   pwsh scripts/m15/run-probe.ps1 -Profile -Model <new-model-id>
   ```
3. Look for the output log summarizing pointing hit rates, thinking style tests, and tool argument validity.
4. Verify that `temp/m15/<new-model-id>-profile.json` is generated with valid suggested profile values.

### Step 4: Run Evaluation Baselines

1. Run evaluation suite for the model:
   ```powershell
   pwsh scripts/eval/run.ps1 -Repeats 3 -FastChecks off -Out eval-out/test-run
   ```
2. Verify that `eval-out/test-run/env.json` and `results.jsonl` record the model and profile.
3. Compare against a baseline of the same model:
   ```powershell
   node scripts/eval/report.mjs eval-out/test-run/results.jsonl --compare docs/evals/baselines/<model>/baseline-<date>-fast-off.results.jsonl
   ```
   Observe the gate PASS/FAIL verdict.
4. Compare against a baseline of a different model:
   ```powershell
   node scripts/eval/report.mjs eval-out/test-run/results.jsonl --compare eval-out/other-model/results.jsonl
   ```
   Observe that it prints the **Model comparison table** with metrics (success rate, duration, steps, tokens) and **never** emits a PASS or FAIL verdict.