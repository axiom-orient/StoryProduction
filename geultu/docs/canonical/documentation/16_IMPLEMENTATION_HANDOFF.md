# 16. Implementation Handoff — target backlog

> **STATUS: TARGET / HISTORICAL PLAN.** 이 문서는 구현 순서를 보존하는 backlog다.
> 아래의 목표·이전 격차·단계 번호는 현재 구현을 설명하는 정본이 아니다. 현재
> 상태와 날짜가 붙은 실행 근거는 [Geultu Current](../../IMPLEMENTATION_STATUS.md)와
> [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)에 둔다. 18장은 2026-08-27 snapshot이다.

## 1. 증거 우선순위와 경로 규칙

구현 여부를 판정할 때의 우선순위는 고정한다.

1. **동작·필드·허용 값:** 실제 source와 tests, 그리고 저장소 루트 기준
   `docs/canonical/contracts/*.schema.json`.
2. **현재 상태:** [Geultu Current](../../IMPLEMENTATION_STATUS.md)의 날짜가 붙은 gate 결과와
   [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)의 source 대조. 기록 시점이 불일치하면 실제 source와 gate를 다시 확인한다.
3. **설계·목표:** 이 문서와 01–16의 설계 서술.
4. **역사 참고:** 21–29 참고 문서. 특히 26장은 archived/non-authoritative다.

`docs/canonical/documentation/`에서 schema를 가리킬 때는 `../contracts/<name>.schema.json`,
저장소 다른 위치나 명령 설명에서는 `docs/canonical/contracts/<name>.schema.json`을
사용한다. canonical root를 생략한 경로 표기는 문맥을 오해하게 하므로 새 문서에서
사용하지 않는다.

## 2. 2026-08-27에 측정한 구현 slice — `[HISTORICAL SNAPSHOT]`

2026-08-27 측정에서 다음 항목을 실제 source와 tests에 연결된 것으로 기록했다.

| 항목 | 현재 surface | 상태 |
|---|---|---|
| flat story export | `story.export` → `GeultuStoryPackage v1`, `np_` JCS identity | `REACHABLE` |
| reviewed synopsis handoff | `synopsis.export_reviewed_handoff` → raw `synopsis.md` + approval-bound handoff JSON | `REACHABLE` |
| validator/decision receipt | `review.run`, `proof.verify`, candidate/changeset approval | `REACHABLE` |
| local visual admission | `visual.admit_package` → `VisualRunState` + PASS receipt | `REACHABLE`, admission only |
| canonical state | `Workspace` journal/snapshot/index/recovery와 project lock | `REACHABLE` |
| full T00–T33 executor | 현재 reducer/executor에 전체 stage graph 없음 | `UNWIRED` |
| full NVC S00–S12 executor | geultu에는 이미지·영상 provider와 render/composition 없음 | `UNWIRED` |

`story.export`는 `synopsis.export_reviewed_handoff`의 별칭이 아니며, 어느 명령도
NarrativeCourier release artifact를 geultu가 대신 발행한다는 뜻이 아니다.

## 3. 보존된 구현 backlog — `[TARGET]`

아래 순서는 과거의 gap을 현재 작업 단위로 보존한다. `[x]`는 해당 아이디어의
현재 geultu slice가 구현되었다는 뜻이고, downstream 전체가 완성되었다는 뜻이
아니다.

1. **IMPL-001 — flat story contract `[x / partial scope]`**
   `docs/canonical/contracts/geultu-story-package.schema.json`을 유일한
   `GeultuStoryPackage v1` payload schema로 사용하고 legacy envelope를 fail-closed
   한다. `crates/geultu/src/story_package.rs`와 contract fixtures/tests가 이 범위를 구현한다.
2. **IMPL-002 — identity/digest `[x / partial scope]`**
   JCS identity utility와 golden vector를 사용한다. package `np_` identity,
   text-master raw digest, reviewed synopsis raw digest가 현재 구현되어 있다.
3. **IMPL-003 — validation authority `[x / local scope]`**
   validator receipt와 human decision을 분리한다. 현재 `review.run`,
   `proof.verify`, `changeset.approve`, visual admission receipt가 이 범위다.
4. **IMPL-004 — common commit `[TARGET]`**
   text/NVC의 named output map과 state/output/receipt atomic commit을 공통
   protocol로 통합한다. 현재 geultu의 `Workspace::transact`와 visual admission은
   일부 상태 전이만 제공하며, downstream 공통 output commit은 아직 없다.
5. **IMPL-005 — frozen input/state binding `[partial]`**
   `TextRunState`의 frozen inputs와 revision/pivot budget 구조는 존재한다.
   모든 generation과 외부 stage가 이 frozen state를 의무적으로 소비하는 전체
   경로는 아직 구현되지 않았다.
6. **IMPL-006 — lineage `[partial]`**
   `ArtifactRef`, package source refs, receipt subject/binding digest는 구현되어
   있다. NarrativeCourier release부터 최종 visual output까지의 전체 lineage는
   후속 저장소 계약에서 연결해야 한다.
7. **IMPL-007 — visual admission/recovery `[partial]`**
   `visual.admit_package`의 package identity·route·run-state preflight와 journal
   receipt는 구현되어 있다. NVC 실행, external effect recovery, cross-object
   receipt binding은 target이다.
8. **IMPL-008 — verification `[local complete / cross-repo target]`**
   canonical schema/fixture, CLI/MCP/Studio smoke와 local admission tests는
   검증된다. 실제 geultu→NarrativeCourier→downstream visual end-to-end와 장편
   성능 benchmark는 별도 검증이 필요하다.

## 4. 인계 조건

다음 executor가 이 backlog를 구현할 때는 각 작업마다 다음을 남긴다.

- producer·consumer·schema·relative path를 하나씩 명시한다.
- source와 contract의 실제 경로를 `docs/canonical/contracts/` 기준으로 적는다.
- 새 public field나 command는 schema, serializer/parser, tests, docs를 동시에
  갱신한다.
- 실패 시 입력·저널·출력의 변경 여부와 recovery command를 기록한다.
- target 설계의 PASS는 실제 실행 증거가 있을 때만 current 상태로 승격한다.

현재 구현에서 해결되지 않은 cross-object receipt, TOCTOU, 외부 process custody,
실제 세 저장소 end-to-end는 다음 bounded outcome으로 분리한다. 이 문서는 그
작업이 완료되었다고 주장하지 않는다.
