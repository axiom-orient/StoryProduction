# promptgen-mcp

Stateless MCP adapter for image prompt compilation, image interviews, the six
entry catalog, and LUT presets. It exposes no non-image compiler or tool.

Modern clients should discover the server, call promptgen_interview for an
incomplete image brief, and repeat the call with answers keyed by each
question id. A ready response places the final model prompt at
structuredContent.compilation.prompt and mirrors it in the first text content
block. Invalid input is fail-closed and never yields a usable prompt.

The default transport is newline-delimited stdio; Streamable HTTP is available
through the CLI on loopback port 4174. HTTP non-initialize requests use
protocol 2026-07-28 headers and per-request metadata. Stdio has no HTTP
headers. See ../../docs/MCP.md for build commands, client configuration,
interview examples, output/error semantics, and security limits.
