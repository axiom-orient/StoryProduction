# StoryProduction — 연구에 근거한 단계별 정밀 작업 명세

2026-09-22 · **Target specification**. 현재 native text/package 실행 근거는 [root status](IMPLEMENTATION_STATUS.md)와 [integration status](../integration/docs/IMPLEMENTATION_STATUS.md)에 둔다. 실제 사용자 원고·모델 품질·사람의 채택은 **NOT_RUN**이다.

사용자가 보는 큰 작업은 5개 구간으로 묶는다. 각 구간은 아래 18개 Target 작업을 찾기 위한 길잡이다. 18개 작업이나 144개 세부 지침은 모델 호출·실행 노드·사용자 승인 횟수가 아니며 현재 CLI의 workflow graph도 아니다.

| 사용자 작업 구간 | Target 단계 | 사용자가 완성하는 것 |
|---|---|---|
| 원문 이해 | S01–S02 | 원문 근거, 의도와 보호할 의미 |
| 이야기 개발 | S03–S08 | 이야기 방향, 실제 본문, 검토·채택 판본 |
| 각색 보존 | S09–S12 | 시놉시스 보존 관계, 원본 해석과 각색 계약 |
| 장면 완성 | S13–S17 | 트리트먼트, 씬, 대본, 텍스트 콘티 |
| 판본 출고 | S18 | 선택한 결과물과 승인·파일 무결성 |

이 문서는 Target만 정한다. 현재 동작은 owner의 공개 API와 Current 문서가 정본이다. 웹의 공식 자료·원논문·저자 GitHub에서 확인한 20개 출처를 사용했다.

## 1. 설계 정본

| 자료 | 역할 |
|---|---|
| 이 문서 | owner·입력·근거·판정·복구·분기·qualification의 공통 규칙 |
| [원문·충실도 연구 및 45작업](research/2026-09-22-fidelity-methods.md) | 01·02·09·10·16의 작업표·입출력·근거·실패 처리 |
| [집필·퇴고 연구 및 50작업](research/2026-09-22-authoring-methods.md) | 03~08의 작업표·집필 절차·부분 수정·문체 |
| [각색·콘티·출고 연구 및 49작업](research/2026-09-22-adaptation-methods.md) | 11~15·17~18의 작업표·실제 STS node 대응·복구 |
| [한국어·근거 위치 연구](research/2026-09-22-korean-evidence-methods.md) | 원문 byte 위치·형태소 분석의 한계·한국어 높임 관계 |
| [집필 워크북](STORY_DEVELOPMENT_WORKBOOK.md) | 실제 원고를 읽고 사용자와 작업할 양식 |

공통 의미는 이 문서, 단계별 세부 절차와 사례는 연구 메모가 소유한다. 별도 파생 JSON과 생성기는 유지하지 않는다. 새 필드명은 구현 목표의 의미 사전이며 기존 CLI에 그대로 보낼 수 없다.

## 2. 18개 단계의 결과와 되돌아갈 위치

각 행의 실제 미시 작업은 위 연구 메모에 수록한다. `기존`은 source 연결의 관측이며 runtime 성공 인증이 아니다. 구조화된 신규 기록·새 검사·인계 보강은 Target이다.

