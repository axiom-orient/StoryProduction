# SMC Release custody — 구현 분석

## Verdict·책임 경계

release pointer와 immutable evidence에 대한 Rust custody transport. caller는 release workflow → TrackBCustodySession → engine/crates/smc-trackb-custody. 방향 **MAINTAIN**. reachable wiring 확신도 높음, 실환경 검증 확신도 낮음(NOT_RUN).

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## Module responsibilities

| Module | Owns | Boundary kept |
|---|---|---|
| `src/trackb-custody-protocol.js` | Custody limits, typed error values, artifact/measurement/receipt DTO shape and byte validation, safe ID/path rules | Protocol and producer receipt invariants; consumes the contracts-owned `hashTrackBRawBytes`; no process or file descriptor lifecycle |
| `src/trackb-custody-files.js` | Project-root validation, no-symlink ancestor checks, stable bounded reads on opened descriptors | Filesystem identity and read boundary only; no process/session state |
| `src/trackb-custody-binary.js` | Supported target selection, fixed helper path, binary/receipt admission and same-descriptor hash validation | Runs before spawn; no PATH/cargo/source-target fallback |
| `src/trackb-custody-session.js` | Internal `TrackBCustodySession`, purpose-specific read/write sequencing, JSON-lines child lifecycle, canonical open entrypoints | Session/protocol process state only; durable mutation remains in the Rust helper; no supported direct-construction API |
| `src/index.js` | Sole application entry point; re-exports consumer-used custody limits, measurement result validators, and purpose-specific session factories | Error implementation, binary admission, packaging constants, transport bounds, and protocol kind lists stay in their defining owner modules |

The custody bridge is divided by effect boundary: protocol DTO checks, descriptor/file reads, pinned
binary admission, then one stateful helper session. The workspace still has one custody owner and one
application entry path. Application services import only their required limits, measurement result
validators, and session factories through `store/src/index.js`; they consume failure codes at the
runtime boundary without importing the store's concrete error class. Build/package scripts and
white-box tests import binary admission details from their defining owner modules; they do not turn
those implementation helpers into store API.
Measurement creation has one public function, `openMeasurementSession`; the duplicate
`openTrackBCustodyMeasurementSession` forwarding export had no repository caller and was removed.
Production callers receive session objects only from the binary-verified, purpose-bound factories;
the implementation class remains directly importable only by package-internal tests. Custody result
hash checks use the Track B contracts byte-hash owner instead of a second protocol-local SHA-256 routine.
Project artifacts, measurement artifacts, and custody snapshots retain separate exact-field,
size-limit, and diagnostic policies, while one private `verifyHashedBytes` helper owns their common
base64url canonicalization, byte-length agreement, and raw-byte hash binding. The store protocol does
not absorb Application's separate custody-session trust boundary.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | release workflow → store barrel → TrackBCustodySession → engine/crates/smc-trackb-custody | locked runtime profile, helper binary/receipt hash, descriptors/typed commands | below owner chain | 정상 result 또는 명시적 오류 | `src/index.js`, `src/trackb-custody-session.js`, `../../engine/crates` |

## 실제 흐름

helper receipt+same inode/bytes 검증 → process session → current release/artifact guard → Rust precondition → atomic pointer effect → result → release status/reconcile

## 현재 state·contract·I/O owner

Node session owns process transport; Rust owns restricted filesystem mutation; browser/Studio는 pointer를 직접 쓰지 않는다. target triple·profile·binary pin은 Configuration/contract, timeout은 effect Policy.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 consumer surface | REACHABLE | PARTIAL | PASS: prior snapshot Node 227/227 + aggregate 10/10; current binary/error export trim and consumer-only barrel **NOT_RUN** per request | `src/index.js`, `src/trackb-custody-session.js`, `../../engine/crates` |
| 독립 배포 library | ABSENT_IN_SCOPE | UNKNOWN | root SMC package is private and store has no separate package manifest; repository callers are traced, arbitrary source-tree consumers are outside this contract | root `package.json`, package imports |

## 의존성·findings

**MC1.** 문제 → Rust helper와 Darwin/arm64 runtime이 없는 환경에서는 source/type tests만으로 custody semantics를 보장할 수 없다. 근거 → `src/trackb-custody-session.js::TrackBCustodySession; ../../engine/crates 경로는 제품 root 기준`. 원인 → platform prerequisite와 runtime 증거 공백. 영향 → caller가 작업 완료/승인/복구 가능성을 잘못 해석할 수 있다. 방향 → runtime 조건을 유지하고 pre-rename/post-rename fault injection과 source descriptor 교체 경합을 macOS에서 확인한다. 우회 Node write fallback 금지.

## 실패·취소·재시도·복구와 남은 gap

rename 전 거절은 기존 pointer 보존; rename 후 ACK/cleanup 오류는 reconcile-required. no approval/no exact artifact binding이면 promote 금지. helper crash/timeout을 정상 terminal success로 바꾸지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

## Rust descriptor custody

하위 [smc-trackb-custody](../../engine/crates/smc-trackb-custody/ANALYSIS.md)는 descriptor/lock/CAS의 별도 효과 소유자다. Node bridge는 semantic release나 durable pointer의 두 번째 owner가 아니다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)

## Custody 실행 구성 파트 — 소스 정리 완료, 실행 미검증

`binaryPath` is the sole explicit helper-path option. The duplicate `applicationConfigPath` alias
is removed from both session composition and binary resolution; either entrypoint rejects it.
Default runtime-root/target-triple resolution, exact receipt/hash admission, retained descriptor,
purpose selection and process/session lifecycle remain with their existing owners. No PATH search,
build fallback or approval policy was added. Tests/build/runtime/manifest verification are NOT_RUN.
