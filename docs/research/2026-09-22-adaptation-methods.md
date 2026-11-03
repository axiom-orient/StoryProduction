# 각색·씬·대본·텍스트 콘티·문서 출고 정밀화 조사

조사일: 2026-09-22. 범위: 리뉴얼 11~15·17~18단계. 상태: **조사·Target 설계 메모**. 외부 소스와 현재 코드를 읽었으며 제품 코드 수정, 외부 패키지 설치, 모델 실행·품질 평가, 라이브러리 적합성 테스트는 수행하지 않았다. 아래 미세 절차는 기존 STS 안에서 사용할 작업 규칙 제안이지 새 runtime 구현 완료가 아니다.

## 1. 적용 결정

이미 STS에는 사건 순서/제시 순서, 정보 원장, 세계·인물 상태, 씬 pre/postcondition, 분할 집필·검토가 있다. 추가할 것은 또 다른 서사 엔진이 아니라 **각 결정의 근거, 인물의 행동 이유, 문서 사이의 검증 가능한 연결, 실패 시 돌아갈 단계**다. 의미 판단은 모델이 근거를 제안하고 사람이 검토한다. ID·해시·범위·형식 검사는 별도로 기계화한다.

- 사건 인과와 인물의 의도를 분리해 기록한다. 사건이 결말에 필요하다는 사실만으로 인물이 그 일을 할 이유가 생기지 않는다.
- 대본 집필 전에 정보 공개와 대화의 작용을 설계한다. 시나리오 길이를 맞추기 위한 무단 서브플롯·설명 대사는 허용하지 않는다.
- 텍스트 콘티의 occurrence는 **누락 없이 적어도 한 번** 연결한다. 같은 occurrence를 여러 패널이 참조할 수 있다. 패널 수나 semantic reference 개수의 일치는 품질 기준이 아니다.
- 패키지의 byte 무결성, 현재 승인과의 일치, 예술적 완성도는 서로 다른 검사다. 해시가 맞아도 의미·승인이 올바르다는 증거는 아니다.

## 2. 근거 원장

요약은 확인한 주장에 한정한다. 후속 절차의 새 필드·판정 방식은 본 프로젝트의 설계 추론이다. 원문 장문 인용과 소스 복사는 하지 않았다.

### M01 — 인과 계획과 인물의 의도