| 단계 | 정밀하게 완성할 결과 | 현재 기반 / 보강점 | 실패 시 우선 복구 |
|---|---|---|---|
| S01 원문·근거 | 원문 snapshot, 읽기 단위 coverage, 별칭·화자·시간·부정·믿음 후보, 정확한 span | geultu 원문 저장 / 위치와 회수 범위 신규 | S01 분석. 원문 변경이면 새 revision |
| S02 의도·보호 | 명시 의도와 해석 분리, 보호할 사건·의미·행위자·관계·형식, 허용 변환 | Brief / 근거와 선택 결속 보강 | S02 충돌 해결 |
| S03 핵심·방향 | 중심 질문, 욕망·대가·최종 선택, 로그라인과 방향별 tradeoff | 집필 지침 / 개발 기록 신규 | S03; 보호 의미는 S02 |
| S04 인물·관계 | 원문 사실과 가설, 방향별 관계, 앎·믿음, 동기·행동·대가 | geultu 노트 Target | S04; 방향은 S03 |
| S05 인과·개요 | 사건과 제시 순서, 계층 개요, 필수 선택, 복선·회수, 의도된 pacing | story 지침 / 참조와 기록 신규 | S05; 동기는 S04 |
| S06 글투 | 실제 구절 근거, 시점·거리·리듬, 한국어 관계별 말투, 시험 문단 | Brief·voice / 근거·관계별 관측 보강 | S06; 보호 정책은 S02 |
| S07 본문 | 문맥 포함 목록, 선택 구간의 실제 이야기, 계획/본문 대응·가정·질문 | prepare/submit/selection / 맥락·의무 검사 보강 | S07; 구조는 S05 |
| S08 퇴고 | 근거 있는 결함과 선호 분리, 수정 범위, 전후 diff, 새 문제, 판본 채택 | review/edit/restore / 진단과 영향 설명 보강 | S08; 큰 인과는 S05 |
| S09 시놉시스 | 제작 요약, 근거 없는 추가와 중요한 누락을 별도 검사, N:M 보존표 | synopsis/handoff / companion 신규 | S09; 원 스토리는 S08 |
| S10 원본 해석 | SRC/REL/GAP/CON, 사실/믿음/주장/미정, Ending Contract, source 승인 | STS step_00 / companion public intake 신규 | S10 또는 geultu 새 인계 |
| S11 각색 계약 | 매체·분량·압력, Fabula/Syuzhet, 관객/인물 정보, 변환·외화 계획 | STS step_01/02·A | S11; 원본 의미는 S10 |
| S12 상태·공간 | 목표를 촉발한 사건, 지식·소품·접근·위치·세계 규칙의 잠정 전이 | STS step_03/04·B | S12; 정보 설계는 S11 |
| S13 트리트먼트 | 처음부터 끝까지 인과·선택·결과로 읽히는 실제 전체 전개 | STS step_05 | S13; 인물/규칙은 S12 |
| S14 씬 전이 | SCN별 기능·진입 상태·행동·turn·퇴장 상태·후속 의무 | STS step_06·C | S14; 전체 전개는 S13 |
| S15 대본 | 행동·대사·지문, 발화의 작용·반응, PART 검토와 합본, format 지원 범위 | STS step_07·PART gates | 해당 PART; 씬은 S14 |
| S16 수정·검증 | 진단, 범위 승인, 수정 대본, 독립 역복원, 양방향 비교, 최종 release | STS step_08·Human gates | 기존 repair cycle; source는 S10 |
| S17 텍스트 콘티 | 패널 목적·구도·동선·시선·소리·전환, occurrence coverage와 검토 | NVC storyboard | S17; 사건 변경은 STS |
| S18 판본 출고 | 같은 판본의 실제 문서·승인·hash·파일 목록과 마지막 manifest | integration package/v1 | 손상 파일의 producer 또는 상위 owner |

S04~05의 인물·인과 노트는 geultu의 집필용 가설이다. NVC가 이를 그대로 정본으로 승격하지 않는다. S12에서는 source/event 기반의 잠정 상태만 만들고, 실제 `SCN-*` 결속·씬 전후 검산은 S14에서 수행한다. PART 1~4는 현재 실행의 집필 분할이며 4막 서사 구조라는 뜻이 아니다.

## 3. 연구에서 채택한 방법과 채택하지 않은 동작

