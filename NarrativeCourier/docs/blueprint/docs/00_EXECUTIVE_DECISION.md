# 00. Executive Decision — Blueprint Target

이 blueprint의 설계 결정은 Narrative 의미를 STS에 단일 소유시키고, 승인된 Narrative를 purpose-specific projection으로 전달하는 것이다.

- STS만 Narrative truth·source fidelity·Human gate·Completed release를 소유한다.
- Delivery는 기존 다섯 durable contract `NarrativePackage`, `DeliveryRequest`, `DirectorPlan`, `AssetIndex`, `DeliverySet`을 사용하며 새 semantic authority를 만들지 않는다.
- DirectorIR은 presentation projection만 정하고, Runtime과 renderer/backend는 자기 effect lifecycle만 소유한다.
- 새로운 durable contract·stage·shared domain은 실제 caller variation과 기존 boundary로 표현할 수 없는 invariant가 확인될 때만 결정한다.
- Capability mismatch, stale/canceled work, partial external effect와 review outcome은 명시적으로 남긴다. 추정 fallback이나 실행 성공으로 바꾸지 않는다.

세부 authority와 field/invariant는 [boundary](01_BOUNDARY_AND_AUTHORITY.md), [canonical contracts](02_CANONICAL_CONTRACTS.md), [pipeline target](03_PIPELINE_AND_RUNTIME.md), [failure model](13_FAILURE_MODEL_AND_INVARIANTS.md)에 둔다. 현재 public CLI와 support scope는 [제품 README](../../../README.md), date-scoped execution evidence는 [제품 Current](../../IMPLEMENTATION_STATUS.md)가 소유한다. 이 페이지는 blueprint의 executive Target locator다.
