# Narrative Visualization Compiler — Contract-First Design Package v1.0

이 패키지는 **확정된 서사(Narrative Package)를 고품질 시각 콘텐츠로 컴파일하는 시스템**을 구현하기 위한 설계·계약·검증 패키지다.

## 최종 경계

- 앞단 **Narrative Development System (NDS)**: 아이디어/원문 → 시놉시스 → 서사 개발 → 최종 스토리 또는 시나리오.
- 후단 **Narrative Visualization Compiler (NVC)**: 확정된 `NarrativePackage` → 의미 IR → 연출 → 장면 → 샷 → 자산 → 렌더 → 합성 → 검증 → Master.
- NVC 정상 진입점은 **`NarrativePackage` 하나**다.
- `canonical_narrative`만 서사적 authority다. `planning_synopsis`는 planning provenance/intent evidence다.
- story와 screenplay가 둘 다 존재해도 NDS가 하나를 canonical로 선택해 넘긴다. NVC가 임의 선택하지 않는다.

## Canonical pipeline

`S00 → S01 → S02 → S03 → S04 → S05 → S06 → S07 → S08 → S09 → S10 → S11 → S12`

각 stage는 `contracts/stages/*.json`에 **입력, 출력, executor, owner, substep, hard gate, failure code, retry, next**가 machine-readable 형태로 있다.

## 검증

```bash
python scripts/audit_contracts.py
bash scripts/grep_contract_usage.sh
```

Golden fixture: `fixtures/shadow-story/`.

## 읽는 순서

1. `docs/00_EXECUTIVE_DECISION.md`
2. `docs/01_BOUNDARY_AND_AUTHORITY.md`
3. `docs/02_PIPELINE.md`
4. `docs/03_IR_AND_SCHEMAS.md`
5. `docs/04_STAGE_CONTRACTS.md`
6. `docs/05_VERIFICATION_REPAIR.md`
7. `docs/06_DIRECTOR_RENDER_ROUTING.md`
8. `docs/07_INCREMENTAL_PROVENANCE.md`
9. `docs/08_IMPLEMENTATION_PLAN.md`
10. `reports/CONTRACT_AUDIT.md`