| 근거 ID | 적용할 구체적 방법 | 제한 또는 제외 |
|---|---|---|
| A01 Re3 | 구간마다 계획·직전 사건·인물 자료를 회수하고 충돌 근거 제시 | 수정했다고 `is_consistent=True`로 확정하는 의미, 고정 3인칭, 오래된 API 의존 제외 |
| A02 DOC | 상위 사건→세부 실현 조건→본문 대응 | token-logit controller를 일반 프롬프트 API로 재현했다고 주장 금지 |
| A03 Dramatron | 로그라인·인물·사건·대사 수준별 후보와 사람의 편집 | 완성 원고도 로그라인부터 재생성하는 강제 경로 제외 |
| A04 CONCOCT | 중요한데 모호한 개요 항목을 먼저 구체화 | 모든 구간의 분량·리듬 균일화 제외 |
| A05 Wordcraft | 원문 유지 옵션과 선택 구간의 표현 후보 | 공개 코드의 현재 모델과 연구 당시 모델을 동일시하지 않음 |
| F01~03 | 원자 주장·질문/답 대조·문장 간 모순 신호 | parsing 불명확 시 지원 추정, 첫 후보만 평가, 앞 100구간 절단을 그대로 이식하지 않음 |
| F04 | 원문 위치별 회수·미검사 범위를 관찰 | 긴 context에 넣었다는 이유로 전수 확인이라 판단 금지 |
| F05~06 | 근거가 있는 관점별 진단과 필요할 때만 A/B·B/A 비교 | 평균 점수·인기도·모델 자신감으로 자동 채택 금지 |
| M01 | 사건 인과와 인물이 행동할 의도를 별도 연결 | 비합리적 선택·사고·충동을 전부 오류로 취급하지 않음 |
| M02~03 | stable ID/표시 번호, 지원 Fountain subset, 추정 시간/측정 시간 분리 | 전체 format 호환·한국어 실연 시간 측정 완료라는 주장 제외 |
| M04~05 | 관객에게 보이는 정보, 발화의 작용·반응·리딩 | 모든 대사·장면에 같은 연출 공식을 강제하지 않음 |
| M06·K03 | 파일 완전성·byte 일치·근거 위치를 분리해 검증 | checksum으로 사람의 승인·작품 품질 인증 금지 |
| K01~02 | 원문 기반 한국어 분석과 화자/청자/주체별 높임 관계 | 자동 교정으로 작가의 방언·생략·무례·친밀감 지우기 금지 |

각 근거의 논문 URL·학회·평가 범위, GitHub SHA·라이선스·실제 코드 위치는 연구 메모에 기록했다. 연구 결과로 직접 입증되지 않은 한국어 적용과 새 계약은 설계 추론이다. 외부 코드를 제품에 복사하거나 runtime dependency로 추가하지 않았다.

## 4. 모든 작업의 입력과 출력 의미

### 4.1 작업 패킷

새 구조화 패킷을 구현할 때 필요한 의미는 다음과 같다. 기존 `Task`, STS task/gate token, storyboard task의 필드를 무단 확장하지 않는다. 실제 producer schema와 버전은 해당 owner가 정의하고 consumer copy는 integration registry에서 pin한다.

| 의미 | 필수 내용 | 목적 |
|---|---|---|
| 작업 identity | owner, owner-local task ID, operation, input snapshot refs | 다른 작업·판본의 결과 혼합 방지 |
| 자료 역할 | original / adopted story / synopsis / author directive / reference / candidate | 참고 자료를 승인이나 명령으로 올리지 않음 |
| 수정 범위 | base revision/hash, selected IDs, allowed changes, protected obligations | 선택 밖의 편집과 이야기 의미 보존 |
| 필수 문맥 | required refs, included refs, omitted refs, truncation/overflow, 사용 한도 | 누락한 원문까지 검토했다고 주장하지 않음 |
| 근거 | typed source refs, exact spans, adopted decision refs | 모든 주요 판단을 확인 가능한 자료에 연결 |
| 실행 한도 | timeout, 최대 후보/수정 횟수, 최대 출력, 외부 writer identity | 무한 생성·숨은 재시도 방지 |
| 결과 요구 | 실제 본문 형식, 진단 기록의 별도 위치, 반환 상태 | 분석·계획을 완성 본문으로 제출하지 않음 |

