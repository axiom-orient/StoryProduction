# NarrativeCourier — STS + DirectorIR Blueprint v1.2

이 패키지는 NarrativeCourier의 **Target specification**이다.

> **하나의 검증된 Narrative → 여러 목적·매체로 loss-controlled projection**

## Current / Target

- **Blueprint v1.2 is a Target specification.** Its five passes and external executor/backend protocols do not assert current wiring.
- Current CLI and supported Delivery capabilities are listed in [제품 README](../../README.md); exact fields and unsupported behavior are in [제품 SPEC](../SPEC.md).
- Date-scoped executable results and remaining qualification are authoritative only in [제품 Current](../IMPLEMENTATION_STATUS.md).
- `AgentExecutor` and external `MediaBackend` are Target boundaries; their presence in blueprint schemas is not a claim of current external execution.

## Authority

- **STS** — Narrative 의미·인과·상태·결말의 유일한 semantic authority.
- **DirectorIR** — 검증된 Narrative의 표현/투영만 결정한다.
- **Runtime** — path/hash/lock/publish/process/cancel 같은 effect primitive를 소유한다.
- **Renderer/MediaBackend** — 승인된 plan을 실행하며 Narrative를 재해석하지 않는다.
- **Verifier** — schema와 semantic invariant를 검증하지만 다른 owner의 artifact를 몰래 고치지 않는다.

```text
Synopsis → STS → NarrativePackage
                    + DeliveryRequest
                    ↓
                 DirectorIR
                    ↓
                DirectorPlan
                    ↓
                 AssetIndex
                    ↓
             render deliveries[]
                    ↓
                 DeliverySet
```

## 고정 구조

- durable cross-boundary contract: **정확히 5개**
- DirectorIR pass: **5개** — `P0 Validate → P1 Direct → P2 Resolve Assets → P3 Render → P4 Verify + Package`
- external execution boundary: **2개** — `AgentExecutor`, `MediaBackend`
- renderer 기술은 adapter이며 core contract를 결정하지 않는다.

## Contract authority

`contracts/`의 5개 JSON Schema는 **shape authority**다. 실제 contract authority는 다음의 합이다.

```text
JSON Schema
+ cross-field invariant validation
+ cross-document invariant validation
```

`protocol/`은 외부 실행 envelope이며 durable artifact 종류를 늘리지 않는다.

```bash
python3 scripts/verify_docs.py
```

## 읽는 순서

1. `ONE_PAGE.md`
2. `docs/00_EXECUTIVE_DECISION.md`
3. `PRODUCT_SPEC.md`
4. `SYSTEM_SPEC.md`
5. `docs/01_BOUNDARY_AND_AUTHORITY.md`
6. `docs/02_CANONICAL_CONTRACTS.md`
7. `docs/03_PIPELINE_AND_RUNTIME.md`
8. `docs/04_DIRECTORIR.md`
9. `docs/05_ASSETS_AND_RENDERERS.md`
10. `docs/06_VERIFICATION_AND_REPAIR.md`
11. `docs/11_PUBLIC_API_AND_CLI.md`
12. `docs/13_FAILURE_MODEL_AND_INVARIANTS.md`
13. `docs/14_TEST_STRATEGY.md`

구현 이력은 제품의 [Current](../IMPLEMENTATION_STATUS.md)와 공개 변경 기록 `CHANGELOG.md`가 담당한다.
