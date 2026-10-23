# Selected read-only Kit tools

Open Settings → Tools. Tools remain off and unselected by default.

## Installation and discovery

Use **rune-kit 0.5.0+** supporting MCP **2026-07-28**. Studio checks an explicit
executable first, then the actual user's `~/rune/rune-kit.exe` on Windows
(`~/rune/rune-kit` elsewhere), then PATH `rune-kit`. It never auto-selects `rune`.
The shared-home lookup works even when a desktop launch inherits an old PATH.
Missing/incompatible binaries show actionable guidance. Install Kit/plugins
separately; Studio never downloads, updates, migrates or installs them.

Shared ecosystem layout is `~/rune` for Kit/Infer executables, `~/rune/tools` for
Kit storage, `~/rune/tools/plugins` for payloads, and `~/rune/infer/config` for
Infer configuration. Kit/Infer installers own their storage migration, preserve
originals until validation, refuse destination conflicts, avoid unrelated models
and projects, preserve each other's files and add the shared root to user PATH
at most once. Studio changes neither PATH nor those directories. Existing
explicit Kit selection stays configured; an old selection must be updated by
the owner. No legacy executable is renamed or deleted. Unresolvable home produces
an error unless an explicit executable was supplied; it never falls back to CWD.

Studio only uses Infer's configured HTTP URL. It has no Infer executable/config
lookup or process management to migrate. Studio's app config, credentials and
trace schema (version 8) stay unchanged. Existing kit.json gains an optional
root field defaulting to empty; old enable/selection/executable fields survive.

## Tools and folder boundary

Refresh discovery and select the tools you want:

- time 0.3.0: get_current_time and convert_time.
- git 0.3.0: validate_commit_message.
- filesystem 0.4.1+: read_text_file and list_directory.

File tools additionally require an existing absolute folder entered by the owner
in Tools settings. Refresh after changing it. Studio passes `-p allowed_dir=...`
as a process argument, never from model arguments. The Kit WASI directory
capability is the confinement boundary; Studio string checks are not a sandbox.
File contents are sent to the selected model and do not inherit Life protections.
Choose only a trusted folder and ancestors. Folder/config changes stop owned
gateways. Missing root leaves file tools unavailable. Missing/conflicting rules,
unsafe tools, unsupported schemas and duplicate identities never grant access.
Discovery failure and empty success remain distinct; filtered approved tools
include a reason in the status. Known tool-incapable models have a clear UI state.

Only exact time/rune-time, git/rune-git and filesystem/rune-filesystem gateway
identities for these five operations are approved. Actual discovered schemas
become kit__-prefixed model tools. Every call rechecks selection and metadata and
passes through the existing gate, including Ask before every action. There is no
write, network or arbitrary command option.

## Public MCP and internal ABI

