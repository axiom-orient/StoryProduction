# Agent Contract

Authority order:
1. User request and Human-approved source.
2. STS Narrative authority under `skills/sts/` and `crates/sts/`.
3. Five canonical DTO/invariant contracts under `contracts/` + `crates/contracts/`.
4. Delivery projection rules under `crates/delivery/`.
5. Domain-neutral effect invariants under `crates/runtime/`.
6. Research/reference documents.

## Hard boundaries

- STS is the only Narrative semantic authority.
- Delivery consumes a released `NarrativePackage`; it must not create StoryIR/MeaningSpec/another SceneIR authority.
- Exactly five durable cross-boundary contracts: `NarrativePackage`, `DeliveryRequest`, `DirectorPlan`, `AssetIndex`, `DeliverySet`.
- Renderer/Agent never interpret Narrative or publish a successful request without verified ownership.
- `crates/runtime` stays domain-neutral; add behavior there only when it is a reusable effect primitive.
- Silent fallback, fake PASS, stale/late publish, and unverified capability advertisement are forbidden.

## Current deterministic DirectorIR

Supported: `PRESENTATION`, `FULL|SELECTION|EXCERPT`, canonical `SCN-*` exact `PRESERVE`.

Unsupported capabilities must fail explicitly rather than be approximated: `SUMMARY`, `MOOD`, audience/locale adaptation, free-form constraints, generative/procedural assets.

## Mechanical gates

```bash
./verify-all.sh --renderers
./verify-all.sh --require-rust --renderers
```

Rust gate is distinct. Never convert `NOT_RUN` or `BLOCKED` into PASS.