- **자료:** Mark O. Riedl·R. Michael Young, *Narrative Planning: Balancing Plot and Character*, JAIR 39, 2010, 217–268. [저자 제공 논문 PDF](https://faculty.cc.gatech.edu/~riedl/pubs/jair.pdf), [arXiv 서지](https://arxiv.org/abs/1401.3841).
- **직접 확인:** §4의 IPOCL은 행동 전제·효과와 인물 목표/동기 연결을 다룬다. 계획에 필요한 행동과 인물이 수행할 의도를 별도로 만족시킨다. §5는 32명 학생의 짧은 생성 이야기 질문·응답 적절성 평가다. 좋은 답 범주의 단측 검정은 p<0.0585, 나쁜 답 범주는 p<0.05다. 길이가 통제되지 않았고 의도를 명시하는 문장이 혼입된 한계를 저자가 기록했다.
- **채택:** 12~14에서 `목표를 촉발한 사건 → 인물의 선택 → 결과` 연결과 인과 위협 점검을 차용한다.
- **제한:** 오늘의 LLM, 한국어 대본, 장편·관객 반응·재미를 검증한 결과가 아니다. 모든 인물을 합리적 계획자로 만들거나 논문 planner를 runtime으로 도입하지 않는다.

### M02 — Fountain 공식 포맷

- **자료:** [Fountain 공식 Syntax](https://fountain.io/syntax/), Scene Headings·Character·Dual Dialogue·Error Handling 절.
- **직접 확인:** 강제 씬 헤딩 `.`, 비로마자 인물명에 쓸 수 있는 `@`, 두 번째 인물 큐 끝의 동시대사 `^`, 강제 action `!`, `#...#` 씬 번호, 비출력 section/synopsis, note/boneyard 규칙이 있다. 모호한 입력의 action 처리와 문서 서식은 이야기 의미 검증과 다르다.
- **채택:** 15에서 internal `SCN-*`와 외부 표시 번호를 분리하고 한국어 대사·동시대사·작업 메모를 포함한 호환성 표본을 설계한다.
- **제한:** NVC는 `fountain.rs`가 명시한 practical subset이다. 포맷 전체 호환·무손실 round-trip을 현재 지원으로 선언하지 않는다. 문법을 통과해도 대본 품질은 미검증이다.

### M03 — Storyboarder의 실제 모델·파서·출력

- **자료:** [Wonder Unit/Storyboarder](https://github.com/wonderunit/storyboarder). GitHub API로 확인한 기본 브랜치 `master`의 SHA는 `8b81a25c71d5f7ca46e8d5b8e3d4f7b3968f95c2`, commit 시간은 2022-06-30T17:42:04Z다.
- **직접 확인:** [board.js](https://github.com/wonderunit/storyboarder/blob/8b81a25c71d5f7ca46e8d5b8e3d4f7b3968f95c2/src/js/models/board.js)는 board duration·scene 기본값·audio duration을 구별한다. [scene ID utility](https://github.com/wonderunit/storyboarder/blob/8b81a25c71d5f7ca46e8d5b8e3d4f7b3968f95c2/src/js/fountain-scene-id-util.js)는 씬 헤딩의 번호가 없으면 ordinal과 생성 ID를 결합한다. [data parser](https://github.com/wonderunit/storyboarder/blob/8b81a25c71d5f7ca46e8d5b8e3d4f7b3968f95c2/src/js/fountain-data-parser.js)의 duration은 단어 수 기반 추정이다. [vendored Fountain parser](https://github.com/wonderunit/storyboarder/blob/8b81a25c71d5f7ca46e8d5b8e3d4f7b3968f95c2/src/js/vendor/fountain.js)도 읽었다.
- **라이선스 관측:** 해당 parser header는 MIT·Matt Daly를 표기한다. 저장소 전체는 GitHub API `license=null`, package.json에 license 필드 없음, [build/license_en.txt](https://github.com/wonderunit/storyboarder/blob/8b81a25c71d5f7ca46e8d5b8e3d4f7b3968f95c2/build/license_en.txt)는 EULA다. 저장소 전체를 MIT라고 단정하지 않는다.
- **채택·제한:** stable ID와 표시 순서 분리, 추정 시간과 측정 시간 분리만 설계에 차용한다. 코드 수입·의존성 추가는 하지 않으며 단어 기반 시간 추정을 한국어 실연 시간으로 간주하지 않는다.

### M04 — Pixar·Khan Academy의 시각적 이야기 교육

- **자료:** [Pixar 공식 프로젝트 소개](https://www.pixar.com/pixar-in-a-box), [공동 교육 과정](https://www.khanacademy.org/computing/pixar/storytelling).
- **직접 확인:** 공동 교육은 인물·구조 외에 visual language, film grammar, pitching/feedback을 별개 주제로 다룬다. 공간·움직임·색과 샷·편집·구도를 이야기 전달에 연결한다.
- **채택:** 12에서는 중요한 내면 의미를 담는 표현 수단을 정하고, 17에서는 패널이 관객에게 무엇을 언제 보이게 하는지를 검토한다.
- **제한:** 교육 과정은 실험적 성능 비교가 아니다. 모든 장면에 색·모티프·카메라 기술 목록을 강제로 넣는 근거로 쓰지 않는다. 이미지·영상 생성 기능을 현재 제품에 추가한다는 의미도 아니다.

### M05 — John August의 대화 집필 과정

- **자료:** 시나리오 작가 본인의 [How to write dialogue](https://johnaugust.com/2007/how-to-write-dialogue), 2007-02-07.
- **직접 확인:** 대화의 흐름·주도권, 화자와 관객의 정보 전달, 거친 초안 후 문장 정제, 소리 내어 읽기, 상대 발화에 실제 반응하는지, 줄여도 같은 기능을 하는지의 관점이 제시된다.
- **채택:** 15에서 발화의 기능과 반응 연결을 먼저 잡고, 실제 한국어 대사로 쓴 뒤 리딩과 압축을 수행한다.
- **제한:** 한 작가의 실무 방법이다. 통계적 우월성·장르 공통 공식으로 주장하지 않는다. 모든 대사에 갈등·서브텍스트를 강제하지 않는다. 한국어 높임말 검사는 StoryProduction의 별도 적용 제안이다.

### M06 — 파일 패키지의 완전성과 유효성

- **자료:** [RFC 8493, BagIt v1.0](https://www.rfc-editor.org/rfc/rfc8493), 2018-10, §2.1.3·§3. Informational RFC이며 Internet Standards Track 문서가 아니다.
- **직접 확인:** 파일 목록·checksum을 기록하고, 필요한 파일이 존재하는 complete와 모든 checksum을 확인한 valid를 구분한다. payload의 내부 의미 해석을 요구하지 않는다.
- **채택:** 18의 파일 목록·byte 검증·재개·최종 완료 표식을 구분하는 검사 설계에 참고한다.
- **제한:** 현재 Story package가 BagIt 호환이라는 뜻이 아니다. 이 요청에서는 BagIt exporter나 새 package version을 구현하지 않는다. 작가의 승인·출처 진실성은 checksum으로 인증되지 않는다.

ScreenSkills의 storyboard/layout 페이지와 공식 animation PDF도 검색했으나 본문 조회가 403/실패였으므로 핵심 설계의 독립 근거로 채택하지 않았다. afterwriting 저장소 metadata는 비교 후보로 조회했으나 실제 파서·라이선스 검토까지 완료하지 않아 채택하지 않았다.

## 3. 기존 구현에 연결되는 위치

| 단계 | 현재 source 근거 | 활용할 기존 산출물·gate | 새 제안의 경계 |
|---|---|---|---|
| 11 | [step_01](../../NarrativeCourier/skills/sts/prompts/analyst/step_01_screenplay_contract.md), [step_02](../../NarrativeCourier/skills/sts/prompts/analyst/step_02_narrative_map.md) | screenplay contract, narrative map, checkpoint A | 정보 공개 레코드의 예시·검토 질문 보강 |
| 12 | [step_03](../../NarrativeCourier/skills/sts/prompts/writer/step_03_cinematic_system.md), [step_04](../../NarrativeCourier/skills/sts/prompts/writer/step_04_character_states.md) | cinematic system, character states, checkpoint B | 동기·공간·상태 전이의 명시적 근거 보강 |
| 13 | [step_05](../../NarrativeCourier/skills/sts/prompts/writer/step_05_treatment.md) | treatment | 실제 전체 전개를 쓰고 scene blueprint와 함께 checkpoint C 검토 |
| 14 | [step_06](../../NarrativeCourier/skills/sts/prompts/writer/step_06_scene_blueprint.md) | Scene IR, checkpoint C | pre/postcondition 의미 검사는 marker 존재 검사와 분리 |
| 15 | [step_07_draft](../../NarrativeCourier/skills/sts/prompts/writer/step_07_draft.md), [merge](../../NarrativeCourier/skills/sts/prompts/writer/step_07_merge.md), [Fountain intake](../../NarrativeCourier/crates/sts/src/fountain.rs) | PART 1~4, PART review, first draft | PART는 작업 분할이며 4막 구조가 아님 |
| 17 | storyboard.py (`../../NarrativeCourier/tools/storyboard.py`, 당시 경로), [craft](../../NarrativeCourier/tools/storyboard-craft.md), [schema](../../NarrativeCourier/contracts/text-storyboard-v1.schema.json) | source-bound task → candidate → exact hash review | 텍스트 필드 의미 검토 보강; 새 필드를 v1 JSON에 임의 추가하지 않음 |
| 18 | Production.package (`../../integration/src/story_integration/text_workflow.py`, 당시 경로), storyboard.bundle (`../../NarrativeCourier/tools/storyboard.py`, 당시 경로) | `story.text-package/1`, 승인 콘티 전체 범위 | 새 개발 기록 포함은 명시 export/version 계약으로 후속 구현 |

상세 node와 gate 연결은 [workflow.json](../../NarrativeCourier/skills/sts/workflow.json)을 따른다. geultu의 집필 노트는 NVC의 정본 인물 상태가 아니다. NVC가 승인된 인계 자료를 해석하여 STS authority를 세운다. integration은 공개 entrypoint와 immutable contract를 사용하며 의미 정본을 소유하지 않는다. intake/support artifact를 NVC의 다섯 canonical delivery DTO와 혼동하지 않는다.

## 4. 단계별 미세 절차 — Target

아래 `11.1` 같은 번호는 작업 설명 ID다. 새로운 workflow node·공개 schema·CLI 옵션이 아니다. 기존 artifact의 서술 규칙으로 시작할 수 있는 것과 구조화 구현이 필요한 것을 구별한다. 각 단계에서 작성하는 결과는 실제 내용이며 계획 문장만 제출하지 않는다.

### 11. 매체·분량·정보 공개

**입력:** Human 승인 Source IR, source-form/ending 의무, 목표 매체·분량·제작 제약. **산출:** 각색 계약과 Fabula/Syuzhet·정보·변환·외재화 원장. **owner/gate:** STS analyst / checkpoint A.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 11.1 매체의 완료 조건 | 관객이 읽는지/보는지, 단일 작품인지/회차인지, 목표 길이·허용 장소·등장인물·기술 제약을 분리. 미정이면 가정을 표시하고 실제 지원 없는 매체 profile을 지원 완료로 표시하지 않음 |
| 11.2 보존·변경 예산 | 보호 의무별 `PRESERVE/DRAMATIZE/CONNECT/REORDER/COMBINE/OMIT/EXPAND/INVENT`와 근거 ID를 연결. 원문 대비 목표 길이가 과도하면 expansion pressure와 필요한 작가 결정을 드러냄 |
| 11.3 사건 순서 | 실제 발생 시간·원인·결과를 정리. 동시·순서 미정과 시간 생략을 구별하고 결말 사건에 이르는 필수 경로를 확인 |
| 11.4 제시 순서 | 각 사건의 관객 노출 시점과 기능을 배치. 플래시백·선취·반복 제시가 인물의 당시 지식을 바꾸지 않도록 실제 시간과 화면 순서를 함께 남김 |
| 11.5 정보 공개 | 중요한 사실마다 진실 상태, 인물별 믿음·앎, 관객의 앎, 최초 단서·오독 여지·확정 시점을 적음. `TOLD`는 전달 사실을 뜻하며 전달 내용의 참을 자동 보증하지 않음 |
| 11.6 복선·회수·마지막 의미 | 결말을 이해하는 데 필요한 관객 단서와 회수 지점을 연결. 모호성을 의도적으로 남긴 구간은 정답 강요 대신 허용 해석 범위를 기록 |
| 11.7 압박 검토 | 시간 추정치·확실성·추정 방법을 표시. 길이를 맞추려고 소스 의무를 누락하거나 승인 없는 사건을 추가하지 않음 |

**기계 gate 제안:** source revision/ID 실재, 참조 범위, 필수 원장 항목, 미허가 invention 표기 누락. **의미 gate:** 인과 변경과 제시 변경의 혼동, 미래 정보 누수, 의도한 오해와 사실 오류의 구별. **실패 복구:** 재배치만 필요하면 11.4, 단서 누락은 11.5, 보호 조건 충돌은 10의 source 해석/승인으로 되돌린다. 길이가 부족하다고 자동으로 새 사건을 채우지 않는다. 근거 M01·M04, 세부 설계는 프로젝트 추론.

### 12. 인물 상태·세계 규칙·공간

**입력:** 승인된 Source IR/11 계약·원장. **산출:** 실행에 필요한 cinematic system·character states. **owner/gate:** STS writer / checkpoint B.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 12.1 인물 상태 경계 | 인물별 현재 욕망·회피·능력·손상·관계/권력·위치·소지품을 장면 실행에 필요한 만큼만 기록. 원문에 없는 생애사로 이유를 메우지 않음 |
| 12.2 의도 형성 | 중요한 선택마다 행동자, 목표, 목표를 촉발한 사건/압박, 비용, 가능한 대안, 선택 근거를 연결. 비의도적 사고·실수는 별도 표시 |
| 12.3 인식 상태 | 사실과 인물의 믿음을 구별. 획득 경로·source/event 근거·누가 접했는지를 추적. 아직 없는 SCN을 발급하지 않으며 S14에서 실제 씬에 결속. 인물이 모르는 것을 관객이 봤다는 이유로 말하게 하지 않음 |
| 12.4 세계 규칙 | 실제 쓰는 규칙의 trigger/scope/effect/limit와 예외 권한을 기록. 결말 해결에 필요한 예외가 없으면 GAP/충돌로 올림 |
| 12.5 공간 작동 | 출입구·문턱·접근 권한·가시/가청 범위·장애물·소요 이동을 필요한 위치만 명세. 인물의 도달/목격/엿듣기가 가능한지 판단 |
| 12.6 표현 수단 | 내면·회상·설명 중 반드시 전달할 의미를 행동·소품·소리·화면 정보·명시적 서술 장치 중 어떤 것으로 구현할지 결정. 수단이 바뀌어도 보호된 의미를 검사 |
| 12.7 시간 순 전이 | source/event에 결속한 잠정 전이를 순서대로 읽고 지식·부상·소품·위치·관계가 생성/소멸한 이유를 확인. 실제 SCN 전후 상태의 결속·검산은 S14에서 수행. 변화하지 않는 인물을 강제로 성장시키지 않음 |

**기계 gate 제안:** 상태 키·등장인물 ID·근거 연결의 존재. **의미 gate:** 인물 목표가 대본의 필요에 종속되어 부자연스러운지, 물리/제도 제약을 위반하는지. **복구:** 행동 근거 미비는 12.2, 단서 부재는 11.5, 이동·접근 불가능은 12.5/14, 세계 규칙을 바꿔야 하면 승인 단계로 복귀. M01의 형식적 의도 연결을 참고하되 충동·모순·자기기만을 오류로 일괄 제거하지 않는다.

### 13. 트리트먼트

**입력:** 10~12의 승인된 의무와 상태. **산출:** 처음부터 끝까지 읽히는 실제 상세 전개. **owner/gate:** STS writer / 14와 함께 checkpoint C.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 13.1 의무 배치 | 결말·중요 관계 변화·필수 정보·세계 규칙 사용을 전개상의 구간에 배치. 구간 수를 막 공식에 맞추지 않음 |
| 13.2 사건 연결 | 주요 사건마다 누가 무엇을 하며 왜 지금 하는지 쓰고, 선택의 직접 결과가 다음 압박을 어떻게 만드는지 연결 |
| 13.3 중간 결과 | 실패/부분 성공/손실/오해가 이후 선택을 바꾸는 과정을 실제 행동으로 작성. 모든 구간을 자극적 반전으로 끝내지 않음 |
| 13.4 외재화 적용 | 12.6의 표현 수단을 전개 속에 구현. '관객은 죄책감을 느낀다' 같은 목표 문장으로 행동을 대체하지 않음 |
| 13.5 결말 검산 | 최종 사건의 실행자·선택·대가·정보 재해석·남은 감정을 각각 회수. 결과만 같은 다른 이야기로 변했는지 확인 |
| 13.6 전체 읽기 | 이유 없는 방향 전환, 생략 때문에 생긴 불가능, 같은 설명 반복, 승인 예산을 넘는 확장을 표시. 고칠 최소 구간과 영향받는 후속 의무를 제시 |

**기계 gate 제안:** 추적할 의무가 모두 배치되어 있는지. **의미 gate:** 사건 목록에 그치지 않고 선택·반응·결과가 읽히는지, 결말을 지탱하는 원인이 앞부분에 있는지. **복구:** 단순 문장 연결은 이 단계 수정, 인물 이유/규칙 추가가 필요하면 12, 새로운 플롯 사실이 필요하면 10~11로 복귀. M01에서 얻은 점검 관점을 기존 step_05에 적용하는 설계다.

### 14. 씬 카드·전이 계약

**입력:** treatment, 현재 상태·정보 원장·공간 규칙. **산출:** 안정된 `SCN-*`가 있는 Scene Blueprint. **owner/gate:** STS writer·critic / checkpoint C.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 14.1 장면 경계 | 장소·시간·행동 연속성과 관객 제시 단위로 경계를 정한다. 먼저 카드 번호를 붙였다는 이유로 사건을 쪼개지 않음 |
| 14.2 필요와 볼 가치 | 의무/상태/정보상 필요한 기능과, 이 구현을 볼 이유가 되는 행동·소리·공간 압박을 따로 한 문장씩 씀 |
| 14.3 진입 조건 | 인물 위치·앎/믿음·신체·관계·소품·접근·열린 약속을 적고 이를 만든 앞 장면/초기 조건을 연결 |
| 14.4 행동 전환 | 당장의 과업·압박, 실행 가능한 선택, 저항·상대 반응, 주된 전환을 구체적인 행동으로 구성 |
| 14.5 퇴장 조건 | 새로 생긴/사라진 상태와 관객 정보의 차이를 적고, 이것을 소비하는 후속 씬을 연결 |
| 14.6 위협·생략 검사 | 중간 장면이 후속 전제를 깨는지, 이 씬을 삭제/통합하면 어떤 의무가 사라지는지 검사. 리듬·정서·공간 정착이 목적이면 그 기능을 설명하여 정당화 |
| 14.7 위험에 맞는 검토 | 충실도·지식·상태·물리·규칙·발명·형식·결말 위험을 구별. 해당 근거와 작은 repair 범위를 제안 |

**기계 gate 현재:** workflow의 필수 marker 검사. **기계 gate Target:** source/scene 참조 무결성·필드 구조 확인. **의미 gate:** precondition의 문장이 실제 앞 장면에서 성립하는지, 전환이 보이는지. **복구:** 씬 안 실행만 바꾸면 14, 인과 구조 변경이면 13, 원문 의무 변경이면 10으로 이동. 단어 'Required preconditions'가 존재한다고 인과 검증 성공으로 표시하지 않는다. M01 참고, Scene IR은 계속 STS만 소유한다.

### 15. 대본·대사 집필

**입력:** 승인 Scene Blueprint와 10~13 자료, 앞 PART의 transfer capsule. **산출:** 읽고 연기할 수 있는 실제 PART·합본 대본. **owner/gate:** STS writer / 기존 PART 검토 후 16 진단·수정·최종 release.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 15.1 제한된 작업 범위 | 요청 PART에 속한 SCN, 진입 상태, 마지막 완료 씬, 이번에 써야 할 내용을 잠금. PART 1~4를 기승전결/4막으로 해석하지 않음 |
| 15.2 행동 먼저 | 장면을 볼/들을 수 있는 행동·상호작용으로 쓴다. 심리 해설은 승인된 서술 장치 또는 연기 가능한 행동으로 변환 |
| 15.3 대화 작용 | 필요한 발화 묶음마다 화자 의도, 청자의 직전 행동/말, 숨기는 것·드러낼 정보, 예상 반응, 실제 결과를 짧은 집필 노트로 확인 |
| 15.4 한국어 실문장 | 관계에 맞는 호칭·높임말·생략·말끝을 적용하고 변할 때 이유를 둔다. 각 화자가 같은 세련된 문장을 반복하지 않게 하고 발화 길이의 차이를 기능에 연결 |
| 15.5 반응·침묵 | 직전 발화를 들었기 때문에 다음 말/행동이 나오는지 확인. 침묵·말 끊김·겹침은 의미를 바꿀 때 사용. 모든 문장을 기계적으로 짧게 만들지 않음 |
| 15.6 리딩·압축 | 실제 소리 내어 읽거나 사람이 읽은 기록이 있으면 측정 시간과 걸린 지점을 기록. 모델의 예상 시간은 추정으로 표시. 줄인 뒤 지식 전달·관계 변화가 사라졌는지 재확인 |
| 15.7 경계·합본 | 각 PART 끝의 상태·지식·소품·열린 약속·다음 precondition을 capsule에 남긴다. 합본에서는 capsule을 제거하고 빠진/겹친 씬·경계 상태를 확인 |
| 15.8 포맷 검사 | 한국어 인물명·동시대사·강제 action·씬 번호·주석을 외부 포맷과 대조. 지원하지 않는 표현은 진단을 남기고 의미를 지운 채 성공으로 처리하지 않음 |

**기계 gate:** canonical SCN 범위·순서, capsule 존재/합본 제거, 포맷 구조. **의미 gate:** 각 대사가 캐릭터의 지식 범위 안에 있는지, 필요한 정보가 전달되는지, 행동으로 구현되지 않은 심리 의무가 남는지. **복구:** 어색한 문장만 15.4~6, 새로운 플롯/앎이 필요하면 14 또는 11. **근거:** M02·M05. 대화 작용 노트는 작업 자료이며 출력 대본에 모두 삽입하지 않는다.

현재 `fountain.rs`는 외부 입력을 plain text와 통계로 정규화하며 SRC/REL IR을 생성하지 않는다고 명시한다. `^`를 인물 큐에서 제거하는 코드가 있어 동시성 구조 보존을 기본 보장으로 삼을 수 없다. vendored Storyboarder parser도 공식 문법 전체를 보증하지 않는다. 외부 라이브러리 도입보다 **지원 문법 목록·손실 경고·실제 fixture 결과**를 먼저 확정할 필요가 있다.

### 17. 텍스트 콘티

**입력:** 현재 STS release·scene map·선택 씬·semantic occurrence·base board. **산출:** schema에 맞는 실제 panel JSON, 렌더링된 텍스트 콘티, 후보 diff. **owner/gate:** NVC storyboard / exact `contentHash` review.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 17.1 현재 정본 결속 | release/package/scene-map hash와 taskHash·baseHash를 확인. 과거 승인본이 현재 source처럼 선택되지 않게 함 |
| 17.2 보여줄 변화 추출 | 씬의 선택·정보 변화·정서 압박을 읽고 패널로 나눌 실제 동작 단위를 고름. 원문 한 문장당 한 컷을 강제하지 않음 |
| 17.3 구도 | composition에 보이는 인물/물체·크기·앞뒤 관계·가리는 정보와 이유를 씀. '영화적으로' 같은 형용사 대신 관객의 실제 접근 정보를 명시 |
| 17.4 동선·시선 | staging에 시작/끝 위치·이동·시선 대상·화면 방향의 연속을 씀. 방향을 바꾸면 관객이 공간을 다시 이해할 수 있는 연결을 제시 |
| 17.5 소리·전환 | soundNotes에 승인된 대사·환경음·침묵을, transition에 행동/시선/소리/정보가 다음 패널로 이어지는 이유를 씀. 새 대사·사건은 추가하지 않음 |
| 17.6 정확한 원문 참조 | 모든 sourceRef는 같은 씬의 semanticId와 entryHash를 사용. 같은 문장 문자열의 서로 다른 occurrence는 각각 유지하고, 한 occurrence의 다중 패널 참조는 허용 |
| 17.7 읽기·가정·수정 | 콘티만 읽고 사건·정보 순서·공간이 이해되는지 검토. 새 카메라/공간 선택은 assumptions, 사건 수정 요구는 openQuestions에 기록하여 STS로 반환. 유지한 패널 ID는 보존 |

**현재 기계 gate:** scene identity/order, 전역 panelId 중복, scene 바깥 참조, entryHash, `required <= covered`, task 취소·source 변경, approval 대상 hash. 승인에는 openQuestions가 없어야 한다. 전체 package에는 모든 현재 씬이 필요하다. **의미 gate:** reference를 연결했어도 자유 서술한 구도·동선·소리가 실제 원문 의미와 맞는지 사람이 판단한다. **복구:** 연출 문제는 선택 씬 후보 재작성, 사건·대사 변경은 STS 16 수정과 재release 후 새 콘티 task. M03·M04 참고.

현재 v1에는 panel duration·shot type enum·lens 등의 독립 필드가 없다. 시간/샷 설명이 필요하면 현재 허용된 서술 필드 안에 추정으로 적을 수 있으나, 기계적인 시간 합계·리듬 검사에는 producer의 versioned schema와 qualification이 필요하다. 이 메모가 새 필드를 허용하지 않는다.

### 18. 판본 패키지

**입력:** 현재 geultu authoring bundle, 동일 synopsis에 결속된 NVC release, 전체 승인 storyboard와 review receipt. **산출:** 실제 파일과 마지막에 쓰는 `story-package.json`. **owner:** integration 조율, 문서·승인 정본은 각 producer 유지.

| 미세 작업 | 정밀 처리와 남길 결과 |
|---|---|
| 18.1 출고 profile | 이번 결과가 텍스트 제작 패키지임을 명시. 문서 package를 영상/이미지 완성으로 부르지 않음 |
| 18.2 판본 선택 | source/story/synopsis의 revision과 STS의 synopsis hash, 현재 release·board source·boardHash·review를 연결 |
| 18.3 파일 목록 | v1 whitelist에 포함된 실제 파일만 수집. 새 개발 기록·연구 노트·작가 메모 추가는 별도 명시 export 또는 versioned 계약으로 설계 |
| 18.4 byte 검증 | producer가 내놓은 bytes와 hash를 확인. 서로 다른 직렬화 결과의 hash를 교체해 승인 결속을 혼동하지 않음 |
| 18.5 사람의 전달물 | 제목·읽는 순서·대본·씬·콘티·미결 상태를 확인. Target provenance 설명이 필요하더라도 v1 manifest 의미를 몰래 확장하지 않음 |
| 18.6 완료·재개 | 파일을 immutable로 발행하고 manifest를 마지막 완료 표식으로 작성. 중단 뒤 같은 파일은 byte equality일 때만 재사용 |
| 18.7 출고 후 검사 | 목록 누락, 변조, 다른 판본의 receipt, 부분 콘티, 오래된 synopsis를 구분해서 실패시킴. 새 판본은 새 출력 위치/정체성으로 추적 |

**현재 gate:** geultu 현재 synopsis와 STS source 일치, producer hash, 허용 파일, 현재 승인·전체 콘티. **Target gate:** 독립 package verifier로 전달 후에도 목록/hash/authority를 재검증하는 실행 경로. **복구:** 파일 손상은 원 producer의 정확한 bytes로 재발행, source mismatch는 STS 재처리·검토, 부분 콘티는 17 나머지 씬 완성. 실패 패키지에서 manifest를 먼저 만들어 완료로 보이게 하지 않는다. M06은 byte 검증 관점의 근거이며 작가 승인 체계를 대체하지 않는다.

## 5. 중요한 음성 사례와 양성 대조

아래는 **추가할 qualification 사례 제안**이다. 테스트를 실행한 결과가 아니다. 사람의 의미 판단이 필요한 항목을 deterministic PASS로 위장하지 않는다.

| 사례 | 기대 판단 | 복구/양성 대조 |
|---|---|---|
| 회상 장면을 먼저 보여준 뒤, 인물이 실제로 배우기 전 그 비밀을 말함 | 정보 누수 | 제시 순서만 바뀌고 인물의 당시 지식이 유지되는 회상은 허용 |
| 열쇠를 버린 다음 씬에서 설명 없이 그 열쇠로 문을 엶 | 상태 전제 실패 | 회수/전달이 승인된 앞 사건에 있으면 통과 가능 |
| 결말에 필요한 행동이지만 인물이 감수할 이유가 없음 | 동기 설명 검토 필요 | 충동·실수·강요를 근거로 한 행동은 의도 부재만으로 기계 거절하지 않음 |
| 대사 압축 후 결말을 이해할 유일한 단서가 사라짐 | 보호 의무 누락 | 동일 의미가 승인된 행동/소리로 이미 전달되면 압축 가능 |
| `@민서`와 동시대사 `^` 입력을 일반 대화로 flatten | format 의미 손실 진단 | 지원 subset에서 손실 없이 처리한 표본은 인정 |
| 같은 문장이 SCN 내 두 번 등장하나 한 semanticId만 콘티에 연결 | occurrence 누락 | 서로 다른 두 ID가 모두 포함되어야 함 |
| 한 semanticId가 세 패널에 반복됨 | 반복만으로는 실패 아님 | 모든 occurrence를 덮고 각 참조 hash·씬이 맞아야 함 |
| ref는 맞지만 staging에 대본에 없는 폭행·정보 발견을 씀 | 기계 참조 검사는 통과해도 의미 검토 실패 | 연출 가정만 바꾸고 승인된 사건은 보존 |
| 해시는 전부 맞지만 review가 다른 boardHash를 승인함 | 현재 승인 결속 실패 | 현재 source와 정확한 boardHash의 승인 필요 |
| 파일 일부만 있고 완료 manifest 없음 | 미완료 | 재시도 때 기존 파일이 정확히 같으면 이어서 발행 |

## 6. 구현 전 남은 확인

1. 11~15의 새 구조화 필드를 넣을 경우 STS artifact/validator를 어느 정도 엄격하게 만들지 owner SPEC에서 결정한다. 자연어 원장을 JSON으로 포장하는 것만으로 의미 검증을 얻지 않는다.
2. Fountain intake의 지원 목록과 경고 정책, 내보내기 필요성은 별도 feature 범위로 정한다. 정적 코드 관측만으로 호환성 PASS를 주장하지 않는다.
3. 콘티 duration과 provenance 확장은 v1 schema의 `additionalProperties: false`·package whitelist를 보존하며 별도 계약으로 결정한다.
4. 이 메모는 읽고 확인한 로컬 파일에 근거한다. 앱 실행·모델 결과·한국어 리딩·실제 사용자 원고에 대한 품질 증거는 **NOT_RUN**이다.
