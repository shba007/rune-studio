# Models in Rune Studio

Rune Studio supports interchangeable models with hot swapping on local servers (like **rune-infer**) or remote endpoints. Model quirks and capabilities are decoupled from the agent loop into structured **profiles**, models are assigned per functional **role**, and model swaps are treated as normal, calm events.

---

## 1. Model Profiles

A `ModelProfile` defines how Studio communicates with a specific model and what capabilities it assumes.

### Schema

```json
{
  "id": "gemma-4-12b-it",
  "thinking": {
    "styles": ["thinking_type", "chat_template_kwargs"]
  },
  "audioIn": {
    "supported": "yes",
    "format": "wav"
  },
  "images": {
    "maxEdgePx": 1280,
    "screenshotAtAppSize": true
  },
  "pointing": "norm1000",
  "toolChoiceRequired": "yes",
  "maxTokens": {
    "agent": 512,
    "agentThinking": 3000,
    "verifier": 512,
    "analysis": 1500,
    "voice": 256,
    "decision": 512,
    "advisor": 1024
  },
  "locality": "local"
}
```

### Profile Attributes

- **`thinking`**: Control styles for disabling reasoning tokens (`"none"` or `"styles": ["thinking_type", "chat_template_kwargs", "budget_tokens"]`). When thinking is disabled, Studio sends all configured styles for that model.
- **`audioIn`**: Audio input capability (`"yes"`, `"no"`, or `"unknown"`) and expected format (e.g. `"wav"`).
- **`images`**: Maximum useful screenshot edge in pixels (`maxEdgePx`, default 1280) and whether full-size app screenshots can be ingested directly.
- **`pointing`**: Coordinate convention produced by the model:
  - `"norm1000"`: 0 to 1000 across the screenshot (Studio native).
  - `"pixels"`: Physical pixel coordinates of the downscaled screenshot.
  - `"norm1"`: 0.0 to 1.0 normalized coordinates.
  - `"unknown"`: Unknown coordinate convention; triggers fallback to element IDs via UI Automation and zoomed views.
- **`toolChoiceRequired`**: Whether forced tool calls (`tool_choice: "required"`) are supported (`"yes"`, `"no"`, `"unknown"`).
- **`maxTokens`**: Default completion token limits tailored per role.
- **`locality`**: Whether the model runs `"local"`, in the `"cloud"`, or is `"unknown"`.

---

## 2. Profile Resolution Precedence

When Studio needs a profile for a model ID, it resolves properties in strict priority order:

1. **User Settings Override**: Explicit overrides configured by the user in `settings.json`.
2. **Bundled Measured Defaults**: Profiles bundled in `src-tauri/src/model/profiles.json`. Only models whose performance has been directly measured by the project are bundled (currently `gemma-4-12b-it`).
3. **Server Capabilities**: Modalities and properties discovered from `/v1/models` (supports both legacy free-text lists and structured INF-7 metadata).
4. **Conservative Default**: Default safe configuration for any unknown model:
   - Thinking switch sends all known switch styles.
   - Pointing convention is `"unknown"`.
   - Tool calling is unforced.
   - Max image edge defaults to 1280 px.

---

## 3. Roles and Model Assignment

Studio separates model configuration into functional roles (`settings.models`):

| Role           | Purpose                                               | Required Modalities        |
| :------------- | :---------------------------------------------------- | :------------------------- |
| **`agent`**    | Core action loop (planning, clicking, typing)         | Vision (`image_in`), Tools |
| **`verifier`** | Independent task completion verification              | Vision (`image_in`)        |
| **`analysis`** | Screen inspection and UI element discovery            | Vision (`image_in`)        |
| **`voice`**    | Hands-free speech transcription and intent extraction | Audio (`audio_in`)         |
| **`decision`** | Fast speculative check endpoint (`/v1/decisions`)     | Text / Fast pass           |
| **`advisor`**  | Background guidance and life track (reserved)         | Text                       |

- **Fallback**: If an optional role (such as `verifier` or `voice`) is left blank, Studio automatically falls back to using the `agent` model.
- **Capability Warnings**: The Settings UI inspects server capabilities and shows clear inline warnings if a role's selected model lacks required modalities (e.g., assigning a text-only model to `voice` or a non-vision model to `agent`). Models that are not local display a prominent `CLOUD` tag.

