# 06. Verification & Repair

검증은 세 층이다.

1. **Semantic** — viewer-facing 결과가 Narrative fact/actor/relation/causality/chronology/world-rule/knowledge/ending을 왜곡하지 않는가.
2. **Visual** — identity/continuity/composition/legibility/safe-area contract를 만족하는가.
3. **Technical** — file/hash/load/dimension/duration/frame/codec contract를 만족하는가.

## Two semantic checkpoints

### Pre-render

DirectorPlan의 semantic-bearing surface를 Narrative authority와 대조한다. BLOCK이면 renderer 호출 0, DeliverySet publish 0.

### Post-render

실제 observable artifact가 plan의 의미를 보존했는지 검증한다. 이 결과는 DeliverySet semantic receipt에 들어간다.

## Repair owner

```text
Narrative defect       → STS
Projection/content defect → DirectorIR P1
Shared asset defect    → P2
Render/backend defect  → P3
Verification/package   → P4
```

Verifier가 다른 owner의 artifact를 자동 수정하지 않는다.

## Minimal repair

failure는 `owner + root_artifact + affected_outputs`로 환원하고 dependency closure만 invalidation한다. 전체 pipeline 재실행은 마지막 수단이다.
