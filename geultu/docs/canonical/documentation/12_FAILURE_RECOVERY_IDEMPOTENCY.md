# 12. Failure, Recovery, and Idempotency

## failure classes

| class | 의미 | 재시도 |
|---|---|---|
| REJECTED | admission/contract 위반, 효과 없음 | 입력 수정 후 새 command |
| FAILED_VALIDATION | candidate가 gate 불충족 | repair/revision budget 내 새 attempt |
| PROVIDER_FAILED | 외부 호출이 명시적으로 실패 | 정책상 transient일 때만 |
| UNKNOWN_OUTCOME | 외부 효과 결과를 확정할 수 없음 | 같은 effect를 재실행하지 말고 reconcile |
| COMMIT_CONFLICT | expected revision 불일치 | state reload 후 재계획 |
| RECOVERY_REQUIRED | 부분 효과/고아 artifact 존재 | doctor/recovery command |

## idempotency

`idempotency_key`는 command kind + project/run id + expected revision + normalized input digest에 묶인다. 같은 key와 다른 input은 거부한다. 완료 receipt가 있으면 동일 committed result를 반환하고 효과를 반복하지 않는다.

## atomicity

local store에서 state, named output refs, commit receipt는 하나의 atomic replace/transaction으로 보인다. 지원할 수 없는 adapter는 해당 route를 제공하지 않는다.
