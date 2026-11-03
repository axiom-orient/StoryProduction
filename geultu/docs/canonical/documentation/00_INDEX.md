# 00. Canonical Documentation Index

## 설계 목표 정체성

이 package의 설계는 Geultu를 승인된 창작 의도·canon·style·genre 계약에서 검증 가능한 텍스트 산출물과 선택적 NVC 시각 제작 입력을 준비하는 로컬 우선 제작 orchestration system으로 모델링한다. 실제 현재 기능과 owner 경계는 package 목표와 별도로 [Geultu Current](../../IMPLEMENTATION_STATUS.md)에서 확인한다.

## 현재 계약 읽기 순서

01–20은 package 설계·자산·검증 자료를 찾기 위한 읽기 순서다. 01–16은 책임·경계·목표 설계를
소유하고, 17장은 2026-08-23 채택 기록, 18장은 2026-08-27 source snapshot을 보존한다.
18장은 최신 Current가 아니다. 실제 제품 상태와 날짜가 붙은 gate 결과는
[Geultu Current](../../IMPLEMENTATION_STATUS.md), 모듈·caller 연결은
[저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)를 따른다. 설계가 현재 코드와 충돌하면
실제 source·tests·contracts와 해당 snapshot의 재현 가능한 실행 근거로 판정한다.

| 순서 | 문서 | 책임 |
|---:|---|---|
| 1 | `01_PRODUCT_SCOPE.md` | 사용자, 문제, 범위, 비목표 |
| 2 | `02_ARCHITECTURE_BOUNDARIES.md` | 구성요소와 의존 방향 |
| 3 | `03_AUTHORITY_STATE_SSOT.md` | 권한, 상태 소유자, SSOT |
| 4 | `04_EXECUTION_PROTOCOL.md` | admission부터 commit까지 |
| 5 | `05_TEXT_PIPELINE.md` | T00–T33 의미와 경계 |
| 6 | `06_SCRIPTFORGE.md` | ScriptForge 실행기 계약 |
| 7 | `07_REFINEMENT_QUALITY.md` | patch와 품질 검증 |
| 8 | `08_T33_GEULTU_STORY_PACKAGE.md` | T33 GeultuStoryPackage 계약 |
| 9 | `09_VISUAL_PIPELINE.md` | 현재 geultu admission과 downstream NVC 목표 설계 (상태 구분 필수) |
| 10 | `10_ARTIFACT_IDENTITY_DIGEST.md` | identity, digest, provenance |
| 11 | `11_VALIDATION_DECISION_RECEIPT.md` | validator와 domain decision 분리 |
| 12 | `12_FAILURE_RECOVERY_IDEMPOTENCY.md` | 실패·재시도·복구 |
| 13 | `13_SECURITY_TRUST.md` | 권한·신뢰·외부 효과 |
| 14 | `14_SCHEMA_VERSIONING.md` | schema와 호환성 정책 |
| 15 | `15_VERIFICATION_RELEASE.md` | 구현·릴리스 gate |
| 16 | `16_IMPLEMENTATION_HANDOFF.md` | Target backlog와 historical implementation plan (현재 status는 Geultu Current) |
| 17 | `17_ADOPTION_BASELINE.md` | 2026-08-23 역사적 채택/기각 기준선 (현재 상태 아님) |
| 18 | `18_CURRENT_STATE.md` | 2026-08-27 역사적 source snapshot과 당시 격차 (최신 Current 아님) |
| 19 | `19_DATA_ASSET_REGISTRY.md` | 이 package가 기록한 데이터 자산과 해시 |
| 20 | `20_VERIFICATION_GATES.md` | 로컬 검증 게이트 명령과 기대 결과 |

## 보조 참고 자료 (현재 계약 읽기 경로 제외)

아래 문서는 상세 참고 자료다. 현재 구현·입출력·공개 계약을 판정할 때는 위 package 문서와
실제 `docs/canonical/contracts/`, source, tests를 대조한다. 참고 자료와 구현이 충돌하면
실제 source·tests, [Geultu Current](../../IMPLEMENTATION_STATUS.md)의 날짜별 evidence를 따른다.

| 참고 | 문서 | 지위 |
|---:|---|---|
| 21 | `21_TEXT_PIPELINE_STAGES_T00_T33.md` | T00–T33 단계별 설계·계약 참고 자료 |
| 22 | `22_NATIVE_TEXT_CRAFT_ADAPTERS.md` | 텍스트 craft 어댑터 설계 참고 자료 |
| 23 | `23_SCRIPTFORGE_FIDELITY_AND_INTEGRATION.md` | ScriptForge 보존·fidelity 참고 자료 |
| 24 | `24_SCRIPTFORGE_26_NODE_CONTRACTS.md` | ScriptForge 노드 토폴로지 참고 자료 |
| 25 | `25_REFINEMENT_REVISION_AND_PIVOT.md` | refinement·revision·pivot 참고 자료 |
| 26 | `26_NVC_VISUAL_COMPILER_S00_S12.md` | **ARCHIVED / NON-AUTHORITATIVE** — 역사적 S00–S12 참고 자료; 현재 계약·구현 읽기에서 제외 |
| 27 | `27_VISUAL_CHECKPOINTS_V00_V27.md` | 시각 품질 checkpoint 참고 자료 |
| 28 | `28_FAILURE_CODE_CATALOG.md` | 확장 failure code 참고 자료 |
| 29 | `29_LICENSE_RIGHTS_AND_DISTRIBUTION.md` | 라이선스·배포 참고 자료 |

읽기 순서 주의: 01–16은 계약 설계(무엇이어야 하는가), 17장은 역사적 채택 기록,
18장은 2026-08-27 source snapshot이다. 현재 제품 source와 실행 qualification은
[Geultu Current](../../IMPLEMENTATION_STATUS.md)에서 확인한다. 19–20은 package 자산·검증
자료이며, 21–29는 현재 계약 읽기 경로에 자동으로 포함되지 않는 참고 자료다. 특히 26장은 보존용 역사 문서이며 현재
구 NVC package 입력·실행 계약으로 사용하지 않는다.

## 정본 규칙

1. JSON 필드와 허용 값은 저장소 루트 기준 `docs/canonical/contracts/*.schema.json`이 유일한 정본이다. `docs/canonical/documentation/`에서 상대 경로를 쓸 때는 `../contracts/*.schema.json`을 사용한다.
2. 문서는 책임, 이유, 순서와 불변조건만 소유한다.
3. runtime state가 실행 현재값의 유일한 정본이다.
4. append-only receipt/event는 과거 증거이며 현재 상태의 대체 정본이 아니다.
5. legacy envelope, caller-owned pass flag, body 내부 self-reference는 금지한다.
