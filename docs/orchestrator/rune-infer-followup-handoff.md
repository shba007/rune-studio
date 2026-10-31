# Handoff to the rune-infer agent: follow-up after your handoff report

For an AI coding agent working inside the **rune-infer** repository. This follows your report `docs/handoff/studio-handoff-report.md` (2026-10-21), read by the Rune Studio orchestrator, who spot-checked the code at commit `b29d03f` (the headers, the refusal codes and `/readyz` exist where the report says) and did **not** run anything else yet. Follow your repository's rules (branches, tests, formatters, release process). Never claim a behaviour you did not run; mark anything unverified **NOT CERTAIN**. Finish with a short report in the same folder, answering each item.

## What was verified

In the code: `x-no-swap` and the `409 swap_refused` refusal, `x-model-served`, `x-model-locality`, `x-rune-priority` with the `429 server_busy` and `Retry-After: 1` refusal, and `/readyz`. Studio's test run of your suite is reported separately by the orchestrator.

## Items (what is still open or looks incomplete)

### IF-1 (Must) INF-3 is half done: a running background request is not pre-empted

- **Finding:** in `src/api/chat/mod.rs` a **new** background request is refused while a foreground one is active, and a foreground request calls `yield_notify.notify_waiters()`. But nothing in the codebase waits on `yield_notify` (the only uses are the `notify_waiters()` call and the field definition). So a background request that is **already running** when a foreground request arrives is not stopped or paused, and the foreground request waits for it, which is exactly the case Studio asked about (an idle reflection call delaying the user's next task).
- **Ask:** when a foreground request arrives, a running background request must **yield**: abort it (the slot-release you built for INF-1 is the mechanism) and return a clear status for it (for example `499` or `503` with `Retry-After`), or, if that is not possible for some engine, say so per engine. State what a client sees. If you disagree with aborting, propose another way; the requirement is that foreground latency is not held up by background work, beyond a small bound you state.
- **Check:** a test that starts a long background request, sends a foreground request, and asserts the foreground starts within the stated bound and the background ended with the documented status.

### IF-2 (Must) Model headers on every model route, not only chat

- **Finding:** `x-model-served`, `x-model-locality` and the residency headers appear in the chat code only (`src/api/chat/`). Studio also calls transcription, decisions, OCR and grounding, and its rule is that sensitive data must never go to a cloud model, checked on the response too.
- **Ask:** the same attribution headers (`x-model-served`, `x-model-locality`, `x-model-resident`, and the load and eviction headers when they apply) on **every route that runs a model**: `/v1/audio/transcriptions`, `/v1/decisions`, `/v1/nlu/intents`, OCR, grounding and the other image routes, embeddings. `x-no-swap` and `x-rune-priority` should also be honoured there, or you say which routes do not.
- **Check:** one test per route.

### IF-3 (Should) INF-5: the speech route answer is incomplete

You confirmed `/v1/audio/transcriptions` follows the OpenAI multipart form. Studio also asked: **which model id serves it** by default (and which others can), the maximum audio length, typical latency for a 5 second utterance on the maintainer's hardware class, the response shape beyond `text` (language, segments, confidence), whether the speech model is resident alongside the chat model or **swaps** it (INF-4), and whether `x-no-swap` applies. Answer from your code and tests, not from a model card.

### IF-4 (Should) INF-17: most of the support matrix is unanswered

Your answer gives the engine and file formats for MiniCPM-o 4.5 and Bonsai 2 27B. Still open, per model: (1) which **modalities the API actually exposes** (the report says MiniCPM-o is "text + vision + audio in/out": is **speech output** reachable through any route, or only audio input?); (2) how **thinking** is switched; (3) how reliable **forced tool calls** and JSON output are in your tests; (4) the image size and **coordinate convention** each uses for pointing; (5) VRAM and load time, and whether the two can be **resident together** in 15.9 GiB. Mark each as tested, from documentation, or unknown. Studio will measure quality itself.

### IF-5 (Should) Idle unload and the 783 MiB

Studio's TUI screenshot showed `Unloaded idle model ... (no requests for 300s)` and `783 MiB` VRAM in use with nothing loaded. Please say: is the 300 s idle timeout **configurable per model**, can a model be **pinned** so it is not unloaded, is the time left exposed in `/v1/models` or the TUI, and what the 783 MiB is. After 300 s idle the next Studio task pays a cold load (7 to 16 s measured).

### IF-6 (Should) INF-9 and INF-10: claims need evidence

- You state decision outputs are "calibrated". Studio's field test found answers on real screenshots unreliable (a chat app visible answered no; "loading" answered yes at 0.65 on a normal screen). Provide, in the repository, a **small fixture test** (an image and the expected answer with a probability range) Studio can reuse, and say how calibration was checked.
- Grounding is documented as `[ymin, xmin, ymax, xmax]` on 0..1000 (Studio uses `[x0, y0, x1, y1]`; it will convert). An earlier probe returned `[500, 500]` for every request. Say whether that was a bug, and add a test with an image whose target is known and off-centre.

### IF-7 (Should) The TUI needs a visual review

The report says the dot TUI is built (dots, monochrome with one accent, red only for errors, seconds). The spec asked for more (now-before-history, one summary line for idle models instead of a table of zeros, a braille sparkline, a `LOCAL` or `CLOUD` tag and capability dots in the model list, the word beside every status, `NO_COLOR` and ASCII fallbacks). Provide screenshots of: idle, busy, loading, the model list, logs, 80 columns, with colour and with `NO_COLOR`, and in the ASCII fallback, and say which parts of the spec you did not do. Studio reviews them.

### IF-8 (Process) Release

The report targets `v0.7.0`. Say when it is released and how Studio can read the version (`/health`). Studio will name a minimum version for its contract.

### IF-9 (Should) A contract test fails on a machine without a GPU

- **Finding (the orchestrator re-ran your suite on Linux, no GPU, Rust 1.98.1, at `b29d03f`):** 119 tests passed and **1 failed**: `test_inf7_inf15_models_structured_capabilities_and_locality` in `tests/studio_contract_tests.rs` asserts `json["vram_free_bytes"].is_number()` (line 150) and fails when the VRAM figures are not available. Your report states 121 passed and 0 failed; the difference (2 tests, plus the failure) may be Windows-only tests, so this is probably an environment difference, not a product bug. But it shows an undocumented case.
- **Ask:** (a) say what `/v1/models` returns for `vram_total_bytes`, `vram_free_bytes` and `vram_used_bytes` when no GPU or no probe is available (absent, null, or zero) and keep it consistent; Studio treats a missing value as unknown, never as zero; (b) make the test pass on a machine without a GPU (assert the documented behaviour for that case); (c) make sure CI does not need a GPU.

## Not now

Anthropic-style `/v1/messages` alignment and SDK work. Studio does not use them.