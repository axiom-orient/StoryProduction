# SAC scene locks·PSD postflight — 구현 분석

## Verdict·책임 경계

장면별 GenerationUnit/lock과 외부 PSD 구조의 사후검사 owner다. caller는 scene-plan/lock/seal/PSD/Doctor CLI와 active skill이다. 이미지·PSD writer가 아니다. 방향 **IMPROVE**(증거 복구). 상위 제품과 별도 제품으로 포장하지 않으며, 이 문서는 독립 material state/effect의 상세를 소유한다. wiring 확신도는 높고 실환경 결과는 아래 범위로 제한한다.

## Module ownership

`scene_plan.py` owns SceneProductionPlan semantics, budget projection, and layout-slot access.
`scene_units.py` owns scene staging, GenerationUnit records, and prompt capsules. `scene_files.py`
owns scene-local path checks and review/lock artifact reads; the shared `util.sha256_file` helper
owns raw file digests. `scene_locks.py` owns the
lock order, prerequisite dependency checks, immutable lock writes, placeholder/asset seals, and
PSD-entry admission. The former `scene.py` forwarding façade was removed; CLI callers import these
owners directly. `psd.py` owns the bounded binary parser, `psd_validation.py` owns generated-PSD
policy findings, and `psd_profile.py` owns profile resolution. The parser no longer re-exports
validators or participates in an import cycle. All are read-only postflight owners; no local pixel
or PSD writer is introduced.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| plan/init/units | CLI → validate_scene_plan/init_scene/compile_units | scene schema·budget·required layers | plan owner + unit graph/capsule owner | generation units or error | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_plan.py, scene_units.py |
| layout/placeholder/layer lock | human reviewed artifact → create_lock/lock_placeholder/lock_layer | artifact and review hash·prerequisites | `scene_locks.py` + `scene_files.py` | lock or stale/blocked | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_locks.py, scene_files.py |
| asset/composite seal | seal_placeholders/seal_assets/validate_psd_entry | every prerequisite/layer bytes | aggregate seal | PSD admission or reject | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_locks.py |
| PSD postflight | CLI/review → validate_generated_psd | actual PSD signature·canvas·ICC·layers·paths·mask policy | read-only parse + findings | structural PASS/FAIL, separate unverified axes | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/psd.py, psd_validation.py, psd_profile.py |
| selective repair | Doctor/reconstruction → make_repair | failed properties, approved raster, retry receipt/attestation | new bounded attempt | repair instructions or STOP | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/repair.py, reconstruct.py |

## 실제 흐름

`SceneProductionPlan → composition/target/layout/placeholder locks → semantic layer units → asset seal → external imagegen PSD → actual bytes postflight → Doctor/selective repair → composite lock`. PSD 없는 호출은 `PREFERRED_OUTPUT_MISSING`이고 PSD 전체 capability 부재라는 결론은 아니다.

## 현재 state·contract·I/O owner

plan/profile/lock semantics는 active skill+schemas가 소유한다. temporary Work Path, Named Saved Path, document clipping path는 구별한다. ordinary alpha와 mask/Smart Object도 같지 않다. geometry/ICC structure와 actual pixel/path-overlap/render-back evidence는 별개다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| unit/lock/PSD parser | REACHABLE | PARTIAL | NOT_RUN: actual passing PSD/render-back; source wiring 확인 | scene_plan.py, scene_units.py, scene_locks.py, psd.py |
| real PSD regression gate | REACHABLE | PARTIAL | FAIL: original PROVENANCE/PSD bytes 없음 | tools/verify_release.py G09 |
| local PSD/pixel generation | ABSENT_IN_SCOPE | UNKNOWN | N/A: 확정 비목표 | active skill |

## 의존성·findings

**AD1.** 문제 → historical PSD PASS와 실제 입력 부재. 근거 → `tests/fixtures/legacy-psd/PROVENANCE.json` 부재, G09. 원인 → input source package에 golden evidence 미포함. 영향 → parser 회귀·미지원 여부·품질 모두 과장 가능. 방향 → gate는 보존하고 원본 해시로 fixtures를 복구해야 한다. 임의 PSD를 만들어 과거 golden으로 등록하지 않는다.

## 실패·취소·재시도·복구와 남은 gap

`SUBMITTER_ATTESTED` comparison은 선언의 내부 일관성·현재 파일 binding이며 인증된 provenance나 독립 렌더링이 아니다. approved raster와 exact retry lock을 요구하는 PSD-only retry는 최대1회; 두 번째 miss는 STOP. fresh PSD/decoded alpha/path alignment는 NOT_RUN/UNKNOWN이다.

## gap·검증·근거

복구 후보의 이름과 보고된 SHA-256은 [historical ledger](../../../experiments/psd-profile-regression/legacy-psd-sha256.txt)가 단독 소유한다. 두 원본 PSD와 `PROVENANCE.json`은 현재 저장소에 없다. ledger의 해시는 과거 기록을 가리키며 현재 읽은 bytes를 검증하지 않는다. expected findings/profile/provenance까지 원본이 함께 복구되기 전에는 G09 evidence나 passing fixture로 취급하지 않는다.

[상위 ANALYSIS](../../../ANALYSIS.md) · [제품 Current](../../IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../PLAN.md). source 근거 경로는 이 분석 단위 기준이다. PASS는 표시한 검사에만 적용하며 PUBLIC_LIBRARY/REACHABLE은 실행 성공을 뜻하지 않는다.