한도 값은 선택한 writer의 실제 capability와 사용자의 작업 범위로 정한다. 연구 논문의 token 수를 현재 모델의 안전한 한도로 복사하지 않는다. 한도 때문에 필수 근거를 넣을 수 없으면 범위를 나누거나 `CONTEXT_INCOMPLETE`를 반환한다. 외부 모델·새 공급자 자동 전환은 하지 않는다.

Target 작업 안내는 사용자가 만드는 결과, 현재 근거, 검토할 실제 후보, 바뀌는 점과 영향, 가능한 다음 선택을 짧게 보여준다. 계획·분석만 제시하고 실제 본문을 만들지 않은 경우 완성 결과로 표시하지 않는다. 가역적인 표현 선택은 후보와 가정을 제시하고 매번 승인을 요구하지 않는다. 결말·정체·핵심 관계 등 이야기 의미가 달라지는 선택은 실제 대안에 대한 판단을 받는다. 이 안내 형식은 특정 제품 화면을 요구하지 않고 현재 CLI 기능도 주장하지 않는다.

### 4.2 근거와 주장

`EvidenceRef`는 owner, artifact identity, revision, hash algorithm, hash domain, raw UTF-8 `[start_byte,end_byte)`, exact quote를 갖는 방향으로 설계한다. 0 기반·end 제외이며 멀티바이트 문자 중간에서 끊는 범위는 거절한다. 동일 구절의 반복은 위치로 구별한다. NFC/NFD·개행 변환을 위한 display view는 원문과 별도다. W3C TextPositionSelector의 문자 위치를 raw byte 위치로 그대로 쓰지 않는다.

raw 파일 SHA-256, geultu blocks의 content hash, export markdown hash, NVC canonical artifact contentHash는 서로 다른 domain이다. 같은 글처럼 보여도 서로 치환하지 않는다. source/hash가 바뀌면 exact quote가 우연히 같더라도 옛 검토를 자동 재결속하지 않는다.

`Claim`은 명제뿐 아니라 actor/target, polarity, epistemic, holder/speaker/viewpoint, narrative time, narrative frame, evidence를 필요에 맞게 갖는다. `FACT/BELIEF/CLAIM/UNKNOWN`과 `AFFIRM/NEGATE`는 다른 축이다. 꿈·편지·가정·믿을 수 없는 화자의 발언도 frame을 보존한다. 복합 부정의 scope를 단일 boolean으로 함부로 축약하지 않는다.

`Relation`은 CAUSES/ENABLES/PREVENTS/BEFORE/AFTER/KNOWS/BELIEVES/IDENTITY/RELATIONSHIP/WORLD_RULE의 의미와 적용 시간을 구별한다. 원문에 나란히 등장하는 두 사건을 자동 인과로 만들지 않는다. 개요 parent-child cycle와 같은 기준의 strict BEFORE 모순은 거절하되, 작가가 의도한 인과 고리·시간 여행은 발생 회차와 시간 의미를 따로 검토한다.

`Obligation`은 보호할 의미·범위·사용자 결정·허용 변환을 기록한다. 모델이 중요하다고 판단한 CORE와 사용자가 변경을 금지한 protected는 다른 축이다. `Alignment`는 N:M 대응·변환 방식·실제 출력 근거·판정 이유를 가진다. COMBINED/DRAMATIZED라는 표지만 붙인 것은 보존의 증거가 아니다.

### 4.3 한국어의 별도 검사

생략된 주어·동명이인·가족 호칭은 자동 entity 병합에 주의한다. 확정되지 않은 후보는 유지한다. 한국어 대사는 **화자→청자**, **화자→문장 속 주체**, 공식/사적 상황, 관계 변화 사건을 분리해 검토한다. 높임말이 많을수록 좋은 대사라는 기준을 쓰지 않는다. 형태소 분석기는 구절 관측의 보조 도구이며 문체·비꼼·친밀감·관계의 정본이 아니다.

