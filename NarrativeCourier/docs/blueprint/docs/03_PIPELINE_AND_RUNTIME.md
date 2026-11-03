# 03. Pipeline & Runtime

> **Current/Target boundary:** P1–P4 below define the durable target boundary. NarrativeCourier 0.12.1 currently realizes the deterministic minimal slice with the in-process exact-PRESERVE planner and built-in backend descriptors; AgentExecutor/MediaBackend wire protocols remain future external boundaries, not claimed Current runtime wiring.

## P0 Validate

- five-contract schema validation
- cross-document invariant validation
- artifact/path/hash validation
- backend `DESCRIBE` 수집 및 deterministic route
- stale/canceled owner validation

P0 실패는 Agent/Renderer를 호출하지 않는다.

## P1 Direct

AgentExecutor가 isolated workspace에서 `DirectorPlan` 후보를 만든다. Runtime/DirectorIR이 schema와 invariant를 검증한 뒤 plan을 승인한다.

그 직후 **independent semantic preflight**가 viewer-facing `ProjectionUnit.content`와 narrative-bearing asset brief/source를 Narrative authority와 대조한다.

```text
semantic BLOCK
→ renderer 0
→ AssetIndex/DeliverySet canonical publish 0
```

PASS evidence는 transient run evidence로 유지하고, 최종 성공 시 DeliverySet semantic receipt에 결합한다.

## P2 Resolve Assets

DirectorPlan의 unique AssetRequest만 materialize한다. 동일 build key는 재사용하고 mixed provider provenance를 AssetIndex item에 기록한다.

## P3 Render

각 output은 독립 effect다. Renderer는 DirectorPlan slice + AssetIndex만 실행하며 Narrative를 해석하지 않는다.

## P4 Verify + Package

semantic preflight evidence + rendered observable result + visual/technical evidence를 검증한다. 성공/실패 상태가 contract와 일치할 때만 `DeliverySet`을 atomic publish한다.

## Runtime responsibility

- project-safe paths
- raw/JCS hash utility
- project/run locks
- atomic publish
- process stdin/timeout/cancel lifecycle
- stale/late owner rejection

Runtime은 STS/DirectorIR domain DTO를 소유하지 않는다.

## Current minimal reuse rule

Output artifact cache identity는 request 전체가 아니라 exact output-local `DeliveryPlan`, 해당 output이 참조하는 materialized assets, backend fingerprint, renderer version에만 결속한다. 기존 artifact는 technical/hash revalidation 후에만 재사용한다.

## Human review closure

Visual observable semantics를 자동 증명할 수 없으면 `NEEDS_REVIEW`로 publish한다. Human approve/reject는 같은 `DeliverySet`을 atomic update하며 approval은 automatic FAIL을 override할 수 없다.
