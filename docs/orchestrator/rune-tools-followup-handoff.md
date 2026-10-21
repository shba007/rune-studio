# Handoff to the rune-tools agent: follow-up after your handoff report

For an AI coding agent working inside the **rune-tools** repository. This follows your report `docs/handoff/rune-tools-report.md` (2026-10-21), read by the Rune Studio orchestrator, who also spot-checked the code at commit `e40d5f1` and compared it with rune-kit's code (rune-kit `develop`, commit `ce8860a`). Follow your repository's rules (branches, tests, formatters, release process). Never claim a behaviour you did not run; mark anything unverified **NOT CERTAIN**. Finish with a short report in the same folder, answering each item.

## What was verified, and the problem found

Verified in your code: `ToolRules` (`read_only`, `risk`, `effects`, `limits`) in `rune-pdk`; the serializer emits a top-level `rules` and an `x-rune-rules` copy inside `inputSchema`; the contract macro tests it. Good work, and more detailed than asked.

**Problem (blocks Studio's first kit integration):** rune-kit does **not** read `rules` or `x-rune-rules`. kit builds each tool's `annotations` (`readOnly`, `destructive`, `idempotent`, `openWorld`) from two places only: the plugin's own `tools/list` output field `annotations`, and `[tools.<name>]` entries in `plugin.toml`. Your plugins emit neither (the filesystem plugin's `plugin.toml` has no `[tools.*]` entries, and `ToolDefinition` in `rune-pdk` has no `annotations`). kit's `rune run --read-only` then treats **every tool of every plugin as undeclared and hides it**, and Studio's client treats every tool as "unknown, ask every time". The classification you built never reaches the gate or the read-only filter. (This was found by reading code, not by running kit with your plugins: check it and say so if it is wrong.)

The cause is partly the orchestrator's: the handoff you worked from did not yet describe kit's `[tools.<name>]` form, because kit had not finished. That is now known.

## Items

### TF-1 (Must) Emit `annotations` derived from your rules
- In `rune-pdk`, make `ToolDefinition` also serialise an `annotations` object **derived from `ToolRules`**, so your rules stay the single source of truth and nothing is declared twice by hand: `read_only` from `rules.read_only`; `destructive` true when `effects` contains `delete` (decide and state whether `exec` also counts; if unsure, say yes: safer); `open_world` true when `effects` contains `network`; `idempotent` only if you can state it truthfully for the tool, otherwise leave it out.
- **Spelling:** kit currently reads `readOnly`, `destructive`, `idempotent`, `openWorld`. The MCP specification's names are, to the orchestrator's knowledge, `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint` (not re-checked), and kit has been asked to accept both on input and emit the specification's names. Until kit confirms, emit **both spellings** in the same `annotations` object, behind one small function so it is easy to drop one later.
- Keep `rules` and `x-rune-rules` as they are (Studio will read them for the finer facts: risk level, effects, limits).
- A tool whose rules are missing emits **no** `annotations` (never a default of "read-only").
- **Check:** `rune run --all` (from rune-kit) lists the annotations for every tool; `rune run --all --read-only` lists exactly the tools with `read_only = true` and nothing that can write, delete, execute or reach the network. Add a contract test in `rune-pdk` (`test_plugin_contract!`): annotations present, consistent with rules (a tool with `delete` is never `read_only`).

### TF-2 (Must) Verify the read-only set is truly read-only
Produce, in your report, the list of tools that end up in read-only mode (name, effects, risk). Studio's first set is filesystem reads, git reads and time. Confirm for each that nothing in the list can write, delete, execute a process or reach the network, **including** options passed as arguments (for example a git tool that can take a path to write to; a filesystem tool that can follow a symlink out of its root; a fetch tool, which reaches the network, must not be in the list). Anything you are unsure of is not read-only.

### TF-3 (Should) A2UI blueprints are drafts for now
You wrote two pilot blueprints and chose a path (`plugins/<plugin>/blueprints/<tool>.a2ui.jsonl`) and style tokens (`style: "dot-monochrome"`, `font`, `variant`). Studio has **not published its component catalog** and rune-kit proposed a different carrier (one blueprint per plugin at `rune://<namespace>/ui/blueprint.jsonl`, v0.4.0). So: keep them, mark them clearly as **draft, not consumed by anyone yet** (a README note beside them), write no more blueprints, and expect to adapt them once Studio publishes the catalog (milestone K2). Do not invent styling tokens; the look comes from Studio.

### TF-4 (Should) Cancellation and process handling
Your response table lists "TOOL-3 subprocess stop handling" with a target in `rune-sidecar` v0.2.0. (In the handoff, TOOL-3 was the closed OCR question; this was your own addition, and a good one: Stop must stop.) Say what is built today and what is only planned, and whether a native tool killed by rune-kit leaves child processes behind (for example `ffmpeg` started by the video plugin). On Windows especially.

### TF-5 (Nice) Memory plugin
The proposed provenance fields (timestamps, source, confidence, soft deletion) are noted. Studio will keep life data in its own store, so this is **not** needed for Studio now. Say only whether `rune-memory` persists anything across restarts (your report says it is in-memory), because users may assume it does.

### TF-6 (Process) Version and release
Say which version of each changed plugin and of `rune-pdk` contains TF-1 once done, and when it is published to the registry, so Studio can name a minimum version. Your report names no release for TOOL-1.

## Not now
Pagination envelope changes, memory schema changes and more plugins. Studio does not depend on them.