표본이 적으면 관측이 부족하다고 표시한다. 실제 원문을 읽고 짧은 시험 문단을 비교한 다음 작가가 선택한 지침으로 집필한다. 영어 평가나 번역 fixture를 한국어 작품 qualification으로 대체하지 않는다.

## 5. 검사·검토·채택을 나누는 규칙

작업 수행 상태, 의미 판정, 사람의 채택은 서로 다른 축이다. 아래 용어는 설계 의미이며 기존 enum 이름을 일괄 교체하지 않는다.

| 축 | 기록할 상태·내용 | 금지되는 변환 |
|---|---|---|
| 실행 | 준비/실행 중/결과 저장/FAILED/CANCELLED/OUTCOME_UNKNOWN | timeout·빈 응답·파싱 실패를 성공 또는 SUPPORTED로 바꾸기 |
| 구조 검사 | PASS/FAIL/NOT_RUN와 실제 검사 범위 | ID·hash가 맞다는 이유로 의미 검토 통과 |
| 의미 근거 | SUPPORTED/CONTRADICTED/UNSUPPORTED/UNCERTAIN와 양측 evidence | 검색 불완전을 원문에 없는 사실로 단정 |
| 해석·취향 | 결함 종류, 우려, 관점별 이유, 선택지 | 선호를 원문 충실도 위반과 같은 척도로 합산 |
| 사람의 검토 | candidate/input hash, scope, actor, 수용한 미정, 결정 | 모델의 추천·READY를 Human release로 대체 |
| 최신성 | current/stale와 원인이 된 upstream 변경 | 부분 수정·옛 판본 복원으로 하위 승인을 부활 |

`UNSUPPORTED`는 정해진 검토 범위를 실제로 확인했지만 지지 근거가 없는 경우다. 그 범위가 작품 전체가 아니면 전체에 대한 부재 주장도 하지 않는다. `UNCERTAIN`은 근거 부족·상충·모호한 지시·부분 검색을 포함한다. 두 상태 모두 정당하게 승인된 창작 추가인지 따로 검토할 수 있다.

기계 검사는 형식·identity·허용 범위의 오류를 거절한다. 자연어 의미 판단은 evidence가 있는 진단을 만든다. 사람이 검토한 source 의미·보호 조건의 위반은 release blocker로 남기고 취향 점수로 상쇄하지 않는다. ‘일관성 90점 이상’ 같은 합격선을 한국어 관측 없이 만들어 넣지 않는다.

### 5.1 양방향 보존 검사

1. **산출물 → 근거:** output의 중요한 주장·관계마다 원문 또는 승인된 발명 결정을 찾는다. 근거 없는 추가·과장·행위자 변경·부정 반전·믿음의 사실화를 찾는다.
2. **보존 의무 → 산출물:** protected/core source 의무마다 실제 출력에서 표현된 위치와 기능을 찾는다. 정확한 문장만 쓴 요약도 중요한 결말을 빼면 이 검사를 통과하지 못한다.
3. **연결 검사:** 사건 A와 B의 등장만 확인하지 않고 A가 B를 일으키는 이유·인물의 정보 획득·규칙의 비용까지 대조한다.
4. **범위 검사:** checked/total/unexamined와 분모의 정의를 함께 기록한다. 분모 0은 N/A이며 100%가 아니다. 텍스트가 들어간 비율과 의미가 보존된 비율은 다른 관측치다.

### 5.2 수정 순서와 독립 검증

기존 STS의 다음 순서를 보존한다.

```text
step_08_diagnosis
  → step_08_scope (Human: 수정 범위)
  → step_08_apply (승인 범위의 실제 수정)
  → step_08_reverse (대본만으로 새 컨텍스트에서 복원)
  → step_08_verify (승인 Source IR ↔ 복원 IR)
  → final_release (Human: 최종 판본)
```

