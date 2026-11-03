# Contract Audit Report

Date: 2026-08-19

## Result

**AUDIT: PASS**

Validated package contents:

- 22 JSON Schemas
- 13 canonical stages
- 40 stage substeps
- 8 versioned PromptSpecs
- 4 Director preset examples
- 76 registered failure codes
- 22 runtime invariants
- 2 external port contracts
- 8 shadow-story golden fixture artifacts

## Checks executed

1. JSON Schema Draft 2020-12 meta-validation.
2. Every stage JSON against `stage-contract.schema.json`.
3. Every declared stage input/output schema path exists.
4. Every stage/substep failure code exists in the central failure catalog.
5. Every substep failure is declared by its parent stage.
6. Every `next.on_pass` target resolves.
7. Stage graph is acyclic and all 13 stages are reachable from S00.
8. Every produced artifact kind has a single producer stage.
9. Artifact catalog schema references resolve.
10. Every PromptSpec validates; stage/output schema references resolve.
11. Every Director preset validates.
12. Both external port contracts validate and reference existing schemas.
13. Runtime invariant IDs are unique.
14. Golden fixture artifacts validate against their schemas.
15. Every golden ShotIR asset reference resolves to AssetManifest.
16. Every fixture meaning obligation reference resolves.
17. Every hard transition obligation is covered in AdaptationIR and ShotIR.
18. No unused golden fixture asset remains.
19. Static grep report emitted for stage IDs, schema refs and failure codes.

## Defect found and corrected during audit

S01 internal substep P03 could raise:

- `SEM_004_CAUSAL_LINK_LOST`
- `SEM_005_INTENTION_LOST`
- `SEM_007_HARD_ASSERTION_MISSING`

but the parent S01 stage initially omitted those three codes from `failure_codes[]`. The parent contract was corrected and the complete audit rerun to PASS.

## Important limit

There is **no NVC runtime source repository yet**, so it is impossible to truthfully claim that actual implementation call sites were grepped. This package instead audits all current schema/stage/prompt/director/failure references and provides `scripts/grep_contract_usage.sh`. When implementation begins, CI must run the same registry/reference audit over the source tree and enforce the call-site rules in `docs/12_CALL_GRAPH_AND_PORTS.md`.

## Command

```bash
python scripts/audit_contracts.py
bash scripts/grep_contract_usage.sh
```
