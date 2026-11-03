# 03. Authority, State, and Source of Truth

## 권한

| 결정 | 유일한 권한 |
|---|---|
| 창작 의도·canon 승인 | Human |
| 후보 생성 전략 | Agent/Provider |
| capability·stage admission | Program |
| schema·binding·digest 검증 | Validator Program |
| 다음 상태 전이 | Domain reducer |
| 외부 효과 실행 | Effect adapter after admission |

`passed: true`를 provider·caller 입력으로 받지 않는다. pass는 validator가 서명 가능한 receipt로만 발행한다.

## 상태 정본

- `ProjectState`: 동결된 synopsis/style/canon/genre/source refs와 project revision.
- `TextRunState`: 현재 stage, attempt, budgets, accepted outputs, pending decision.
- `VisualRunState`: NVC stage, named outputs, pending repair.
- `Artifact body`: immutable content.
- `Receipt/Event`: 과거 검증·결정·효과의 append-only evidence.

현재 상태를 receipt replay와 snapshot 양쪽에서 독립적으로 소유하지 않는다. snapshot이 current SSOT이며 event는 감사 증거다. replay를 제품 요구로 채택하려면 별도 ADR과 완전한 reducer parity test가 필요하다.

## 불변조건

1. 상태 변경은 reducer가 하나의 admitted command로 수행한다.
2. command는 expected project/run revision을 포함한다.
3. validator receipt는 정확한 body digest와 binding digest에 묶인다.
4. commit 이후 body는 변경하지 않는다. 변경은 새 revision이다.
5. 사람이 승인한 입력 ref는 run 중 암묵적으로 교체되지 않는다.
