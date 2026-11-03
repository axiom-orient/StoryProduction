# NarrativeCourier MCP — 분석

연결된 설정 정책의 정본은 [app-config 분석](../app-config/ANALYSIS.md)에 둔다.

## Verdict·책임 경계

`crates/mcp`는 canonical STS/Delivery engine을 호출하는 stdio transport다. 서사 의미·durable delivery state·human decision은 소유하지 않는다. MCP tool schema는 `params.rs` DTO derive에서 나오며 별도 JSON schema/상태 모델을 복제하지 않는다.

| Module | Owner | Responsibility |
|---|---|---|
| `src/lib.rs` | crate public facade | `StsMcp`, `serve_stdio`, parameter DTO, MCP-specific root argument parser를 export한다; `Roots` remains owned/exported by app-config. |
| `src/tools.rs` | MCP tool surface | 단일 `#[tool_router]` registry, public tool signatures/schema, and route delegation를 소유한다. |
| `src/tools/{projects,delivery,verified_synopsis,screenplay}.rs` | private tool adapters | 기존 Engine/DeliveryEngine 호출, verified-synopsis import, screenplay file effects를 tool group별로 조정한다. 새 state owner는 아니다. |
| `src/tools/support.rs` | response helper | blocking worker 실행·JSON response/error 변환을 한 helper로 제공한다. |
| `src/server.rs` | transport lifecycle and server identity | MCP server instructions/capabilities와 stdin/stdout service startup/wait를 소유한다. |
| `src/params.rs` | request contract | `JsonSchema`/Serde input DTO만 정의한다. engine validation을 대체하지 않는다. |
| `src/roots.rs` | MCP argument adapter | `resolve_roots(args)` parses `--assets-root` / `--projects-root` and delegates precedence; it returns app-config's `Roots` without re-exporting that type. |
| `src/main.rs` | binary entry | Tokio runtime, `serve_stdio`, stderr/exit status만 조합한다. |

## 실제 흐름·권한

`main → lib::serve_stdio → roots::resolve_roots → Engine::with_roots → StsMcp → stdio`. `tools.rs`는 단일 router와 schemas를 유지하고 group adapter로 위임한다. Blocking Engine/DeliveryEngine call은 `tools/support.rs::blocking_json`을 통과해 `spawn_blocking`과 MCP JSON result/error 변환을 사용한다. Worker join, engine, serialization 오류는 MCP internal error로 전달되며 성공 result를 꾸며내지 않는다. STS `decide`와 delivery `approve|reject`는 router에 없으며 human-only CLI/Tauri surface로 남는다. `run_delivery`, `cancel_delivery`는 delivery engine을 호출하지만 MCP가 DeliverySet이나 canonical state를 직접 쓰지 않는다.

## 분리 근거와 보존 범위

`lib.rs`의 이전 composition은 engine adapters, `ServerHandler`, server metadata, stdio startup을 한 파일에 두었다. Tool definitions는 하나의 router/`StsMcp` 소유자로 유지하고, transport lifecycle은 sibling `server.rs`로 옮겼다. CLI와 MCP가 각각 구현하던 environment/platform root precedence는 `crates/app-config`가 소유하며, MCP는 flags parser만 둔다. `Roots`는 app-config public type path 하나만 유지한다. Existing crate-root `StsMcp`/`serve_stdio`/`resolve_roots`, tool names/annotations, params DTOs, root precedence, human authority notice, engine call outcomes를 보존한다. No new transport, durable contract, fallback, or approval tool is introduced.

MCP lifecycle, effect adapters, and common root resolver를 포함한 가장 최근 verified source graph는 2026-09-24에 `verify-all.sh --require-rust`(60 Rust tests/fmt/clippy), 480-entry source manifest, and an actual stdio process run으로 확인했다. stdio run은 10개 도구의 목록/status/doctor/verified-synopsis import/screenplay import·export/delivery context·HTML delivery/list를 실행했고, `NEEDS_REVIEW`를 보존하며 human-only 도구를 노출하지 않았다. Public owner-chain CLI E2E도 같은 source graph에서 통과했다. 이후 STS manifest validation과 screenplay parser ownership splits는 source-only이며, 이 새 graph의 검증은 **NOT_RUN**이다. 합성 story의 의미 품질과 실제 사용자 원고·사람 승인은 별도 **NOT_RUN**이다. 세부 command/log는 [2026-09-24 실사용 검증 기록](../../../docs/research/2026-09-24-real-use-validation.md)에 있다.

## Current source-only slice — screenplay export filename admission

The MCP screenplay export adapter accepts only one bare filename component and rejects absolute paths,
separators, dot and parent path values before joining the project output directory. Tool names, parameter DTOs,
and the STS-owned export effect remain unchanged. Tests/build/fmt/lint/runtime and source-manifest
qualification are **NOT_RUN**.

## NVC MCP tool contract/dispatch — source-only closure

MCP parameter DTOs are closed with deny_unknown_fields. Required text and present optional text
reject blank/null input while preserving supplied bytes; omission alone selects documented import
name/title or export filename defaults. Positive duration admission and its JsonSchema constraints
share this transport owner. Export format is a literal fountain/fdx/docx enum, exported with the
parameter types; case-fold and empty-string compatibility paths are retired. Export filename remains
project-output-contained and must have the selected format extension, preventing canonical Markdown/
JSON authority files from being selected as export targets.

RunDeliveryParams embeds a cached schema projection directly from the producer DeliveryRequest
resource. Producer-local refs are expanded for nesting; no durable consumer schema copy or second
DeliveryRequest DTO is introduced. The typed DeliveryRequest validator also runs before the worker
accesses project state. Invalid transport values remain invalid_params; domain execution failures
return CallToolResult::error and do not become successful content. Worker join failures retain possible
unknown outcomes, and serialization failures retain possible already-applied effects. Text JSON
success output remains compatible with current consumers. Tool descriptions and server scope match
the actual ingress/read/export/delivery surface; Human semantic/adoption tools remain absent.

This closes params/schema, registered tool dispatch, domain adapters, blocking-result handling, public
parameter exports, and metadata descriptions. Existing producer schema bytes/pins and authoritative
state owners are unchanged. No MCP server, schema generator, tool call, provider, or project mutation
was executed. Transport cancellation does not by itself qualify running-worker cancellation. Tests/
build/fmt/lint/schema generation/protocol/runtime and manifest qualification are **NOT_RUN**.
