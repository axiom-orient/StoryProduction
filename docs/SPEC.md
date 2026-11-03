# StoryProduction — 제품 계약

## 텍스트 콘텐츠 제작의 주 계약

현재 제품 목표는 원문에서 스토리·시놉시스·씬·텍스트 콘티를 완성하는 것이다.
아래의 기존 cinematic/media 계약은 선택 확장의 독립 계약으로 보존한다.
텍스트 제작의 필수 단계나 완료 조건으로 적용하지 않는다.
원문부터 판본까지의 다섯 작업 구간·18개 상세 단계는 [Target 명세](STAGE_EXECUTION_SPEC.md)에 있다. 이 설계는 현재 CLI workflow graph나 호출·승인 횟수를 정의하지 않는다.

| 단계 | owner | 완료와 수정 의미 |
|---|---|---|
| 원문·브리프·글투 | geultu | 원문 bytes 저장, 명시된 집필 방향과 관찰 자료 |
| 스토리·시놉시스 | geultu authoring | 실제 blocks, 후보 hash, source/selection CAS, 별도 사람 채택 |
| 인계 | 기존 reviewed-synopsis contract | 현재 시놉시스만 immutable handoff로 발행 |
| 씬 설계·대본 | NarrativeCourier STS | typed Source IR, 원문 coverage ledger, 기존 task/gate token과 최종 Human release |
| 텍스트 콘티 | NarrativeCourier | 현재 씬의 semantic occurrence를 참조하는 패널과 별도 검토 |
| 문서 패키지 | integration | coherent writing snapshot + 승인된 전체 씬 콘티 + actual files + 마지막 manifest |

텍스트 콘티는 이미지 파일을 가진 storyboard라고 주장하지 않는다. 패널 구도·동선·
시선·소리·전환은 사람의 검토를 받는 연출 문장이고, 행동·대사는 원 씬에서 정확히
해결한다. 콘티 계약은 NVC delivery의 기존 다섯 DTO를 변경하지 않는 authoring
artifact 계약이다. producer schema와 consumer copy는 registry에 등록한다.

STS STEP 0의 source-span coverage는 NVC 내부 workflow artifact다. 각 typed SRC/REL을 승인된
synopsis의 정확한 구절에 묶고 Source IR Human gate에서 주장·믿음·부정·unknown 분해를
검토한다. 이 ledger는 다섯 cross-component DTO를 변경하지 않으며 semantic completeness나
대본 품질을 자동 승인하지 않는다.

`authoring.prepare`는 저장된 작업 준비이며 외부 호출이 아니다. `run`은 한 번의
명시적 작성 프로그램 호출이며 후보 저장까지만 한다. `export`는 집필 문서만,
`package`는 source/story/synopsis/scene blueprint/screenplay/scene map/전체 콘티와
검토 기록을 포함한다. 각 출력은 명칭에 맞는 완료 상태를 반환한다.

원문·상위 문서·guidance가 바뀌면 후보 채택을 막는다. 선택 블록 밖의 독립된 편집은
보존한다. 원문이 바뀐 문서의 부분 갱신을 전체 최신 상태로 승격하지 않는다.
직접 복원은 과거 의존 관계를 유지한 새 revision이며 과거 승인을 부활시키지 않는다.

작성 실행은 timeout/출력량/attempt 수를 제한하고 외부 효과가 불명확하면 unknown을
유지한다. CLI actor/reviewer는 trusted local operator claim이다. geultu MCP는 새
사람 전용 authoring 명령을 숨기는 것뿐 아니라 실행도 거절한다. 셸 권한이나 별도
HTTP 호스트의 인증까지 이 정책으로 대체하지 않는다.

`status`의 `complete`는 현재 원고·STS·전체 콘티 승인 상태의 관찰이다. 파일을 발행하려면
`package`를 호출해야 한다. 최종 package는 원고 3개와 NVC 산출물 6개를 고정하고,
현재 owner 결과를 다시 읽은 뒤 마지막 `story-package.json`을 완료 marker로 발행한다.
Marker 없는 부분 파일, 별도 writing `export`, 후보 미리보기를 전체 package와 혼동하지 않는다.

상세 입력과 실패·복구 의미는 [텍스트 운영](TEXT_PRODUCTION.md), 소스 완료 범위와
실행 근거는 [Current](IMPLEMENTATION_STATUS.md)에 둔다.

## 선택 animatic 계약