역복원에는 원문·의도·blueprint·과거 대화를 주지 않는다. 실행기가 source를 조회할 도구나 같은 대화 기억을 허용하면 입력 파일 목록만으로 독립 검증을 인증할 수 없다. 입력 manifest·허용 도구·실제 실행 격리를 함께 확인하고, 불가능하면 `SOURCE_BLIND_UNVERIFIED`로 남긴다. 비교 단계는 승인 Source IR과 복원 IR을 사용하며 의도 문서로 대본의 결손을 메우지 않는다.

수정 후 같은 진단이 반복되면 설정된 횟수 안에서 멈추고 unresolved와 선택 가능한 대안을 반환한다. 새 provider를 선택하거나 승인 범위를 넓혀 계속 쓰지 않는다. outcome unknown은 기존 attempt의 관측·복구를 먼저 수행한다. 취소된 task의 늦은 결과는 보관할 수 있어도 자동 채택하지 않는다.

## 6. 단계가 실제로 판단해야 하는 예시

다음은 **설계 검토용 합성 문장**이며 사용자의 작품이 아니다.

> 미나는 준이 떠났다고 믿었다. 준은 창고에 숨어 있었다. 미나는 마지막에 편지를 태우지 않고 돌려주었다.

S01은 `미나의 BELIEF(준이 떠남)`, `FACT(준이 창고에 있음)`, `FACT(미나가 편지를 돌려줌)`을 각각 근거 위치에 연결한다. S02에서 사용자가 ‘편지를 돌려주는 마지막 선택’을 보호했다면, **사건·선택 주체·관계 의미**를 구별해 기록한다.

| 후보·변형 | 관측할 문제 | 돌아갈 작업 |
|---|---|---|
| “준이 떠나자 미나는 편지를 돌려준다.” | 믿음을 사실로 바꿨고, 인과 관계도 새로 추가했을 수 있음 | S09.5~09.8에서 양측 근거 확인 |
| “미나는 준이 떠났다고 생각한다.”로 끝난 요약 | 쓴 문장은 맞지만 보호한 마지막 선택이 빠짐 | S09.6의 obligation→output 검토 |
| 대본에서 조연이 편지를 돌려줌 | 사건은 남아도 결정적 행위자가 바뀜 | S16 protected/agency 검토 후 승인된 repair |
| 대사 없이 미나가 편지를 준의 손에 놓음 | 문자 그대로의 원문이 없어도 행동으로 의미를 보존할 수 있음 | S16 역복원에서 실제 행동 근거 검토 |
| 같은 편지 행동이 두 콘티 패널에 걸쳐 표현됨 | 반복 참조 자체는 오류가 아님 | S17의 전체 occurrence coverage와 내용 검토 |
| 참조는 맞는데 staging에 “편지를 불태운다”를 추가 | 구조 검사와 의미 검토가 다른 결론을 낼 수 있음 | S17 연출 문장 수정 또는 STS 의미 변경 검토 |

이런 최소 대조쌍으로 ‘정확해 보이는 결과’와 ‘실제로 의미를 보존한 결과’를 구별한다. 이 문서를 썼다는 사실은 해당 검출기가 이미 동작한다는 증거가 아니다.

## 7. 경로·캐시·동시 편집·출고

### 7.1 입력 형태별 경로

카탈로그의 의존 관계는 현재 목표인 전체 텍스트 제작 경로를 표현한다. 다음 선택 경로는 별도 owner 구현과 검증이 필요한 Target이다.

- 메모·단편 소재: S01~08에서 발전시켜 채택한 뒤 S09부터 각색한다.
- 완성 원고·표현 수정만 요청: S01·02·06에서 근거와 범위를 잡고 기존 본문을 명시적으로 채택/편집한다. S03~05의 새 인물·새 결말 제안을 강제하지 않는다. 현재 `next`의 Story/Synopsis 순서를 몰래 건너뛰지는 않는다.
- 읽는 원고만 출고: S08의 채택 판본을 별도 원고 출고 profile로 내보내는 Target이다. 현재 전체 `package`에는 STS·콘티가 필요하며 이를 원고 출고로 위장하지 않는다.
- 숏폼·웹툰 등: 공통 원문에서 별도 adaptation revision으로 분기한다. 원작 판본을 덮지 않는다. 매체별 계약이 없으면 지원 완료라고 광고하지 않는다.

