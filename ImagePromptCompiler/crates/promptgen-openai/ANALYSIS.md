# IPC OpenAI edit effect — 구현 분석

## Verdict·책임 경계

한 base image의 external image-edit transport. caller는 CLI → execute_image_edit. 방향 **MAINTAIN**. 이전 source snapshot의 crate tests/build/fmt/clippy는 통과했고 official repository gate는 historical evidence 부재로 PARTIAL이었다. 이후 hash owner 변경은 NOT_RUN이다. actual provider/network result remains NOT_RUN.

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
| CLI path preflight | CLI → openai_paths_alias | reference/result/image-output path pair | provider-owned symlink/canonical/inode alias resolution | bool or I/O error | reference.rs |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | CLI → crate-root `execute_image_edit` facade | matching compiled prompt + request + exactly1 base reference + env credential; standalone receipt | `execution.rs` composes the private owners | 정상 result 또는 명시적 오류 | `src/lib.rs`, `src/api.rs`, `src/execution.rs`, `src/request.rs`, `src/response.rs`, `src/reference.rs` |

## 모듈 owner

| Module | Owner | Responsibility |
|---|---|---|
| `src/lib.rs` | stable crate facade | `execute_image_edit`와 public config/receipt/error DTO를 노출한다. Endpoint/model constants는 provider implementation 내부에 둔다. |
| `src/api.rs` | public DTO contract | `OpenAiRunConfig`, standalone `OpenAiEditReceipt::to_json`, and typed execution error shape를 소유한다. |
| `src/execution.rs` | provider workflow composition | compile/request equality → preflight → snapshot → curl → bounded decode → PNG publish → receipt sequence를 소유한다. |
| `src/support.rs` | shared internal projection | I/O error mapping만 제공한다. Raw-byte digest는 core `image_artifact::sha256::digest_hex`를 직접 사용한다. 이번 변경은 NOT_RUN이다. |
| `src/tests.rs` | effect-boundary fixtures | 기존 fake-curl/environment/reference/timeout tests를 production facade 밖에 둔다. Test module name and self-reexec identifiers remain `tests::...`. |
| `src/request.rs`, `src/reference.rs`, `src/transport.rs`, `src/response.rs`, `src/files.rs`, `src/limits.rs` | focused effect owners | reference.rs owns the exported OpenAI-specific path-alias rule shared with CLI preflight; other modules keep validation, credential/process, reference identity, bounded response, file I/O, and limits. |

## 실제 흐름

recompile/equality → `reference.rs` alias guard + private snapshot → curl config on stdin → bounded network result → status/base64/PNG dimensions verify → publish PNG → receipt

## 현재 state·contract·I/O owner

core owns typed semantics; `request.rs` owns compilation and one-base edit-contract validation; `reference.rs` owns source identity, alias checks, MIME recognition, and private snapshot; `api.rs` owns public config/receipt/error DTOs; `execution.rs` composes the effect sequence behind the existing crate-root function; `limits.rs` owns shared response/log ceilings; private `transport.rs` owns environment credential ingress/validation, curl configuration, and direct-child lifecycle; private `response.rs` owns bounded response decoding and verified PNG publication; private `files.rs` owns private workspace, lexical path normalization, output preflight, and bounded file operations. `support.rs` contains only shared internal hash/error projection. The API key stays out of argv; endpoint/model values are implementation constants, external support UNKNOWN.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 공개 surface | REACHABLE | PARTIAL | PASS: current source tests/build/quality gate; NOT_RUN: actual credential/API/network effect | `src/lib.rs`, `src/api.rs`, `src/execution.rs`, `src/response.rs`, `src/reference.rs` |
| 독립 호출 library 확장 | PUBLIC_LIBRARY | UNKNOWN | NOT_RUN: 외부 caller 미조사 | public exports/상위 composition |

## 의존성·findings

**PO1.** 문제 → 서버가 edit를 처리했지만 로컬 timeout/output publication이 실패할 수 있다. generic typed error는 전체 remote rollback을 의미하지 않는다. 근거 → `src/execution.rs::execute_image_edit`, `src/response.rs::publish_png`. 원인 → local process outcome와 remote side effect 차이. 영향 → caller가 작업 완료/승인/복구 가능성을 잘못 해석할 수 있다. 방향 → integration은 dispatch 후 불확실성을 별도로 보존하고 자동 retry를 하지 않는다. 현재 standalone receipt를 nested execution으로 바꾸지 않는다.

## 실패·취소·재시도·복구와 남은 gap

실제 API credential/model support NOT_RUN. fake-curl tests는 response decoder와 lifecycle seam만 검증한다. one-base 제한/transport bounds/alias checks는 intentional optional contract로 KEEP.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)
