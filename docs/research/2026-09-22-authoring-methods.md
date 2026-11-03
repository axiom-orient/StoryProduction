# 03~08 스토리 개발·집필·퇴고: 논문과 구현 조사

조사일: 2026-09-22. 상태: **DONE / 조사·Target 설계**, 제품 실행·모델 품질: **NOT_RUN**. [정밀 작업 명세](../STAGE_EXECUTION_SPEC.md)의 S03~S08을 세분화한다. 사용자 원고가 제공되지 않았으므로 실제 작품의 해석·집필·채택 결과는 없다. 아래 필드명과 통과 조건은 구현을 위한 설계 제안이며 현재 호출 가능한 schema/API가 아니다.

## 1. 판단

상세 개요, 본문에 필요한 문맥의 반복 주입, 후보 비교, 제한된 퇴고는 채택할 가치가 있다. 그러나 논문의 오래된 모델·영어 평가 결과를 한국어 작품의 품질 보증으로 옮겨서는 안 된다. **실제 원문 근거 → 집필 가설 → 작가가 채택한 선택 → 후보 본문 → 명시적인 판본 채택**을 나누어 남기는 것이 StoryProduction의 적용 방향이다.

03~06 개발 기록은 geultu의 집필 자료다. NVC의 Source IR·인물 상태·씬 권한을 복제하지 않는다. 새 기록 종류와 producer 계약은 owner가 확정하고 integration은 공개 상태를 읽는다. 스토리 단계에서 생긴 가설은 각색 단계의 승인 사실로 자동 승격하지 않는다.

## 2. 검증한 출처

각 요약은 직접 열람한 논문과 저자 저장소에 근거한다. 검색 결과·2차 해설은 증거로 쓰지 않았다. 커밋은 2026-09-22에 GitHub 공개 API로 조회한 기본 브랜치 HEAD이며, 실제 파일은 해당 SHA의 raw content로 읽었다. 외부 코드는 설치·실행·프로젝트에 복사하지 않았다. 라이선스 표기는 조회한 파일의 선언이며 전체 의존성의 재배포 가능성을 인증하지 않는다.

### A01 — Re3 / EMNLP 2022

