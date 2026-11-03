# Handoff

## Canonical decision

앞단 NDS가 synopsis와 최종 story/screenplay를 만든다. NVC의 유일한 정상 입력은 canonical narrative 하나를 담은 `NarrativePackage`다.

## 구현 시작점

- `schemas/narrative-package.schema.json`
- `schemas/stage-contract.schema.json`
- `contracts/stages/`
- `contracts/prompts/`
- `catalogs/failure-codes.json`
- `fixtures/shadow-story/`
- `scripts/audit_contracts.py`

## Phase 0 Done

1. schema meta-valid
2. stage contract valid
3. prompt/director contract valid
4. dangling schema/failure/next ref 0
5. stage cycle 0
6. output kind multi-owner 0
7. fixture schema validation PASS
8. fixture asset/obligation ref PASS
9. hard failure가 downstream PASS로 누출되지 않는 runtime test
10. failure가 root owner로 route되는 runtime test

문서 완료는 구현 완료가 아니다. 이 패키지는 **구현 가능한 계약 기준선**이다.
