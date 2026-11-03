# SAC scene-release — 구현 분석

## Verdict·책임 경계

SMC durable projection을 입력으로 asset pack/audit/binding/승인/handoff를 소유한다. caller는 scene-release CLI와 SMC consumer다. 방향 **MAINTAIN**. 상위 제품과 별도 제품으로 포장하지 않으며, 이 문서는 독립 material state/effect의 상세를 소유한다. wiring 확신도는 높고 실환경 결과는 아래 범위로 제한한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| projection import | init_run → `asset_release_projection_import.import_nvc_released_scene_projection` | SMC receipt raw hash·root·semantic graph coverage | imported source binding | authority artifacts 또는 integrity error | `workspace.py`, `asset_release.py`, `asset_release_projection_import.py` |
| prepare | CLI → `asset_release.prepare_scene_asset_release` | release request·rights·source/runtime file hash·requirements | prepared pack/audit/binding | approval subject candidate | `asset_release.py`, `asset_release_builder.py` + contracts/files/projection owners |
| approve | CLI human → `asset_release.approve_scene_asset_release` | current prepared subjects·reviewer | scene asset approval | subjectHash-bound receipt | `asset_release.py`, `asset_release_contracts.py` |
| validate/publish | CLI → `validate_scene_asset_release`/`publish_scene_assets` | rehash source·artifacts·coverage·approval | handoff publication | scene-motion-asset-handoff/v2 | `asset_release_validation.py`, `asset_release.py` + contracts/files/projection owners |

## 실제 흐름

`SMC receipt → exact copy/projection validation → requirements(graph) → asset request → prepare(pack/audit/binding) → human approve(subjectHash) → validate current bytes → publish handoff → SMC verify before compose`.

## 모듈 경계

`asset_release.py`는 prepare/approve/publish 순서를 조합한다. Read-only `_load_prepared`와
`validate_scene_asset_release`는 `asset_release_validation.py`가 소유한다. `_validate_release_request`와
`_relative_path`는 filesystem을 건드리지 않는 SAC-owned contract 규칙이므로
`asset_release_contracts.py`가 소유한다. `_verify_file_ref`는 contained regular-file read, raw
hash/size, byte signature를 결속하므로 `asset_release_files.py`가 소유한다.
`asset_release_projection_contracts.py`는 producer-owned SMC receipt, graph, semantic manifest와 cinematic-source binding을 pure value contracts로 검증한다. `asset_release_projection_import.py`는 source root/file read, raw-byte hash comparison, 검증한 bytes의 exact copy만 소유한다. `workspace.py`와 `asset_release.py`는 import owner를 직접 사용하며 `narrative.py`와 `asset_release_projection.py` 재수출 facade는 제거했다. SAC가 NVC/SMC authority를 새로 확정하지 않는다.
`asset_release_builder.py`는 요청·projection·verified file refs를 받아 pack/audit/binding 후보를
조립한다. Run request project ID는 files owner의 safe-path resolver를 거쳐 읽고, lifecycle
coordinator가 builder 결과를 승인 가능한 단계로 게시한다. Validation facade는 approval subject와
current source/assets를 함께 확인한다. 새 public module path나 cross-component import를 만들지 않는다.

CLI는 prepare/approve/publish effect를 `asset_release.py`, read-only validate를
`asset_release_validation.py`, 예외 타입을 `asset_release_contracts.py`에서 직접 가져온다.
따라서 lifecycle module의 내부 import가 validator나 오류 계약의 소유권을 대신하지 않는다.

최신 preparation-builder와 read-only validation owner 분리는 source-only다. 앞서 기록한
publication-boundary 및 owner-chain PASS는 이 이동 직전 source layout의 증거이며, 현재 module
placement의 runtime 보존 여부는 아직 확인하지 않았다.

Projection validation/import도 pure receipt·SceneGraph·semantic/cinematic checks와 filesystem exact-copy effect로 분리했다. `_validate_handoff`는 SAC output contract인 `asset_release_contracts.py`로 옮겼고 production callers use projection owners directly. `asset_release.py`, builder/validation, visual-reference, and intake use owner modules. This source-only clean break was not tested or verified.

## 현재 state·contract·I/O owner

NVC is upstream semantic owner; SMC is projection producer; SAC owns asset selection/rights declarations/release. 일반 AssetPack의 approval marker와 다른 public DTO다. source kind `SMC_NARRATIVE_SCENE_PROJECTION`, schema versions/hash algorithms는 contract constants. canonicalizer는 이 계약의 제한된 JSON domain을 따른다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| scene release ingress→consumer | REACHABLE | SATISFIED | PASS: SMC boundary8, synthetic pixels/PSD | asset_release.py, SMC boundary tests |
| 실제 rights·human identity·픽셀 품질 | UNKNOWN | UNKNOWN | NOT_RUN: 선언과 실제 권한/사람을 동일시하지 않음 | approval/rights fields |

## 의존성·findings

**AR1.** 문제 → 일반 승인 결함을 이 경계에도 적용하면 기존 bound subject owner를 중복 생성하게 된다. 근거 → `_load_prepared`, `approve_scene_asset_release`, `validate_scene_asset_release`. 원인 → 유사 파일명을 같은 workflow로 오인. 영향 → 중복 승인·breaking schema. 방향 → 이 경계의 기존 subjectHash와 raw-byte validation을 유지하고 일반 workflow만 고쳤다.

**AR2 · 승인 입력 snapshot 게시 — 정상 경로 실행 확인, race injection 미실행.** 문제 → projection을 parse/hash 검증한 뒤 source path를 다시 열면 parsed object와 run의 stored bytes가 달라질 수 있었다. 개선 → 최초 검증한 raw bytes/hash를 그대로 복사하고, destination 경로에는 같은 bytes만 재사용하며 충돌 bytes는 덮지 않는다. 모듈 분리 뒤 14 boundary tests와 public owner-chain E2E가 통과했으며, verify/copy 사이 source mutation injection은 아직 직접 실행하지 않았다. 새 output format이나 cross-component DTO는 없다.

## 실패·취소·재시도·복구와 남은 gap

해시 검사는 multi-writer lock/host-crash transaction을 대신하지 않는다. 예외·stale/hash/rights 오류는 명시 거절한다. prepared와 approved를 SMC 최종 release 승인으로 해석하지 않는다.

## gap·검증·근거

public CLI boundary8은 source-kind, subject drift, runtime asset/contract 부정 사례와 최종 HTML 소비를 확인했다. 전체 geultu→NVC 실행 또는 실제 PSD decode/render-back 증거가 아니다.

[상위 ANALYSIS](../../../ANALYSIS.md) · [제품 Current](../../IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../PLAN.md). source 근거 경로는 이 분석 단위 기준이다. PASS는 표시한 검사에만 적용하며 PUBLIC_LIBRARY/REACHABLE은 실행 성공을 뜻하지 않는다.