[논문](https://aclanthology.org/2022.emnlp-main.296/) · [본문 PDF](https://aclanthology.org/2022.emnlp-main.296.pdf) · [저자 README](https://github.com/yangkevin2/emnlp22-re3-story-generation/blob/3a97ebde04e3333962c2825146897efe1dc87dd8/README.md)

배경·인물·개요를 먼저 만들고, 직전 본문과 이전 사건 요약을 매 구간 프롬프트에 다시 넣는다. 이어쓰기 후보를 인과적 일관성과 premise 관련성으로 비교하고 인물 속성 충돌을 별도로 고친다. 영어 premise 100개, 주 평가 길이 2,000~2,500단어, GPT-3 계열과 rolling baseline 비교다. 논문의 14%p 일관성·20%p 관련성 개선은 이 조건의 결과다. Edit 모듈 자체는 주 지표를 개선하지 못했으며, 오탐과 불필요한 수정도 보고한다. 이를 근거로 자동 오류 탐지가 자동 수정·채택까지 정당화된다고 볼 수 없다. [§3~5와 Limitations](https://aclanthology.org/2022.emnlp-main.296.pdf)

코드 SHA `3a97ebde04e3333962c2825146897efe1dc87dd8`, MIT. [beam_candidate.py](https://github.com/yangkevin2/emnlp22-re3-story-generation/blob/3a97ebde04e3333962c2825146897efe1dc87dd8/story_generation/draft_module/beam_candidate.py)의 `construct_prompt`, `edit_update_contradictions`와 [entity.py](https://github.com/yangkevin2/emnlp22-re3-story-generation/blob/3a97ebde04e3333962c2825146897efe1dc87dd8/story_generation/edit_module/entity.py)의 속성 추출·관계 대조를 확인했다. 수정 뒤 `is_consistent=True`로 설정하는 동작과 고정 3인칭 지침은 StoryProduction으로 가져오지 않는다.

**적용 판단:** 04의 근거 있는 인물 속성 기록, 07의 구간별 문맥 패킷, 08의 충돌 증거 기반 부분 수정에 개념을 적용한다. 과거 API·모델·reranker를 runtime dependency로 채택하지 않는다.

### A02 — DOC / ACL 2023

[논문](https://aclanthology.org/2023.acl-long.190/) · [본문 PDF](https://aclanthology.org/2023.acl-long.190.pdf) · [저자 README](https://github.com/yangkevin2/doc-story-generation/blob/9d727cdbae40c72169ab03b729bff4419a113dac/README.md)

개요를 계층적으로 확장하고 각 세부 사건을 본문에서 실현하도록 controller를 결합한다. 20개 이야기의 상위 개요에 대응하는 passage 비교에서 Re3 대비 일관성 22.5%p, 개요 관련성 28.2%p, 흥미 20.7%p 차이를 보고했다. DOC 평균 길이는 3,875단어다. 전체 소설의 장기 품질이나 한국어 성능 측정이 아니며, 사람 간 평가 일치도도 제한적이다. 상세 개요만 추가하면 동일한 효과가 난다는 실험도 아니다. [Table 1·18·20](https://aclanthology.org/2023.acl-long.190.pdf)

코드 SHA `9d727cdbae40c72169ab03b729bff4419a113dac`, MIT. [outline.py](https://github.com/yangkevin2/doc-story-generation/blob/9d727cdbae40c72169ab03b729bff4419a113dac/story_generation/plan_module/outline.py)는 부모·형제·선행·후행 맥락을 사용해 노드를 확장한다. [fudge_controller.py](https://github.com/yangkevin2/doc-story-generation/blob/9d727cdbae40c72169ab03b729bff4419a113dac/story_generation/common/controller/models/fudge_controller.py)는 token logits를 조정한다. 단순 프롬프트 API에 이 controller를 그대로 붙일 수 있다고 가정하지 않는다. README가 후속 chat-model 구현을 안내하지만 본 조사의 코드 증거는 논문 버전이다.

**적용 판단:** 05의 상위 사건→구간별 실현 조건, 07의 계획/실제 본문 대응표를 채택한다. StoryProduction의 의미 대조는 별도 구현·검증 대상이며 DOC 재현 성공이라고 부르지 않는다.

### A03 — Dramatron / CHI 2023, arXiv 초판 2022

[출판 DOI](https://doi.org/10.1145/3544548.3581225) · [저자 논문](https://arxiv.org/abs/2209.14958) · [본문 PDF](https://arxiv.org/pdf/2209.14958) · [README](https://github.com/google-deepmind/dramatron/blob/2e7c36afadacf8321b77a468940024371b7a8c7a/README.md)

로그라인에서 인물·사건·장소·대화로 내려가는 계층적 생성과 사람의 편집을 연구한다. 영화·연극 전문가 15명이 참여한 약 2시간 협업 연구다. 완전자율 작품 생산 성능을 검증한 연구가 아니며, 공개 공연에 사용한 대본도 상당한 인간 편집을 거쳤다. 연구와 README는 위에서 아래로 계획하는 방식이 모든 작가에게 맞지는 않는다고 지적한다. [연구 설계·논의](https://arxiv.org/pdf/2209.14958)

코드 SHA `2e7c36afadacf8321b77a468940024371b7a8c7a`. 소프트웨어 Apache-2.0, 기타 자료 CC-BY-4.0 선언. [colab/dramatron.ipynb](https://github.com/google-deepmind/dramatron/blob/2e7c36afadacf8321b77a468940024371b7a8c7a/colab/dramatron.ipynb)의 `StoryGenerator.rewrite`는 수준/대상별 수정과 개입 기록을 갖는다. README는 공개 Colab의 모델 접속 부분을 사용자가 구현해야 한다고 명시한다.

**적용 판단:** 03의 작품 핵심 후보, 04의 인물 대안, 08의 변경 수준·대상 기록에 적용한다. 이미 쓴 원고를 로그라인부터 강제로 재생성하지 않는다. 제품 화면 코드는 가져오지 않는다.

### A04 — CONCOCT: Improving Pacing in Long-Form Story Planning / Findings of EMNLP 2023

[논문](https://aclanthology.org/2023.findings-emnlp.723/) · [본문 PDF](https://aclanthology.org/2023.findings-emnlp.723.pdf) · [저자 README](https://github.com/YichenZW/Pacing/blob/7bfe7040c3018a208333e04b3415cdd6122dc2ed/README.md)

두 사건 중 더 구체적인 것을 판정하는 평가기를 학습하고, 가장 모호한 개요 항목부터 확장하며 새 하위 항목도 걸러낸다. 문장 길이 대신 의미의 세밀도를 다룬다. 영어 개요 연구이며 story excerpt의 pacing 선호는 57.18%였다. 다른 품질 지표의 유의한 개선까지 입증한 결과는 아니다. 일정한 pacing을 목표로 했으므로 의도적인 생략·지연·클라이맥스 확장까지 균일화하는 규칙으로 사용하면 안 된다. [§3~4·Table 5·Limitations](https://aclanthology.org/2023.findings-emnlp.723.pdf)

코드 SHA `7bfe7040c3018a208333e04b3415cdd6122dc2ed`. 루트 라이선스 선언은 확인되지 않았다. 내장 `story_generation/LICENSE`의 MIT를 전체 코드에 확대 적용하지 않는다. [run_outline.py](https://github.com/YichenZW/Pacing/blob/7bfe7040c3018a208333e04b3415cdd6122dc2ed/run_outline.py)의 `expand_outline_detail`와 [ranker4outline.py](https://github.com/YichenZW/Pacing/blob/7bfe7040c3018a208333e04b3415cdd6122dc2ed/ranker4outline.py)의 상대 순위 계산을 읽었다.

**적용 판단:** 05에서 중요한 선택이 한 문장으로 압축되었는지, 사소한 연결 사건만 과도하게 상세한지 검토한다. 코드 복사 없이 Story-owned 기준과 한국어 fixture로 다시 설계한다.

### A05 — Wordcraft 전문 작가 연구 / arXiv 2022

[논문](https://arxiv.org/abs/2211.05030) · [본문 PDF](https://arxiv.org/pdf/2211.05030) · [저자 워크숍 설명](https://wordcraft-writers-workshop.appspot.com/learn) · [공개 구현 README](https://github.com/PAIR-code/wordcraft/blob/4ea64b035e3995aabeeaef5c25853c5dbe80249a/README.md)

13명의 출판 경험이 있는 작가가 LaMDA 기반 도구로 작품을 쓰고 인터뷰·작업 기록을 남겼다. 발상·세부 묘사·세계 구축의 쓰임과 함께 문체 보존·깊은 작품 이해의 한계를 보고한다. 소규모 정성 연구이고 다른 모델군으로 일반화하는 가설은 시험하지 않았다. 이 메모는 확인한 arXiv 판을 인용하며 별도 peer-reviewed venue를 추정하지 않는다. [논문 §4·9](https://arxiv.org/pdf/2211.05030)

공개 코드 SHA `4ea64b035e3995aabeeaef5c25853c5dbe80249a`, Apache-2.0. 연구 당시 모델과 달리 해당 README는 Gemini를 안내한다. [rewrite_selection_operation.ts](https://github.com/PAIR-code/wordcraft/blob/4ea64b035e3995aabeeaef5c25853c5dbe80249a/app/core/operations/rewrite_selection_operation.ts)는 원래 선택 구절을 첫 후보로 남기며 [choice_operation.ts](https://github.com/PAIR-code/wordcraft/blob/4ea64b035e3995aabeeaef5c25853c5dbe80249a/app/core/operations/choice_operation.ts)는 미리보기·선택·취소를 분리한다. 공개 코드가 2022년 실험의 동일 버전이라고 보지 않는다.

**적용 판단:** 06의 원문 문체 근거·짧은 시험 문단, 08의 원문 유지 옵션·지정 구간 재작성에 적용한다. UI 구현이나 클라이언트 API 키 방식을 StoryProduction에 들이지 않는다.

## 3. 현재 geultu에 연결할 위치

| 실제 근거 | 현재 의미 | Target 확장과 경계 |
|---|---|---|
| [authoring/mod.rs](../../geultu/crates/geultu/src/authoring/mod.rs)의 `Kind` | `Source`, `Story`, `Synopsis`만 존재 | 개발 노트가 이미 독립 문서 종류인 것처럼 표시하지 않는다 |
| 같은 파일의 `Brief` | audience/genre/tone/POV/tense/voice/preserve/avoid/ending | 각 값에 원문 근거·작가 선택·미정 여부를 결속하는 새 기록 필요 |
| `Task`와 `task_reasons` | inputs/base/guidance hash/selection으로 오래된 작업 판단 | 원문·브리프·개발 결정의 revision을 한 집필 패킷에 결속 |
| `Draft` | blocks/change_summary/assumptions/open_questions | 실제 본문은 blocks, 검토 자료는 별도 producer-owned 기록. 본문을 진단 설명으로 채우지 않는다 |
| `Review`와 [authoring API](../../geultu/crates/geultu/src/authoring/api/mod.rs)의 `human_only` | candidate hash와 current inputs 검증, review/edit/handoff/restore는 사람 경로 | 에이전트가 고득점 후보를 자동 채택하지 않는다 |
| [story.md](../../geultu/crates/geultu/src/authoring/story.md) | 선택·인과·원문 결말·말투·한국어 높임말과 안정 block ID 지침 | 원칙을 근거가 있는 미시 작업과 검사 사례로 구체화 |

07의 본문 내 인물 사실 추출은 파생 캐시다. 채택된 원문·스토리·개발 결정을 수정하는 별도 권한을 갖지 않는다. 요약이 원문과 다르면 요약을 수정하고 원문을 바꾸지 않는다.

## 4. 공통 실행 계약 — 아래부터 StoryProduction 설계 제안

이 절부터는 A01~A05와 현행 코드에서 도출한 **새 설계**다. 논문에 동일한 단계·필드·게이트가 있다고 주장하지 않는다.

각 작업 패킷은 `source_hash`, `story_base_hash`, `brief_revision`, `guidance_hash`, `adopted_decision_refs`, `selected_block_ids`, `instruction`, `operation_scope`를 명시한다. 개발 기록은 `evidence_ref`, `claim_type`, `candidate_state`, `supersedes`를 갖는 방향으로 owner가 확정한다. `claim_type`은 원문 명시 사실/인물의 믿음/해석/새 창작 제안/미정의 구분이다. 확률 숫자로 사실성을 가장하지 않는다.

통과 판단은 세 층이다. **구조 검사**는 hash·ID·범위·중복·출력 유무를 기계적으로 검사한다. **의미 검토**는 충돌과 근거를 제시하되 모델 판정을 사실로 확정하지 않는다. **작가 판단**은 원문 보호 조건·핵심 선택·채택 판본을 결정한다. 모든 미시 작업이 새 사용자 승인 창을 만들지는 않는다. 개발 자료의 채택도 의미 변경이 필요한 선택에 묶어 수행한다.

수정 요청마다 `change_intent`, `allowed_changes`, `protected_elements`, `affected_refs`, `unresolved_risks`를 남긴다. 구현 초기에는 정확한 영향 관계가 없으면 `UNKNOWN`으로 보고 재검토 범위를 넓힌다. source/hash가 바뀌면 기존 검토를 자동 재사용하지 않는다. 실패·취소·outcome_unknown은 결과 없음 또는 미확정으로 남긴다.

## 5. 03 이야기 핵심: 해석의 방향을 정하는 7작업

**입력:** 원문 snapshot, 02 의도·보호 조건, 입력 유형. **결과:** 핵심 질문·주인공/집단·원하는 변화·대가·최종 선택의 연결, 로그라인 후보와 채택 이유. **근거:** A03의 구조적 출발점; 세부 계약은 본 설계.

| ID | 처리 | 남길 결과 | 검사·실패 때 복구 |
|---|---|---|---|
| 03.1 | 원문에서 반복되는 욕구·대립·손실·이미지를 찾고 구절을 연결 | `motif_candidates`와 원문 참조 | 참조가 없으면 해석/제안으로 표시; 사실로 단정하면 FAIL |
| 03.2 | 누가 무엇을 선택하는 이야기인지 판별 | 중심 인물/집단 후보, 행동의 주체 | 수동적 관찰자도 의도된 형식이면 허용; 주인공을 임의 교체하지 않음 |
| 03.3 | 겉 목표와 그것을 이루며 잃는 것을 분리 | `external_goal`, `cost`, 모순/미정 | 막연한 성공·행복이면 원문에서 관찰 가능한 선택으로 좁힘 |
| 03.4 | 독자가 끝까지 갖고 갈 질문을 한 문장으로 작성 | `dramatic_question`, 답변/열린 결말 의도 | 질문에 없는 새 사건이 필요하면 창작 추가로 분리 |
| 03.5 | 보호된 결말로 역추적하여 필요한 선택만 적음 | `ending_choice`, `required_preconditions` | 결말 보호 조건을 바꿔 해결하면 FAIL; 02로 변경 후보 반환 |
| 03.6 | 필요할 때만 서로 다른 발전 방향을 2~3개 제안 | 후보별 변하는 축·보존 요소·대가 | 형용사만 다른 후보는 병합. 완성 원고에는 복수 후보를 강제하지 않음 |
| 03.7 | 선택된 방향을 로그라인과 한 문단 premise로 압축 | `core_direction` 후보/채택 기록 | 모든 요소가 원문 또는 표시된 창작 제안에 연결되어야 함 |

**완료:** 작가가 의도한 핵심과 보호 조건의 미해결 충돌이 없거나, 해당 쟁점이 명시적으로 보류되어 후속 가역 작업 범위가 제한되어 있다. 로그라인이 ‘재미있다’는 자동 점수는 통과 조건이 아니다.

## 6. 04 인물·관계: 정적 프로필을 선택 가능한 인물로 바꾸는 8작업

**입력:** 03 방향, 원문 인물 구절, 보호 관계. **결과:** 인물별 집필 카드·관계 전이 가설·앎/믿음 구분. **근거:** A01의 속성 대조, A03의 인물 수준 편집; 동기·시간·관점 모델은 본 설계.

| ID | 처리 | 남길 결과 | 검사·실패 때 복구 |
|---|---|---|---|
| 04.1 | 이름·호칭·별칭을 원문 근거로 연결 | 안정 인물 ID와 alias | 동일 호칭이 다른 인물을 가리키면 자동 병합 금지 |
| 04.2 | 명시 사실과 해석·새 제안을 분리 | 나이/역할/과거의 provenance | 묘사가 없다는 이유로 성별·직업·상처를 확정하지 않음 |
| 04.3 | 현재 욕망·피하는 결과·행동 전략을 적음 | `wants`, `avoids`, `tactics` | 추상 형용사만 있으면 다음 사건에서 할 행동 후보를 작성 |
| 04.4 | 믿음과 객관 사실, 알고 모르는 정보를 나눔 | `belief`, `known_at`, `evidence_event` | 아직 듣지 못한 비밀을 사용하는 행동은 위반 후보로 표시 |
| 04.5 | 관계를 방향별로 작성 | A→B의 기대/의존/권력/호칭, B→A 별도 | 서로 친밀하다는 설명으로 비대칭 관계를 지우지 않음 |
| 04.6 | 관계·태도 변화의 계기를 사건에 묶음 | `before`, `trigger`, `choice`, `after` | 과거/현재 변화는 모순으로 잡지 않음; 계기 없는 반전은 05 수정 후보 |
| 04.7 | 압박이 큰 상황에서 가능한 행동을 시험 | 선택 A/B, 각각의 비용과 원문 근거 | 결말에 맞추기 위해 인물이 전능해지는 선택은 재설계 |
| 04.8 | 말투와 행동의 구분 가능한 특징만 06에 전달 | 호칭·금기 표현·회피 행동·예외 | 사람을 한 성격 유형으로 환원하거나 모든 인물에게 동일 말투 적용 금지 |

**완료:** 중심 선택을 설명할 동기와 정보 접근 근거가 있고, 명시 사실의 충돌은 해결/보류 상태가 드러난다. 단순한 과거 트라우마 추가를 인물 심화의 필수 조건으로 삼지 않는다.

## 7. 05 사건·인과·결말: 개요의 세밀도를 제어하는 9작업

**입력:** 채택한 핵심 방향·인물 가설, 원문 사건·결말. **결과:** 사건 계층, 인과 연결, 정보 공개 계획의 집필용 초안, pacing·복선 장부. **근거:** A02의 계층적 개요와 A04의 세밀도 비교; graph·복선 필드는 본 설계.

| ID | 처리 | 남길 결과 | 검사·실패 때 복구 |
|---|---|---|---|
| 05.1 | 원문 사건과 새 사건 후보를 구분 | 안정 event ID, provenance, 유지/추가 상태 | 원문 사건 삭제·새 사건 추가를 숨기면 FAIL |
| 05.2 | 사건이 성립하는 순서와 제시 순서를 분리 | `happened_before`, `presented_before` | 회상을 시간 모순으로 판정하지 않음. 독자 공개 순서는 11에서 재확정 |
| 05.3 | 각 주요 사건의 전제·선택·결과를 적음 | `precondition`, `actor_choice`, `consequence` | ‘그리고’만 이어진 전개는 실제 원인인지 우연 병치인지 명시 |
| 05.4 | 결말부터 필요한 원인 경로를 역추적 | payoff→선택→압력→발단 연결 | 새 능력/소품이 결말에서만 등장하면 앞선 설정 또는 결말 수정 후보 |
| 05.5 | 상위 사건을 목적이 다른 하위 행동으로 분해 | 부모/자식 event와 실현 조건 | 자식이 부모보다 추상적이거나 부모의 결과를 바꾸면 재분해 |
| 05.6 | 중요한 선택의 세밀도를 인접 사건과 비교 | `granularity_issue`, 핵심 누락 행동 | 모든 leaf의 깊이를 같게 강제하지 않음. 작은 연결 사건은 요약 허용 |
| 05.7 | 어디를 장면화·요약·생략할지 지정 | `render_mode`, 목표 분량 범위와 이유 | climax를 요약하고 이동만 길게 쓰는 배분은 작가 의도와 대조 |
| 05.8 | 복선의 심기·해석·회수·의도적 미회수를 연결 | setup/payoff refs, 독자 지식 상태 | 미회수는 항상 오류가 아님. 의도 없거나 결말 의존 회수 누락만 쟁점 |
| 05.9 | 전체를 읽고 중복·지연·성급한 결말을 진단 | 집필할 leaf 순서, 미결 항목 | 특정 사건 삭제 시 영향 받는 인과/복선만 표시, 근거 없는 최소 범위 보장 금지 |

**완료:** 필수 사건이 집필 구간에 배정되어 있고 보호된 결말의 전제 누락이 없다. 개요의 parent-child cycle와 같은 시간 기준의 strict BEFORE 모순은 오류다. 의도된 인과 고리·시간 여행·반복 구조는 발생 회차·시간 의미·작가 결정을 구분해 검토하며 일괄 금지하지 않는다. 장르 공식·3막·영웅의 여정을 필수 schema로 삼지 않는다.

## 8. 06 글투·서술: 한국어 원문에 결속한 문체 시험 8작업

**입력:** 사용자의 원문, Brief/voice profile, 인물 관계·목표 매체. **결과:** 문체 근거표·서술 계약·시험 문단·채택한 지침. **근거:** A05의 voice 보존 한계와 선택적 수정 방식; 한국어 검사는 본 설계이며 논문의 검증 대상이 아니다.

| ID | 처리 | 남길 결과 | 검사·실패 때 복구 |
|---|---|---|---|
| 06.1 | 서술·대화·감정·행동에서 대표 구절을 고름 | style exemplar refs, 선택 이유 | 분량 부족 시 ‘작가 문체 확정’ 금지; 잠정 방향으로 표시 |
| 06.2 | 시점과 서술자가 알 수 있는 범위를 정함 | POV, 초점 인물, 내면 접근 허용 범위 | 제한적 시점에서 타인 내면 확정은 오류 후보, 의도적 시점 전환은 예외 기록 |
| 06.3 | 서술 거리·시제·시간 압축의 원문 패턴을 적음 | 관찰 내용과 예시 | 숫자 평균만으로 문체를 설명하지 않음 |
| 06.4 | 문장 호흡·생략·반복·구체 명사의 역할을 관찰 | 지킬 패턴/피할 패턴 | 단문 비율 맞추기용 쪼개기나 낯선 어휘 삽입은 보류 |
| 06.5 | 화자→청자→상황별 호칭·높임말을 매핑 | `speaker`, `addressee`, `register`, 전환 계기 | 감정 격화로 말끝이 변하는 의도까지 오류로 정규화하지 않음 |
| 06.6 | 원문을 과잉 설명하는 지점을 찾음 | 설명/행동/침묵 후보와 의미 차이 | 모든 감정을 ‘보여주기’로 강제하지 않음; 원문의 서술 방식 존중 |
| 06.7 | 같은 사건을 원문 유지안과 시험안으로 짧게 작성 | 실제 문단 1~2쌍, 변한 요소 | 사건·관계·결말까지 바뀌면 문체 시험 범위 초과 |
| 06.8 | 채택한 시험의 이유를 지침으로 압축 | 활성 voice guidance와 근거 참조 | 문체 지침 변경 시 guidance hash와 후속 stale 갱신 필요 |

**완료:** 지침마다 예시가 있거나 작가의 명시 선택이며, 출처가 없는 ‘작가 고유 문체’ claim이 없다. 맞춤법·문장 길이는 보조 관측값이다. 한국어 주어 생략·대명사 생략·존대는 영어 tokenizer/문장 휴리스틱을 그대로 쓰지 않는다.

## 9. 07 실제 스토리: 계획을 본문으로 실현하는 9작업

**입력:** 05 집필 순서·실현 조건, 06 guidance, 04 인물 정보, 원문/현재 story snapshot. **결과:** 읽을 수 있는 완결 본문 candidate, stable block ID, 계획/본문 대응표, 창작 추가·미해결 항목. **근거:** A01 문맥 반복 주입, A02 개요와 본문 정합; 패킷·검사는 본 설계.

| ID | 처리 | 남길 결과 | 검사·실패 때 복구 |
|---|---|---|---|
| 07.1 | 현재 hash/revision과 집필 범위 확인 | frozen task packet | stale/cancelled이면 새 입력으로 prepare; 기존 후보 덮어쓰기 금지 |
| 07.2 | 구간의 진입 상태·이번 변화·퇴장 상태 명시 | `entry`, `required_change`, `exit` | 필수 상태가 미정이면 관련 개발 작업만 보완 |
| 07.3 | 현재 구간에 필요한 근거만 구성 | 핵심 방향·관련 인물·최근 본문·회수할 복선·미래 공개 금지 | 요약만 남기며 핵심 원문 근거를 유실하면 문맥 패킷 재작성 |
| 07.4 | 사건을 행동·대화·내면·묘사로 실제 집필 | 본문 block candidate | 계획 설명/줄거리 목록만 나오면 본문 출력 실패 |
| 07.5 | 구간 의무를 실제 문장과 대조 | 실현/누락/과도한 추가와 evidence spans | 개요가 요구한 선택이 설명으로만 처리되면 해당 부분 재작성 |
| 07.6 | 인물 정보·소품·위치·시점 충돌을 검사 | 근거 두 개를 가진 issue list | 시간에 따른 합당한 변화를 모순으로 확정하지 않음 |
| 07.7 | 필요한 경우만 소수 대안 비교 | 후보별 차이와 비용 | 최고 점수 자동 채택 금지; 원문 보호 조건을 위반한 후보는 명시 |
| 07.8 | 구간 전환과 전체 결말을 읽고 마무리 | 중복 없는 연결·결말 본문 | 토큰 한도로 중단되면 PARTIAL; 완료로 표시하거나 결말을 자동 요약하지 않음 |
| 07.9 | 모든 block·변경·가정을 제출 | `Draft`와 별도 검토 자료 | 중복 ID/선택 밖 변경/허위 원문 참조는 구조 FAIL |

**완료:** 본문이 있으며 계약상 필수 내용이 대조되고, candidate로 제출되었다. 이 단계 완료는 작품 채택 완료가 아니다. 새 요약은 후보 본문을 반영하는 파생 자료로 보관하고 원문 authority와 혼동하지 않는다.

## 10. 08 퇴고·채택: 증거가 있는 문제부터 제한해서 고치는 9작업

**입력:** 현재 candidate/본문, 원문·개발 결정·보호 조건, 작가의 수정 요청. **결과:** 문제 장부, 수정 의도·허용 범위, 전후 본문, 채택/거절 결정. **근거:** A01 자동 교정 한계, A03 수준별 수정, A05 선택 구절 후보 유지; 진단·채택 계약은 본 설계.

| ID | 처리 | 남길 결과 | 검사·실패 때 복구 |
|---|---|---|---|
| 08.1 | 결함과 취향 제안을 분리 | violation/ambiguity/preference와 evidence | 인용 구절 없는 비평은 실행할 repair로 승격하지 않음 |
| 08.2 | 먼저 구조·인과·결말의 문제를 읽음 | 중요한 문제의 우선순위, 영향 | 문장 미화로 구조 문제를 가리지 않음; 모든 문제를 한꺼번에 수정하지 않음 |
| 08.3 | 인물 동기·앎·관계의 문제를 확인 | 충돌하는 사실/행동의 위치 | 사실·믿음·거짓말·회상·상태 변화를 구분하여 오탐 제거 |
| 08.4 | 그다음 중복·호흡·대화·높임말을 확인 | 문장 수준 쟁점 | 의도적 반복·생략을 기계적으로 삭제하지 않음 |
| 08.5 | 수정안을 변경 수준에 맞게 준비 | COPYEDIT/LOCAL_REWRITE/STRUCTURAL 제안, 허용 block | 용어는 Target. 결말·정체·보호 관계 변경은 의미 변경 후보로 명시 |
| 08.6 | 원문 유지안과 실제 수정 후보를 생성 | before/after, 변경 이유, 남은 위험 | 선택 밖 블록 내용·순서의 무단 변경은 거절 |
| 08.7 | 원래 문제의 해소와 부작용을 별도로 검사 | issue별 resolved/unresolved/new issue | ‘수정했으므로 해결’ 금지; 새로운 인과·문체 문제면 추가 수정 또는 원문 유지 |
| 08.8 | 최신 source/guidance/선택 블록과 hash 대조 | 채택 가능 여부와 stale reason | 동시 편집·취소·오래된 후보의 검토 재사용은 FAIL |
| 08.9 | 사람 경로로 정확한 candidate를 채택/거절 | actor, candidate hash, note, 새 판본 | 모델 진단·좋은 점수·후보 제출은 approval을 대신하지 않음 |

**완료:** 선택한 판본이 채택되었고 unresolved 의미 문제와 후속 영향이 드러나 있다. 단순 수정이 끝났다는 이유로 시놉시스·대본·콘티의 stale을 해제하지 않는다.

## 11. 도입 전에 필요한 판별 fixture

아래는 아직 실행하지 않은 Target 사례다. 기계 구조 검사와 사람의 작품 품질 평가를 별도로 기록한다.

| ID | 자극 | 기대 관측 |
|---|---|---|
| AU-01 | 원문에는 ‘용서하지 않고 떠남’, 후보는 화해로 끝남 | 결말 보호 위반을 근거 구절과 표시, 자동 채택 없음 |
| AU-02 | 이름이 같은 두 인물, 별칭 하나가 미정 | 자동 인물 병합 없음; alias 충돌 표시 |
| AU-03 | 인물이 거짓말하거나 과거 시점의 직업이 다름 | 곧바로 객관 사실 모순 판정하지 않음 |
| AU-04 | 비밀 공개 전 인물이 그 사실을 알고 행동함 | 앎의 근거 누락을 해당 행동과 연결 |
| AU-05 | 클라이맥스는 요약, 사소한 이동만 상세 | 세밀도 불균형 후보 표시, 모든 장면의 분량 균등화 없음 |
| AU-06 | 한국어 호칭·존대가 갈등 뒤 의도적으로 변함 | 전환 계기를 확인; 일괄 존대 교정 없음 |
| AU-07 | 문체 수정 요청으로 결말/정보 공개가 바뀜 | 작업 범위 초과와 의미 차이 표시 |
| AU-08 | 선택하지 않은 블록을 동시에 사람이 편집 | 현행 selection 계약에 맞게 외부 변경 보존; 후보가 덮지 않음 |
| AU-09 | task 준비 후 원문/voice 지침 변경 또는 취소 | 오래된 후보 채택 거절, 완료 상태 승격 없음 |
| AU-10 | 본문 대신 개요만 출력, 또는 출력 중단 | 각각 wrong-artifact/PARTIAL로 진단; story 완성으로 처리하지 않음 |
| AU-11 | 수정 후 원 문제는 사라지지만 새 모순 발생 | unresolved/new issue를 남기며 자동 해결 선언 없음 |
| AU-12 | 완성 원고 입력, 문장 호흡만 수정 요청 | 03~05 재창작·새 인물·새 결말을 강제하지 않음 |

실제 품질 비교는 같은 한국어 원문·같은 모델 설정·같은 허용 수정 범위를 사용해 기존 방식과 제안 방식을 비교한다. 작가 의도 보존, 근거 있는 문제 검출의 오탐/누락, 수정 후 사용자 재작업 범위, 실제 검토 시간, 본문의 인과 이해도를 기록한다. 사람 평가에는 원문을 읽는 보존 평가와 원문 없이 읽는 독자 평가를 구분한다. 재미·문체·감정 전달을 모델 자기 점수 하나로 합격 처리하지 않는다.

## 12. 검증 상태

- PASS / STATIC: 리뉴얼 명세와 geultu authoring/mod.rs·api.rs·story.md의 해당 구조를 읽었다.
- PASS / RESEARCH: A01~A05의 1차 논문을 열람했고 저자 저장소 5곳의 README, 실제 코드 파일, 기본 브랜치 SHA 및 license 선언을 확인했다.
- NOT_RUN: 외부 논문 코드 실행, 모델 호출, 한국어 품질 비교, StoryProduction build/test/runtime. 문서 연구 범위에 해당하지 않아 실행하지 않았다.
- UNKNOWN: 제안 미시 단계의 실제 비용·효과·사용자 검토 시간. 실제 사용자 원고와 한국어 fixture의 후속 검증이 필요하다.
