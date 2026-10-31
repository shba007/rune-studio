# Handoff to the rune-infer project manager

## Agent brief (read first)

You are an AI coding agent working **inside the rune-infer repository**. The owner gave you this document. It was written by the Rune Studio orchestrator **from outside your repository**: what it says about rune-infer comes from Studio's observation (a field test and earlier reading) and **may be wrong or out of date**. Your first job is to check every statement about rune-infer against your code and say where it is wrong.

How to work:

1. Read this whole document and your repository's own contributing rules, branch rules and test commands. **Follow your repository's rules**, not any in this document. Do not push to a protected or release branch, and do not cut a release.
2. For every request below, find the current state in your code (exists, partly, missing) and write it down before you change anything.
3. **Build** the items in the build list (INF-1, INF-2, INF-3, INF-4 (parts 1 to 3), INF-7, INF-8, INF-15 and INF-16; INF-14 (the TUI) as a last, separate work package). For the others, report feasibility, and implement one only if it is small and clearly safe; otherwise leave it for the owner. Items marked Question or Process are answered in your report, not built.
4. Where a request offers options, choose the simplest that works, state the choice and why, and keep it compatible with existing clients (add, do not break; announce anything that changes behaviour).
5. Add tests for what you build. Run your repository's tests, lints and formatters and report the numbers before and after. **Never claim a behaviour you did not run.**
6. Mark anything you could not verify **NOT CERTAIN**. Do not invent facts about models, protocols or other projects.
7. Do not change the goals or scope of rune-infer beyond this document. If a request conflicts with your code or design, say so and propose the smallest alternative.
8. Finish with a report file in your repository's usual place for such notes (for example `docs/handoff/<name>-report.md`) containing: the corrections to this document, the state of each request, what you built with test evidence, your answers to the Questions, and the filled response table at the end of this document.

## 0. About this document

