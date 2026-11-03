# SceneAssetCompiler — 아키텍처

## 경계·책임·의존 방향

실행 root는 `.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py`이며 같은 Python package 안의 다음 material unit을 분리한다. 이는 새 배포 package를 요구하지 않는다.

| 단위 | authoritative owner | 교환 경계 |
|---|---|---|
| `workspace/execution/review/repair/publish` | RunRequest, task/attempt, selection, 일반 AssetPack 승인/발행 | prompt/capsule→외부 artifact→receipt |
| `asset_release` | SMC projection binding, asset pack/audit/binding, subjectHash 승인, handoff | SMC producer/consumer schemas |
| `visual_reference` | reference/keyframe candidate pack, bound human approval, derived lookup index | 현재 SMC v3 capsule/graph·실제 images·SMC motion consumer |
| `scene_plan/scene_units/scene_locks/psd/psd_validation/psd_profile/reconstruct` | generation-unit graph, placeholder/layer/asset locks, PSD 사후검사·수정 지시 | 외부 imagegen/PSD와 검토 증거 |
| `scene_provider` | restricted PNG dispatch, typed process/inspection receipt·custody | exact provider/PerfectPixel protocol |
| `native/src` → `sac-animatic` | NVC packet 사본 수용·실제 PNG pack 검사·별도 review·불변 발행 | producer schema copy만 소비; NVC source import 없음 |
| active skill/templates/schemas | 작업 계약·선택 정책 | docs mirror는 runtime이 아님 |

## 실행·상태 모델

Packaging preservation은 `packaging_preservation.py`, visual review는 `review_contracts.py`,
기계 검사는 `mechanical_review.py`가 소유한다. Provider identity/preflight/attempt/protocol/process/
inspection은 각 내부 owner에 있고 public facade가 실제 dispatch evidence와 실행을 조합한다.

일반 workflow는 `입력/authority → 전체 plan preflight → staged tasks → attempt preparation → external output → review → selection → Human policy → publish`다. 준비 실패는 생성 중이라는 상태로 남기지 않아야 한다. 상태 projection은 durable 파일의 검증 결과로 계산하며 status 문자열을 두 번째 원본으로 저장하지 않는다.

일반 Human 승인과 scene-release 승인은 다른 계약이다. 일반 public approval DTO를 임의 확장하지 않고 내부 marker로 **현재 selected read set + exact approval receipt**를 결합한다. scene-release는 기존 subjectHash 계약을 따른다. 변한 내용은 과거 승인을 재사용하지 않는다. 암호학적 서명자 인증을 제공한다고 주장하지 않는다.

## persistence·transaction·권한·복구

원본 request/run과 approved 입력은 identity/hash로 고정한다. staging은 자신이 만든 파일만 정리하고 existing task/output을 강제로 삭제하지 않는다. content-addressed artifact는 복사 뒤 다시 해싱하고 commit 직전 read set을 검증한다. 파일 집합 전체 crash transaction과 multiwriter exclusion의 실제 범위는 Current에 남긴다.

provider는 immutable dispatch 이후 최종 결과를 확인할 수 없으면 UNKNOWN을 남기며 자동 retry/downgrade하지 않는다. `misegen.provider/2` lane은 reference-free PNG에 한정한다. reference/edit/cutout/PSD를 이 경로에 조용히 우회시키지 않는다. native pixel/layer/Path/PSD 제작은 외부 imagegen이며 local 대체 writer를 추가하지 않는다.

## 값과 확장

Animatic 입력은 NVC plan/review/packet consumer schema에 pin하고 SAC assets/review producer schema는
`docs/contracts/`에 둔다. JSON/PNG를 bounded regular-file reader로 capture해 hash·alpha·coverage를
확인하고 `animatic/` 아래에 같은 bytes만 불변 발행한다. 동일 review는 첫 timestamp를 재사용하며
다른 결정은 충돌한다. ContentHash·provenance는 신원 인증이나 Human final approval이 아니다.

Rust animatic package는 SAC 내부의 독립 실행 경로다. `contracts`는 pinned producer 형식과
canonical identity, `png`는 CRC·bounded inflate·scanline·실제 alpha, `files`는 anchored
directory FD와 no-follow 불변 발행, `lib`는 submit/inspect/review 흐름을 소유한다.
Integration·NVC 구현 crate를 의존하지 않으며 기존 general asset workflow의 authoritative
state나 provider dispatch를 가져오지 않는다.

schema/source-kind/profile 이름은 Constant, human_final_review/retry ceiling/timeout은 Policy, binary/root/runtime lock은 Configuration, run/task/attempt/lock/approval은 State다. 사용자 UI Settings·Feature Flag·공유 DB는 필요가 확인되지 않으면 추가하지 않는다.

[active skill](../.agents/skills/scene-asset-compiler/SKILL.md) · [계약](SPEC.md) · [운영](OPERATIONS.md) · [release 의무](RELEASE_GATES.md)
