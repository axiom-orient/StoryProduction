# 원문·시놉시스·대본의 의미 보존: 연구와 구현 적용 메모

조사일: 2026-09-22. 범위: 리뉴얼 작업 01~02, 09~10, 16. 상태: **연구·Target 제안**. 논문 원문/학회 페이지, 저자 GitHub와 지정 commit의 Python 소스를 읽었다. 외부 패키지 설치·모델 실행·코드 이식·제품 코드 변경은 하지 않았다. 아래의 절차·자료 구조·검증 사례는 StoryProduction을 위한 설계이며 논문이 입증한 한국어 소설 품질이나 현재 제품 구현으로 해석하지 않는다.

## 1. 결론과 현재 출발점

필요한 것은 한 개의 ‘충실도 점수’가 아니라 **근거 있는 주장, 보존할 의미의 목록, 실제 산출물과의 양방향 대조, 근거 부족을 유지하는 검토**다. 생성문이 원문에 없는 내용을 말하는 문제와 원문의 핵심이 생성문에서 사라지는 문제를 따로 잡는다. 허구에서 확인할 기준은 우선 승인된 작품 세계다. 현실 자료는 별도의 조사 근거이고, 인물의 오해를 현실의 참·거짓으로 정정하지 않는다.

현재 확인한 구현/지침:

- [geultu authoring](../../geultu/crates/geultu/src/authoring/mod.rs): Source/Story/Synopsis 문서, `Brief.preserve/avoid/ending`, document dependencies, guidance hash, candidate review가 있다. 문서 내용 hash는 blocks의 JSON 직렬화에 대한 digest이고 export markdown bytes hash와 같다고 가정하면 안 된다.
- [geultu handoff](../../geultu/crates/geultu/src/store.rs): `export_authoring_synopsis`는 reviewed synopsis revision·markdown SHA-256을 검사하고 불변 handoff를 쓴다. 원문 구절/보존표의 자동 동봉은 이 함수에서 확인되지 않는다.
- [STS step_00](../../NarrativeCourier/skills/sts/prompts/analyst/step_00_source_graph.md): `FACT/BELIEF/CLAIM/UNKNOWN`, `AFFIRM/NEGATE`, materiality, protected, SRC/REL/GAP/CON 및 ending contract를 이미 요구한다. 새 사실 권한 모델을 integration에 만들 필요가 없다.
- [audit-source-ir schema](../../NarrativeCourier/skills/sts/schemas/audit-source-ir.schema.json): evidence는 quote/note이고 원문 byte range·document hash 결속은 없다. 이 부분은 보강 목표다. schema에 적혀 있다는 사실과 자연어 의미가 맞다는 검증은 다르다.
- [역복원](../../NarrativeCourier/skills/sts/prompts/critic/reconstruct_screenplay.md)과 [독립 비교](../../NarrativeCourier/skills/sts/prompts/critic/step_08_verify.md): 대본만으로 OBS/OREL을 회수한 뒤 승인 Source IR과 비교하는 경계를 이미 둔다. 의도한 treatment나 blueprint로 결손을 보충하는 평가를 금지한다.
- [별도 대본 audit 도구](../../NarrativeCourier/tools/sts_audit.py): ID/coverage/verdict coherence의 정적 검증 함수가 있다. 이 도구와 제작 STS 흐름은 같은 실행 경로라고 단정하지 않는다. 이번에는 실행하지 않았다.

## 2. 선정 연구와 실제 GitHub 확인

각 논문을 일반 이론·정밀 동작 아이디어로 사용한다. 공개 점수와 학회 게재가 이 프로젝트의 runtime 성공을 입증하지 않는다. 라이선스 표기는 지정 commit에서 확인한 코드 라이선스 식별이다. 모델 가중치·별도 데이터의 사용 조건까지 포괄하지 않는다.

### F01 — FActScore: 생성문을 원자 주장으로 분해해 근거를 확인

