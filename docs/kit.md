# Kit HTTP integration

Studio connects to a separately running Kit server at
`http://127.0.0.1:47601/mcp` by default. Configure another loopback HTTP `/mcp`
endpoint in Settings → Tools. Minimum supported HTTP release: **Kit 0.6.0**.
Kit's current local preview artifact may still report 0.5.0; debug-only testing
can explicitly opt in with `RUNE_STUDIO_ALLOW_KIT_HTTP_PREVIEW=1`. Published
0.5.0 is not an HTTP compatibility guarantee; release builds reject it.

Start Kit with `rune-kit serve --headless` for background/CI use. With an attached
interactive terminal Kit provides its own TUI. Studio never starts or stops Kit.
Kit owns installation, plugins, filesystem roots and their migrations. Filesystem
reads need an explicit server `-p allowed_dir=<absolute existing folder>` and
read-only/allow-tool configuration. Studio cannot replace that root in a call.
An optional `RUNE_KIT_HTTP_TOKEN` environment value supplies bearer authentication;
restart Studio after setting it. It is not written into Studio's Kit settings.

Old executable/stdio settings are disabled on read. Selection is retained, and
saving the new endpoint first preserves the original bytes in
`kit.stdio-backup.json` without overwriting an existing backup. Explicitly start
the new server, configure its root, verify discovery, then re-enable selected
tools. There is no silent launch or stdio fallback.

## Protocol contract

The exact revision lives in `mcp-kit-version.txt`, embedded at compile time.
Studio sends per-request metadata, mirrored MCP method/name/parameter headers,
and accepts JSON or request-scoped SSE. It handles complete results only;
unsupported/paginated results and malformed/error responses fail visibly.
Cancellation closes only the in-flight HTTP response. Disconnects/timeouts are
not replayed. Every new operation discovers the server afresh, allowing reconnect
after a restart without stale cached schemas. Tool selection only narrows server
permissions. Unsafe metadata, unknown schemas, invalid header annotations,
write tools and filesystem root overrides remain refused.

Reference: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http
The external-agent bridge has a separate compatibility boundary, documented in
`mcp-bridge-versions.txt`; private native/WASM plugin operations are not public MCP.