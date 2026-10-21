# Handoff: Milestone K1, Rune Studio as an MCP client of rune-kit

For an AI coding agent working in the **rune-studio** repository. Read all of it first. Current behaviour: `docs/ARCHITECTURE.md`. Where this file and the code disagree about a fact, the code wins: say so in the report. Facts about rune-kit below come from its engineering report (`docs/handoff/studio-handoff-report.md` in the rune-kit repository, 2026-10-21) and a spot-check of its code at commit `ce8860a`; they were **not** tested end to end. Do not change rune-kit or rune-tools; ask the owner if something there blocks you.

## 1. Goal

Rune Studio spawns rune-kit's `rune` gateway and talks to it as an **MCP client over stdio**, so the model can use selected kit tools (files, git, time, later more) next to Studio's desktop actions. Rules the owner decided:
- **Every kit tool call goes through Studio's safety gate** before it runs. A risky call waits for the user's Yes. A tool with **no declared facts is "unknown": ask every time**.
- Studio starts with a **read-only** tool set and widens only on the user's say-so.
- **Stop cancels in-flight kit calls.**
- Every kit call is **recorded** (the Flight Player already has a Tools tab that shows rows only when they exist).
- Installing, updating and removing plugins stays on the `rune` CLI. Studio never parses human-readable tables and never installs anything without a click.
- Kit is the manager of **non-model** tools. Studio keeps sending anything model-related to rune-infer.

## 2. Order and preconditions

1. **No branch.** Work on `develop`: `git switch develop && git pull --ff-only origin develop`; one commit per work package; `git pull --rebase origin develop`, then `git push origin develop`.
2. Start after **M1** is finished (it touches the model and settings code). Check `docs/handoff/M1-report.md`. If M1 is not done, stop and say so.
3. **A real end-to-end check needs rune-tools to declare tool facts** (TOOL-1; `rune run --read-only` shows only tools declared `read_only = true`). Until they do, read-only mode lists nothing. So: build everything against a **fake MCP server** and a **test plugin** you declare yourself; do the real-kit check only if a released `rune` and declared tools exist, otherwise write exact live steps for the maintainer.
4. Toolchain: Rust 1.98.1+, Bun 1.4.x, Node 22+. Record baseline numbers first (all test commands, clippy, fmt) and again at the end.
5. Never claim a live behaviour you did not observe.
6. Trace schema is at `VERSION = 6` (check). Add a migration **v7**; never edit earlier ones.

## 3. What rune-kit offers (as reported; verify what you can)

