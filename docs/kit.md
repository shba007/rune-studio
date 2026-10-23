# Selected Kit tools

Cycle 2 adds a small read-only Kit integration to the existing desktop agent.
Open Settings → Tools. Tools are off and unselected by default.

## Setup

Install Kit and plugins yourself; Studio never downloads, installs or updates them.
The tested baseline is **Kit 0.3.1**, **rune-time 0.3.0**, and **rune-git 0.3.0**.
With a separately installed Kit, the manual plugin commands are:

```text
rune install rune-time --version 0.3.0
rune install rune-git --version 0.3.0 --native
```

Use the corresponding `rune-kit` command if your Kit release renamed the executable.
Studio tries an explicit executable path first, otherwise PATH `rune-kit` then
legacy `rune`. It accepts both CLI version titles, including a renamed executable
that still reports `rune 0.3.1`. Versions before 0.3.1 are unavailable.

Press Refresh / discover, then select the tools you want. A failed discovery is
shown as an error; an empty successful discovery is reported separately. Old
plugins without safe annotations/rules, unknown tools, duplicate identities and
unsupported schemas are not advertised. A missing Kit/plugin leaves desktop
control available. A model reported not to support tool calling gets an explicit
unavailable state in Tools settings.

Supported operations:

- `get_current_time`: current UTC or an IANA timezone.
- `convert_time`: conversion between IANA timezones.
- `validate_commit_message`: validation only, never creating a commit.

The approved exact gateway identities are the `time__` / `rune-time__` and
`git__` / `rune-git__` forms for those operations. Studio prefixes model-facing
names with `kit__` and maps them back exactly. Tool schemas come from discovery.
No write-mode or arbitrary-tool switch is provided.

## Execution and review

The existing agent receives selected Kit tools alongside desktop tools. Every
external action uses Studio's existing gate, including Ask before every action.
Tool output is untrusted data and cannot change selection or settings. Arguments
are validated against the discovered schema before dispatch. The supported
schema subset is object/string/boolean/number/integer, required properties,
additionalProperties boolean, enums, numeric minimum/maximum and string length
bounds. Arrays, pattern, format, references and other constructs make a tool
unavailable rather than being silently ignored.

Each call starts a Studio-owned read-only gateway with the selected allowlist,
rechecks discovery facts and selection, performs one call and closes the gateway.
Calls are serialized. There is no automatic replay after disconnection. Startup,
initialization, discovery and call waits are bounded. Stop sends cancellation,
ends the wait and kills the owned gateway if needed; late replies are discarded.
Changing Tools configuration and application exit stop owned gateways. We do not
claim that force-killing a gateway terminates every possible descendant process.

Session review reuses the existing step trace and Tools tab. It records name,
bounded/redacted arguments and result, duration, gate decision and outcome,
including refusal, failure and cancellation. Normal overlay step completion is
emitted after tool execution. Replies above the output budget are explicitly
marked truncated; protocol messages above 64 KB fail with a bounded error.
No trace migration was needed; schema stays version 8.

## Filesystem release blocker

File tools are deliberately unavailable in this release. In a direct test with
released Kit 0.3.1 and `rune-filesystem` 0.4.0, supplying
`-p allowed_dir=<disposable folder>` still left `read_text_file` declared
`read_only: false`, `readOnly: false`, and `readOnlyHint: false`. Consequently
`--read-only` exposes an empty tool list. No file read is claimed shipped.

The smallest required follow-up is a released filesystem/gateway combination
that exposes bounded read/list tools as read-only, with its host/guest path and
outside-root refusal verified. Studio can then add the user-owned root and those
exact identities; string path checks alone will not be treated as a sandbox.
Ordinary file contents would be sent to the selected model and would not inherit
unfinished Life protections.

## Verification and remaining owner trial

Deterministic coverage includes partial framing, discovery/call success,
discovery errors versus empty success, malformed/oversized messages,
timeout/disconnection, ignored cancellation and late replies, unselected tools,
conflicting facts, argument validation, error results, output truncation and
RPC cancellation/refusal classification. Rust: 481 passed, 10 ignored. Frontend:
159 passed. Application typecheck, production build, lint, Clippy and formatting
passed. Restricted loopback/temp-file failures were rerun outside the sandbox.

Real portable-release checks passed with Kit 0.3.1 and time/git 0.3.0: current
Kolkata time, UTC 12:00 → Kolkata 17:30 on 2026-10-23, and Conventional Commit
message validation. Artifacts were run directly through Kit, without changing
installed plugins. Installed older plugins exposed an empty read-only list.
No live Infer was available at the configured local endpoint, so no model-driven
or interactive Windows acceptance is claimed.

Five-minute owner trial:

1. Install compatible Kit/plugins separately and discover them in Settings → Tools.
2. Enable and select the three tools; use a tool-capable agent model.
3. Ask for current time, a UTC/Kolkata conversion, and commit-message validation.
4. Stop a call, then start a new task; inspect the session's Tools and Safety tabs.
5. Report Studio/Kit/plugin/Infer versions, model, session ID and any failing step.