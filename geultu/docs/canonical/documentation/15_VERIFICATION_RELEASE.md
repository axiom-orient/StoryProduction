# 15. Verification and Release Gates

## 계약 gate

- 모든 schema 자체 검증
- valid fixture accept / invalid fixture reject
- unknown field reject
- duplicate key reject
- JCS digest golden vector
- T33 deterministic package_id vector
- legacy envelope reject
- receipt subject/binding mismatch reject
- named multi-output commit fixture

## 구현 gate

- Requirement → Schema → Implementation symbol → Runtime wiring → Test 추적성 완결
- Text route smoke: approved project → TextMaster commit
- Full route smoke: TextMaster → T33 → NVC admission → first visual output commit
- crash/atomicity test
- stale revision/idempotency/UNKNOWN_OUTCOME recovery test
- ScriptForge lock and doctor test

## 완료 정의

문서·schema만 통과하면 `SPEC_READY`다. 실제 runtime wiring과 toolchain test까지 통과해야 `IMPLEMENTED`; 실제 provider route와 recovery를 검증해야 `OPERATIONALLY_VERIFIED`다.