승인된 text board를 NVC 연출 계획·review·packet으로 연결한다. SAC는 packet에 결속된
실제 PNG의 구조·크기·alpha·hash·coverage를 검사하고 별도 자산 review를 소유한다.
SMC는 승인 입력과 고정 renderer 환경에서 정확한 frame 합성·인코딩·파일 관찰을 수행한다.
결과는 편집 가능한 HTML, VTT 자막과 opaque VP9 WebM 후보다. `audio`는 `none`,
`finalApproval`은 `false`이며 최종 작품 release 채택을 대신하지 않는다.

Integration은 렌더 전후 현재 NVC packet을 관찰하고 `story.animatic-observation/1`에
`current`·`changed`·`unobserved`를 구분한다. SMC producer receipt를 확장하거나 다시 seal하지
않고 `render`에 보존한다. `changed`는 `source_stale`, `unobserved`는 `outcome_unknown`이다.
Status 조회는 과거 실행 기록이며 현재 작품에서의 채택 가능성을 보장하지 않는다.

같은 attempt의 다른 입력을 거절하고 실패·취소·unknown을 자동 재실행하지 않는다.
취소와 candidate 발행은 첫 terminal receipt로 정렬한다. 실제 자산 생성·그림 품질,
음원·3D·I2V·투명 WebM·최종 release는 이 candidate 계약의 완료 조건에 포함하지 않는다.
입력 schema와 producer/consumer 사본은 [registry](../integration/contracts/registry.json),
호출·복구 절차는 [animatic 운영](ANIMATIC_PRODUCTION.md)을 따른다.

## 보존된 미디어 확장 계약

## 범위와 필수 기능

본 문서는 공통 의미/관찰 계약을 소유한다. field·enum·버전의 정본은 아래 producer schema이며 현재 지원 여부는 [Current](IMPLEMENTATION_STATUS.md), 남은 구현은 [PLAN](PLAN.md)이 소유한다.

기존 미디어 확장을 사용할 때의 의미는 reviewed story intake, Human narrative approval, 명시적인 cinematic intent, 정확한 shot projection, source-bound visual candidate/approval, media candidate/QC, 별도 final approval이다. generation provider·automatic proposer/evaluator·audio engine·NLE adapter는 선택 기능이다. 선택 기능이 비활성이라고 완료 상태를 위조하지 않는다.

## Stage 입력·출력·전제조건

| Stage·owner | 입력·전제조건 | 출력·관찰 계약 | 오류/금지 |
|---|---|---|---|
| Story intake / geultu | 사용자 story, reviewed synopsis | source-bound reviewed handoff | review 없이 narrative authority로 승격 금지 |
| Narrative / NVC STS | reviewed handoff, current source, task/gate tokens | Completed release/package/scene map | source drift·token mismatch·non-Completed 거절 |
| Continuity / NVC | 승인된 원본 scene map | 추출 사실·stable entity IDs·미확정 시각 상태 | 근거 없는 외모/wardrobe 확정 금지 |
| Cinematic / NVC | current narrative+bible, 명시적 draft, production constraints | shot IDs/order/frames/camera/semantic assignment를 가진 plan + 별도 review receipt | renderer가 shot을 발명·추가·삭제·재정렬 금지 |
| Projection / SMC | release, bible, plan, approval의 일치 | SceneGraph v3 + raw source capsule를 가진 projection receipt | missing approval·hash drift·occurrence 누락/중복·unknown field 거절 |
| Visual development / SAC | projection, 기존 reference 또는 provider candidates | reference/keyframe pack candidate + 별도 approval + derived memory index | 생성 성공/자기 hash만으로 승인 대체 금지 |
| Motion / SMC mechanism | shot intent와 승인된 visual refs, 명시 transfer policy | provider-neutral request, 외부 receipt와 bytes에 bound candidate | provider 전용 옵션을 core 의미로 승격 금지 |
| QC / SMC mechanism+reviewer | candidate, 실제 관찰, source-bound review evidence | contract checks, 미검토 상태, repair scope/owner | NOT_RUN을 PASS로 축소·QC 자동 승인 금지 |
| Editorial / SMC | current QC-passed clips, 명시 trim/audio/caption intent | frame 기반 timeline candidate | unknown media·shot order drift·부족한 handles 거절 |
| Final release / Human+custody | 범위가 확정된 release subject, 현재 evidence | 별도 승인 후 immutable release와 current pointer | timeline/QC 생성이 release pointer를 수정하면 안 됨 |

## 시간·semantic·binding 계약

