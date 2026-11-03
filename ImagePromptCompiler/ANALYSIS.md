# ImagePromptCompiler — 구현 분석

## 현재 분석·개선 범위

Codex and OpenAI adapters directly use `image_artifact::sha256::digest_hex` in promptgen-core for
buffered raw-byte identities. Their duplicated private SHA helpers are removed. Streaming copy
hashes stay with the copy effect, and process/network/error/approval meanings remain provider-local.
The CLI result-file contract also now has one canonical `--result` option; the generic `--output` synonym was removed. Tests/build/lint/runtime and source-snapshot qualification are NOT_RUN for these edits.

### CLI 경로 처리 파트 — 소스 정리 완료, 실행 미검증

`promptgen-cli/src/paths.rs`는 CODEX_HOME 선택과 lexical output identity만 소유한다. OpenAI
alias preflight는 `commands/image/openai.rs`가 provider의 canonical path identity API를 직접
호출하고 CLI I/O 오류를 매핑한다. 누락된 `std::io` import를 복구하고
lexical resolution의 I/O 실패를 `false`로 바꾸던 경로를 제거했다. Codex command는 `Result`를
전파해 경로를 결정하지 못했으면 provider effect 전에 중단한다. Lexical 비교와 OpenAI의
canonical/symlink/inode 비교는 다른 의미이므로 같은 boolean helper로 합치지 않았다.
기존 옵션·provider 의미·output policy는 유지하며 tests/build/lint/runtime은 NOT_RUN이다.

## Verdict·책임 경계

사용자 brief/typed decisions를 검증 가능한 image prompt로 컴파일하는 독립 제품이다. 선택적으로 실제 Codex 이미지 생성/OpenAI 이미지 편집/Studio를 제공한다. caller는 CLI/MCP/웹과 Story integration이다. 방향 **MAINTAIN + IMPROVE(경계)**. 2026-09-30 기록된 parent workspace snapshot에서는 cargo check, Clippy, docs/release build와 CLI/Web smoke는 통과했지만 component-local 문서 경계 검사와 rustfmt는 실패했고, historical image-evidence 입력도 없다. 이번 owner import/서식 정리는 source-only이며 **NOT_RUN**이다. 실제 Codex/OpenAI provider와 browser/client session은 NOT_RUN이다. `promptgen`은 유지해야 할 crate/binary public name이다. 세부 범위는 [구현 현황](docs/IMPLEMENTATION_STATUS.md)을 보고, 부모 workspace 증거 경로는 `../docs/research/2026-09-30-live-use-validation.md`다.

| 단위 | 판정·이유 |
|---|---|
| promptgen-core | [분석](crates/promptgen-core/ANALYSIS.md): deterministic typed prompt compiler, image interview, catalog, and PNG artifact parser. The crate root and `json`, image, and interview facades retain the established public entrypoints; leaf-level ownership and clean-break history live in the component analysis. |
| promptgen-cli | [분석](crates/promptgen-cli/ANALYSIS.md): public binary CLI grammar and command adapters; provider routing, bounded input/result publication, and boundary-specific errors. `--result` is the sole result-file option. |
| promptgen-mcp | [독립 adapter 분석](crates/promptgen-mcp/ANALYSIS.md): `lib.rs` owns tool dispatch/result mapping, `tool_contract.rs` owns tool schema, and `transport.rs` owns stateless stdio/HTTP lifecycle and auth/bounds. CLI help now reads its default port from `DEFAULT_MCP_PORT`. MCP에 이미지 provider execution을 임의 추가하지 않는다. |
| promptgen-codex | [분석](crates/promptgen-codex/ANALYSIS.md): optional Codex image effect with bounded process/event/artifact/fidelity lifecycle; it owns no story state. |
| promptgen-openai | [독립 effect 분석](crates/promptgen-openai/ANALYSIS.md): `lib.rs` preserves crate-root exports; `api.rs` owns config/receipt/error DTOs; `execution.rs` composes the edit effect; request, reference, transport, response, files, limits, and support remain focused owners. |
| promptgen-web | [분석](crates/promptgen-web/ANALYSIS.md): loopback Studio transport, typed request boundary, local authority, transient generation guard, and artifact routes. |
| catalog/examples/docs specialized specs | 제품 계약/입력. C6 app_web_ui는 image category이며 별도 UI 제품 authority가 아니다. |

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| image prompt-only | CLI image → core compile_image_prompt | typed JSON·catalog/category/profile·invariants | pure compiler | compilation/diagnostics, invalid면 prompt 없음 | core lib/CLI main |
| interview/schema/catalog/lut | CLI/MCP → core exports | current decisions/catalog request | typed intermediate data·embedded catalog | missing questions/schema/catalog/LUT | `crates/promptgen-core/src/lib.rs`, MCP |
| codex-imagegen·선택 | CLI/Web → codex effect | valid compilation·no reference route·output collision | owned subprocess/private output | verified PNG/receipt 또는 typed failure | codex 분석 |
| openai-image-edit·선택 | CLI → openai effect | 정확히 하나 base image·env credential·no path alias | private request/curl/network | image+standalone OpenAiEditReceipt | openai 분석 |
| serve | CLI → WebConfig/server | loopback/catalog/project paths/timeouts | web request/generation guard | browser response/artifact | web 분석 |
| mcp | CLI → rmcp server | advertised tool schema | stateless core operations | tool result, provider capability 없음 | `crates/promptgen-mcp` |