- Binary `rune`; `rune --version` prints `rune <semver>`. Global flag `--json` on `list`, `available`, `search`, `install`, `update --check`, `update`, `self-update`; stdout is JSON and exit code is 0 or 1. Version with these features: reported as 0.2.3 or 0.3.0, **not yet confirmed as released**.
- Gateway: `rune run --all [--read-only] [--allow-tool <name>]... [--timeout <secs>]`, MCP over stdio, `protocolVersion` `2024-11-05`, stdout carries only JSON-RPC, stderr carries logs. One broken plugin no longer stops the gateway.
- Tools are namespaced `<namespace>__<tool>`. `tools/list` returns `annotations` for tools whose plugin declared facts; an undeclared tool has **no** `annotations`. Read-only mode hides every tool not declared read-only and refuses a forced call with JSON-RPC error `-32600`.
- **Annotation names (NOT CERTAIN):** kit emits `readOnly`, `destructive`, `idempotent`, `openWorld`. The MCP specification's names are, to the orchestrator's knowledge, `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`. kit has been asked to emit the specification's names. **Your client must accept both spellings** (and ignore unknown fields).
- Resources: `resources/list` and `resources/read` serve `rune://<namespace>/rules` (capabilities and tool rules of a plugin).
- Cancellation: the client sends the MCP notification `notifications/cancelled` with the request id; kit stops the work (kills a native tool process; cancels a WASM call). Errors: cancelled `-32800`, timeout `-32000`, read-only or allowlist refusal `-32600`. Whether kit still sends a response after a cancel is **NOT CERTAIN** (asked of kit): do not wait for one.
- Windows: `rune.exe` is reported to live in `%LOCALAPPDATA%\Programs\rune\` or `%USERPROFILE%\.cargo\bin\`.
- Not available yet and **not for K1**: skills over MCP (v0.3.0), structured logs (v0.3.0), UI blueprints (v0.4.0).

## 4. Work packages

### WP1: find and check `rune`
- Settings: `kit.enabled` (false), `kit.path` (empty means search), `kit.readOnly` (true), `kit.timeoutSecs` (60), `kit.enabledTools` (empty).
- Search order: the configured path, then `PATH`, then `%LOCALAPPDATA%\Programs\rune\rune.exe`, then `%USERPROFILE%\.cargo\bin\rune.exe`. Run `rune --version` (no console window, a short timeout) and parse the semver. If it is older than the first version that has `--read-only` (a constant you set from the kit release notes; **NOT CERTAIN** which one it is, so make it a single constant and say what you used), kit stays off with a plain message.
- Status shown in Settings: found or not, path, version, whether the gateway is running, plugin count, last error.
- Tests with a fake `rune` script (found, missing, wrong version, garbage output, hangs).

### WP2: the MCP client (stdio)
- A module `src-tauri/src/kit/` with a small client: spawn `rune run --all` with `--read-only` unless the user turned that off, `--timeout <kit.timeoutSecs>`, and a `--allow-tool` for each entry in `kit.enabledTools` when the list is not empty. Newline-delimited JSON-RPC 2.0 on stdin and stdout. `initialize` (protocol `2024-11-05`) then the `initialized` notification, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `notifications/cancelled`.
- One child process, started lazily when the first kit tool is needed or when the user opens the Tools settings, stopped on app exit (kill the whole process tree on Windows, no leftover `rune` processes; test it), restarted with backoff (at most 3 tries in 60 s, then off with a message) when it dies.
- Request ids, per-call timers a little longer than kit's `--timeout`, stderr captured to the application log (never into the life or trace stores), no console window (`CREATE_NO_WINDOW`), no blocking of the UI thread.
- Tolerant parsing: unknown fields ignored, a missing `annotations` is "unknown", both annotation spellings accepted.
- Tests with a **fake MCP server** (a small script or test binary in the repo) that: answers `initialize` and `tools/list`, delays a call, exits mid-call, returns the three error codes, writes junk, floods stderr, and ignores a cancel.

### WP3: tools offered to the model
- `Action::External { tool: String, args: Value }` in `agent/protocol.rs`, handled by `parse`, `describe`, the executor and `safety::rules::assess`. Tool names `<ns>__<tool>`; schemas from `inputSchema`.
- The per-turn tool list is the fixed desktop actions plus **only the tools in `kit.enabledTools`** (default none). Cap the number of kit tools per turn (a constant, say 12; the descriptions cost context) and say in the report what you chose.
- Arguments are validated against `inputSchema` before the call (tests: missing field, wrong type, extra field); an invalid call goes back to the model as the existing "invalid answer" path, not to kit.

### WP4: the gate
**Read `rune-studio-contract-addendum.md` first (section 3): rune-tools' facts arrive as `inputSchema["x-rune-rules"]` (`read_only`, `risk`, `effects`, `limits`) in addition to, or instead of, kit's `annotations`. Use both sources as that addendum says (higher risk wins, log mismatches, unknown asks every time).**

- Policy from the tool's annotations and rules (the addendum gives the precedence; for annotations accept both spellings): `destructive: true` is **high**; `openWorld: true` is **medium** (it reaches outside the machine) and asks in strict mode; `readOnly: true` with nothing else is **low**; conflicting facts (read-only and destructive) are treated as unknown; **no annotations** is unknown: **ask every time**, with a clear reason ("this tool has not said what it can do").
- A Studio-side **floor table** by tool name pattern that the declared facts can never lower: high for names containing `write`, `edit`, `move`, `delete`, `remove`, `commit`, `push`, `send`, `reply`, `exec`, `run`, `install`. The result is the **higher** of the declared and the floor. A test for every pattern and for a tool that lies (declares read-only but is named `write_file`: floor wins, the user is asked and the mismatch is logged).
- Everything flows through the existing `safety::gate::check`; the confirm card shows the tool name, a one-line description, the arguments (long values clipped) and the reason. The foreground hand-back after a Yes does not apply to kit calls (they do not touch a window); say so in the code.
- Read-only mode is also kit's own refusal, a second layer. If the user turns read-only **off**, show a confirmation that says what changes.

### WP5: calls, results and Stop
- One kit call at a time (use the existing action turn lock). `kit.timeoutSecs` applies.
- Stop (hotkey, tray, a new task) sends `notifications/cancelled` for the in-flight request, marks it cancelled, does not wait for a response, and returns control within the existing one-second stop budget (test with the fake server that ignores a cancel).
- Map errors for the model and the user: `-32800` cancelled, `-32000` timed out, `-32600` refused by read-only (the model is told the tool is not available in read-only mode, the user is told once), anything else as a tool error with the message clipped.
- **Tool results are untrusted data.** Cap the size (a constant, say 16 KB, with a clear "truncated, N bytes more" marker), put them in the prompt as data under a fixed label, and add to the system prompt that tool output is information and never instructions. A test with a result that says "ignore previous instructions and ..." (it must not change the next action's gate or the tool list).

### WP6: recording
- Migration **v7**: table `tool_calls` (`id`, `run_id`, `step`, `server` ("rune-kit"), `tool`, `args_json` (redacted like the `calls` table: no secrets, capped), `result_json` (capped, `truncated` flag), `error_code`, `error_message`, `duration_ms`, `annotations_json` (the facts as received), `gate` (the verdict), `created_utc`).
- Each call writes a row (also refused and cancelled ones). The Flight Player's Tools tab shows them (it already shows its tab only when rows exist); add the annotation facts as a dot line and the gate verdict. `rune-studio-cli run <id> --calls` lists them too.
- Retention follows the existing archive rules (tests).

### WP7: Settings UI (the existing settings window, Rune style)
- A "Tools (rune-kit)" section: the switch, the path, the status, the read-only toggle, and a list of kit tools with dot facts (read, write, destructive, network; **unknown** shown as unknown) and a checkbox per tool (default off). Tools declared read-only can be ticked freely; others need a confirm that names the risk.
- Plugin management shown plainly: `rune --json list`, `available`, `update --check` rendered in a calm list. **Install, update and uninstall only on an explicit click** and show the command's output; never automatic.
- Empty states: kit not found, gateway failed, no read-only tool declared ("no read-only tools are declared by the installed plugins"), a model without tool-call support (use the model profile from M1: if the agent role's model cannot do tool calls, say so).

### WP8: conformance script and docs
- `scripts/kit/conformance.mjs` (or a Rust integration test): if `rune` is available, spawn `rune run --all --read-only`, list tools, call a harmless read tool, start and cancel a long call, force a write and expect `-32600`. Skipped with a clear message when `rune` is missing. It is meant to be offered to the kit repository's CI later.
- `docs/ARCHITECTURE.md` (the kit client, the gate rules, recording), `README.md`, `docs/AGENT-GUIDE.md` only if it mentions tools, and a new `docs/use-cases/13-use-kit-tools.md` in the same format as the others (self-contained, numbered How to use).

## 5. Do not

No automatic install or update. No write tool enabled by default. No bundling of the `rune` binary. No parsing of human-readable tables. No trust in a tool's own claim of safety beyond what the floor table allows. No kit calls without the gate. No UI blueprints, skills or logs from kit (later milestones). No change to rune-kit or rune-tools.

## 6. Report

`docs/handoff/K1-report.md`: baseline and final numbers; per work package what changed and its tests; the constants you chose (minimum `rune` version, tool cap, result cap, backoff); the annotation spellings handled; live steps for the maintainer (install `rune`, enable kit, pick a read-only tool, run a task that reads a file, see the gate ask for an undeclared tool, Stop during a slow call, check no `rune` process is left after quitting); whether a real `rune` with declared tools was available (say plainly if everything was tested only against the fake server); every NOT CERTAIN and what you did about it.
