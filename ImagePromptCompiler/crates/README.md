# Rust Workspace

`crates/`는 promptGen의 first-party Rust packages다. 의존성은 domain을 권위로 두고 각 runtime adapter가 아래를 향하는 방향으로만 흐른다. CLI는 web과 MCP bootstrap을 소유한다.

| Crate | Runtime | Public surface | 책임 |
| --- | --- | --- | --- |
| [`promptgen-core`](promptgen-core/README.md) | library | compiler, interview, schema, catalog | pure domain authority |
| [`promptgen-codex`](promptgen-codex/README.md) | library | `execute_image_generation` | Codex process와 PNG publication |
| [`promptgen-web`](promptgen-web/README.md) | library | `serve`, `serve_listener` | loopback HTTP Studio |
| [`promptgen-mcp`](promptgen-mcp/README.md) | library | `serve_stdio`, `serve_http`, `build_router` | stateless MCP adapter |
| [`promptgen-cli`](promptgen-cli/README.md) | binary | `promptgen` | CLI options, I/O, exit codes |

모든 package는 workspace Rust `1.97.1`과 체크인된 `Cargo.lock`을 사용한다. MCP adapter만 `rmcp`, `axum`, `tokio` 등 transport runtime 의존성을 가진다.