### 7.2 재사용과 수정 영향

재사용 가능한 것은 current 입력에 결속한 파생 관측이다. source hash, guidance, 채택한 결정, selected scope, operation/method version이 달라지면 기존 결과를 그대로 최신으로 승격하지 않는다. 같은 요청 ID+같은 bytes만 멱등이며 다른 bytes는 충돌이다.

수정 영향표는 실제 참조 관계에서 계산한다. 표현 수정은 선택 블록과 관련 맥락을 검토하고, 결말·인물 지식·규칙 변경은 의존하는 사건·씬·콘티까지 표시한다. 필요한 참조가 없으면 영향은 UNKNOWN이다. 모델이 추정한 관계만으로 ‘다른 장면에는 영향 없음’을 보장하지 않는다. 지워지는 정보와 복선은 삭제 전에 드러낸다.

owner의 CAS와 public 명령을 사용한다. integration이 geultu journal·NVC state 파일을 직접 수정하지 않는다. 이전 판본 복원은 새 revision이며 옛 승인 부활이 아니다. 여러 component를 하나의 transaction으로 잠갔다고 주장하지 않는다.

### 7.3 텍스트 콘티와 출고의 정확한 경계

NVC 텍스트 콘티의 현재 구조 검사는 해당 씬의 모든 occurrence를 **적어도 한 번** 참조하게 한다. 한 occurrence를 여러 패널에서 참조할 수 있다. 같은 대사가 원 씬에 두 번 있으면 서로 다른 occurrence ID를 둘 다 덮어야 한다. cinematic plan의 정확히 한 shot 배정 규칙을 텍스트 콘티에 가져오지 않는다.

패키지는 현재 source/story/synopsis, scene blueprint, screenplay, scene map, 전체 storyboard와 review로 구성한다. v1의 whitelist와 manifest 의미를 임의로 확장하지 않는다. 개발 노트·근거 원장·연구 자료를 넣으려면 명시 export 또는 versioned producer 계약을 먼저 정한다.

수집한 bytes·producer hash·current approval·판본 연결을 검사하고 파일을 immutable로 쓴 뒤 manifest를 마지막에 기록한다. manifest가 없는 부분 출력은 미완료다. 같은 bytes는 재개에 재사용할 수 있지만 다른 bytes를 덮지 않는다. 전달 후 verifier의 파일 검사와 owner 조회를 통한 현재성 판단도 분리한다. 파일만으로 확인 가능한 것은 판본에 결속된 무결성이며 이후 owner 변경까지 오프라인에서 알 수는 없다.

## 8. 구현 순서와 검증 계획

### 8.1 먼저 구현할 범위

| 순서 | 변경 책임·구체적 위치 | 닫아야 할 기준 |
|---|---|---|
| 0 현재 경로 qualification | geultu authoring·NVC CLI·storyboard·integration의 기존 local gates | 현재 create→후보→review→handoff→STS→board→package 경로와 negative cases. 기존 실패를 PASS로 바꾸지 않음 |
| 1 원문 근거·개발 기록 | geultu `authoring/{mod,api}.rs`, 기존 Fact/Timeline/Thread의 실제 소비자 조사 | 중복 정본 없이 span/결정/개발 후보 저장, journal replay·CAS·부분 수정·stale 의미 유지 |
| 2 보존표·인계 | geultu producer contract/export, NVC public intake, integration registry | 기존 reviewed-synopsis/v1 보존, companion의 선언된 역할·source precedence·byte pin·변경 시 승인 무효화 |
| 3 task packet·진행 표시 | owner의 공개 조회와 native `storyctl` 조율 | 필수 context coverage, missing/unknown 표시, 실제 owner task/gate 대응, 전역 서사 DB 없음 |
| 4 대조·수정·독립 검증 | geultu 보존 검토, NVC 기존 STS task/critic/repair 경로 | 추가/누락·epistemic/time/agency 검출, scope-before-apply, 역복원 접근 격리 |
| 5 판본·매체 경로 | owner export와 versioned package/profile | 선택한 결과물 완료 조건, 실제 파일·current approval, 미지원 형식 명시 |

