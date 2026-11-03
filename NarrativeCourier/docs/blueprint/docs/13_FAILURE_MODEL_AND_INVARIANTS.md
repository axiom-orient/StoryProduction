# 13. Failure Model & Invariants

## Invariants

1. STS alone may mutate Narrative meaning.
2. Every non-OMIT projection unit has valid Narrative refs and viewer-facing content.
3. semantic-bearing asset brief/source cannot create unsupported Narrative claims.
4. DeliveryRequest output IDs, DirectorPlan asset/unit IDs, and DeliverySet output IDs are unambiguous.
5. DirectorPlan asset refs cannot dangle.
6. TIMED unit frame sum equals requested duration frames.
7. Backend cannot publish canonical files directly.
8. canonical JSON/file hash must match before consumption.
9. pre-render semantic BLOCK means renderer calls 0 and DeliverySet publish 0.
10. stale/canceled owner cannot publish.
11. renderer selection is deterministic, never silent fallback.
12. top-level/delivery PASS cannot coexist with failed required verification.
13. repair returns to the smallest owning artifact.

## Failure codes

### P0
`INVALID_SCHEMA · INVALID_INVARIANT · HASH_MISMATCH · MISSING_ARTIFACT · PATH_ESCAPE · UNSUPPORTED_CAPABILITY · AMBIGUOUS_BACKEND`

### P1
`LINEAGE_MISSING · ILLEGAL_OMISSION · NARRATIVE_DRIFT · INVALID_ASSET_REQUEST · INVALID_VISUAL_GRAMMAR`

### P2
`ASSET_FAILED · ASSET_HASH_MISMATCH · ASSET_IDENTITY_CONFLICT · BACKEND_TRANSIENT · BACKEND_PERMANENT`

### P3
`RENDER_FAILED · RENDER_CANCELED · STALE_RESULT · INVALID_RECEIPT`

### P4
`SEMANTIC_FAIL · VISUAL_FAIL · TECHNICAL_FAIL · NEEDS_HUMAN_REVIEW`

## Retry

- transient execution failure: same semantic/build input, new attempt.
- contract/semantic failure: no blind retry; owning artifact를 수정한다.
- cancel: attempt terminal, canonical output unpublished.
