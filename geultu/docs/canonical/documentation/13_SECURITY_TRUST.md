# 13. Security and Trust

## trust boundary

- Human: 의미 승인 권한, 실행 권한과 동일하지 않음
- Program: capability와 state transition 권한
- Provider: untrusted candidate producer
- Validator: 제한된 read-only evidence producer
- Effect adapter: 명시적으로 승인된 외부 영향 수행자

## 필수 통제

- workspace-relative path와 root containment
- symlink/path traversal 거부
- input/output size budget
- provider secret의 artifact/evidence 유출 금지
- capability allowlist
- external effect 전 explicit admission
- digest 재계산
- log redaction

GeultuStoryPackage는 실행 명령을 포함하지 않는 data contract다. 문자열을 shell/template로 암묵 평가하지 않는다.
