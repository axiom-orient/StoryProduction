# 04. Execution Protocol

## 단일 실행 흐름

`Command → Admission → Candidate Generation → Materialization → Validation → Domain Decision → Atomic Commit → Receipt/Output`

1. **Admission:** schema, actor/session, expected revision, capability, input refs, budget를 검사한다.
2. **Generation:** provider는 candidate bytes와 provider metadata만 반환한다.
3. **Materialization:** program이 canonical body를 만들고 JCS digest를 계산한다.
4. **Validation:** validators가 body·input binding·policy를 독립 검사한다.
5. **Decision:** reducer가 required receipt selector와 현재 상태로 accept/reject/repair를 결정한다.
6. **Commit:** named output map, 새 state, commit receipt를 하나의 transaction boundary에서 기록한다.
7. **Output:** committed refs와 결정 receipt를 반환한다.

## Named output map

모든 stage는 `outputs: { logical_name: ArtifactRef }`를 사용한다. 단일 산출 stage도 동일하다. S01·S10처럼 여러 산출물을 만드는 NVC stage를 별도 API 없이 표현한다.

## 금지

- raw model response를 바로 ArtifactRef로 저장
- caller가 전달한 digest·passed를 재검산 없이 수용
- state 저장 후 artifact 저장처럼 부분 commit을 허용
- validation 실패를 warning으로 바꾸어 진행
