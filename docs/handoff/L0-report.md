# L0 policy and test-keyring follow-up — 2026-10-21

This follows the C1 closeout commit currently recorded as `944985b`. The original L0 work-package handoff was not available in the current tree, repository history, Downloads, or available chat list. This report covers the existing L0 foundation and the requested follow-up; it does not claim completion of unspecified packages. No L1 jobs, medical features, or new external integrations were started.

## Test credentials and encryption recovery

Added `credentials.rs`: production re-exports the OS keyring entry; unit-test builds compile an in-memory implementation instead. Life encryption, settings API keys and bridge tokens all use this entry point. Test entries share storage within their thread; separate test threads have independent storage. Existing Entry handles can still share their Arc-backed storage safely. No unit-test path can create or delete OS credentials through this adapter.

Tests cover missing/create/load/delete/recreate key behavior, authenticated encryption with the recreated key, entry sharing and isolation, and actual key-loss recovery. Recovery now checks that rejected confirmation preserves the original key and database, then simulates key deletion and verifies START FRESH clears consent and invalidates the old ciphertext. Credential deletion errors propagate before database deletion. Malformed non-ASCII hex key material returns InvalidFormat instead of potentially panicking on UTF-8 slicing.

## Reviewed and completed local-only integration

- Untagged and general chat requests retain existing behavior. Sensitive classes require a known local model, or a loopback address when model locality is absent. An explicitly unknown/cloud/LAN profile denies sending. Model names never decide locality.
- A missing model profile now permits the documented address fallback instead of supplying a synthetic unknown profile that disabled the fallback entirely.
- `x-model-locality` is authoritative over legacy aliases. Cloud/LAN responses to sensitive requests are discarded before parsing, streaming callbacks, error-body handling or trace response storage. This also applies to non-success statuses.
- Sensitive requests use a transport with proxy forwarding and redirects disabled, preventing a loopback endpoint from redirecting tagged data elsewhere. Normal requests keep the existing transport. This transport uses default TLS configuration; custom TLS settings on a caller-supplied HTTP client do not carry over to sensitive requests.
- Address fallback validates parsed HTTP/HTTPS hosts, exact localhost, IPv4 loopback, IPv6 loopback and IPv4-mapped loopback. It rejects misleading hosts/userinfo, LAN/public addresses and localhost subdomains rather than assuming a subdomain resolves locally.
- Added a route-aware policy helper: tagged non-chat routes require loopback until attribution headers exist. No life jobs or tagged non-chat senders exist in this implementation; future non-chat senders must call this helper at their send boundary.
- Ran `cargo fmt` across the crate, including `life/policy.rs`.

Tests inspect real fake-server requests: a cloud profile never reaches the listener; cloud-attributed JSON, error and streaming replies never enter trace response bodies; redirects never reach a second listener. Pure policy tests cover header precedence, tagged non-chat restrictions, loopback addresses and hostile URLs.

## Existing packages reviewed

The existing store/schema, consent/retention/export/delete, and encryption implementations remain in place. Their unit tests were rerun, including encrypted-on-disk canaries, export, deletion, backup without the key, recovery and the 10,000-record benchmark. No new live or performance claim is made from these checks. The CLI and settings continue to use their existing interfaces.

## Verification

- `cargo test --lib life::`: 23 passed inside the sandbox; no OS keyring access.
- `cargo test --lib`: 396 passed, 0 failed.
- `cargo test --lib --features agent-bridge`: 436 passed, 0 failed.
- `cargo test --lib --features eval,agent-bridge`: 468 passed, 0 failed.
- All full configurations retain 10 ignored live tests. Full suites run outside the sandbox because their loopback-server and existing filesystem tests require it, independently of keyring access.
- `cargo check --lib --features eval,agent-bridge`: passed for production configuration (OS keyring implementation compiled), existing warnings remain.
- `cargo fmt --check` and `git diff --check`: passed.

No model server or live desktop behavior was observed. No push was performed. Remaining-package scope awaits the missing L0 handoff; the original full package checklist cannot be verified from the available evidence.