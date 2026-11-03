# IPC Studio transport — 구현 분석

## Verdict·책임 경계

loopback interview/generation UI와 artifact serving. caller는 CLI serve → serve_listener → HTTP route. 방향 **MAINTAIN**. reachable wiring 확신도 높음, 실환경 검증 확신도 낮음(NOT_RUN).

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | CLI serve → serve_listener → HTTP route | host/origin/body limit; typed interview/generate request | 아래 owner 구분 | 정상 result 또는 명시적 오류 | `src/server.rs::serve_listener/ConnectionGuard`, `src/routes.rs::route`, `src/routes/generation.rs::handle_generate/GenerationGuard`, `src/routes/catalog.rs`, `src/request.rs`, `src/security.rs`, `src/http.rs::HttpRequest/Response` |

## 실제 흐름

validate config → loopback accept → connection guard → `http.rs` parse/serialize → authority + route → core interview/compile → generation guard → optional Codex effect → artifact registry → HTTP result

## 현재 state·contract·I/O owner

`server.rs::ServerState` owns transient connection admission and generation lease context; `routes.rs` owns endpoint dispatch, `routes/catalog.rs` owns runtime catalog assets/cache, and `routes/generation.rs` owns interview and generation effects plus published artifact reads; `request.rs` owns the HTTP JSON-to-typed-request boundary; `security.rs` owns loopback Host/Origin/client authority and output-name decisions; `http.rs` owns byte framing only. `lib.rs` embeds `index.html`, `app.css`, and one `app.js` at compile time; `routes.rs` serves them only from fixed paths. `asset_contract.rs` is guarded by `cfg(test)` and checks the embedded bytes/design hooks; it is not a runtime contract parser. Core owns category/profile; provider owns generation files. Browser selections are UI state, not second compiler policy.

`assets/app.js` remains a single-page state machine with shared interview, catalog, preview, and recent-result state. The server exposes it as one fixed `/app.js` response, and the design contract inspects the exact `include_str!` payload. Splitting it into independently loaded scripts would add routes or a new build/concatenation contract; the present owner boundary matches the runtime surface, so it remains one asset.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 공개 surface | REACHABLE | PARTIAL | NOT_RUN: Rust/실제 provider/제품 runtime; 정적 호출만 확인 | `src/lib.rs::serve_listener/handle_generate/GenerationGuard`, `src/http.rs` |
| 독립 호출 library 확장 | PUBLIC_LIBRARY | UNKNOWN | NOT_RUN: 외부 caller 미조사 | public exports/상위 composition |

## 의존성·findings

**PW1.** 문제 → UI 상태·도달성과 real browser generation 성공은 별개다. source only로 모든 async disconnect/shutdown races를 검증하지 못했다. 근거 → `src/lib.rs::serve_listener/HttpRequest/handle_generate/GenerationGuard`. 원인 → 실환경 검증 공백. 영향 → caller가 작업 완료/승인/복구 가능성을 잘못 해석할 수 있다. 방향 → 현 single-generation guard와 request identity를 유지; shutdown/disconnect/duplicate generate/late result를 focused runtime test한다. 새 queue/DB를 선제 도입하지 않는다.

## 실패·취소·재시도·복구와 남은 gap

ShutdownSignal/ConnectionGuard/GenerationGuard의 drop cleanup을 보존. UI가 category 바꾸면 stale profile을 지우지만 원문/Story approval을 변경하지 않는다. real browser NOT_RUN.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)
