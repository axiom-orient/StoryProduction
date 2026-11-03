# promptgen-mcp — ownership and transport boundary

## Product role

`promptgen-mcp` is a stateless protocol adapter for the image-only prompt compiler. It owns MCP
tool dispatch and the stdio/Streamable HTTP transport. It owns no prompt semantics, persisted story
state, image generation, or user approval. Core owns typed request validation and compilation;
`promptgen-codex` and `promptgen-openai` are separate optional effects composed by the CLI/Web paths.

## Module owners

| Module | Responsibility | Boundary |
|---|---|---|
| `src/lib.rs` | Tool dispatch, argument decoding, structured success/error result projection, protocol advertisement | Invalid requests remain protocol errors; compiler/provider success is not implied |
| `src/tool_contract.rs` | MCP tool definitions and JSON input contract projection | Describes only the public image compile/interview/catalog/LUT tools |
| `src/transport.rs` | `McpHttpConfig`, default loopback bind, body policy, stateless HTTP router, bearer/host enforcement, stdio lifecycle | No session persistence; remote binds require both a token and allowed host |

The 4 MiB `DEFAULT_MAX_BODY_BYTES` and 16 MiB `MAX_BODY_BYTES` have different roles: the former is
the default configuration and the latter is the accepted upper bound. The CLI help now renders the
default port from `DEFAULT_MCP_PORT`, whose value is also used by `McpHttpConfig::default`.
`MCP_PROTOCOL_VERSION` is the health endpoint's string projection; rmcp advertises the matching
typed `ProtocolVersion::V_2026_07_28` in the protocol handshake.

## Failure and state limits

The HTTP adapter is stateless and binds to IPv4 loopback by default. Configuration rejects zero or
over-limit bodies, empty credentials/host entries, and non-loopback binds without both required
security inputs. `promptgen_interview` may return `needs_input`; only a ready interview includes a
compilation. Invalid input remains invalid and no transport outcome is translated into provider or
image-generation success.

The CLI help constant consolidation and this ownership record are source-only. Rust tests, build,
lint, CLI/runtime, MCP client session, and source-snapshot verification were not run.
