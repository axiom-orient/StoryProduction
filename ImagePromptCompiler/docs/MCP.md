# Image MCP

promptgen-mcp is the image-only adapter for the deterministic
promptgen-core compiler. It exposes four read-only, idempotent tools:

- promptgen_compile_image validates and compiles a complete typed image
  request.
- promptgen_interview completes a request through a stateless question loop.
- promptgen_catalog reads the six immutable image outcomes.
- promptgen_lut_presets lists the photographic LUT presets used by the
  compiler.

The adapter never calls an image provider. A valid result is a prompt and its
typed compilation; provider execution remains an external, unverified boundary.

## Build and start

Build the release binary from the repository root:

```sh
cargo build --release -p promptgen-cli --locked --offline
```

The resulting target/release/promptgen command starts MCP. The default
transport is newline-delimited JSON over stdio:

```sh
./target/release/promptgen mcp
```

Streamable HTTP is stateless and listens on loopback port 4174 by default:

```sh
./target/release/promptgen mcp --transport http
```

Use --bind 127.0.0.1:PORT to choose a loopback address. A non-loopback bind
fails closed unless both --bearer-token TOKEN (or
PROMPTGEN_MCP_BEARER_TOKEN) and at least one --allowed-host HOST are
provided. Request bodies are bounded by 4 MiB by default and may not exceed
16 MiB (--max-body-bytes).

## Modern client contract

The server advertises only MCP protocol 2026-07-28. A client should discover
the server, inspect tools/list, and then call the tool by name. Streamable
HTTP sends MCP-Protocol-Version: 2026-07-28, Mcp-Method, and JSON
Content-Type/Accept headers. Every non-initialize HTTP request also carries
matching _meta.io.modelcontextprotocol/protocolVersion,
clientInfo, and clientCapabilities. Stdio has no HTTP headers; each JSON
RPC message is one newline.

The server is stateless: do not rely on a session ID or server-side interview
state. Keep the original brief and merge answers locally:

1. Call promptgen_interview with
   {"kind":"image","brief":"<user brief>","answers":{}}.
2. If structuredContent.status is needs_input, ask the user for every
   item in structuredContent.questions. The stable id (for example
   image.wardrobe) is the answer key.
3. Call the same tool again with the same brief and all answers collected so
   far.
4. Stop when the status is ready. The final model prompt is
   structuredContent.compilation.prompt; the same prompt is mirrored in the
   first text content block.

Example request parameters:

```json
{
  "kind": "image",
  "brief": "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보",
  "answers": {
    "image.wardrobe": "세이지색 불투명 리넨 테일러드 재킷, 크림 이너, 와이드 팬츠, 여유로운 핏"
  }
}
```

promptgen_compile_image is appropriate when the caller already has a
complete typed request. promptgen_interview is the preferred entry point for
a natural-language brief or an agent that must ask only missing fields.

## Output and errors

Successful tool calls return JSON-RPC result with:

- structuredContent: the canonical typed response;
- content[0].text: a human-readable rendering;
- isError: false.

Interview statuses are needs_input, ready, or invalid. ready includes
compilation.prompt; needs_input includes questions and no compilation;
invalid includes diagnostics and no usable prompt. Invalid tool arguments are
rejected as JSON-RPC -32602 (invalid_params) and unknown tools as -32601.
HTTP validation failures use a 4xx response; the JSON-RPC error body remains
authoritative. Do not treat an invalid response as a generated prompt.

## Security and operating boundary

All tools are read-only and idempotent. The compiler does not access the
network, write files, or execute a provider. Keep HTTP on loopback unless the
deployment has an explicit bearer token, host allowlist, and network policy.
The body limit, strict schemas, fail-closed validation, and opaque PNG output
contract are part of the public boundary. Provider authentication, image
generation, and visual fidelity are NOT_RUN_EXTERNAL in local verification.

## Reproduce the contract

```sh
cargo test -p promptgen-mcp --test streamable_http --locked --offline
cargo test -p promptgen-cli --test cli mcp_stdio_external_agent_interview_retries_questions_and_returns_prompt --locked --offline
```
