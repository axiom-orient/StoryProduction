# 05. Verification, Failure, RCA Repair

## 검증 계층

### Structural
schema, IDs, refs, DAG, decode, fps, duration.

### Semantic
hard assertion 100%, hard transition recoverability 100%, contradiction 0.

### Continuity
character/world/prop/time/location state.

### Production invariants
예: `NO_HUMAN_SHADOW`, source-exact text, fixed object count, precise camera path.

### Soft quality
미학, 감정, novelty, pacing. hard gate를 override하지 않는다.

## Repair ownership

| Failure family | Root owner |
|---|---|
| SRC | S00 |
| SEM | S01 |
| DIR | S02 |
| ADP | S03 |
| SCN | S04 |
| SHT | S05 |
| AST | S06 |
| GEN | S07 |
| RND planning | S08 |
| MOT / render | S09 |
| CMP | S10 |
| QAV | 원인이 처음 발생한 upstream stage / uncertain이면 HUMAN |
| PKG | S12 |

## Bounded repair

`최초 실행 + StageContract.max_repairs`. budget 소진 시 `SYS_005_RETRY_BUDGET_EXHAUSTED`.

무한 reflection loop를 만들지 않는다.

## Semantic recoverability

transition obligation마다 질문과 accepted meaning을 가진다. 최종 visual segment를 기반으로 evaluator가 답하게 하고 의미 등가성을 평가한다.

- high-confidence equivalent → PASS
- contradiction/non-recoverable → FAIL
- low confidence → **NEEDS_REVIEW**

low confidence를 자동 PASS로 승격하지 않는다.
