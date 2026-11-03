# 09. Visual Pipeline — 현재 경계와 downstream 목표

> **STATUS: SPLIT.** 이 문서는 현재 geultu의 visual admission 경계와, 아직
> geultu에 구현되지 않은 downstream NVC pipeline 설계를 분리해 기록한다.
> 현재 동작의 정본은 저장소 루트 기준 `docs/canonical/contracts/*.schema.json`,
> `crates/geultu/src/visual_run.rs`, 관련 테스트다. 이 문서에서 `Target`로
> 표시된 항목은 구현 완료를 뜻하지 않는다.

## 1. 현재 geultu 경계 — `[CURRENT / REACHABLE]`

현재 geultu는 이미지·영상 생성기나 S00–S12 executor가 아니다. 실제 공개 명령은
`Application::call("visual.admit_package", arguments)`이며, 다음 admission만
수행한다.

| 단계 | 현재 동작 | 정본 |
|---|---|---|
| 입력 | `project_id`, `run_id`, `stage`, `route_admission`과 필수 inline `GeultuStoryPackage`; unknown fields와 bare `package_id`는 거절 | `crates/geultu/src/visual_run.rs::AdmitPackageInput` |
| body 확인 | inline body만 사용하며 별도 reference resolver는 없음 | `crates/geultu/src/application/requests/visual.rs` |
| package gate | schema/값 검증, `np_` identity 재계산, 현재 Geultu export projection과 source refs 일치 | `docs/canonical/contracts/geultu-story-package.schema.json`, `crates/geultu/src/story_package.rs` |
| route gate | `route_id = NVC`, `command_id = visual.admit_package`, route budget/refs/selector shape | `docs/canonical/contracts/route-admission.schema.json`, `crates/geultu/src/visual_run.rs` |
| run gate | stage `S00`–`S12`, initial READY/revision 0, output/effect 없음과 admission PASS receipt | `docs/canonical/contracts/visual-run-state.schema.json` |
| 성공 출력 | Project revision CAS 아래 run/receipt를 기록하고 실제 stored binding 확인 | `crates/geultu/src/domain.rs`, `crates/geultu/src/store.rs`, `crates/geultu/tests/visual_run.rs` |

현재 admission은 route의 `required_receipts`와 `input_refs`가 package source와
동일한지까지 project-aware하게 묶지 않는다. 따라서 "route가 schema-valid"라는
사실과 "모든 upstream receipt가 실제 PASS이고 package와 일치한다"는 목표를
혼동하지 않는다. 이 검사는 downstream release handoff 전에 보강해야 한다.

현재 `VisualRunState.narrative_package`는 geultu flat story package를 가리키는
`ArtifactRef` 필드명이다. 이것은 NarrativeCourier가 별도로 발행하는 screenplay
release object의 body가 아니다.

입장 receipt는 로컬 preflight 결과이며 upstream receipt 승인이나 NVC execution 완료가 아니다.

## 2. geultu에서 실제로 연결되는 입력·출력

```text
Project scenes/text
      │
      ├─ story.export ──> flat GeultuStoryPackage v1 (응답, pure read)
      │                         │
      │                         └─ visual.admit_package ──> VisualRunState + PASS receipt
      │
      └─ synopsis.export_reviewed_handoff
            └─ synopsis.md + reviewed-synopsis-handoff.v1.json ──> NarrativeCourier
```

`story.export`는 geultu 내부 flat package projection이고, NarrativeCourier로
나가는 주 STS 경계는 `synopsis.export_reviewed_handoff`다. 두 경로 모두 서로의
출력을 암묵적으로 대체하지 않는다.

## 3. downstream NVC 목표 설계 — `[TARGET / NOT IMPLEMENTED HERE]`

아래 설계는 NarrativeCourier가 발행한 release와 후속 visual compiler가 처리할
수 있는 목표 토폴로지다. 현재 geultu runtime command나 현재 package schema로
읽지 않는다.

### 3.1 admission과 stage output

Target NVC route는 Completed release package reference와 upstream validation
receipt selector를 요구한다. 각 S stage는 named output map을 반환하고, stage
output 개수 자체를 API shape으로 사용하지 않는다. 정확한 field·enum·artifact
identity는 향후 NVC 저장소의 machine-readable catalog가 소유한다.

예시 target output:

- S01: `scene_plan`, `visual_constraints`
- S10: `render_manifest`, `quality_report`, 선택적 `repair_plan`

이 예시는 geultu current output이 아니며, 현재 `visual.admit_package`가 이
outputs를 만들거나 검증한다고 해석하면 안 된다.

### 3.2 target 실패·복구 의미

- validation failure: artifact commit 없음
- provider failure: 명시된 retry budget이 허용할 때만 동일 admitted input으로 재시도
- partial external effect: effect receipt를 기록하고 `UNKNOWN_OUTCOME` 또는
  `RECOVERY_REQUIRED`로 전이
- repairable quality failure: 기존 body를 수정하지 않고 새 repair command만 생성

이 의미는 NVC target execution contract다. geultu current admission의 실제
failure surface는 invalid input, not found, conflict, corrupt, busy와 journal/
snapshot 오류이며, 위 target executor 상태를 현재 geultu가 생성한다고 주장하지
않는다.

## 4. 경계별 책임과 금지

| 경계 | 소유 | 금지 |
|---|---|---|
| geultu `story.export` | canonical project에서 flat story package projection | NVC release package 발행 |
| geultu `visual.admit_package` | package/route/run-state/receipt admission과 journal | 이미지 생성, 렌더, composition, S00–S12 실행 |
| NarrativeCourier | reviewed synopsis를 STS screenplay와 Completed release로 고도화 | geultu canonical project state 직접 수정 |
| downstream NVC compiler `[TARGET]` | release를 scene/shot/asset/render output으로 컴파일 | 승인 없는 upstream body 변경 |

## 5. 구현 순서

1. NarrativeCourier release schema와 receipt selector를 먼저 확정한다.
2. geultu admission이 필요로 하는 project/package/receipt cross-object binding을
   명시하고 회귀 테스트한다.
3. downstream NVC가 consumed release hash와 graph generation을 원자적으로
   기록한다.
4. 그 뒤에 stage output map, provider effect receipt, repair/recovery를 구현한다.

현재 문서만으로 3–4단계가 완료되었다고 판정하지 않는다. 현재 source와 qualification은
[Geultu Current](../../IMPLEMENTATION_STATUS.md)와 [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)가 소유한다. `18_CURRENT_STATE.md`는 2026-08-27 snapshot을 보존한다.
