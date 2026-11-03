# 원작자와 에이전트의 공동 집필 — 조사와 구현 결정

기준일: 2026-09-22. 입력은 사용자의 제품 방향, 저장소의 owner 규칙,
[/Users/ax/Documents/Agent-UX-Guide/Agent-UX-Guide.md](/Users/ax/Documents/Agent-UX-Guide/Agent-UX-Guide.md),
기존 geultu/STS 소스와 아래 1차 자료다. 웹 조사와 소스 분석을 수행했다.
사용자 요청에 따라 빌드·테스트·실행 실험·모델 품질 평가는 수행하지 않았다.

## 1. 제품 문제를 다시 정의한 이유

기존 루트 README의 주요 흐름은 shot projection, visual reference, motion,
editorial candidate였다. 이 흐름의 구현 유무와 별개로, 이번 사용자가 완성하려는
작업은 자신이 쓴 이야기에서 출발한 **읽고 고칠 수 있는 콘텐츠 문서**다.
동영상 생성이나 이미지 provider는 이 완료 조건에 필요하지 않다.

이미 있는 강점도 분명하다. geultu에는 local journal, revision, candidate,
글투 관찰과 공개 Application 경계가 있다. STS에는 source lineage, 인물 상태,
트리트먼트, SCN 씬 계약, 대본, task/gate token, 사람의 최종 확정이 있다.
따라서 새 중앙 서사 엔진을 만들거나 STS와 경쟁하는 scene model을 추가할
이유가 없다. 빠진 것은 두 능력을 **작가의 실제 작업 단위**로 이어 주는 흐름이다.

## 2. 연구·제품 사례에서 가져온 판단