---

## 4. Coordinate Adapters

Studio's internal engine operates in normalized `0..1000` coordinates:
$$\text{Point} = (x, y) \in [0, 1000] \times [0, 1000]$$

The pure adapter module (`src-tauri/src/model/coords.rs`):

- `adapt_point_to_norm1000(convention, x, y, width, height)`
- `adapt_box_to_norm1000(convention, [x0, y0, x1, y1], width, height)`

converts from the model's reported convention (`norm1000`, `pixels`, `norm1`) into Studio's `0..1000` system, correctly accounting for non-square aspect ratios and OS display scaling (100%, 125%, 150%).

---

## 5. Model Swaps and Loading Awareness

When running multiple models on a single-GPU server, switching roles triggers a hot swap. Studio treats this as a normal operational state:

- **Swap Detection**: `model::swap::detect_swap_signal` detects model loading headers (`x-rune-infer-loading`, `x-rune-infer-swapped`) and HTTP 503 retry responses.
- **Calm UI State**: During a model swap, the status pill enters `AgentState::LoadingModel`, displaying a calm dot-arc animation without raising errors.
- **Trace Separation**: Call latency in `trace.sqlite3` records `swap_ms` separately from inference `decide_ms`.
- **First Call Allowance**: Initial calls after idle apply `first_call_idle_allowance_secs` (default 30 s) to prevent premature client timeouts while a model is loading into VRAM.
- **Optional Role Skipping**: When `skip_swap_for_optional_roles` is enabled (default `true`), optional roles (`decision`, `verifier`) skip execution if their model is not resident, avoiding unnecessary 7–16 second swap stalls.
- **Served Model Tracking**: Every API response records the actual `served_model`. Mismatches between requested and served models produce a warning and are highlighted in Flight Player's API tab.

---

## 6. Voice Seam and Audio Validation

Voice input is abstracted through the `Transcriber` trait (`src-tauri/src/voice/seam.rs`):

- `ChatModelAudio`: Default implementation streaming base64 WAV audio through `input_audio`.
- `DedicatedTranscriber`: Extensibility point for local speech-to-text engines (e.g., Whisper, Parakeet).
- **Clean Voice Disabling**: If the model assigned to the `voice` role lacks audio input (`audio_in == no`), voice listening is cleanly prevented:
  - `toggle_listening` rejects with: `"Voice is disabled: model '<name>' does not support audio input."`
  - Recorded utterances are not sent to the model, and the microphone automatically closes with an audible/visual notice.

---

## 7. Calibrating a New Model

To calibrate a new, unknown model without modifying source code:

1. Ensure the model server (rune-infer) is running and serves the target model.
2. Run the calibration probe in profile mode:

   ```powershell
   pwsh scripts/m15/run-probe.ps1 -Profile -Model <model-id>
   ```

3. The probe will:
   - Test candidate pointing conventions against known synthetic targets and calculate hit rates.
   - Test request styles (`thinking_type`, `chat_template_kwargs`, `budget_tokens`) to determine which switch off thinking.
   - Measure tool call argument schema validity over repeated trials.
   - Probe safe maximum image resolutions (1280 px vs 1920 px).
   - Generate a suggested `ModelProfile` JSON at `temp/m15/<model-id>-profile.json`.
4. Review the generated profile and add it to your `settings.json` or bundled profile definitions.

---

## 8. Evaluation Baselines

Evaluation runs track the exact `model` and resolved `profile` in `env.json` and every line of `results.jsonl`.

- **Storage Convention**: Baselines are stored per model under `docs/evals/baselines/<model>/`.
- **Same-Model Gating**: When comparing runs with the same model and profile, `scripts/eval/report.mjs --compare` evaluates the regression gate (success rate not lower by >1 run, median duration within +10%).
- **Cross-Model Comparisons**: When comparing runs of different models or profiles, the harness produces an informational **Model Comparison Table** comparing success rates, durations, step counts, token usage, and box qualities, with no PASS/FAIL judgment.