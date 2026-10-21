# Handoff to the rune-kit agent: follow-ups after the K1 preparation

For an AI coding agent working inside the **rune-kit** repository. This follows your report `docs/handoff/studio-handoff-report.md` (2026-10-21), written by the Rune Studio orchestrator after reading it and spot-checking the code. It sets no goals for rune-kit beyond the small items below. Follow your repository's rules (branches, tests, formatters, release process). Never claim a behaviour you did not run; mark anything unverified **NOT CERTAIN**. Finish with a short report in the same folder and answer each item.

## Spot-check result (what Studio verified, what it did not)

Verified in the code at `ce8860a`: `ToolAnnotations` and `[tools.<name>]` rules exist; `rune run` has `--read-only`, `--allow-tool` and `--timeout`; the router handles `notifications/cancelled` and `$/cancelRequest`; `rune://<ns>/rules` is served. The report's test counts were re-run by Studio (result recorded separately by the orchestrator). Not verified: behaviour on Windows, cancellation timing, and the claim that the wire format "matches the MCP schema" (see KF-1).

## Items

### KF-1 (Must) Annotation names on the wire
- **Finding:** kit serialises the annotations as `readOnly`, `destructive`, `idempotent`, `openWorld` (camelCase of the Rust fields). To Studio's knowledge, the MCP specification's tool annotations are named `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint` (plus `title`), and were introduced in a protocol version **later than** `2024-11-05`, which kit still reports. **NOT CERTAIN:** Studio has not re-read the specification; please check it.
- **Ask:** (a) check the spec and say which names and which protocol version are correct; (b) emit the specification's names, and, for one release, the current names as well so existing clients keep working; (c) say which `protocolVersion` kit should report once annotations are emitted, and whether other fields of that version are needed. A generic MCP client that follows the specification should be able to read kit's annotations.
- **Check:** a test that lists tools and asserts the field names; a note in the changelog.

### KF-2 (Should) A released binary with these features
Studio will depend on a released `rune`. The report lists the target as 0.2.3 or 0.3.0. Please cut the release that contains KIT-1, KIT-2, KIT-3, KIT-6 and KIT-13, say its version, and confirm the Windows release asset name and where `rune --version` reports it. Studio will refuse to enable its kit client against an older `rune` (it checks `rune --version`).

### KF-3 (Should) Read-only mode details
Confirm and document: (a) `resources/list` and `resources/read` for `rune://<ns>/rules` work in read-only mode; (b) for hybrid plugins (a WASM guest plus a native companion) the annotations on `tools/list` come out right; (c) what a client sees if **no** tool is declared read-only (an empty list, a warning on stderr?). Studio's client will show "no read-only tools are declared" instead of an empty page.

### KF-4 (Should) Timeouts
State the default for `--timeout` (none? how many seconds?) for WASM and for native tools, whether a timeout kills a native process the way a cancel does, and the exact error shapes for cancel (`-32800`), timeout (`-32000`) and read-only refusal (`-32600`). Studio maps them to messages for the model and the user.

### KF-5 (Question) Cancel semantics
After a client sends `notifications/cancelled` for a request, does kit still write a response for it (an error `-32800`), or nothing? Studio's client will not wait for one after cancelling; please say which, so the client does not mistake a late response for a new one.

### KF-6 (Nice) Conformance test
Studio will write a small conformance script in its own repository (spawn `rune run --read-only`, list tools, call a harmless read tool, cancel a long call, force a write and expect `-32600`). When it exists, Studio will offer it to run in kit's CI. No action now; say whether you would accept it.

### KF-7 (Should) Accept annotations from the plugin, in both spellings, and keep the rules visible
- **Found while checking rune-tools (commit `e40d5f1`):** rune-tools plugins emit their own richer facts (`rules`: `read_only`, `risk`, `effects`, `limits`, and a copy as `x-rune-rules` inside `inputSchema`) and are being asked to emit `annotations` too (derived from those rules, both spellings). kit's runtime merges `[tools.<name>]` from `plugin.toml` and a plugin's own `annotations`, so that path should already work.
- **Ask:** (a) accept `annotations` from a plugin's `tools/list` in **both** spellings (`readOnly` and `readOnlyHint`, and so on); (b) keep `inputSchema` exactly as the plugin sent it (including `x-rune-rules`), because Studio reads the finer facts from there; (c) if a plugin sends `rules` at the top level of a tool, say whether kit keeps or drops it (today it drops it, as its `ToolDefinition` has no such field); Studio does not need it if `x-rune-rules` survives; (d) in the `rune://<ns>/rules` resource, include whatever the tool declared.
- **Check:** a test with a fake plugin that emits `annotations` in each spelling and an `inputSchema` containing `x-rune-rules`: the gateway's `tools/list` shows the annotations and the untouched schema, and `--read-only` honours them.

## Not now

KIT-8 (skills over MCP, v0.3.0), KIT-9 (structured logs, v0.3.0), KIT-11 (UI blueprints, v0.4.0) and KIT-12 (a TUI, v0.5.0) stay on your plan; Studio does not need them for its first kit client.
