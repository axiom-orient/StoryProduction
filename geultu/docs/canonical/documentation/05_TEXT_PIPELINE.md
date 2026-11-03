# 05. Text Pipeline

## 역할

T00–T33은 하나의 거대한 함수가 아니라 동일 실행 protocol을 공유하는 ordered stage graph다. 각 stage의 필드·validator selector·input/output kind는 machine-readable stage catalog가 소유한다.

## 단계군

| 단계군 | 책임 | 출력 성격 |
|---|---|---|
| T00–T05 | 승인 입력 정규화·project binding | 계획·제약 artifact |
| T06–T17 | 구조·서사·장면 설계 | intermediate semantic artifacts |
| T18–T29 | 초안·refinement·quality | candidate/review artifacts |
| T30–T32 | 최종 검증·승인·TextMaster commit | immutable TextMaster |
| T33 | Geultu story projection | GeultuStoryPackage payload |

## TextMaster

TextMaster body는 텍스트와 semantic lineage만 가진다. 자기 ArtifactRef, commit receipt, 승인자 signature를 body에 넣지 않는다. provenance와 human approval은 commit record에서 연결한다.

## budget

`attempt`, `revision_budget`, `pivot_budget`는 선언형 type만 두지 않고 TextRunState가 소유한다. 각 command admission에서 감소 가능 여부를 검사하고 reducer만 갱신한다. 소진 시 자동 fallback이 아니라 명시적 terminal failure 또는 HumanDecision이다.
