# Addendum for the Studio agents (M1, L0, K1): the real contracts from rune-infer, rune-kit and rune-tools

For AI coding agents working in the **rune-studio** repository. Give this file to the M1, L0 and K1 agents together with their handoffs. It replaces guesses in those handoffs with names reported by the three projects on 2026-10-21 and spot-checked in their code by the orchestrator (rune-infer `b29d03f`, rune-kit `ce8860a`, rune-tools `e40d5f1`). **Nothing here was run end to end.** Where a handoff says "NOT CERTAIN, isolate it behind one function", use the names below, still behind one function, and write tests with fixtures that follow these shapes. Where this addendum and the real servers disagree, the servers win: say so in your report. None of these projects has a confirmed release yet (rune-infer targets v0.7.0, rune-kit 0.2.3 or 0.3.0, rune-tools none stated).

## 1. rune-infer (reported targets: v0.7.0)

**`/v1/models`** (and the model cards) now carry, per model: `modalities_in` (for example `["text","image"]`), `modalities_out`, `tool_calls` (boolean), `thinking_control` (`"thinking"` or `"none"`), `max_image_edge_px` (default 4096), `context_tokens`, `roles`, `aliases`, `locality` (`"local"` or `"cloud"`), `resident` (boolean), `status` (`"available"`, `"loaded"` or `"remote"`), `vram_mb_estimate`, `load_ms_estimate`; and server-wide `vram_total_bytes`, `vram_free_bytes`, `vram_used_bytes`. Treat every field as optional (unknown, never "no"). Not reported, so keep treating as unknown: speech-output availability, forced tool-call reliability, pointing convention.

**Response headers** on chat (other routes: **not yet**, requested): `x-model-served` (the canonical model id that actually answered), `x-model-locality` (`local` or `cloud`), `x-model-resident` (`true` or `false`), `x-model-load-duration-ms` (when a cold load happened), `x-model-evicted` (the model that was evicted). The body's `model` is also the canonical id.

**Request headers**

- `x-no-swap: true`: if the model is not resident, the server answers `409 Conflict` with error type and code `swap_refused` and does not load it.
- `x-rune-priority: background` (also `x-priority: background` or `low`): while a foreground request is active, a **new** background request gets `429` with `Retry-After: 1` and error code `server_busy` (type `server_busy`, code `busy`). **A background request already running is NOT pre-empted today** (asked of rune-infer, item IF-1 in its follow-up): Studio must keep background calls short and one at a time until that is fixed.

**Roles and aliases:** a request's `model` is resolved by exact id, server role, server alias, model role, model alias, case-insensitive name, then a generic fallback. Studio keeps its own per-role map (M1) and may send a role name as the model; the answer says what actually ran.

**Usage:** `usage` is reported to be honest now, with `prompt_tokens_details.image_tokens`, `audio_tokens`, `cached_tokens`, and `stream_options.include_usage`. Studio still marks the numbers as "server-reported" until it has measured them (the field test found `completion_tokens` of 1 on every step).

**Other:** `/livez` and `/readyz` need no auth (`/readyz` returns `{"status":"ready","active_requests":N,"busy":bool}`); `/health` reports a real GPU backend (`cuda`, `metal`, `cpu`). `/v1/audio/transcriptions` follows the OpenAI multipart form (`file`, `model`, `language`, `prompt`, `temperature`, `response_format`); the model that serves it, its latency and whether it swaps the chat model are **unknown** (asked). Grounding boxes are `[ymin, xmin, ymax, xmax]` on 0..1000: **Studio's own convention is `[x0, y0, x1, y1]`**, so a conversion function with tests is required wherever grounding output is used. Constrained (grammar or JSON schema) tool-call output is reported for `llama-server` models.

**What to do in the milestones**

- **M1 WP1 (capability client):** replace the guessed field names in your fixtures and parser with the ones above; keep the tolerant parsing. **WP2:** `thinking_control` of `"none"` means do not send thinking switches; `"thinking"` means send the thinking switch. **WP3:** show `locality`, and `resident` in the role picker. **WP4:** the swap signals are the headers above and the refusal `409 swap_refused`; use `x-no-swap: true` for the optional roles (decision, verifier) when the user chose "do not swap for optional roles"; show the loading state when `x-model-resident: false`; read `x-model-served` as the served model for the record. Background calls (reflection, later life jobs) send `x-rune-priority: background`, handle `429 server_busy` by waiting `Retry-After` and trying later (not as an error), and stay short.
- **L0 WP4 (local-only):** the authoritative locality is, in order: the `x-model-locality` response header (check it **after** the call and drop the response on `cloud`), then the model's `/v1/models` `locality`, then the address fallback. A model with `locality: "cloud"` or unknown never receives a tagged request. Because the header is chat-only today, a tagged request to any other route (transcription, decisions, OCR) is **refused** unless the address fallback says local, until rune-infer adds the header there.

## 2. rune-kit (reported: `ce8860a`, 23 tests pass, re-run by the orchestrator)

As in the K1 handoff, with these confirmed in the code: `--read-only`, `--allow-tool`, `--timeout`, `--json`; `notifications/cancelled` and `$/cancelRequest` handled by a background reader; `tools/list` returns `annotations` (`readOnly`, `destructive`, `idempotent`, `openWorld`) only for tools whose plugin declared them (via `[tools.<name>]` in `plugin.toml` or the plugin's own `annotations`); `rune://<ns>/rules` resource. Accept both annotation spellings (`...Hint` too). Still unconfirmed: whether kit forwards `inputSchema` untouched in every case (rune-tools relies on it).

## 3. rune-tools (reported: `e40d5f1`, 228 tests pass, **not re-run by the orchestrator**)

Each tool declares `rules`: `read_only` (bool), `risk` (`low`, `medium`, `high`), `effects` (a list of `read`, `write`, `delete`, `network`, `exec`) and optional `limits` (`max_bytes`, `timeout_seconds`, `allowed_paths`, `allowed_hosts`, `allowed_binaries`). The plugin's `tools/list` output carries them as a top-level `rules` **and** as `inputSchema["x-rune-rules"]`. **kit drops the top-level `rules` but keeps `inputSchema` as sent**, so Studio reads **`inputSchema["x-rune-rules"]`**.

**Known gap (under repair):** rune-tools did not emit `annotations`, and kit builds annotations only from `plugin.toml` or the plugin's `annotations`. So today `rune run --read-only` lists **nothing** from rune-tools, and Studio would see no annotations. Both projects have follow-up handoffs. Until they land, test against fixtures only, and write the real-kit check as steps for the maintainer.

**What to do in K1:** the tool's facts come from, in order: kit's `annotations` (both spellings) and `inputSchema["x-rune-rules"]` (`read_only`, `risk`, `effects`, `limits`). Use both when both exist:

- `risk` from the rules sets the gate level directly (`low`, `medium`, `high`); `effects` containing `delete`, `exec` or `write` is at least medium; `network` makes it open-world; `delete` or `exec` is high.
- Where the two sources **disagree**, take the **higher** risk and log the mismatch.
- No facts from either source: unknown, ask every time.
- The name-based floor table stays and can only raise the level.
- Show `limits` (allowed paths, allowed hosts) in the confirm card when present, because they say what the tool may touch.
- Record the facts as received in `tool_calls.annotations_json` (store `x-rune-rules` too).

A2UI blueprints exist in rune-tools as **drafts** (`plugins/<plugin>/blueprints/<tool>.a2ui.jsonl`, invented style tokens) for `get_current_time` and `list_directory`. Do not read or render them in K1 or M1; they wait for Studio's catalog (K2).