# SAC 일반 generation workflow — 구현 분석

## Verdict·책임 경계

RunRequest→task/attempt→selection→AssetPack을 소유한다. caller는 active skill CLI이며 scene-release의 승인과는 다른 state domain이다. 방향 **IMPROVE**. 상위 제품과 별도 제품으로 포장하지 않으며, 이 문서는 독립 material state/effect의 상세를 소유한다. wiring 확신도는 높고 실환경 결과는 아래 범위로 제한한다.

## Module ownership

`mechanical_review.py` owns objective attempt/receipt/output checks, and `visual_review_template.py`
writes a rejected-by-default form for a Human to complete; neither publishes selection or approval.
`output_manifest.py` owns output enumeration, byte hashing, and `result.json` publication.
`png_inspection.py` owns the bounded structural PNG parser and alpha observations; provider
protocol/inspection modules depend on it directly instead of importing the review subsystem. PNG
parsing validates bytes and does not create or approve pixels. The old `review.py` forwarding module
was removed, and its callers import the required owner modules directly.

This source-only caller migration has not been exercised under the current no-verification
instruction; prior PASS rows refer to the preceding module layout.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| init/compile-plan | CLI → init_run/compile_plan | request·authority·dependency/anchor/reference | run identity; staged task commit | task files/compiled marker 또는 error | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/workspace.py |
| ready/begin | CLI/repair → task_status/begin_attempt | dependency selections·reference raw hash·edit target | attempt directory | attempt/prompt/capsule 또는 error | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/workspace.py |
| record/review/select | CLI → execution/mechanical_review/approve_attempt | returned files·mechanical/visual/layout evidence | attempt receipts → selection | candidate acceptance/reject/repair | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/execution.py, mechanical_review.py, publish.py |
| human-approve | CLI → human_approve | whole publish read set·reviewer | public receipt + internal subject marker | bound approval | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/publish.py |
| publish | CLI → publish | readset/receipt/blob rehash·policy | content-addressed blobs; asset-pack final marker | portable pack / stale/conflict error | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/publish.py |

## 실제 흐름

`init staging → authority + generation plan → compile staging → tasks → begin input preflight → attempt → external output → mechanical/visual review → selection → whole-selection human approval → verified blob staging → final readset recheck → AssetPack`.

## 현재 state·contract·I/O owner

request/run은 immutable identity; task_status는 파일들의 projection. selection은 승인 candidate; public receipt는 사람의 주장이고 서명된 신원 증거가 아니다. human-approval-subject는 같은 내용에 대한 승인 무결성 marker, 새 공용 계약이 아니다. no-human-review는 request 정책으로 보존한다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| compile/begin/approval/publication | REACHABLE | PARTIAL | PASS: actual-filesystem14; crash/multiwriter 제외 | tests/test_publication_boundaries.py |
| 실제 생성·품질 | UNKNOWN | UNKNOWN | NOT_RUN: 외부 imagegen/provider | execution receipt boundary |

## 의존성·findings

**AW1.** 문제 → approval에 subject가 없고 실패한 준비가 GENERATING/partial plan을 남겼다. 근거 → workspace/publish 및 focused14. 원인 → validation, allocation, commit, 승인 identity의 혼합. 영향 → 바뀐 자산 승인 재사용·재시도 불가. 방향 → hash-bound marker, staged preparation, exclusive allocation/owned cleanup, readset 재검사로 개선했다. schema/call 이름·optional policy는 보존했다.

## 실패·취소·재시도·복구와 남은 gap

exception cleanup은 host kill/crash의 완전한 복구가 아니다. tasks/publish의 다중 writer lock·multi-file transaction은 남은 결정이다. 이미 복사된 content-addressed blob이 실패 뒤 남을 수 있으나 pack success가 아니며 무조건 삭제하지 않는다. 기존 unbound approval은 재승인한다.

## gap·검증·근거

14개는 원래105개 suite 복원이나 실제 이미지 검증이 아니다. `../../../../SceneMotionCompiler/tests/scene-asset-compiler-asset-boundary.test.js`는 별도 scene-release consumer 증거다. 주 계약·파일의 exact bytes는 original generator manifest로 별도 확인한다.

[상위 ANALYSIS](../../../ANALYSIS.md) · [제품 Current](../../IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../PLAN.md). source 근거 경로는 이 분석 단위 기준이다. PASS는 표시한 검사에만 적용하며 PUBLIC_LIBRARY/REACHABLE은 실행 성공을 뜻하지 않는다.

## 파일 발행 파트 — 소스 정리 완료, 실행 미검증

`util.atomic_bytes`가 temporary allocation·write/fsync·replacement·owned-temp cleanup의 단일
구현이다. `atomic_json`과 `atomic_text`는 각 format을 UTF-8 bytes로 인코딩해 이를 호출한다.
`exclusive_json`은 별도 no-replace claim이며 partial claim도 보존해야 하므로 atomic replacement와
합치지 않는다. 효과가 없는 `except Exception: raise`만 제거했다. Asset release의 safe-file
path parts는 한 번 해석하고 각 parent/leaf의 기존 type·symlink policy를 유지한다.
JSON/text artifact shape와 error tokens는 바꾸지 않았고 tests/runtime/manifest 검증은 NOT_RUN이다.
