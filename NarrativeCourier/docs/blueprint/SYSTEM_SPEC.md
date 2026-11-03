# System Specification

## Canonical data flow

```text
STS final Human release
  → NarrativePackage
Human/App
  → DeliveryRequest
P0
  → schema/hash/ref/capability validation
P1
  → DirectorPlan
  → independent semantic preflight
      BLOCK → stop; renderer calls 0; DeliverySet publish 0
      PASS  → continue
P2
  → AssetIndex
P3 per output
  → isolated render artifact + backend receipt
P4
  → semantic/visual/technical final verification
  → DeliverySet atomic publish
```

## Source of truth

- Narrative truth: STS artifacts referenced by `NarrativePackage`.
- Projection truth: `DirectorPlan`.
- Shared materialized asset truth: `AssetIndex`.
- Published delivery truth: `DeliverySet`.
- Run/process truth: Runtime lock/cancel/publish state.

## Dependency direction

```text
runtime ← sts
runtime ← director-ir
contracts/protocol ← director-ir + adapters
renderer adapters ← contracts/protocol
app → sts/director-ir

sts ↛ director-ir
director-ir ↛ sts internals
renderer ↛ Narrative authority
```

## Contract authority

- JSON Schema validates shape and local field constraints.
- invariant validator validates uniqueness, dangling refs, lifecycle/status coherence, timed duration, and cross-document identity.
- runtime verifies actual referenced file bytes/hashes before consumption/publish.

## Hashing

- Cross-boundary JSON content hash: **RFC 8785 JCS bytes → SHA-256**.
- File artifact hash: **raw bytes → SHA-256**.
- Fixture verifier uses only an integer/string JSON subset compatible with JCS; it is **not** production JCS proof.

## Backend selection

1. explicit `backend_id` pin → exact backend only.
2. otherwise configured preference among compatible descriptors.
3. no preference + exactly one compatible backend → select it.
4. zero compatible backends → `UNSUPPORTED_CAPABILITY`.
5. multiple compatible backends + no preference → `AMBIGUOUS_BACKEND`.

Silent fallback is forbidden.
