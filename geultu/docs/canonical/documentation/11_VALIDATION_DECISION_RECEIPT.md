# 11. Validation, Decision, and Receipt

## 분리

- **ValidationReceipt:** 관찰 가능한 계약 사실을 증명한다.
- **DomainDecision:** 현재 state와 receipt set을 기반으로 다음 상태를 결정한다.
- **ArtifactCommit:** accepted body와 state transition을 원자적으로 기록한다.

validator는 `accepted`, `advance`, `approved`를 결정하지 않는다. reducer는 schema 검사 결과를 위조하지 않는다.

## receipt binding

receipt는 최소 다음에 묶인다.

- validator id/version
- subject body digest
- binding digest(input refs, project/run revision, policy version)
- outcome PASS/FAIL
- finding codes
- created_at 또는 deterministic run sequence

receipt selector는 문자열 비교가 아니라 typed 조건(`validator_id`, version range, subject name, outcome)을 사용한다.

## human decision

Human approval은 일반 validator pass가 아니다. 별도 `HumanDecision` receipt로 기록하며 exact subject digest, decision, scope, actor, timestamp를 포함한다.