시간의 기본 단위는 양의 정수 frame과 명시적 fps다. shot duration은 plan intent이며 고정 fallback 시간이 아니다. scene duration은 소속 shot duration 합, timeline의 편집 길이는 순서대로 배열된 clip의 duration 합이다. dissolve는 인접 clip의 handle을 사용하며 plan의 cut-point 길이를 몰래 줄이지 않는다. renderer/interchange가 지원하지 않는 camera/transition/audio 표현은 거절하거나 명시적으로 미지원이어야 한다.

Narrative semantic manifest의 각 occurrence는 정확히 하나의 shot에 배정한다. scene order, occurrence identity/source text/hash, dialogue speaker와 dialogue order를 보존한다. cinematic 자연어의 의미 평가와 위 구조 검사는 별개이며 승인/평가 주체가 검토해야 한다.

approval은 해당 원본 hash와 plan/reference/asset bytes에 묶인다. source revision이 달라지면 하위 artifact는 stale이고 이전 approval을 재사용할 수 없다. hash는 무결성 식별자이지 서명자·사람 존재·저작권 증명이 아니다. local reviewer claim을 사용하는 경계는 trusted operator 환경과 승인 책임을 명시해야 한다.

## 외부 효과·취소·재시도·저장

외부 bytes 전송은 명시적 permission이 필요하며 wrapper의 정책 검사를 OS sandbox로 표현하지 않는다. `FAILED`, `CANCELLED`, `OUTCOME_UNKNOWN`은 구별한다. cancel 후 늦은 성공은 자동 채택하지 않는다. outcome unknown은 provider job 조정 없이 새 유료 실행을 하지 않는다.

새 artifact의 immutable publication은 동일 경로+동일 bytes에 대해서만 멱등이다. 충돌 시 overwrite나 자동 legacy 변환을 하지 않는다. 여러 파일을 발행하는 operation은 commit marker, partial effect와 복구 책임을 명시해야 한다. source hash를 검증했다는 이유로 장기 media 보관·파일 집합의 transaction·적대적 filesystem 격리를 보장하지 않는다.

## 수용 기준

대표 계약 수용 fixture는 2 scenes/5 shots, 서로 다른 frame 수, dialogue/action occurrences, source-bound visual refs, clip observation, editorial candidate를 포함한다. positive case뿐 아니라 unknown field, missing/stale refs, source/hash drift, duplicate ID/occurrence, invalid frame/enum, cancelled/unknown/partial media, changed review evidence, trim/order/handle 오류를 거절해야 한다.

fixture/probe/형식 검사와 actual provider 품질·native Human release·복구 검사를 각각 구분한다. `SATISFIED`는 표기한 계약 scope의 판단이며 모든 실환경 검증 PASS의 다른 표현이 아니다.

## Intentional contract 원본

| owner | 정본·참조 |
|---|---|
| geultu | [제품 SPEC](../geultu/docs/SPEC.md), reviewed synopsis producer schema |
| NVC STS/delivery | [제품 SPEC](../NarrativeCourier/docs/SPEC.md), `NarrativeCourier/contracts/`, Rust 5개 delivery DTO |
| NVC cinematic | [CDP](../NarrativeCourier/contracts/cinematic-director-plan-v1.schema.json), [bible](../NarrativeCourier/contracts/continuity-bible-v1.schema.json), [approval](../NarrativeCourier/contracts/cinematic-approval-v1.schema.json) |
| SMC projection | [SceneGraph](../SceneMotionCompiler/contracts/schemas/scene-graph.schema.json), [receipt](../SceneMotionCompiler/contracts/schemas/narrative-scene-projection.schema.json) |
| SAC visual | [pack](../SceneAssetCompiler/docs/contracts/visual-reference-pack-v1.schema.json), [approval](../SceneAssetCompiler/docs/contracts/visual-reference-approval-v1.schema.json), 기존 scene-release/asset/handoff schemas |
| SMC production | [closed inputs/outputs](../SceneMotionCompiler/contracts/schemas/production/production-v1.schema.json), [제품 SPEC](../SceneMotionCompiler/docs/SPEC.md) |
| integration | [제품 SPEC](../integration/docs/SPEC.md), [producer/consumer registry](../integration/contracts/registry.json) |

SceneGraph v3 전환이 모든 다른 version의 동시 인상을 의미하지 않는다. NarrativeSceneMap v2, SceneMotionAssetHandoff v2, TrackB release 계열의 현재 버전은 각자의 intentional contract로 유지한다. 명칭 `promptgen`·`TrackB*`를 미적 이유로 바꾸지 않는다.