| 자료 | 확인한 내용 | 이번 구현의 선택 |
|---|---|---|
| [Microsoft, Guidelines for Human–AI Interaction, CHI 2019](https://www.microsoft.com/en-us/research/publication/guidelines-for-human-ai-interaction/) | AI 기능의 이해, 수정과 통제에 관한 18개 지침을 제안하고 디자인 실무자 연구로 검토했다. | 준비·후보·채택·실제 저장 상태를 구별하고, 실제 변경 내용과 후속 행동을 제공한다. |
| [CoAuthor, Stanford](https://coauthor.stanford.edu/) | 사람이 직접 쓰고, 제안을 요청·선택·거절하고, 채택한 문장을 다시 고치는 상호작용을 기록한다. | 후보를 받은 후 직접 편집할 수 있어야 한다. 새 candidate + based_on으로 편집 계보를 남긴다. 이 연구가 특정 한국어 문체의 재현을 보증하지는 않는다. |
| [Wordcraft 연구](https://arxiv.org/abs/2211.05030) | 출판 경험이 있는 작가 13명이 AI 집필 도구를 사용한 경험을 다룬다. 단순 자동 완성보다 작가의 창작 과정에서의 쓰임을 분석한다. | 작가의 원문·지시·선택 범위를 중심에 두고 전체 원고를 통째로 대체하는 흐름을 기본으로 삼지 않는다. |
| [Dramatron 연구](https://arxiv.org/abs/2209.14958) | 구조적 맥락을 단계적으로 이어 대본을 쓰는 접근과 영화·연극 전문가의 공동 창작 경험을 보고한다. | 원문→스토리→시놉시스→STS 씬→콘티의 계층적 맥락을 사용한다. 논문을 근거로 생성물의 장편 품질이 자동 보장된다고 주장하지 않는다. |
| [Dramatron 공식 구현 설명](https://github.com/google-deepmind/dramatron) | 작가가 생성된 인물·플롯·장소·대사를 편집·재작성할 재료로 다루며, 결과가 공식적이거나 상투적일 수 있음을 설명한다. | 채택과 창작 품질 판단을 모델 출력과 분리한다. 코드를 가져오지 않고 기존 Story 소유 모델을 확장한다. |
| [Final Draft, 1-2-3 Method](https://www.finaldraft.com/blog/tech-tip-1-2-3-method) | beat를 구조화한 outline을 대본 작성으로 연결한다. 아이디어 카드와 실제 집필을 이어 가는 제품 흐름이다. | 문서 단계 간 연결과 안정된 block/scene 식별자를 제공한다. 모든 작품에 특정 beat 수나 고정 구조를 강요하지 않는다. |
| [Scrivener, Snapshots](https://www.literatureandlatte.com/blog/use-snapshots-in-scrivener-to-save-versions-of-your-projects) | 문서 단위의 이전 버전을 남겨 수정 전후 내용을 돌아볼 수 있다. 문서 snapshot은 프로젝트 전체 백업과 다르다. | journal에서 이전 문서 revision을 조회하고 현재 문서에 새 revision으로 복원한다. 복원이 새로운 원문 기준까지 재승인하는 효과를 갖지 않게 한다. |
| [StudioBinder, Shot List vs Storyboard](https://www.studiobinder.com/blog/shot-list-vs-storyboard/) | 촬영용 목록과 장면의 시각적 전개를 보여 주는 스토리보드의 목적·표현이 다르다. | 이번 산출물은 **텍스트 스토리보드/콘티**라고 명확히 표시한다. 패널의 구도·동선·시선·소리·연결을 제공하며 생성 이미지가 있는 것처럼 표시하지 않는다. |
| [Fountain 공식 문법](https://fountain.io/syntax/) | 대본 요소와 작성용 section/synopsis를 구별하는 읽을 수 있는 텍스트 문법이다. | 기존 NVC Fountain/FDX 경로를 유지한다. 단순 파일 확장자 변경으로 자체 Markdown을 Fountain으로 가장하지 않는다. |
| [ScreenSkills, Screenwriter](https://www.screenskills.com/job-profiles/browse/film-and-tv-drama/development-film-and-tv-drama-job-profiles/screenwriter-film-and-tv-drama/) | 스토리·감정·인물·장소·행동을 대본으로 표현하는 글쓰기 역량을 설명한다. | 씬의 품질은 개수나 스키마 통과가 아니라 구체적 행동·갈등·인물 선택으로 판단하도록 집필 지침을 쓴다. |
| [NN/g, The 3 Roles of Context, 2026-09-18](https://www.nngroup.com/articles/3-agent-context-roles/) | 숙련 사용자가 맥락을 지속 지침, 작업별 자료, 주변 정보 등의 역할로 다루는 양상을 소개한다. | 작품의 brief, 현재 작업의 source/selection, 실제 원문을 구분한다. 필요하지 않은 메일·계정·외부 자료 수집은 추가하지 않는다. |
| [Anthropic, Writing effective tools for AI agents](https://www.anthropic.com/engineering/writing-tools-for-agents) | 업무 단위의 도구 선택, 명확한 이름과 목적, 필요한 맥락의 반환을 제안한다. | 상태/문서/후보/집필 작업으로 도구를 나누고 `next`가 한 번에 다음 행동을 안내한다. 내부 필드별 범용 CRUD 플랫폼을 만들지 않는다. |

위 자료는 설계의 참고 근거다. 아래 동시성·저장·승인 정책은 저장소의 요구와
현재 코드에서 도출한 구현 결정이며 논문의 실험 결과를 그대로 이식한 것이 아니다.

## 3. Agent UX Guide를 실제 경로에 적용

| 가이드 절 | 적용한 업무 계약 | 구현 위치 |
|---|---|---|
| 1. 결과·대상·보호 조건 | 원문·집필 brief·글투를 읽고 실제 story/synopsis 문서를 만든다. | geultu authoring state와 craft 문서 |
| 2. 사실의 owner | geultu는 집필, STS는 씬, NVC는 씬에 근거한 콘티. integration에는 서사 상태를 저장하지 않는다. | 공개 CLI 연결과 기존 reviewed handoff |
| 3. 실제 후보와 차이 | 추가·교체·삭제·순서 변경, 직접 고친 후보, 선택 블록만의 수정 | authoring.candidate / prepare / submit |
| 4. 범위와 수명 | task에 원문·상위 문서·brief·글투 기준을 저장하고 자료의 역할을 명시 | Task.inputs / guidance_hash / packet |
| 5. 업무 도구 | status→next→submit→review→export; 긴 문서는 개별 document/task 조회 | Application / storyctl text |
| 6. 구체적인 승인 | candidate hash와 source 조건을 실제 채택 시 판정, MCP의 사람 전용 명령 실행 거절 | review / handoff / MCP transport |
| 7. 수정·복구 | 프로젝트 잠금 안에서 조건 비교와 journal 반영, 이전 문서 복원도 새 revision | Workspace.transact / history / restore |
| 8. 지속 작업의 한도 | 한 attempt당 한 작성 호출, 호출 전 독점 예약, timeout과 출력 한도, 자동 재시도 없음 | text_process / attempts |
| 9. 작업물 중심 표현 | UTF-8 Markdown, 텍스트 차이, scene/panel 단위 구조. 새 제품 UI는 추가하지 않음 | 문서 export / storyboard render |
| 10. 프로토콜 경계 | CLI stdout JSON, stderr 진단, owner의 공개 entrypoint, terminal 상태 구분 | storyctl text / NVC storyboard CLI |
| 11. 직접 수정과 이어쓰기 | 사용자가 고친 후보의 실제 bytes 채택, nonselected 블록 보존, source 변경 시 이유 표시 | geultu apply / stale_reasons |

## 4. 서사 품질을 코드와 모델에 배분

### 결정적으로 처리하는 것

원문의 저장, 선택 ID 범위, 중복 ID, 문서 revision, dependency/hash,
실제 채택한 candidate, 원 씬의 occurrence와 콘티 sourceRefs의 관계,
패키지 파일의 실제 bytes, 명령 중복과 실행 한도는 코드가 소유한다.
콘티의 원 대사·행동은 sourceRefs를 통해 원문 그대로 내보낸다.

### 집필과 사람의 판단에 남기는 것

읽는 재미, 감정의 설득력, 대사의 뉘앙스, 원작자의 생략과 문체, 갈등의 밀도,
구도의 의미는 모델이 후보로 쓰고 사람이 판단한다. 자연어 composition/staging이
은근히 사건을 바꾸는지까지 해시로 알 수는 없다. 이 한계를 감추는 자동 점수나
"서사 PASS"를 추가하지 않았다.

스토리 집필 지침은 인물의 욕망·압력·선택·결과를 연결하고, 시놉시스 지침은
원작의 인과·반전·결말을 보존하도록 한다. 기존 STS scene blueprint의
precondition/postcondition, knowledge, relationship, props, downstream dependency를
재사용한다. 콘티 지침은 장면의 정보와 감정 변화에 따라 패널을 선택하도록 한다.

### 선택한 수정 단위

집필에서는 안정된 block id 묶음을 선택한다. 선택 블록의 기존 bytes와 상위 자료가
같으면 현재 문서의 다른 블록을 보존하며 반영한다. 상위 자료가 바뀐 상태에서 일부
블록만 바꿔 문서 전체가 최신이라고 표시하지 않는다. 이 경우 전체 갱신이나 직접
편집이 필요하다.

콘티에서는 씬 전체가 독립 재작성 단위다. task.base의 다른 씬을 그대로 유지하고
선택한 씬의 panels만 교체한다. 사건이 여러 컷에서 반복될 수 있으므로 occurrence는
한 번 이상 연결할 수 있지만, 다른 씬으로 옮기거나 누락할 수 없다. 이 규칙은
영상 cinematic plan의 occurrence를 정확히 한 shot에 배정하는 규칙과 별개다.

## 5. 실패·취소·복구의 의미

| 상황 | 남는 상태 | 다음 행동 |
|---|---|---|
| source/brief 변경 중 이전 후보 도착 | 후보는 보존, adoption blocker 표시 | 최신 task로 새 후보 작성 |
| 사용자가 후보 수정 | 기존 후보 유지, 새 candidate에 based_on | 새 후보 hash로 검토 |
| 문서 복원 | 이전 blocks를 새 revision에 복사, 과거 의존 관계 유지 | source가 달라졌다면 갱신 필요 |
| 작성 프로그램 호출 후 timeout/중단 | outcome_unknown과 request 기록 | 외부 효과를 확인하기 전 새 attempt 금지 |
| 작성 응답만 저장되고 submit 실패 | response.json과 owner 상태 보존 | recover로 기존 응답 저장 시도, 재생성 없음 |
| STS token이 이미 소비됨 | 현재 노드 artifact를 공개 API로 관찰 | 내용이 같으면 관찰 결과를 기록, 다르면 미확정 유지 |
| 내보내기 중 프로세스 종료 | 일부 파일, commit marker 없음 | 같은 bytes의 파일만 재사용해 재개 |
| 이전 STS 프로젝트와 새 시놉시스가 다름 | source_changed | 새 narrative-project로 명시적 인계 |

프로세스 종료가 외부 서비스의 취소를 증명하지는 않는다. POSIX process group
정리의 범위를 넘어선 실행·원격 비용·공급자 내부 재시도까지 통제한다고 주장하지
않는다. local actor/reviewer는 trusted operator claim이며 별도 인증 시스템은 아니다.

## 6. 실제 변경과 남는 한계

| 기존 gap | 추가한 구현 | 남는 한계 |
|---|---|---|
| 원문과 스토리·시놉시스가 제품 수준의 연결된 문서로 드러나지 않음 | geultu journal 안의 authoring 문서·태스크·후보·브리프 | 기존 scene 저장 모델과 자동 양방향 동기화하지 않음 |
| 후보 부분 수정과 채택 이력의 업무 접근이 부족 | 블록 비교·새 후보 편집·CAS·history/restore | 문장 내 문자 범위 편집 UI는 없음 |
| 루트 진입점이 미디어 중심 | storyctl text의 준비/자동 작성/인계/이어하기 | 작성 프로그램과 native binaries는 명시적으로 준비해야 함 |
| 씬에서 텍스트 콘티로 가는 문서 경로 부족 | NVC task/draft/review/export/bundle, 씬 단위 수정 | 그린 storyboard 이미지나 영상은 생성하지 않음 |
| 완료물이 여러 제품 폴더에 분산 | source/story/synopsis/scene blueprint/screenplay/scene map/board의 불변 패키지 | 멀티 owner의 최신 상태를 하나의 전역 transaction으로 잠그지는 않음 |
| 재실행으로 중복 생성 위험 | attempt 예약·요청/응답/결과 기록·recover | 응답도 잃은 외부 작업은 자동 해결할 수 없음 |

이번 작업에서는 저장소의 사용자 원고를 찾아 임의로 작품을 생성하거나, 실제
사람 대신 승인하지 않았다. 빌드·정적 검사·테스트·end-to-end 실행과 모델 품질
판정은 모두 미실행이다. 기존 문서의 과거 PASS는 이번 변경의 검증 결과가 아니다.
