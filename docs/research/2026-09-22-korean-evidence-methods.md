# 한국어 원문·글투·근거 위치 연구

조사일: 2026-09-22. 웹 표준·원 논문·공개 GitHub 소스를 읽었다. 아래 ‘적용’은 StoryProduction의 Target 설계이며 라이브러리 설치·제품 통합·한국어 작품 품질 실험을 수행했다는 뜻이 아니다.

## K01 — Kiwi/kiwipiepy: 한국어 분석은 원문 위의 보조 관측

- 원본: [저장소](https://github.com/bab2min/kiwipiepy), [고정 코드](https://github.com/bab2min/kiwipiepy/blob/5d5e174426ee29e0796527dfb006624303cf3992/kiwipiepy/_wrap.py#L2038).
- 조사 snapshot: `5d5e174426ee29e0796527dfb006624303cf3992`, commit timestamp 2026-09-17. `split_into_sents`의 입력 옵션과 원문 token span으로 Sentence를 조립하는 코드를 확인했다.
- 라이선스: [LICENSE.txt](https://github.com/bab2min/kiwipiepy/blob/5d5e174426ee29e0796527dfb006624303cf3992/LICENSE.txt)에 LGPL 2.1 또는 이후 버전 표시. GitHub 자동 인식은 NOASSERTION이므로 파일을 기준으로 기록한다. 모델·배포물의 조건은 실제 통합 때 별도 확인한다.
- 관측: 문장 분리·형태소 관측과 start/end 위치를 얻을 수 있다. `normalize_coda` 등 정규화 옵션이 있으며 출력 위치를 raw UTF-8 byte offset과 동일시하면 안 된다.
- **적용 제안:** S01에서 원문 bytes를 보존하고 분석 view만 만든다. Python 문자 인덱스를 byte 위치로 옮길 때 `len(text[:index].encode('utf-8'))`로 명시적으로 변환하고 원문 slice와 인용문을 대조한다. S06/S15에서는 말끝·호칭·서술/대화 비율을 관찰 자료로 사용한다. 도구 교정 결과를 원문이나 작가 문체 정본으로 덮지 않는다.
- 한계: 형태소 분석은 화자 의도·비꼼·관계 변화·문학적 품질을 판정하지 않는다. 같은 비율은 같은 문체의 증거가 아니다. 고유명사·방언·생략·인용 안의 문장 경계를 실제 원고에서 확인해야 한다.

## K02 — 한국어 높임법에 대한 최근 실험의 제한

- 논문: Youngdong Cho, Chloe Dokyung Kwon, *Can language models process linguistic deference?*, SCiL 2026, 369–378. [ACL 원문](https://aclanthology.org/2026.scil-main.35/), [PDF](https://aclanthology.org/2026.scil-main.35.pdf).
- 관측: 연구 대상 모델에서 표면적인 형태·통사 불일치 감지와 사회적 맥락에 맞는 주체 높임 처리 사이의 차이를 보고한다. 이는 특정 실험의 결과이며 모든 최신 모델·문학 대사·상대 높임 체계 전체를 검증한 결과가 아니다.
- **적용 제안:** S06/S15의 대사 검토에 화자, 청자, 문장 속 주체, 공식/사적 상황, 관계 변화, 기본 말투, 의도적인 말투 전환을 따로 기록한다. 높임말이 많을수록 좋은 대사라고 점수화하지 않는다. 상대에게 공손한 말투와 문장 속 주체를 높이는 표현을 구분한다. 모욕·거리두기·화해로 말투가 변할 때는 해당 사건과 연결한다.
- 한계: 논문으로 StoryProduction의 화용 검증기를 만들었다고 주장할 수 없다. 한국어 독자의 판단과 원문의 실제 표현 근거가 필요하다. 제품 규칙은 연구에서 직접 검증된 알고리즘이 아니라 연구가 드러낸 위험을 다루는 설계다.

## K03 — W3C Web Annotation: 위치와 인용을 함께 참조

- 1차 자료: W3C Recommendation, *Web Annotation Data Model*, 2017-02-23. [Text Quote Selector](https://www.w3.org/TR/annotation-model/#text-quote-selector), [Text Position Selector](https://www.w3.org/TR/annotation-model/#text-position-selector), [Data Position Selector](https://www.w3.org/TR/annotation-model/#data-position-selector).
- 관측: 인용 문자열 기반 선택과 위치 기반 선택을 구분한다. 텍스트 위치와 데이터 byte 위치도 같은 단위가 아니다. 원본이 바뀌면 위치 참조의 의미가 달라질 수 있다.
- **적용 제안:** source hash + raw UTF-8 `[start,end)` byte 범위 + exact quote를 한 묶음으로 확인한다. 위치는 0 기반, end는 제외한다. 멀티바이트 문자 중간 경계는 거절한다. prefix/suffix는 중복 구절을 사람이 구별하는 보조값이며 stale source를 자동 재결속하는 근거로 사용하지 않는다. normalized view를 만들면 별도 view hash·변환 지도를 둔다.
- 한계: 이 설계는 W3C의 JSON-LD 프로토콜을 그대로 구현한다는 뜻이 아니다. 기존 producer의 raw/canonical hash 의미를 변경하지 않으며 semantic claim과 인용문의 논리 관계는 별도 검토한다.

## 적용 경계

K01은 선택적 분석 adapter 후보, K02는 검증 한계의 근거, K03은 근거 참조 설계의 참고다. 새 공유 NLP core·외부 의존성을 추가하지 않았다. 실제 데이터 전송·API 호출 없이 공개 연구·코드만 조회했다. 인터넷 검색에 사용자 원문을 보내지 않는다.