Written by the Rune Studio orchestrator for the project manager of this project. It does **not** set your goals, scope or roadmap, and it does not describe your code: those are yours. It states what **Rune Studio needs from this project as a consumer**, why (with evidence from Studio's field test of 2026-10-19), how Studio will check it, and what Studio does in the meantime. Anything about your project that is written here comes from Studio-side observation and may be out of date or wrong: correct it. Items marked **NOT CERTAIN** are unverified.

You answer each request with one of: **Accept** (with an owner and a target release), **Counter** (what you will do instead), **Decline** (with the reason), or **Question** (what you need to know). Use the table at the end.

## 1. The ecosystem in one page

| Project                   | Role                                                                                                                                                                                                                                                | Who uses it                                                                                          |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **rune-infer**            | The manager of all **models and engines**, local and cloud. Everything that needs a model is done here: chat, audio and speech, vision, OCR, grounding, decisions, embeddings. If the model fits in VRAM it stays loaded, otherwise models swap     | Rune Studio, other clients. Has a library and a TUI (to be redesigned, see `rune-infer-tui-spec.md`) |
| **rune-kit** (`rune` CLI) | The manager of all **non-model tools**, the way rune-infer is the manager of models: installs and runs plugins and skills, runs them as one MCP gateway over stdio, and owns the **generic rules** that apply to any tool                           | Rune Studio (planned MCP client, milestone K1), other agents. Has a library; no TUI yet              |
| **rune-tools**            | The individual tools (plugins): filesystem, fetch, memory, git, browser, email, audio, video, image, slides, figma, time, sequential-thinking, and others. Each tool owns its **tool-specific rules** and its own **UI blueprint** (see A2UI below) | Through rune-kit                                                                                     |
| **Rune Studio**           | The harness: a Windows desktop agent that sees the screen, acts, records everything, checks its own work and (planned) reflects on it, and renders tool UI                                                                                          | End users                                                                                            |

**Ownership rule (decided by the owner):**

- Anything that needs a **model** (OCR, vision, speech, grounding, decisions) is rune-infer's. rune-kit and rune-tools do not host models.
- Anything that is **not a model** (files, web, git, mail, browser, time, memory, UI blueprints) is rune-kit's to manage and rune-tools' to implement.
- **Rules:** a rule that is specific to one tool lives with that tool in rune-tools. A rule that applies to tools in general (defaults, the vocabulary tools declare in, what happens when a tool declares nothing, sandbox and permission enforcement) lives in rune-kit. Rune Studio applies its own safety gate on top of both and never relies on a tool being safe because it says so.

**A2UI (Agent-to-User Interface), decided by the owner.** Tools describe their own user interface as data. Each tool holds its own **JSONL blueprint** for UI widgets; Rune Studio renders those widgets with **its own design system**. The UI code is **not written at runtime** (generating Vue code with a model would add inference time and risk). A2UI is an open protocol (Apache 2.0, created by Google; as found on 2026-10-20 its site lists v0.8 as the stable release and v0.9 as current, a JSON/JSONL declarative format, and a catalog of trusted client-side components; renderers exist for Lit, Flutter, React and others; a Vue renderer was not found, **NOT CERTAIN**). Studio would write its own renderer for its own component catalog. See section 4 and the per-project requests.

**Interfaces (decided by the owner).** Both rune-infer and rune-kit have a **library**. rune-infer has a **TUI** today; **rune-kit has no TUI yet**. The rune-infer TUI is to become cleaner (see `rune-infer-tui-spec.md`) and a future rune-kit TUI is to be built in the same style. Both follow the design principle of Rune Studio, a Nothing-phone style dot interface: dots as the visual language, monochrome with one accent, red only for attention and failure, compact and calm. The shared description, with a suggested terminal translation, is in `rune-design-language.md`. It does not set what the interfaces do, only how they look and feel.

Goal of the whole ecosystem (set by the owner, updated 2026-10-21): a **life harness**, built on a universal, self-reflecting AI harness. Rune does not assume: it works from **evidence** (what you say is stored as what you said, and checked against what tools can observe), it expects that a user can be mistaken and that it can be too, it can **change the trajectory mid-session** when the evidence says the current plan will not work, and it **collaborates** instead of taking over: it improves the user and itself, and automates what is automatable only after asking. Studio's life track (L0 to L10) is in `rune-life-harness-milestones.md` (sent separately).

**Interchangeable models (decided by the owner, 2026-10-21).** Models will be swapped freely, for example **MiniCPM-o 4.5**, **Bonsai 2 27B** and others, and rune-infer supports hot swap. So **nothing in Studio, kit or tools may assume one model.** Studio will keep per-model profiles and per-role model choices (milestone M1) and will ask rune-infer what a model can do instead of guessing. Facts found on 2026-10-21 (secondary and vendor sources, **not verified by running anything**): _MiniCPM-o 4.5_ is an open 9B omni model from OpenBMB (vision, speech in and out, full-duplex live streaming; built from SigLip2, Whisper-medium, CosyVoice2 and Qwen3-8B; Apache 2.0; a GGUF for llama.cpp exists). _Bonsai 2 27B_ is a ternary-weight 27B model from PrismML (reported as based on Qwen3.8-27B, a file of about 5.95 GB, a 262,144-token context, Apache 2.0, reported to need PrismML's fork or its MLX loader rather than stock llama.cpp or Ollama). Sources: huggingface.co/openbmb/MiniCPM-o-4_5, huggingface.co/OpenBMB/MiniCPM-o-4_5-gguf, modemguides.com/blogs/modemguides-blog/run-bonsai-2-27b-locally, cloudprice.net/models/prism-ml-bonsai-2-27b.

Studio's own plan, in order (dates are not committed): R5 agent identity, demo mode, idle reflection shell (done). R6a screen analysis as an overlay (done). R6b session video and Flight Player (done). **M1 model profiles and roles** (next). R6c post-mortems and notes. R7 notes in the prompt, proved by an A/B. R8 trust (redaction, retention). R9 universal abstractions. **K1** the rune-kit MCP client, with every kit tool call going through Studio's safety gate (a risky call waits for the user's Yes; an unknown tool defaults to ask). **K2** A2UI surfaces. **L0 to L10** the life track, starting with L0 foundations (consent, store, trust slice).

## 2. How the projects fit together (the contract)

1. Studio talks to rune-infer over HTTP (OpenAI-compatible, bearer token). Studio talks to rune-kit by spawning `rune run --all` and speaking MCP over stdio (planned). Studio never talks to rune-tools directly.
2. **Safety is Studio's job at the point of action, but it needs facts from below:** whether a tool reads, writes, deletes, sends or reaches the network. Tool-specific facts come from the tool (rune-tools) through the generic form kit defines (rune-kit). Without them, Studio treats the tool as unknown and asks the user every time.
3. **Stop must work everywhere.** When the user presses Stop, Studio cancels in-flight work. Each layer is asked to actually stop (not just stop reporting).
4. **Background work must yield.** Studio will run reflection while idle. It must never delay a real task.
5. **Models are rune-infer's only.** Studio calls rune-infer for anything model-based, including OCR and vision. It does not look for model features in kit or tools.
6. **UI is data.** Tools carry JSONL blueprints; Studio renders them from a fixed catalog of components in its design system. Actions from a widget that trigger a tool call are ordinary tool calls and go through the safety gate.
7. **Compatibility:** each project announces breaking changes in release notes. Studio keeps a conformance script (`scripts/wire-audit.mjs`, checks what Studio sends against a live rune-infer) and offers to run it on request; equivalent checks for kit are proposed below.
8. **Shared test:** Studio's field test and evaluation suite (15 fixed desktop tasks, success and time recorded) are how Studio measures whether a change in any project helped or hurt.
9. **Models are interchangeable.** Studio asks rune-infer what the loaded or available models can do and keeps its own per-model profile; a model change never needs a Studio release. Sensitive data (finance, health, mind) is only ever sent to a **local** model; rune-infer must say which models are local and which are cloud.

## 3. Phases and where this project is needed

| Phase   | Studio work                                      | Needed from the other projects                                                                                         |
| ------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| A (now) | M1 model profiles and roles; L0 life foundations | rune-infer: structured capabilities and locality, cancel, honest `usage`, residency visibility, a model support matrix |
| B       | R6c post-mortems, R7 notes                       | rune-infer: a way for background calls to yield without causing a model swap                                           |
| C       | K1: kit client                                   | rune-kit and rune-tools: JSON output, risk facts, cancel and timeouts, a curated starter set                           |
| D       | K2: A2UI surfaces                                | rune-kit: carry blueprints. rune-tools: a blueprint for a pilot tool. Studio: renderer and catalog                     |
| E       | R8, R9 universal                                 | Stable contracts, conformance tests, a second surface (browser through kit)                                            |

Studio's need-by points are stated per request below. Studio does not impose a date; tell Studio what is realistic and Studio will adjust order.

## 4. What Studio uses from rune-infer today

- Endpoints: `/v1/chat/completions` (the agent loop, the verifier, screen analysis, voice, planning), `/v1/models`, `/health`, `/v1/decisions` (optional fast checks, off by default), and `/v1/images/grounding` (a probe only).
- Chat requests carry: a screenshot as an image part, a forced tool call (`tool_choice: required`), `max_tokens`, `thinking: {"type": "disabled" | "enabled"}` and also `chat_template_kwargs.enable_thinking`; voice sends the whole WAV file as `input_audio {data, format: "wav"}`.
- Studio retries a failed call up to 3 times in all (waits of 0.5 s and 1.5 s) for 429, 502, 503, 504 and transport errors.
- Studio sends a bearer token when configured.
- Default model in use: `gemma-4-12b-it`. Server version in the field test: 0.6.0, one model loaded at a time.

## 5. Facts Studio established (field test, 2026-10-19, Windows 11, rune-infer 0.6.0)

- `input_audio` with a whole WAV is accepted. `thinking: {"type":"disabled"}`, `reasoning_effort: none` and `enable_thinking: false` are all honoured. Thinking is on by default (reasoning present with no parameter) and is about 4 times slower (decide median 4.5 s off, 18 s on).
- `/v1/decisions` uses the image (same question: 0.986 yes with the image, 0.42 no without) and returns `{decision, probability}` / `{decision, confidence, probabilities}`.

## 6. Requests

Priority: **Must** = Studio cannot ship the related feature well without it. **Should** = noticeable quality gain. **Nice** = convenience.

### INF-1 (Must) A cancelled request stops generating

- **Evidence:** after Studio cancelled a screen analysis (it closes the HTTP connection; verified by a test with a throwaway server), the next task'"'"'s planning call took 75.7 s, as if the server kept generating the old answer.
- **Ask:** when the client disconnects, abort generation (and, if possible, prefill) and free the slot. Say what is and is not abortable (**NOT CERTAIN** whether an in-progress prefill in the underlying engine can be interrupted).
- **Check:** Studio starts a long request, disconnects after 1 s, then sends a short one; the short one must answer in normal time.
- **Meanwhile:** nothing; Stop looks instant in Studio but the server stays busy.

### INF-2 (Should) Honest `usage`

- **Evidence:** with thinking off, `completion_tokens` was 1 for 62 of 62 steps and `prompt_tokens` was 1; with an audio call prompt and completion tokens were equal. Image tokens are not reported.
- **Ask:** correct counts, image tokens included (or reported separately), and `stream_options.include_usage` if streaming is supported.
- **Meanwhile:** Studio stores the server number, marks it unreliable, and estimates from characters.

### INF-3 (Must, before Studio ships idle reflection) Background work must yield

- **Why:** Studio will run reflection calls while the user is idle. rune-infer has no request priority (as read from its code earlier, **NOT CERTAIN** now) and may run one request at a time, so a background call already running delays the user'"'"'s next task.
- **Ask:** any one of: a priority hint on the request (for example a header) so foreground work goes first and can pre-empt; a clear busy response (`429`/`503` with `Retry-After`) when the server is occupied by other work; or queue depth in a ready/health endpoint that Studio can poll. Studio will pick whichever you choose.
- **Check:** start a background-marked long request, then a foreground request: foreground waits at most the agreed time.
- **Meanwhile:** reflection stays off by default and uses short calls one at a time.

### INF-4 (Should) Residency: "fits in VRAM stays loaded, otherwise swaps" made visible and controllable

- **Decided policy (owner):** if a model fits in VRAM it stays loaded; if not, models swap. Studio accepts that. The owner is **not sure how** to make this workable for clients, so these are Studio's suggestions, from the least to the most work. The rune-infer PM chooses; none is mandatory.
- **Evidence for why it matters:** with one model loaded, each decision call loaded the decision model (4.7 s) and evicted the chat model (7 to 16 s to reload). Studio's client gave up after 4 s while the swap still happened, so fast checks only made tasks slower. The decision model Studio defaulted to (`d1-3b-q8-0`) was not the id the server exposed (`liquid-d1-3b`).
- **New fact (2026-10-21):** the candidate models are small enough that two may fit together in 15.9 GiB (Bonsai 2 27B reported at about 5.95 GB, MiniCPM-o 4.5 a 9B model). If so, the best answer to most swaps is to keep them both resident (suggestion 5). **NOT CERTAIN** until you check real sizes.
- **Suggestions**
  1. **Tell clients the truth.** On `/v1/models` (or a sibling route): per model `resident` (yes/no), an estimate of its VRAM size, and the last observed load time; for the server: total and free VRAM. Studio then knows whether a second-model call will swap, and what it costs, before it calls.
  2. **Say when a swap happened.** A response header (for example which model was loaded or evicted and how long the load took) and a queue-wait figure, so Studio does not mistake a swap for a slow answer and can show the user "loading a model".
  3. **Let a caller refuse a swap.** An opt-in request header meaning "do not swap for this request"; the server answers with a clear status (for example `409`/`503` plus a reason) if it would have to. Studio would use it for optional fast checks and for background reflection so they never evict the chat model.
  4. **Pin and warm.** A way to pin a model so it is not evicted (or a priority order for eviction instead of plain least-recently-used), and a preload call so Studio can warm the models it needs while the user is idle.
  5. **Pick what stays by size.** If both a chat model and a small decision model fit together, they stay loaded and none of this is needed; Studio can show the user the combined footprint of its chosen models against free VRAM.
- **Seen in the TUI log (2026-10-20):** `Unloaded idle model 'gemma-4-12b-it' (no requests for 300s)`. Studio would like the idle timeout to be configurable per model, visible to clients (time left), and covered by pinning (suggestion 4). After 300 s idle the next Studio task pays a cold load.
- **What Studio will do on its side (no action for you):** treat a model swap as a normal, slow event: wait for it instead of abandoning the call; avoid calls to a non-resident second model unless the user allowed it; use the chat model for verifier and decisions where the quality is enough, so fewer models are needed; keep fast checks off by default.
- **Check:** Studio calls with the "do not swap" header while another model is resident and must get the refusal in under a second.

### INF-5 (Must for voice quality) A speech-to-text route

- **Evidence:** of 5 recorded utterances, a speech model (named "parakeet" in the test report; presumably served by rune-infer, **NOT CERTAIN**) transcribed all 5 correctly; the chat model hearing the audio returned "Open PowerPoint", "Open terminal" and nonsense, and a spoken "Open Notepad and write hello" started a Calculator task. With an empty system prompt the chat model looped "warn warn warn" for 22 s.
- **Ask:** which endpoint and model id Studio should call for transcription; request format (WAV bytes? header or raw?), maximum length, expected latency, response shape (text, language, confidence?), and whether using it swaps the resident model (see INF-4).
- **New (2026-10-21):** the owner may use MiniCPM-o 4.5, an omni model reported to hear speech natively (Whisper-medium based audio encoder). Whether it hears better than the chat model Studio used in the field test, and whether it is reachable for audio input through rune-infer, is part of INF-17 and **NOT CERTAIN**; Studio will measure it with recorded utterances before choosing.
- **Meanwhile:** Studio keeps sending audio to the chat model with a "transcribe only" instruction, a token cap and repeat rejection, and shows the words for 2 s before a spoken task starts. Studio will switch to the route once you confirm it.

### INF-6 (Should) Valid tool-call arguments, and constrained output

- **Evidence:** malformed arguments such as `{"name":"calc\",reason:","\"risk\"":"low"}` (valid JSON with wrong keys) and an analysis finding with text leaking into a field. Studio repairs only trailing junk and otherwise asks the model again.
- **Ask:** whether grammar-constrained or schema-strict tool calls are available for the served models (`response_format` strict schema is already mentioned for json_schema), and a way to request them for forced tool calls. If not possible, tell Studio which models are reliable.
- **Also (optional, for A2UI):** if Studio ever asks the model for a small ad-hoc UI description (A2UI JSONL, for example a result card for a screen analysis), schema-constrained output would keep it valid. This is optional and off until measured; the normal path is static blueprints (no model call).
- **Check:** Studio replays its recorded malformed cases.

### INF-7 (Must, upgraded) Structured capability discovery per model

- **Why now:** models are interchangeable (MiniCPM-o 4.5, Bonsai 2 27B, gemma-4-12b-it and others). Studio must not guess what a model can do. Today `/v1/models` lists capabilities as free text; Studio logs "the configured model does not list Audio" and cannot warn the user reliably.
- **Ask:** per model, structured fields: `modalities_in` (text, image, audio, video), `modalities_out` (text, audio/speech), `tool_calls` (yes/no, and whether a forced tool call works), `thinking_control` (which request field switches it: `thinking`, `reasoning_effort`, `chat_template_kwargs`, or none), `max_image_edge_px` or max pixels, `context_tokens`, `max_output_tokens`, `engine`, `quantization`, `locality` (INF-15), `vram_mb_estimate` and `load_ms_estimate` (INF-4), `resident` (INF-4), and `status` (available, loading, failed with reason).
- **Check:** Studio lists the models with their fields; a model that cannot take images is not offered for the screen; a model without audio is not offered for voice.

### INF-8 (Nice) Health that tells the truth

- **Evidence:** `/readyz` returned 404 in the field test; `/health` reports a fixed backend string (as read earlier).
- **Ask:** a real ready endpoint, the real backend and build version.

### INF-9 (Question, then Should) Decision answers on real screens

- **Evidence:** "is a chat app visible" answered **no** on a screenshot of one; "is it loading" answered **yes** at 0.65 on a normal screen. The meaning of `confidence` versus `probabilities` is not documented (confidence 0.38 for a pick with probability 0.587).
- **Ask:** document `confidence`; say whether the decision model is meant for full-screen screenshots; a published example image and expected answer Studio can use as a test.

### INF-10 (Should) OCR, vision and boxes: confirmed as rune-infer's

- **Decided (owner):** all model-based OCR and vision is done by rune-infer; rune-kit and rune-tools will not host such a model. Studio will call rune-infer for reading text on the screen and for locating things.
- **Why:** Studio now always draws boxes for findings (the chat model got 1 of 8 right), and its verifier would benefit from real text. An earlier probe of `/v1/images/grounding` returned `[500,500]` every time.
- **Ask:** (a) document the coordinate order and units of `box_2d` and detection boxes (Studio's convention is `[x0, y0, x1, y1]` in 0..1000; grounding was read as `[ymin, xmin, ymax, xmax]`); (b) document `/v1/ocr` input and output (text, boxes, confidence) and which model serves it; (c) say which route returns boxes for described regions reliably, or whether chat with a JSON schema is the intended path; (d) these are models, so INF-4 applies to them too (do they swap in and out?).

### INF-11 (Question) Contract confirmations

Please confirm in writing, so Studio can rely on them: default thinking when no parameter is sent; whether `thinking: {"type":"enabled"}` needs a budget; that a whole WAV including the header is the intended `input_audio.data`; that `enable_thinking` is ignored (Studio sends it only for other servers).

### INF-12 (Nice) Correlation and timing for the Flight Player

Studio'"'"'s new Flight Player shows every model attempt with its timings. Useful and cheap if available: an echoed request id header, and server-side timings (queue wait, prefill, decode) in headers or the response.

### INF-13 (Process) Compatibility

Release notes that call out changes to request or response shapes, default behaviour and model ids. Studio offers its `scripts/wire-audit.mjs` as a conformance check to run before a release.

### INF-15 (Must) Say which models are local and which are cloud

- **Why:** Studio's life track handles finance, health and mental-health data. Studio must **refuse to send** those to any model that leaves the machine. rune-infer has cloud adapters, so Studio cannot tell from the model name.
- **Ask:** a field `locality` per model with values `local` or `cloud` (and, if useful, `lan`), present on `/v1/models` and echoed on responses (a header with the locality of the model that actually answered, so a fallback or alias cannot hide a cloud model). The value must be derived from the configuration, not from the model's name.
- **Check:** Studio sends a test request tagged as sensitive to a cloud-configured model and must be able to refuse before sending; and after a response, the echoed locality matches.

### INF-16 (Should) Hot swap with interchangeable models: names, roles and identity

- **Context:** the owner will use models interchangeably and relies on rune-infer's hot swap. Studio will choose a model **per role** (agent, verifier, analysis, voice, decision, advisor) in its own settings (milestone M1), so a model change is a settings change.
- **Ask:** (a) every response states the **model that actually served it** (the `model` field and a header), including after an alias or fallback; (b) optional **server-side aliases or roles** (for example `agent`, `advisor`, `voice`) that the owner maps to model ids in rune-infer's configuration; Studio works without this (it keeps its own map) but would use it if present; (c) when a requested model is not available, a clear error naming the reason (not loaded, failed to load, not found, would swap), not a silent substitution; (d) a way to ask whether a model is **ready now** or would need a swap (INF-4).
- **Check:** Studio requests model A, B and an unknown id and the answers are right, clearly attributed and clearly refused.

### INF-17 (Question, answer in your report) Support matrix for the candidate models

The owner may use **MiniCPM-o 4.5** and **Bonsai 2 27B** among others. For each model rune-infer supports or could support, please answer, from your code and tests (not from the model cards):

1. Which **engine** serves it in rune-infer (stock llama-server, a fork, another engine), and does it work **today**?
2. Which **modalities are exposed through the API**: image in, audio in (and in what format), speech out, video, streaming. MiniCPM-o 4.5 is reported to do speech out and full-duplex streaming as model features; whether any of that is reachable through a llama-server GGUF route is **NOT CERTAIN**.
3. **Bonsai 2 27B** is reported (secondary sources, vendor figures) to need PrismML's fork or its MLX loader and not to run in stock llama.cpp or Ollama; is there a supported way in rune-infer, and what does it need on Windows?
4. How is **thinking** switched for each; how reliable are **forced tool calls** and **JSON output**; what image sizes and which **coordinate convention** (if any) does each use for pointing and boxes.
5. VRAM and load time for each (and whether MiniCPM-o 4.5 and Bonsai 2 27B can be resident **together** in 15.9 GiB; reported sizes: Bonsai 2 27B about 5.95 GB; MiniCPM-o 4.5 is a 9B model with GGUF quantisations whose sizes were not checked).
   Studio will measure quality itself per model (its evaluation suite and an advisor test suite); it only needs the facts above to know what to try.

### INF-14 (Should) A cleaner TUI, and a library that serves it

- **Decided (owner):** the current TUI should become cleaner yet informative, in the design principle of Rune Studio (dot interface, Nothing-phone style). The full spec, written from a screenshot of v0.6.0, is `rune-infer-tui-spec.md`; the principle and a terminal translation are in `rune-design-language.md`.
- **Summary of the spec:** show what is happening now before history; hide idle and zero rows behind one line; at most one frame; dots for state and quantity (VRAM as a dot row, a braille sparkline instead of two bar charts); grey scale plus one accent with red only for failure; seconds instead of milliseconds; never cut a name silently; the word as well as the colour; normal events are not warnings.
- **Ask:** (a) the TUI follows the spec, at your pace; Studio reviews screenshots on request; (b) the TUI is a thin view over the library, and the library exposes the data listed in section 8 of the spec; (c) tell Studio whether the library is meant to be used by outside programs and how stable its API is (Studio uses the HTTP API today).
- **Data issues the screenshot revealed** (spec section 7): token counts of 1 on every request (INF-2), an expected idle unload logged as a warning, a 300 s idle unload that Studio would like to be configurable and visible (INF-4), and 783 MiB of VRAM in use with no model loaded.
- **Not asked:** what the TUI does or which screens it has beyond what the spec shows.

## Response table (fill in and return)

| ID  | Accept / Counter / Decline / Question | Owner | Target release | Notes or answer |
| --- | ------------------------------------- | ----- | -------------- | --------------- |
|     |                                       |       |                |                 |

Also tell Studio: (a) the one contact for Studio-facing questions, (b) how you announce breaking changes, (c) anything in section 1 or 2 that is wrong about your project.