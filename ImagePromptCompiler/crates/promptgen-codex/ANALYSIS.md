# IPC Codex effect — 구현 분석

## Verdict·책임 경계

이미지 생성·artifact/fidelity 검사·제한된 repair를 소유하는 optional library. caller는 CLI/Web → execute_image_generation. 방향 **IMPROVE**. reachable wiring 확신도 높음, 실환경 검증 확신도 낮음(NOT_RUN).

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | CLI/Web → execute_image_generation | valid compilation + same typed request + CodexRunConfig; CodexExecutionReceipt | 아래 owner 구분 | 정상 result 또는 명시적 오류 | `src/lib.rs::execute_image_generation`, `src/model.rs`, `src/preflight.rs`, `src/artifact/publication.rs::validate_copy_publish`, `src/process.rs::OwnedChild` |

`CodexRunConfig::generated_root` is a crate-private derived-path helper consumed only by the
Codex orchestration/preflight owners. The documented external config and execution entrypoints do
not expose this internal path projection.

`inspect_png_bytes`는 core의 `image_artifact::png::inspect_bytes`를 호출자 없이 감싸며, 오류와
검사 결과도 축약한다. Codex adapter 표면에서 제거하고 PNG 검사는 core owner를 직접 사용한다.
중복 `sha256_hex_parts`와 adapter-local `sha256_hex`는 제거했다. Buffered-byte identity는
core의 `image_artifact::sha256::digest_hex`를 직접 호출한다. Streaming publication은 복사하면서
같은 core Sha256을 누적하므로 그 effect-local 경로를 유지한다. 이번 변경은 실행 검증 NOT_RUN이다.

## 실제 흐름

validate → probe → isolated codex exec → event contract check → thread/call-bound artifact discover → PNG validation/dimension normalization → fidelity gate → bounded repair attempts → publish receipt

## 현재 state·contract·I/O owner

core owns prompt meaning; provider owns actual generation and event artifact evidence. `model.rs` owns the stable config, receipt, fidelity DTO, and receipt JSON projection; `preflight.rs` owns compilation/backend constraints and runtime path authority; `instruction.rs` serializes the already-compiled prompt identity and typed output parameters into the bounded one-image instruction; `candidate.rs` owns strict PNG inspection and candidate dimension preparation; `event_stream/events.rs` decodes raw event records, `event_stream/policy.rs` owns the explicit item-kind and exact skill-read allowlist, and `event_stream.rs` owns turn identity, phase, and terminal invariants; `fidelity/schema.rs` owns strict output schemas, `fidelity/instruction.rs` owns the gate-specific reviewer prompts, `fidelity/response.rs` decodes structured result fields, `fidelity/placement.rs` enforces subject counts and typed geometry, and `fidelity/text.rs` enforces exact copy; `fidelity.rs` composes those checks into a validated result; `artifact.rs` only declares the discovery and publication owner modules, which sibling callers import by owner path; artifact/discovery.rs owns thread-bound candidate selection and identity, while artifact/publication.rs owns output admission, validated byte copy, and atomic publication; `fidelity_gate.rs` owns gate sequencing and bounded repair; `process.rs` owns direct-child spawn/write/wait/reap, version-probe, bounded log lifecycle, and the temporary working directory; descendant cleanup or network rollback is not asserted. max_fidelity_attempts/timeouts는 Policy, binary/CODEX_HOME/output는 Configuration, receipt는 observed State/result.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 공개 surface | REACHABLE | PARTIAL | NOT_RUN: Rust/실제 provider/제품 runtime; 정적 호출만 확인 | `src/lib.rs::execute_image_generation/validate_copy_publish`, `src/process.rs::OwnedChild` |
| 독립 호출 library 확장 | PUBLIC_LIBRARY | UNKNOWN | NOT_RUN: 외부 caller 미조사 | public exports/상위 composition |

## 의존성·findings

**PC1 · Codex public dual-input binding — 수정, 실행 검증 남음.** 문제 → valid compilation과 supplied request가 서로 다른 내용일 수 있었다. 근거 → `promptgen-codex::execute_image_generation`; 원인 → core prompt authority를 effect boundary에서 확인하지 않음. 영향 → request가 아닌 다른 prompt를 실행하고 receipt를 오해할 수 있음. 방향 → core로 같은 request를 재compile하고 valid/prompt equality를 검사한다. 불일치는 `CODEX_COMPILATION_MISMATCH`로 spawn 전에 거절한다. public 함수와 정상 receipt는 보존했다.

## 실패·취소·재시도·복구와 남은 gap

OwnedChild kill/reap는 direct child에 한정. descendant cleanup과 network rollback은 증명되지 않았다. fidelity agent/실제 pixels/normalization platform 경로는 NOT_RUN. 제한된 repair도 각 결과를 검증하며 실패 시 output을 승격하지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)