[Min et al., EMNLP 2023](https://aclanthology.org/2023.emnlp-main.741/). 전기 생성문의 원자 사실 중 지식원으로 지지되는 비율을 평가한다. 채택할 개념은 문서 전체의 인상 평가를 주장별 근거 판정으로 분리하는 방식이다. StoryProduction에서는 Wikipedia 대신 승인된 작품 snapshot을 비교 대상으로 삼는다. 이 precision만으로 시놉시스가 빠뜨린 사건·인과·결말은 잡을 수 없으므로 source→output coverage를 별도로 둔다. 허구·한국어·불신 서술자에 대한 성능은 이 논문에서 이 프로젝트에 맞게 검증되지 않았다.

[저자 저장소](https://github.com/shmsw25/FActScore), SHA `f28272deffcf33efc1f1117d5479c10bb75221a9`, commit 2025-04-13, [MIT](https://github.com/shmsw25/FActScore/blob/f28272deffcf33efc1f1117d5479c10bb75221a9/LICENSE). 실제 [factscorer.py](https://github.com/shmsw25/FActScore/blob/f28272deffcf33efc1f1117d5479c10bb75221a9/factscore/factscorer.py#L209)는 atom별 passage retrieval과 supported 판정을 반환한다. **그대로 이식하지 않음:** 판정 문구 해석 실패 시 부정 키워드 부재를 지원으로 보는 fallback이 있고, [atomic_facts.py](https://github.com/shmsw25/FActScore/blob/f28272deffcf33efc1f1117d5479c10bb75221a9/factscore/atomic_facts.py#L21)는 영어 spaCy·전기 모드를 사용한다. Story에서는 파싱 실패를 UNKNOWN/FAILED로 보존한다.

### F02 — QAFactEval: 중요한 질문에 원문과 요약이 같은 답을 주는지 검사

[Fabbri et al., NAACL 2022](https://aclanthology.org/2022.naacl-main.187/). 요약에서 질문을 만들고 답할 수 있는 질문인지 거른 다음 원문 답변과 비교한다. StoryProduction에는 행위자·원인·대가·결말·누가 무엇을 믿는지에 대한 질문을 남기는 진단 방식으로 적용한다. 추가로 보호 조건에서 질문을 만드는 역방향 검사를 제안한다. 이것은 원 논문의 metric을 그대로 재현한 것이 아니다. 함축, 반어, 비유, 화자의 거짓말, 영주어는 오탐 가능성이 있어 질문 자체와 답 근거도 검토해야 한다.

[저자 저장소](https://github.com/salesforce/QAFactEval), SHA `01177f11cc05f8b3e75511a0cfa54bac19324d09`, commit 2026-06-02, [BSD-3-Clause](https://github.com/salesforce/QAFactEval/blob/01177f11cc05f8b3e75511a0cfa54bac19324d09/LICENSE.txt). [qafacteval.py](https://github.com/salesforce/QAFactEval/blob/01177f11cc05f8b3e75511a0cfa54bac19324d09/qafacteval.py#L17)는 answerability와 F1 필터를 두고 source 답변을 평가한다. **이식 유보:** 영어 spaCy 의존, `summaries_cons = [x[0] ...]`와 first-summary TODO가 있어 여러 후보를 넣으면 전부 독립 검증한다고 가정할 수 없다. 후보별 별도 진단 및 한국어 qualification이 필요하다.

### F03 — SummaC: 작은 근거 단위의 모순 신호를 유지

[Laban et al., TACL 2022](https://aclanthology.org/2022.tacl-1.10/). 원문·요약을 문장 단위로 나누고 문장 쌍의 NLI 결과를 집계해 요약 일관성을 판단한다. 채택할 것은 어떤 원문 조각이 어떤 생성 주장과 충돌하는지 역추적 가능한 대조표다. 소설의 시점·시간 변화·믿음 주체를 제거한 문장 쌍은 정상적인 변화를 모순으로 오판할 수 있다. 원문의 인과관계처럼 여러 문장에 걸친 의미도 별도 관계 검사로 다룬다. 논문의 요약 benchmark 점수를 작품 합격선으로 사용하지 않는다.

[저자 저장소](https://github.com/tingofurro/summac), SHA `c1f3da93cd074c24d8033eb27a88b5a7cc5c08fa`, commit 2025-01-30, [Apache-2.0](https://github.com/tingofurro/summac/blob/c1f3da93cd074c24d8033eb27a88b5a7cc5c08fa/LICENSE). [model_summac.py](https://github.com/tingofurro/summac/blob/c1f3da93cd074c24d8033eb27a88b5a7cc5c08fa/summac/model_summac.py#L97)는 premise/hypothesis 조합, entailment/contradiction/neutral 행렬과 집계를 구현한다. **이식 유보:** 기본 `max_doc_sents=100`으로 원문 조각을 자르고 짧은 문장을 거른다. 장편 후반이나 짧은 한국어 핵심 대사가 사라질 수 있으므로 truncation·누락을 기록하지 않는 기본값을 채택하지 않는다.

### F04 — Lost in the Middle: 긴 입력은 위치별 회수 검증이 필요

[Liu et al., TACL 2024](https://aclanthology.org/2024.tacl-1.9/) — arXiv 초판 2023. 다문서 QA와 key-value retrieval에서 관련 정보 위치에 따른 성능 차이를 조사했다. StoryProduction에는 긴 원문을 한 번 보냈다는 이유로 전부 읽었다고 판정하지 않는 원칙, 중간 부분의 단서까지 회수했는지 보는 평가를 적용한다. 챕터별 추출·검색·관계 병합은 이 연구에서 직접 입증된 해결책이 아니라 본 프로젝트의 설계 가설이다. 실제 사용할 모델과 한국어 길이에 대한 반복 측정이 필요하다.

[저자 저장소](https://github.com/nelson-liu/lost-in-the-middle), SHA `29b8a6d042ce29abccee3db1a73171a107d7e6af`, commit 2024-01-04, [MIT](https://github.com/nelson-liu/lost-in-the-middle/blob/29b8a6d042ce29abccee3db1a73171a107d7e6af/LICENSE). [make_qa_data_from_retrieval_results.py](https://github.com/nelson-liu/lost-in-the-middle/blob/29b8a6d042ce29abccee3db1a73171a107d7e6af/scripts/make_qa_data_from_retrieval_results.py#L32)는 gold passage를 지정 index에 삽입하고 distractor를 유지한다. 이 통제 실험 형태를 빌리되 실제 작품의 사건 순서를 무작위로 바꾸어 일관성을 훼손하는 실험은 하지 않는다.

### F05 — MT-Bench / LLM-as-a-Judge: 심사자의 편향을 기록하고 억제

[Zheng et al., NeurIPS 2023 Datasets and Benchmarks](https://arxiv.org/abs/2306.05685). 위치·장황함·자기 선호 등의 편향과 추론 한계를 조사한다. StoryProduction에서는 후보의 작성 모델명을 가리고, 두 후보의 제시 순서를 바꾼 평가를 비교하며, 이유와 실제 문장을 요구한다. 순서를 바꾸었을 때 승자가 달라지면 불안정 판정으로 남긴다. 문학적 선호는 작가의 선택을 대리하지 못하며 두 번의 모델 평가를 독립된 인간 평가로 취급하지 않는다. 길이가 길다는 이유만으로 세부 묘사가 우수하다고 점수화하지 않는다.

[저자 저장소 FastChat](https://github.com/lm-sys/FastChat), SHA `587d5cfa1609a43d192cedb8441cac3c17db105d`, commit 2025-06-02, [Apache-2.0](https://github.com/lm-sys/FastChat/blob/587d5cfa1609a43d192cedb8441cac3c17db105d/LICENSE). [play_a_match_pair](https://github.com/lm-sys/FastChat/blob/587d5cfa1609a43d192cedb8441cac3c17db105d/fastchat/llm_judge/common.py#L313)는 A/B와 B/A를 각각 호출하고 승자를 원래 모델에 되매핑하며 두 판단·prompt를 저장한다. 순서 교환 프로토콜만 참고하며 전체 chat serving stack은 도입하지 않는다.

### F06 — StoryER: 취향 점수와 실제 수정 근거를 분리

[Chen et al., EMNLP 2022](https://aclanthology.org/2022.emnlp-main.114/). 순위·관점별 평가·이유 코멘트를 함께 다루는 이야기 평가 연구다. Reddit WritingPrompts 기반 선호와 관점 자료를 사용하며 영어권 annotator를 중심으로 구축했다. 여기서는 ‘도입·인물·긴장·결말 중 어느 부분이 왜 약한지’ 설명하는 형식만 참고한다. 인기도는 작가 의도와 같지 않으며 관점 confidence를 판정의 정확도 확률로 읽으면 안 된다. 한국어 연재물·대본·사용자 개인 취향에 대한 검증은 별도다.

[저자 저장소](https://github.com/sairin1202/StoryER), SHA `0f59b7f04d4b37b05bdd8ff68d6728d6d97c35a3`, commit 2024-11-14. GitHub API의 license는 null이고 조사한 tree에서 LICENSE 파일을 찾지 못했다. [model.py](https://github.com/sairin1202/StoryER/blob/0f59b7f04d4b37b05bdd8ff68d6728d6d97c35a3/model.py#L5)는 LED 위에 overall/aspect rating/aspect confidence head와 코멘트 생성 손실을 둔다. **복사·dependency 채택 대상이 아니다.** 연구 아이디어를 Story-owned 설계로 재작성하고 모델·데이터 사용 검토는 별도 수행한다.

## 3. 적용 방식: 기존 owner 안에서 정밀하게 분해

아래 소단계는 호출 횟수나 사용자 확인 횟수가 아니다. 입출력과 중단 지점을 분명히 하는 작업 명세다. geultu의 집필 분석 결과가 NVC 승인 Source IR로 자동 승격되지 않는다. 중간 기록은 owner가 보관하고 integration은 공개 entrypoint/immutable contract를 운반한다.

### 01 — 원문 수용과 근거 기반 마련

| 소단계 | 정확한 동작 | 결과·실패 처리 |
|---|---|---|
| 01.1 | raw bytes, 언어, 출처 종류, 문서 역할을 보관한다. 작가 원문·추가 메모·현실 조사 자료를 분류한다. | 원문 snapshot과 별도 display text. 정규화본으로 raw hash를 대체하지 않음 |
| 01.2 | 메모/미완성/완성 원고 여부와 목표 산출물을 판별하되 추정 근거를 남긴다. | 입력 프로필. 불확실하면 UNKNOWN; 내용을 임의 완성하지 않음 |
| 01.3 | 장·문단·화자·문서 삽입 경계를 이용해 읽기 단위를 나누고 인접 맥락을 함께 참조한다. | 모든 raw 구간이 어디에 속하는지 coverage index. 누락·중복 표시 |
| 01.4 | 각 단위에서 인물·별칭 후보와 사건·상태·부정·발언·믿음을 추출한다. | source-bound claim 후보. 대명사·생략된 주어는 미정 가능 |
| 01.5 | 인물의 동일성을 근거로 병합한다. 같은 이름이라는 이유만으로 결합하지 않는다. | entity candidate map와 미해결 충돌 |
| 01.6 | 등장 순서와 사건 시간을 분리하고 상태가 적용되는 구간을 표시한다. | temporal scope. ‘아직 모른다’와 나중의 ‘알았다’를 동시 모순 처리하지 않음 |
| 01.7 | 원문 전체 index를 대조해 결말·중간 단서·원인·부정 진술이 회수되었는지 확인한다. | source→claim 누락 목록. context overflow/truncation은 실패 또는 부분 수행 |
| 01.8 | 모든 근거 span의 raw bytes/hash를 확인한 뒤 해석 불확실성만 검토 목록으로 올린다. | 구조 검증 결과와 의미 검토 결과 분리; 문구 일치가 해석 승인이라는 뜻은 아님 |

근거: F01의 주장 단위, F04의 위치 민감성. 문단 분해와 두 단계 coverage는 본 프로젝트 제안이다.

### 02 — 작가 의도·보호 조건

| 소단계 | 정확한 동작 | 결과·실패 처리 |
|---|---|---|
| 02.1 | 작가가 직접 말한 의도와 작품에서 추정한 주제를 다른 기록으로 만든다. | AUTHOR_EXPLICIT / INTERPRETATION 구분. 해석은 승인 전 후보 |
| 02.2 | 결말 사건, 결말 의미, 결정적 행위자, 관계, 문체·형식 기능을 분리한다. | 보호할 원소 목록. ‘같은 결말’의 의미를 구체화 |
| 02.3 | 보호 조건마다 해당 span·사용자 결정·적용 범위를 결속한다. | protected obligation. 문자열만 일치하면 통과하는 규칙을 피함 |
| 02.4 | 허용되는 압축·재배치·장면화와 작가 판단이 필요한 발명을 구분한다. | transform permission scope. 열린 공간은 이미 승인된 발명이 아님 |
| 02.5 | 원문과 메모, 옛 결정과 새 결정의 충돌을 드러낸다. | CON 후보와 superseded 관계. 최신 파일이라는 이유로 자동 우선하지 않음 |
| 02.6 | 각 보호 조건을 실제 확인 질문으로 만든다. | 예: ‘누가 마지막 결정을 내리는가?’, ‘인물이 믿는 것인가 작품의 사실인가?’ |
| 02.7 | 기존 review 흐름에 필요한 검토를 결합하고 현재 source/brief hash에 선택을 묶는다. | 인계 가능한 현재 결정. source 변경 시 재판정 필요 |

### 09 — 제작 시놉시스와 보존표

| 소단계 | 정확한 동작 | 결과·실패 처리 |
|---|---|---|
| 09.1 | 채택 Story와 Source snapshot, 보호 조건을 고정한다. | 서로 다른 revision 혼합 거부 |
| 09.2 | 핵심 행위·원인·전환·결말의 최소 설명 단위를 고른다. | CORE/SUPPORT/TEXTURE별 요약 의무. 단순 등장 빈도로 중요도 결정 금지 |
| 09.3 | target 분량에서 보존 불가능한 조건을 먼저 계산·설명한다. | 충돌하는 압축 요구; 보호 조건을 몰래 제거하지 않음 |
| 09.4 | 결말을 포함한 제작 요약을 작성하고 보호 요소를 연결한다. | 실제 synopsis 후보와 source IDs |
| 09.5 | synopsis→source 방향으로 모든 material 주장과 관계의 근거를 찾는다. | 추가·과장·극성/화자/시간/행위자 변형 목록 |
| 09.6 | source obligations→synopsis 방향으로 보존·압축·누락을 전수 대조한다. | 누락 recall. 지지된 문장만 썼어도 핵심 누락은 별도 실패 |
| 09.7 | 인과 연결과 결정적 행위자를 질문으로 검토한다. | 두 사건이 모두 등장해도 CAUSES/ENABLES/PREVENTS 소실 감지 |
| 09.8 | 상충 근거를 함께 회수하고 시놉시스의 해석이 어디까지 가능한지 표시한다. | SUPPORTED/CONTRADICTED/UNSUPPORTED/UNCERTAIN, 별도 retrieval completeness |
| 09.9 | 보존표에 유지·장면화·합침·재배치·생략·발명과 이유를 남긴다. | protected 생략·drift는 review blocker; 허용된 texture 압축은 설명 후 진행 가능 |
| 09.10 | 실제 수정문과 근거를 검토한 다음 기존 geultu 채택·handoff로 진행한다. | 후보=승인 아님. 새 근거 bundle은 producer 소유 versioned 계약이 필요 |

근거: F01+F02+F03. 원문 의무에서 요약으로 되묻는 역방향 검사는 precision의 공백을 메우기 위한 제안이다.

### 10 — NVC 원본 해석 확정

| 소단계 | 정확한 동작 | 결과·실패 처리 |
|---|---|---|
| 10.1 | public intake에서 synopsis와 동봉 근거의 선언된 역할·hash·review identity를 검사한다. | reference가 몰래 새 narrative authority가 되지 않음 |
| 10.2 | synopsis와 근거 bundle의 누락·충돌·권한을 확인한다. | 동일 문장이라도 source/보호 조건이 달라지면 이전 검토 자동 재사용 금지 |
| 10.3 | STS의 SRC assertion·REL relation 구조로 독립 해석한다. | geultu 집필 가설을 자동 복제하지 않음 |
| 10.4 | actor/target/polarity/epistemic을 채우고 믿음·발언의 주체와 적용 시간을 명시한다. | 누가 아는지·언제부터인지 불명확하면 UNKNOWN |
| 10.5 | CAUSES/ENABLES/PREVENTS를 BEFORE/AFTER와 구별하고 관계 근거를 찾는다. | 선후관계만으로 인과를 확정하지 않음 |
| 10.6 | 세계 규칙·능력의 trigger/scope/cost/limits/reversibility와 knowledge acquisition을 회수한다. | 원문에서 모르는 규칙은 보완 발명 대신 GAP |
| 10.7 | ending event/decisive actor/semantic reframe/residue와 essential form 기능을 대조한다. | ‘마지막 사건이 같음’으로 의미 보존 대체 금지 |
| 10.8 | 보호 원소와 높은 위험의 CON/GAP를 실제 문장과 함께 source approval에 올린다. | NVC owner의 승인 Source IR; 미해결을 평균 confidence로 덮지 않음 |

### 16 — 대본 진단·수정·독립 검증

| 소단계 | 정확한 동작 | 결과·실패 처리 |
|---|---|---|
| 16.1 | 대본·Source IR·선택된 수정 범위 snapshot을 고정한다. | stale 변경은 중단·재준비 |
| 16.2 | 근거 없는 진단을 분리하고 모든 결함을 실제 SCN/행동/대사에 붙인다. | 관점별 issue: 증상, 원인 가설, 중요도, 제안, 보호 조건 영향 |
| 16.3 | 충실도 결함과 작품 선호를 분리한다. | ‘인물의 믿음을 사실로 바꿈’은 fidelity; ‘더 긴 묘사를 선호’는 preference |
| 16.4 | 구조·인물·장면·대사 수정의 의존 범위를 산정한다. | 없는 dependency는 UNKNOWN. 선택 외 수정 자동 허용 금지 |
| 16.5 | 기존 step_08_scope Human gate에서 진단과 수정 범위를 검토한다. | 승인된 범위와 현재 입력을 결속; 의미 변경·범위 확대를 미리 자동 승인하지 않음 |
| 16.6 | step_08_apply에서 승인된 범위의 실제 수정 대본을 만들고 차이·남은 문제를 기록한다. | 범위 밖 변경은 거부 또는 별도 검토; 수정 뒤 자동 일관성 확정 금지 |
| 16.7 | 새 컨텍스트에서 대본만 제공하여 OBS/OREL·결말·지식 변화를 역복원한다. | 입력 manifest뿐 아니라 허용 도구·이전 대화 접근도 제한; 실행 격리가 없으면 source-blind로 인증하지 않음 |
| 16.8 | 승인 Source IR과 recovered IR만 양방향 비교한다. | N:M alignment 허용, 연결마다 recovered evidence 필요 |
| 16.9 | protected drift·결말 의미·행위자·극성·epistemic·인과·필수 형식 기능을 개별 검사한다. | 어느 하나의 hard failure도 총점으로 상쇄하지 않음 |
| 16.10 | 다른 해석도 가능한 장면은 수정 필요와 의도된 모호성을 구별한다. 후보 우열을 비교할 때만 익명 A/B와 B/A를 사용한다. | 순서별 상충은 unstable; 작가가 승인한 모호성을 과잉 교정하지 않음 |
| 16.11 | 검증 뒤 기존 final_release Human gate에서 작품 의도·불확실성 수용과 최종 판본을 결정한다. | STS_STATUS와 current gate를 확인; 모델의 READY는 Human release 자체가 아님 |
| 16.12 | 확정한 판본에 결속된 결과를 내보내고 변경에 영향받은 후속 콘티/문서의 stale을 유지한다. | 다른 artifact의 승인을 자동 갱신하지 않음; 실패하면 승인된 repair cycle로 복귀 |

## 4. 증거·판정 의미의 Target 계약

아래 필드는 schema 초안의 의미 사전이다. 기존 public DTO를 변경하거나 새 authoritative state를 integration에 만드는 명세가 아니다. 실제 producer 계약 버전·consumer pin은 owner와 함께 결정해야 한다.

| 기록 | 필요한 필드·의미 | 자동 확정하면 안 되는 것 |
|---|---|---|
| EvidenceRef | producer, document role, immutable snapshot ID, hash algorithm+hash domain, raw UTF-8 `[start,end)` bytes, exact quote | 동일 문구가 여러 번 나오면 quote만으로 위치 선정 금지 |
| Claim | owner-local ID, proposition, actor, target, polarity, epistemic, holder/speaker/viewpoint, narrative time, textual position, evidence refs | FACT는 그 승인된 작품 해석에서 사실이라는 뜻; 현실 사실 검증 완료라는 뜻 아님 |
| Relation | typed endpoints, relation type, applicable time, acquisition/world-rule facets, evidence refs | 한 문장에 같은 인물이 등장했다는 이유로 인과 생성 금지 |
| Obligation | protected meaning, scope, source/author decision refs, sensitivity, permitted transforms | 모델이 중요하다고 분류한 CORE를 사용자 보호 승인과 동치 처리 금지 |
| Alignment | source IDs, output span/OBS IDs, N:M mapping, transformation, support verdict, rationale | COMBINED·DRAMATIZED라는 이름만으로 의미 보존 통과 금지 |
| RetrievalRecord | query, searched snapshot/chunks, returned spans, overflow/truncation, missing portions, retrieval status | 검색 결과가 없다는 이유로 원문 전체에 사실이 없다고 결론 금지 |
| Assessment | method/version/model/prompt IDs, input hashes, scope, judgment, uncertainty, evidence, judge-order result | 숫자 confidence를 보정된 정확도 확률로 포장 금지 |
| ReviewDecision | owner, actor, exact candidate/input hashes, approved scope, accepted unresolved items, timestamp | AI recommendation을 Human approval로 변환 금지 |

`FACT/BELIEF/CLAIM/UNKNOWN`과 `AFFIRM/NEGATE`는 서로 다른 축이다. “미나는 준이 떠났다고 믿었지만, 준은 창고에 숨어 있었다”에는 미나의 `BELIEF(준이 떠남)`과 작품의 `FACT(준이 창고에 있음)`가 공존한다. “그는 떠나지 않은 것은 아니다”처럼 부정 범위가 복합인 문장은 명제의 scope를 보존하고 단일 NEGATE로 단순화하지 않는다. 확정할 수 없으면 원문과 해석 후보를 함께 남긴다.

시간·화자 필드는 실제 정보가 있을 때만 채운다. 인물의 믿음은 holder가 필요하고, 대화의 CLAIM은 speaker가 필요하며, 제한 시점의 추론은 viewpoint와 근거를 보존한다. ‘모른다’와 ‘거짓이다’, ‘언급되지 않는다’와 ‘없다’는 다르다. 한 장에서 살아 있고 다음 장에서 죽은 것은 transition이 있으면 모순이 아니다.

판정 축도 분리한다.

- **SUPPORTED:** 현재 검색 범위에서 동일한 actor·polarity·epistemic·time·관계 의미를 지지하는 근거가 있다. 모델 판단인 경우 HUMAN 검토 여부는 별도다.
- **CONTRADICTED:** 같은 scope의 명시 근거가 충돌한다. 충돌 evidence를 함께 제시한다.
- **UNSUPPORTED:** 정해진 검토 범위를 다 확인했지만 지지 근거를 찾지 못했다. 허용된 창작 추가인지 별도 transformation review가 필요하다.
- **UNCERTAIN:** 생략 주어·모호성·상충·불완전 검색 등으로 판단할 수 없다. 이 상태를 UNSUPPORTED나 SUPPORTED로 자동 낮추거나 올리지 않는다.
- **FAILED / CANCELLED / OUTCOME_UNKNOWN:** 수행 상태다. 의미 verdict와 별도이며 성공으로 치환할 수 없다.

## 5. 점수와 검토 관문

제안하는 관측치는 합격 숫자를 미리 정하기 위한 것이 아니다. baseline과 한국어 검토 집합을 만들고 의미별 실패를 드러내기 위한 것이다.

1. **구조 검사:** source/hash/span/ID/현재 revision/coverage가 일치하는가. 합치면 STATIC PASS이며 의미 품질을 입증하지 않는다.
2. **추가 검출:** output의 material claim·relation이 승인 source 또는 승인된 발명 결정에 연결되는가. 자동 결과는 항목별 진단으로 유지한다.
3. **누락 검출:** source protected/core obligation이 output에서 회수되는가. `supported output / assessed output`과 `preserved required / all required`를 혼합하지 않는다. 분모가 0이면 N/A이며 100%로 표시하지 않는다.
4. **의미 검사:** polarity·epistemic·행위자·관계·결말·형식 기능별 위반과 uncertainty를 분리한다. 모든 material alignment의 판정 유무를 먼저 보고 평균을 본다.
5. **작품 검토:** 실제 전후 문장을 읽고 작가 의도·매체 효과·선호를 결정한다. 동일한 의미를 여러 표현이 보존할 수 있고 의도된 침묵·모호성도 작품 선택이다.

현재와 다른 cutoff, 새 automatic PASS, 모델 종합점수 기반 release는 이 메모에서 승인하지 않는다. qualification에서는 사람의 라벨과 모델 진단을 비교해 결함 종류별 false negative/false positive·abstention을 측정한다. 같은 작품의 비슷한 변형이 train/dev/test에 나뉘어 성능을 부풀리지 않도록 작품 단위 분리를 사용한다. 실제 사용자 원고가 없으므로 아직 해당 측정은 NOT_RUN이다.

## 6. 필수 적대 사례와 기대 결과

모두 향후 fixture 사양이다. 아래의 기대값은 자동 검사만으로 달성했다는 뜻이 아니다.

| ID | 입력·변형 | 반드시 관측할 것 |
|---|---|---|
| F-Q01 | 원문: ‘미나는 준이 떠났다고 믿었다. 준은 창고에 숨었다.’ → 요약: ‘준은 떠났다.’ | belief→fact 및 반대 원문 근거 제시; fidelity 실패 |
| F-Q02 | 원문: ‘미나는 떠나지 않았다.’ → 요약: ‘미나는 떠났다.’ | polarity drift, 짧은 문장도 제거되지 않음 |
| F-Q03 | 1장: 준은 비밀을 모름. 3장: 편지를 읽고 앎. | 시간에 따른 정상 transition으로 보존; 사건 전부터 아는 대본은 acquisition 누락 |
| F-Q04 | 사건 A와 B는 남았으나 ‘A 때문에 B’가 삭제됨 | event coverage 통과와 relation failure를 함께 표시 |
| F-Q05 | 모든 요약 문장은 맞지만 마지막 선택·결말을 생략 | precision이 높아도 protected coverage 실패 |
| F-Q06 | 원문 중간에만 있는 ‘문을 열면 기억을 잃는다’ 규칙이 후반에 무시됨 | 중간 근거 회수와 cost/world-rule drift 표시 |
| F-Q07 | 같은 문장 두 번, 이름 같은 다른 인물, 겹치는 대사 | 해석 근거를 정확한 occurrence/span에 결속; 임의 병합 금지 |
| F-Q08 | 한글 NFC/NFD, CRLF/LF, 앞부분 한 글자 수정 | raw hash 불일치 탐지; display normalization으로 stale 통과 금지 |
| F-Q09 | 영주어·높임말 변화가 있는 ‘가셨어요. 그런데 안 가셨대요.’ | 화자/대상/전언 scope가 불명확하면 UNCERTAIN |
| F-Q10 | 믿을 수 없는 화자·꿈·가정·편지 속 거짓말 | narrative frame 보존; 세계 FACT로 평탄화 금지 |
| F-Q11 | 허구 세계에서 사후 편지가 도착함; 현실 검색은 불가능하다고 답함 | 작품 규칙을 현실 지식으로 덮어쓰지 않음 |
| F-Q12 | 행동으로 표현된 용서를 원문의 ‘용서했다’ 문장이 없다는 이유로 누락 처리 | recovered 행동 근거로 DRAMATIZED 가능; 문장 일치 강요 금지 |
| F-Q13 | 동일한 대본 사건이나 마지막 결정을 주인공→조연으로 이동 | decisive actor ownership drift |
| F-Q14 | 단서가 있는 원문의 일부 chunk가 retrieval/token limit으로 누락 | retrieval PARTIAL·UNCERTAIN; evidence 부재를 거짓으로 확정 금지 |
| F-Q15 | A/B 순서를 바꾸자 더 나중에 나온 후보가 매번 승리 | judge unstable; 자동 채택 중단 |
| F-Q16 | 평가자가 JSON 대신 빈 문자열·설명을 반환하거나 취소 뒤 늦게 완료 | FAILED/CANCELLED/OUTCOME_UNKNOWN 유지; 지원·성공 fallback 금지 |
| F-Q17 | source는 비극, 보호 메모는 ‘행복한 결말 유지’이며 결정 이력 없음 | CON으로 노출; 새 파일·더 구체적인 문장만으로 우선권 추정 금지 |
| F-Q18 | synopsis text는 같고 보호 관계·ending 의미만 바뀜 | approval identity 차이와 하위 검토 stale |
| F-Q19 | inverse reviewer에게 source/blueprint를 넣으면만 결말을 회수함 | source-blind 재검증 실패; 의도 문서에 의해 결과 결손을 은폐하지 않음 |
| F-Q20 | texture 생략은 허용되어 있지만 핵심 불확실성을 자동 해소함 | 허용 omission과 unauthorized invention을 분리 |

위치 실험은 작품의 사건 순서를 섞지 않는다. 동일 snapshot의 근거 packet에서 결정적 자료 위치만 통제하거나 합성 QA fixture를 사용하고, 원문 order와 event time을 계속 보존한다. 한국어 fixture는 원문형·자연스러운 paraphrase·실제로 의미가 달라진 최소 대조쌍을 함께 둔다. 단순 영어 번역만으로 한국어 qualification을 대체하지 않는다.

## 7. 실제 구현 시 우선순위

- **먼저:** geultu 원문 근거와 보존표가 NVC까지 전달될 계약, raw snapshot+span validation, current hash 검사를 보강한다. 이미 존재하는 STS typed source와 reverse reconstruction 경계를 유지한다.
- **다음:** 추가 검출·누락 검출의 양방향 evidence table, retrieval completeness, typed epistemic/time 질문을 owner의 검토 자료로 만든다.
- **그다음:** 실제 한국어 원고에서 판단 기준을 맞추고, provenance와 보호 조건을 손상시키지 않는 부분 수정·독립 검증을 측정한다.
- **유보:** 외부 benchmark scorer를 점수 API로 바로 설치, 자동 모델 점수로 source approval/final release 대체, StoryER 코드 복사, single score leaderboard 최적화.

## 8. 조사 식별과 검증 상태

외부 repository의 commit SHA는 2026-09-22 GitHub API default branch 조회값이다. 해당 SHA의 raw 소스·LICENSE를 정적으로 읽었다. StoryER는 API license null과 LICENSE 미발견이므로 라이선스 확인으로 표시하지 않는다. API 조회와 원문 다운로드는 read-only 조사이며 외부 코드 실행은 하지 않았다.

| 로컬 조사 파일 | SHA-256 |
|---|---|
| geultu/crates/geultu/src/authoring/mod.rs | `41f36eb8709d36a3c4746ea426127860a1e6bffa7fce30936ce063823f208726` |
| geultu/crates/geultu/src/store.rs | `a5296d1776c4f57252b39d6be8ff14c14483f697553c8e9ff6142d65f4c98561` |
| NarrativeCourier/skills/sts/prompts/analyst/step_00_source_graph.md | `eb7519fa6ce1254756bb4368cf357cee7718fad8d1775ba558f9e42d39dc7a81` |
| NarrativeCourier/skills/sts/prompts/critic/reconstruct_screenplay.md | `2089f4285cc5ebe2b3a8750a1a38b413673df8d5257f55c7f57341329064bc9d` |
| NarrativeCourier/skills/sts/prompts/critic/step_08_verify.md | `ad3883d21a6a968858a4bffe5db8720f22ddbcbf148407f8c6f3859d7ff19bd2` |
| NarrativeCourier/skills/sts/schemas/audit-source-ir.schema.json | `3f1fc150893301b6c17941cf2e9a54dd96c668b5adf9b1ac09a5bc1380283739` |
| NarrativeCourier/tools/sts_audit.py | `1c4df27bd25fbeba1a8c28df18d6945bf55b4ddf7588fcc1426aefd864519733` |

연구·정적 코드 조사: DONE/STATIC. 제품 구현·외부 모델 성능·한국어 원고 품질·20개 적대 fixture 실행: NOT_RUN. 링크의 존재와 해시를 확인하는 문서 점검은 작품 의미 검증과 별개다.
