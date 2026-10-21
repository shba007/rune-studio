# Testing and acceptance

Run these commands from the repository root. Rust tests use an in-memory
credential store, not OS credentials. Loopback fake servers and temporary-file
operations require an environment that permits local networking and file access.

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --lib --features agent-bridge
cargo test --manifest-path src-tauri/Cargo.toml --lib --features eval,agent-bridge
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --features eval,agent-bridge -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
bun run test:unit -- --run
bun run test:eval
bun run test:m15
vp lint
vp fmt --check src/
bun x --no-install vue-tsc --noEmit -p tsconfig.app.json
bun run build
```

Use the package scripts for Node suites: passing a directory to `node --test`
does not expand its test files on the supported Node runtime. The frontend
production build does not run the separate Vue template type checker.
TypeScript 6 is retained for compatibility with the current vue-tsc/Volar checker.

## Model contracts

Fake-server tests cover request headers, model attribution, swap/loading states,
retry attempts and sensitive-response refusal. They do not establish real model
quality, speech latency, grounding accuracy or co-residency. The maintained
[model guide](models.md) describes profiles and the calibration probe.

The probe implementation is `src-tauri/src/grounding_probe.rs`. Documented
`m15_probe`, `m15_calibrate_profile`, `test:m15`, script paths and output formats
remain unchanged. See [architecture](ARCHITECTURE.md) for grounding conventions.

## Life data

Keep consent, encryption, local-only policy, canary, recovery, export/deletion and
migration tests. Missing-key recovery requires explicit confirmation and must not
access the OS keyring in unit tests. Passing these tests is not evidence that new
life jobs or live integrations exist. The life module's exported interfaces and
persisted data formats are unchanged.

## Session review

Component fixtures cover Flight Player rendering and seeking. They do not prove
Windows Media Foundation encoding, WebView2 H.264 playback or the asset protocol.
Use the [session review guide](use-cases/12-review-a-session.md) for live checks.
Historical field observations and unobserved fixes remain distinguished in the
[roadmap](ROADMAP.md); do not infer a live pass from a unit-test pass.

## Live checks and historical evidence

Ignored tests require a Windows desktop or a real model server. The system runner
moves the real mouse and types; run it only in a suitable acceptance session:

```text
pwsh scripts/m1-system-test.ps1
pwsh scripts/m15/run-probe.ps1
```

Keep the target-window script and probe/eval harnesses. The [evaluation guide](../scripts/eval/README.md)
and [baseline convention](evals/README.md) describe separate real-desktop runs.
No generated run is an approved baseline merely because it exists locally.

Historical closeout reports are retained by the local Git archive tag
`archive/milestone-docs-2026-10-21-cleanup`. For example:

```text
git show archive/milestone-docs-2026-10-21-cleanup:docs/handoff/C1-report.md
```

The other archived reports are L0-report.md, M1-report.md and R6b-report.md under
the same historical directory. Architect working documents in `docs/orchestrator/`
remain untouched and can refer to those historical paths.