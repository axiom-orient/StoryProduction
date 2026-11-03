# SceneMotionCompiler MCP server — module ownership

## Product boundary

The MCP server is a transport adapter for the registered SMC Application commands. Its executable
entrypoint is `src/cli.js`, which is also the `smc-mcp` binary declared by the private monorepo root
package. This app owns argument parsing, stdio lifecycle, MCP tool registration, and response-envelope
validation; it does not own story, release, approval, or custody state.

## Module map

| Module | Responsibility | Caller/boundary |
|---|---|---|
| `src/cli.js` | Parse launch options, start the stdio transport, emit diagnostics only to stderr, and close on SIGINT/SIGTERM | Sole executable entrypoint configured in root `package.json` |
| `src/launch-config.js` | Accept only root/project arguments, reject duplicates and resolve each path against launch cwd | `cli.js`; parser errors carry stable `code` values |
| `src/server.js` | Build tool schemas from the public Application command registry, serialize commands through one promise tail, invoke the Application service, and validate returned envelopes | `cli.js` constructs one server per transport process |

The prior `src/index.js` barrel had no in-repository consumer, and
`listIfscMcpToolNames()` had no caller. The MCP protocol's `tools/list` response is already derived
from the same registry-backed `TOOL_DEFINITIONS`, so the duplicate names-only export is removed.
`createSmcMcpServer` is the direct composition function used by `cli.js`; the old IFSC name is removed without an alias.

## State and failure boundary

The server serializes local commands to preserve the process-local command ordering, then validates
Application envelopes before returning MCP results. A transport success does not turn an Application
failure into success, and MCP does not expose Studio-only Human approval commands. The app has no
durable state of its own.

This source-only surface cleanup is not qualified by tests, build, lint, CLI/MCP runtime, aggregate
release gates, or manifest verification. The source manifest is regenerated as metadata only.

## 실행 구성 파트 — 소스 정리 완료, 실행 미검증

`--input`과 application config의 `inputPaths`는 실행에 쓰이지 않으므로 제거했다.
MCP launch는 root/project 두 개의 단일 값을 보관하며 입력 목록·comma split·array accumulation을
하지 않는다. Application은 retired `inputPaths` 옵션을 조용히 무시하지 않고 거절한다.
CLI·Studio launcher·maintenance callers와 기존 fixtures는 현재 config를 사용한다.
Verified handoff의 실제 input byte paths와 protected output path admission은 유지한다.
공개 MCP command set·Studio-only approval·error envelope는 바꾸지 않았다. NOT_RUN.