실제 schema 확정 시 `additionalProperties`, enum, hash domain, ID namespace, 실패/복구, 버전을 producer에 둔다. consumer copy는 byte equality로 pin한다. NVC intake 지원 자료를 기존 다섯 delivery DTO에 섞지 않는다. SMC/SAC/IPC·기존 데이터·Jangmyeondo 경계를 보존하고 제품 화면 코드는 추가하지 않는다.

### 8.2 검증 사례와 품질 실험

연구 메모의 집필·충실도·각색 사례와 이 문서의 공통 경계를 후속 구현에 적용한다. 형식 검사로 잡는 case, 모델 진단이 필요한 case, 사람의 의미 판단이 필요한 case를 실제 구현의 test harness에서 분리해야 한다. 아직 기대 관측을 적은 **test specification**이며 테스트 PASS가 아니다.

필수 범위는 원문 byte/반복 인용/정규화, 믿음·부정·시점·시간·행위자, 원문의 중간 단서·누락·부분 검색, scope/CAS·선택 밖 편집, 판정 파싱 실패·cancel·unknown·late result, format subset, 콘티 반복/누락, package 중단/재개다.

의미 검증을 평가하려면 다음과 같이 실험한다.

1. 실제 사용자가 허용한 한국어 원고를 메모/부분/완성, 길이, 서술/대화, 회상/복합 시점 등의 범주로 구성한다. 작품 단위로 개발·평가 자료를 분리한다.
2. 원문 보존 평가자와 원문 없이 결과를 읽는 평가자의 역할을 나눈다. protected 항목과 최소 의미 변형의 정답·모호한 경우를 먼저 사람이 기록한다.
3. 같은 모델·설정·원문·허용 수정 범위에서 현재 방식과 개선 방식을 비교한다. 원문 근거, 상세 개요, 양방향 대조를 각각 뺀 조건도 비교해 무엇이 도움이 되는지 확인한다.
4. 결함 종류별 false positive/false negative/abstention, 미검사 범위, 사용자 재수정 범위, 검토 시간, 호출 비용을 기록한다. 재미·글투·감정 전달은 실제 본문 비교와 작가 선택으로 평가한다.
5. 반복성·신뢰 범위를 함께 보고한다. 한 fixture·한 작품·모델의 자기 평점을 일반적인 작품 품질 증거로 사용하지 않는다. 관측하지 않은 모델/장르/언어의 성능은 UNKNOWN으로 둔다.

연구 예제나 합성 문장은 사용자 작품이 아니다. 실제 원고가 지정되지 않았으므로 본 요청에서 작품을 작성·채택하거나 외부 writer에 보내지 않았다.

## 9. 이번 작업의 증거 범위

완료한 것은 1차 자료 조사, 고정 GitHub 코드·라이선스 관측, 조사 시점 코드 대응, 세부 작업 지침, 입출력·gate·복구의 설계 의미와 사례 사양이다. 독립 읽기 검토에서 S12의 미래 SCN 의존과 S05의 순환 규칙을 수정했고, S16은 당시 STS의 scope 승인→apply→reverse→verify→release 순서에 맞췄다.

제품 build/test/runtime·외부 연구 코드 재현·한국어 모델 품질·사용자 작품 완성은 이 명세의 결과가 아니다. 설계 검토와 제품 qualification을 합쳐 보고하지 않는다.

이전 파생 카탈로그의 정적 검사 기록은 [당시 snapshot 보고서](research/2026-10-01-live-use-validation-after-owner-cleanup.md)에 보존한다. 현재 문서나 제품 소스의 검증 결과로 적용하지 않는다.