## 실제 흐름

대표 pure path: `image JSON → compile_image_prompt → category/profile validation → deterministic rendering → compilation outcome → JSON/text output`. 실패 입력은 provider에 도달하지 않는다.

선택 effect: `valid compilation → preflight output/credentials/reference → provider execution → bounded response/artifact validation → publish PNG → execution receipt → CLI output`. integration은 이 결과를 Story candidate로 감쌀 뿐 IPC가 서사나 승인 상태를 갖지 않는다.

## 현재 state·contract·I/O owner

core owns prompt meaning/diagnostic/interview rules. catalog data is configuration with advertised profiles; category ids/schema versions are contract constants. Web owns transient generation guard, not narrative state. providers own timeout/artifact-wait policy and temporary files; reference credentials never compiler state. provider result cannot auto-approve source text/assets.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| image compile·interview·catalog | PUBLIC_LIBRARY | SATISFIED | PASS: current Rust workspace tests; generated media quality 별도 | `crates/promptgen-core`, CLI/MCP/Web |
| category/profile·pose/storyboard/LUT | REACHABLE | SATISFIED | PASS: current workspace tests; actual image output fidelity NOT_RUN | specialized specs, core |
| Codex image generation | REACHABLE | PARTIAL | PASS: fake-process lifecycle/receipt tests; NOT_RUN: real Codex process/provider | `promptgen-codex/src/lib.rs` |
| OpenAI one-base edit | REACHABLE | PARTIAL | PASS: fake-transport adapter tests/build/quality; NOT_RUN: actual credential/API/network | `promptgen-openai/src/lib.rs` |
| Studio/MCP | REACHABLE | PARTIAL | PASS: current Rust protocol/server tests; NOT_RUN: actual browser/client session | 하위 Studio 분석, `docs/MCP.md` |
| source snapshot tool | REACHABLE | SATISFIED | PASS: `scripts/test-source-snapshot.py` | snapshot source/tests |
| 일반 문서/개발 compiler·전역 story approval | ABSENT_IN_SCOPE | UNKNOWN | N/A: 본 저장소의 제품 범위 아님 | image-only identity |

## 의존성·findings

**P1 · Codex public dual-input binding — 수정, 실행 검증 남음.** 문제 → valid compilation과 supplied request가 서로 다른 내용일 수 있었다. 근거 → `promptgen-codex::execute_image_generation`; 원인 → core prompt authority를 effect boundary에서 확인하지 않음. 영향 → request가 아닌 다른 prompt를 실행하고 receipt를 오해할 수 있음. 방향 → core로 같은 request를 재compile하고 valid/prompt equality를 검사한다. 불일치는 `CODEX_COMPILATION_MISMATCH`로 spawn 전에 거절한다. public 함수와 정상 receipt는 보존했다.

**P2 · 선택 effect의 검증 범위 — 유지·검증 필요.** 문제 → 실제 Codex/OpenAI 실행·reference 전달·PNG quality는 소스에서 확정 불가. 근거 → 두 provider adapter의 process/HTTP 경계. 원인 → Rust·자격·외부 도구 부재. 영향 → 연결 여부를 success로 오인. 방향 → prompt-only를 유지하고 지원하지 않는 mode/reference는 명시 실패한다. 실제 API 가용성·비용·provider 응답은 UNKNOWN이다.

## 실패·취소·재시도·복구와 남은 gap

provider별 output/input collision, response bounds, artifact validation, timeout 처리는 하위 분석에 있다. pure compiler에는 cancellation/recovery DB를 발명하지 않는다. provider cancellation API가 없는 경로를 global Story cancellation으로 오인하지 않는다. 유효 optional C6 UI image/profile/pose/cinematic 기능을 Story UI제품 분리라는 이유로 삭제하지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.