Studio → Kit is public MCP 2026-07-28 stdio: one UTF-8 JSON-RPC message per line,
server/discover, tools/list and tools/call. Every request supplies protocolVersion,
empty clientCapabilities and clientInfo in the io.modelcontextprotocol/* metadata
keys under params._meta. No initialize/initialized handshake is used. Only complete
results are supported; no subscriptions, elicitation, tasks or resource browser
is advertised. Discovery is refreshed per gateway (no persisted cache).

Kit's normative schema and protocol tests are the local integration reference:
../code-kit/crates/core/tests/protocol_tests.rs and docs/MCP-CONTRACT.md. The
advertised fixtures/mcp-2026-07-28.json was absent during verification; Studio's
fake server asserts modern metadata and rejects the old handshake. The native
plugin JSON-RPC and WASM exports are private Kit/Tools ABI, unchanged here.
Reference: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio

The validated schema subset covers object/string/boolean/number/integer, required,
additionalProperties boolean, enum, numeric bounds and string length bounds.
x-rune-rules and x-rune-read-limits are metadata; Kit/plugin enforce file output
limits. Unsupported constraints make tools unavailable. Results are bounded and
redacted, with explicit truncation. Plugin error envelopes and MCP isError are
failures, never successful empty results. Session review reuses existing traces.

Requests retain unique IDs, finite deadlines, bounded stderr/framing, cancellation,
late-reply discard and no uncertain call replay. Each serialized call starts an
owned read-only allowlisted gateway. Stop/config changes/exit terminate owned
gateways when needed. Descendant-process termination is not guaranteed.

## Local acceptance and owner trial

Actual Windows Studio client checks used local Kit target/debug/rune-kit.exe
0.5.0, time 0.3.0 WASM, git 0.3.0 Windows native and filesystem 0.4.1 WASM from
code-tools build outputs, through an isolated RUNE_KIT_DATA_DIR registry.
Five operations, in-root read/list, outside-root/missing-file errors, deselection
and recovery passed. No installed registry was modified. The ignored test
actual_kit_plugins_through_studio is reproducible with STUDIO_KIT_TEST_EXE and
an isolated RUNE_KIT_DATA_DIR containing those three plugins. These are local
artifacts, not a claim that Kit 0.5.0 is publicly released. Linux/macOS runtime
and live model/desktop acceptance are untested.

Owner trial: install a compatible Kit/plugins, refresh Tools, choose a disposable
folder, enable/select tools, ask a tool-capable model to use all five, stop a call
and inspect Tools/Safety session review. Try an outside-root path and verify a
visible refusal. Report exact Studio/Kit/plugin/Infer versions, model and session.

## Owner-reviewed automated release

Agents leave changes uncommitted and unstaged. After owner pushes develop and
strict frontend/Rust checks pass, CI calculates versions and generates notes
from reviewed conventional history. It updates one release/update-version PR;
the owner reviews/merges it, then promotes develop to main with a merge commit
whose title contains ci(release):. Merging into develop alone does not publish.
Publication extracts the exact prepared changelog version section. No AI service
or second history-based prose generator is used. Historical notes stay intact.

Use `cargo run --locked --manifest-path xtask/Cargo.toml -- prepare` for a dry run
and `cargo run --locked --manifest-path xtask/Cargo.toml -- notes` for exact notes.
See [release rules and token/event requirements](RELEASING.md). The legacy Node
release helper is superseded by Rust. No release workflow was dispatched here.
Verification results on Windows: 483 Rust tests passed, 11 ignored; the separate
real-Kit acceptance test passed; 160 frontend tests passed across 22 files.
Typecheck, production build, lint, Clippy (-D warnings), cargo fmt, frontend
formatting, diff whitespace and workflow YAML syntax checks passed. Eight Rust release
utility tests passed, covering preparation, idempotence and exact extraction. Clippy reported
only host incremental-cache hard-link fallback warnings. CI publication itself
was not run, and Linux/macOS execution and live model/GUI trials remain untested.

Exact local artifact SHA-256 values:

- Kit 0.5.0: a194a7768e72f836a797230b8ddac2b43499cd1c5890aa6909e33f29d8a1498e
- filesystem 0.4.1: 13da65a89d73f40155b9baddd59b49c2195cd70249e6305290713c3994a51e24
- time 0.3.0: 1f4ad30d00f0e250611fb642b965dea2b9a843bb4d6f729e03256cc161b7f3a8
- git 0.3.0: 2f00f4ac969aa371e16ee6251e0b1973e943a5f323821b5f7c0ff3c61ea9b442

Changed files: AGENTS.md, README.md, docs/kit.md, CHANGELOG.md, package.json;
src-tauri/Cargo.toml and Cargo.lock; src-tauri/src/kit/client.rs, mod.rs and
schema.rs; src-tauri/tests/fixtures/kit_gateway.rs; src/components/SettingsTools.vue
and SettingsTools.test.ts; the Rust release utility under xtask and docs/RELEASING.md;
.github/workflows/integrate.yml, release.yml and deploy.yml.