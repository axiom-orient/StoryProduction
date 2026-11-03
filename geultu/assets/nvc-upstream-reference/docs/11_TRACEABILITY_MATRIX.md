# 11. Requirement → Contract → Gate Matrix

| Requirement | Contract | Gate |
|---|---|---|
| 최종 narrative authority 하나 | NarrativePackage.canonical_narrative | S00 |
| synopsis는 planning evidence | SynopsisArtifact.purpose | S00 |
| 원문 traceability | SourceSpan | S01 |
| explicit/inferred/creative 분리 | origin enum | S01 |
| 인과/의도 보존 | transition_obligations | S03 + S11 |
| world consistency | state_in/delta/out | S04 |
| shot renderability | ShotIR | S05 |
| hard invariant enforcement | hard_constraints → RenderIR.invariants | S05/S08/S09 |
| asset 누락 방지 | asset_refs ↔ AssetManifest | S06 |
| identity drift 방지 | forbidden_mutations + asset eval | S06/S07 |
| backend 교체 가능 | CapabilityProfile/RenderIR | S08 |
| 정확한 timing | TimelineIR frame index | S10 |
| semantic final QA | EvaluationReport | S11 |
| RCA repair | failure namespace + repair stage | runtime/S11 |
| bounded retries | max_repairs | runtime |
| incremental rebuild | artifact graph/consumers | scheduler |
| 배포 provenance | parent_ids + optional C2PA | S12 |
