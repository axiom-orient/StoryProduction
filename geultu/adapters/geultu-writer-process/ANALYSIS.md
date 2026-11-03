# 글투 ProcessWriter — 구현 분석

## Verdict·책임 경계

외부 writer 실행과 JSON protocol의 library owner다. caller는 CLI/Studio composition에 등록된 GenerationRegistry와 외부 Rust caller다. 생성물을 canonical 원문으로 만들지 않는다. 방향 **IMPROVE**. 상위 제품과 별도 제품으로 포장하지 않으며, 이 문서는 독립 material state/effect의 상세를 소유한다. wiring 확신도는 높고 실환경 결과는 아래 범위로 제한한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| provider 등록 | CLI/Studio → ProcessWriter::new/with_policy | id·program 비어 있음, timeout/byte policy validity | caller configuration / ProcessPolicy | provider 또는 InvalidInput | src/lib.rs |
| generation.generate | GenerationRegistry → GenerationProvider::generate | GenerationPlan 직렬화; core가 plan identity 검증 | `workspace.rs`의 ProcessFiles·byte-bounded file I/O와 `process.rs`의 OwnedChild·dispatch | GeneratedDraft 또는 I/O/JSON/adapter error | src/lib.rs::generate · src/workspace.rs |
| 자식 종료·실패 | poll loop → child.try_wait; Drop | stdout/stderr 크기·elapsed | direct child ownership | kill/reap·자기 임시 파일 정리 | src/process.rs::OwnedChild · src/workspace.rs::ProcessFiles |

## 실제 흐름

`GenerationPlan → private stdin file → explicit Command(no shell) → stdout/stderr files → bounded poll/status → bounded JSON read → GeneratedDraft → core draft validation → caller candidate`. canonical 쓰기·승인은 이후 별도 경계다.

## 현재 state·contract·I/O owner

program/args는 Configuration, timeout/output caps는 ProcessPolicy, 20ms poll은 dispatch mechanism이다. `workspace.rs`가 private temp directory, monotonically allocated local names, and bounded stdout/stderr file reads를 소유한다. core text limit을 재정의하지 않고 JSON escaping용 transport 한도를 둔다. Settings/Feature Flag/DB는 없다.

## 내부 workspace owner 분리 — source-only

`lib.rs`는 `ProcessWriter` public API, policy, JSON plan·draft 전송과 workspace/process owners를 조합한다.
`process.rs`는 direct-child spawn/poll/timeout/kill/reap을 소유한다. `workspace.rs`는 충돌 회피용 private temp directory/file 생성·정리와 stdout/stderr byte-bounded access를 소유한다. API, request bytes, policy defaults, adapter errors, and direct-child ownership semantics were carried with the existing operations. Rust build/test/runtime was **NOT_RUN** under the active no-verification instruction.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| JSON writer boundary | PUBLIC_LIBRARY | PARTIAL | NOT_RUN: Rust 회귀 | src/lib.rs tests |
| 원문 commit·승인 | ABSENT_IN_SCOPE | UNKNOWN | N/A: core/caller 책임 | GenerationProvider 반환 계약 |

## 의존성·findings

**GP1.** 문제 → pipe 대기와 자식 수명주기가 오류 경로 전체에 묶이지 않았다. 근거 → `GenerationProvider::generate`. 원인 → JSON stdin/stdout I/O와 process owner 분리. 영향 → hang·큰 출력·자식 잔류. 방향 → file-backed duplex I/O와 ProcessPolicy, OwnedChild RAII로 정리했다. 실제 provider SDK나 retry framework를 추가하지 않았다.

## 실패·취소·재시도·복구와 남은 gap

직접 자식만 kill/reap하며 descendant tree·전송 취소·OS hang을 보장하지 않는다. polling 사이 disk output overshoot가 가능하다. timeout은 정상 실패이며 네트워크 side effect가 있었다면 상위 caller가 결과 불명을 별도 판단해야 한다. 암묵적 재시도는 없다.

## gap·검증·근거

원래 정상 JSON, output-before-large-input, timeout, stdout 한도, zero policy 회귀가 있다. 이번 변경에서는 사용자 실행 제한에 따라 모두 NOT_RUN. 소스·public signature·caller만 검사했다. ProcessPolicy의 기본240초/stdout decoded-text×6+64KiB/stderr64KiB는 운영 검증된 최적값이라는 주장이 아니다.

[상위 ANALYSIS](../../ANALYSIS.md) · [제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md). source 근거 경로는 이 분석 단위 기준이다. PASS는 표시한 검사에만 적용하며 PUBLIC_LIBRARY/REACHABLE은 실행 성공을 뜻하지 않는다.

## Writer-process 파트 — 소스 정리 완료, 실행 미검증

`lib.rs`는 provider configuration/policy와 protocol composition, `workspace.rs`는 private files의
생성·byte-bounded read·cleanup, `process.rs`는 direct-child lifecycle을 소유한다. 20ms poll은
process owner로 이동했고 tuple child handle과 cleanup 구현을 public root에서 제거했다.
Process exit 관측 전후 모두 stdout/stderr 한도를 확인한다. 특히 success exit에서 마지막 poll
이후 늘어난 stderr를 검사하지 않고 성공 draft로 넘어가던 경로를 제거했다.

`GeneratedDraft`의 JSON decoder는 content/model/optional request_id 외의 fields를 거절한다.
Producer DTO의 public field 이름·serialization과 CLI/Studio provider 등록은 유지하며 local
protocol wrapper나 compatibility translation을 만들지 않았다. Private input/output file names,
policy defaults, malformed JSON/nonzero exit/timeout/overflow 오류, same direct-child kill/reap과
workspace lifetime을 유지한다. Canonical commit·review는 adapter 밖의 core/caller 소유다.

실행 검증은 NOT_RUN이다. Child spawn/cleanup의 OS 실패, descendants/remote effects, poll 사이의
output overshoot와 별도 cancellation/reconciliation 공백은 그대로이며 이번 분리로 해결됐다고
주장하지 않는다. Scratch output은 persistent recovery ledger가 아니다.
