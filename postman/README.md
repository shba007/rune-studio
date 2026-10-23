# Rune Studio Postman collection

This is Postman Collection **v3 Native Git YAML**, generated with Postman CLI
1.75.0. Open this repository as a local project in Postman v12 and select
`postman/collections/Rune Studio`. No cloud workspace IDs or sync are required.

## Configure and run

Set local values for `studio_token`, `infer_token` (if Infer requires one), and
`model` (an installed model ID from **Infer models**). Defaults use Studio's
loopback bridge on port 47600 and Infer on port 8080. Change `infer_url` to the
same trusted endpoint configured in Studio. Keep tokens in local values or
Postman Vault; do not save secrets in these files or cloud sync them.

Studio must be running with the agent bridge available and External agent
explicitly enabled by the owner. Use the owner's existing authorized connection
token; these requests do not retrieve tokens, enable the bridge, or bypass
access checks. Missing authorization produces HTTP 401; disabled External agent
produces HTTP 403. Do not add an Origin header (the bridge rejects browser origins).

Run Infer health, then Infer models; choose a model before Infer chat smoke.
The chat request can load a model and consume compute, but sends only a synthetic
greeting. The bridge requests inspect identity, ping, tools and the agent guide;
none begin a session, capture the screen, click, type, or change desktop state.
Unknown method checks the actual JSON-RPC -32601 failure contract.

For local validation:

```powershell
postman collection lint 'postman/collections/Rune Studio' --fail-severity warning
postman collection run 'postman/collections/Rune Studio'
```

Set local variables before running. A collection run needs live services and
credentials. No live requests or desktop actions were executed when authoring.
The collection has response assertions; lint alone does not prove server behavior.

## Protocol boundaries

Studio's external HTTP bridge currently implements initialize with MCP revision
2025-06-18 (see src-tauri/src/bridge/mcp.rs). This collection matches that actual
server, including its JSON responses. Studio -> Kit is a different interface:
MCP 2026-07-28 over stdio with per-request metadata. Kit has no HTTP route here;
these HTTP requests neither test nor claim Kit conformance. Studio's Tauri IPC
commands are also not HTTP endpoints. See docs/kit.md for Kit acceptance.

Infer health, model discovery and non-streaming chat mirror the HTTP paths in
src-tauri/src/backend.rs and src-tauri/src/model/client.rs. No model ID, GPU,
installed plugin, or optional capability is assumed.

Format references:

- https://learning.postman.com/docs/use/use-collections/collections-schemas/
- https://learning.postman.com/docs/use/native-git/setup