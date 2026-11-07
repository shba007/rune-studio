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
[model guide](models.md) describes model profiles.

The test-only probe implementation is `src-tauri/src/grounding_probe.rs`; its
former scene generator and runner have been retired. See [architecture](ARCHITECTURE.md)
for historical grounding results.

## Life data

Keep consent, encryption, local-only policy, canary, recovery, export/deletion and
migration tests. Missing-key recovery requires explicit confirmation and must not
access the OS keyring in unit tests. Passing these tests is not evidence that new
life jobs or live integrations exist. The life module's exported interfaces and
persisted data formats are unchanged.

## Session review

Component fixtures cover Preview rendering and seeking. They do not prove
Windows Media Foundation encoding, WebView2 H.264 playback or the asset protocol.
Use the [session review guide](use-cases/12-review-a-session.md) for live checks.
Historical field observations and unobserved fixes remain distinguished in the
[roadmap](ROADMAP.md); do not infer a live pass from a unit-test pass.

## Live checks and historical evidence

Ignored tests require a prepared Windows desktop or a real model server. Their former
target-window and probe runners have been retired. The [evaluation guide](../scripts/eval/README.md)
and [baseline convention](evals/README.md) describe separate real-desktop runs.
No generated run is an approved baseline merely because it exists locally.

Historical closeout reports remain in Git history under the local archive tag
`archive/milestone-docs-2026-10-21-cleanup`.