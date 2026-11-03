# StoryProduction — 아키텍처

기준: Story native 실행 계층의 owner/public-entrypoint 경계. Integration의 제품 계약은 [SPEC](../integration/docs/SPEC.md), 구현 경계는 [아키텍처](../integration/docs/ARCHITECTURE.md), 최신 qualification은 [구현 현황](../integration/docs/IMPLEMENTATION_STATUS.md), 저장소 전반의 owner 불확실성은 [Current](IMPLEMENTATION_STATUS.md)에 둔다.

## 공개 진입점과 의존 방향

```text
사용자·에이전트 → storyctl text
  ├─ geultu CLI → Application → authoring/domain → journal
  ├─ reviewed synopsis 인계 → NVC STS CLI → Human gates → Completed release
  ├─ NVC storyboard CLI → task/candidate/review → bundle
  ├─ integration export/package → 불변 파일과 완료 marker
  └─ 선택 animatic → NVC 연출 export → SAC PNG pack/review → SMC render candidate
```

기본 text dispatch는 [mod.rs](../integration/crates/storyctl/src/cli/mod.rs), 공개 owner 조합은 [text.rs](../integration/crates/storyctl/src/workflows/text.rs)에 있다. integration은 sibling 구현을 import하거나 domain state를 직접 변경하는 대신 공개 entrypoint와 immutable artifact를 사용한다.

## 상태의 소유자

| 상태·의미 | 정본 owner | 다른 컴포넌트가 하는 일 |
|---|---|---|
| 원문·브리프·글투·스토리·시놉시스·집필 후보 | geultu | 현재 snapshot 조회와 검토된 시놉시스 인계 |
| Source IR·씬·대본·semantic occurrence·Completed release | NVC STS | producer artifact 검사·투영 |
| 텍스트 콘티·cinematic/animatic 연출·각 검토 | NVC | 현재 서사 참조와 원본 hash를 보존해 소비 |
| 자산 attempt·PNG/PSD 검사·자산 검토·handoff | SAC | 승인된 실제 파일과 lineage 재검사 |
| projection·합성·측정·media 후보·custody pointer | SMC | 기계 결과를 별도 사람 승인과 구분 |
| Prompt compilation과 provider별 image effect | IPC | prompt/result 계약을 소비하며 Story 상태는 소유하지 않음 |
| 실행 기록·contract pin·cross-owner source 관찰 | integration | domain 승인이나 작품 DB를 복제하지 않음 |

NVC의 presentation `DirectorPlan`, cinematic plan, animatic plan은 목적과 schema가 다르다. 하나의 범용 plan으로 합치지 않는다. Provider는 각 adapter의 effect이며 공유하는 것은 [Story envelope](../integration/contracts/)와 명시된 계약이다.

## 텍스트 저장·인계·발행

`story-project.json`은 project 참조와 최초 `intakeSourceSha256`을 저장한다. `writing/.geultu`는 geultu journal, `narrative/<project>`는 NVC workflow, `attempts/<id>`는 외부 작성 요청·응답·결과를 소유한다. 이 참조 파일과 실행 기록이 작품 승인 정본은 아니다.

[원문 intake](../integration/crates/storyctl/src/workflows/text.rs)는 첫 입력·요청 크기를 owner 효과 전에 결속한다. Geultu journal은 기존 prefix와 새 command event 묶음을 stage한 뒤 atomic replace하고, projection 복구는 snapshot·index owner가 맡는다. Journal 전체 복사에 비례하는 I/O와 과거 손상/no-op 복구 한계는 geultu Current에 둔다.

Geultu authoring handoff는 동일 검토 intent → synopsis bytes → public receipt 순서로 발행한다. [인계 수용](../integration/crates/storyctl/src/workflows/text.rs)은 현재 작품·revision·본문과 대조하고 캡처한 바이트를 NVC public intake로 전달한다. NVC가 저장한 source binding과 현재 원고를 다시 관찰한다. 이미 생성된 프로젝트는 응답 실패나 source 변경 때문에 삭제하지 않는다.

NVC storyboard는 STS와 같은 `.workflow.lock` 아래 현재 Completed release·씬·base·취소 기록을 다룬다. Candidate·미리보기·검토·export는 서로 다른 결과다. 공개 bundle이 전체 씬의 현재 승인 상태를 확인한다.

[writing export](../integration/crates/storyctl/src/workflows/publication.rs)는 coherent geultu snapshot만 내보낸다. [최종 package](../integration/crates/storyctl/src/workflows/publication.rs)는 그 snapshot과 NVC bundle을 함께 검사해 정확한 9개 파일을 쓰고, 두 owner를 다시 읽은 뒤 마지막 manifest를 발행한다. 여러 컴포넌트 전체를 하나의 transaction으로 잠그는 구조는 아니다. 중간 파일이 남았어도 완료 marker 없이는 완성 판본이 아니다.

## 외부 작성과 복구

[attempts.rs](../integration/crates/storyctl/src/workflows/attempts.rs)는 task 준비 전에 attempt를 독점 예약한다. 하나의 writer를 제한 시간·출력량 안에서 실행하고 요청/응답/result를 저장한다. 제출과 `recover`는 같은 attempt 잠금을 쓴다. 복구는 저장된 응답의 schema·identity·현재 owner 상태를 읽으며 provider를 재호출하지 않는다.

`not_applied`와 `not_started`는 확인된 미적용/미시작, `failed`·`cancelled`는 관측된 terminal 결과, `outcome_unknown`은 effect 후 결과 불명이다. 취소·늦은 결과·정리 실패를 성공으로 축소하지 않는다. 로컬 process-group 관리가 escaped descendant·host crash·OS sandbox를 해결한다는 보장은 하지 않는다.

## 선택 animatic과 미디어

[animatic.rs](../integration/crates/storyctl/src/workflows/animatic.rs)는 현재 NVC packet을 자산 root에 불변 export하고 SMC public renderer를 호출한다. 반환된 candidate 뒤 NVC source를 다시 관찰해 Story의 `story.animatic-observation/1`에 current/changed/unobserved를 기록한다. SMC producer receipt는 그대로 `render`에 보존한다. 이 관찰이 Human final approval은 아니다.

기존 cinematic 경로는 NVC intent/review → SMC v3 projection → SAC visual/asset approval → SMC motion/editorial/custody다. Renderer는 shot을 발명·재정렬하지 않고 원본 의미와 검토 hash를 보존한다. Animatic candidate와 기존 Track B release promotion은 별도다.

## 계약·권한·확장

Producer가 schema·field·hash domain을 소유하고 [registry](../integration/contracts/registry.json)가 consumer 사본을 byte-equality로 pin한다. Raw bytes hash와 canonical content hash는 각 계약의 정의를 따른다. Hash·reviewer 문자열은 신원·저작권·사용자 동의의 증명이 아니다.

SEMI provider는 startup-selected workspace descriptor와 project identity를 고정하고 공개 geultu 작업만 중계한다. 준비·사용자 승인·실행·취소 drain은 별도 경계다. 자세한 정책은 [provider 문서](../integration/macos-provider/README.md)에 둔다.

원문·사용자 workspace·vendor·legal·계약 사본을 임의 migration이나 정리 대상으로 취급하지 않는다. Jangmyeondo source/runtime dependency, shared-image-core, VisualScreenCompiler 및 새 제품 화면 제작 코드는 추가하지 않는다.
