# One Page

> Compact summary of the Blueprint Target. Current CLI support and qualification are in the [product README](../../README.md) and [Current](../IMPLEMENTATION_STATUS.md).

## 한 문장

**Blueprint Target — NarrativeCourier = STS + DirectorIR + Shared Runtime:** STS가 한 번 확정한 Narrative를 DirectorIR이 다시 쓰지 않고 여러 delivery로 표현하고, 공유 asset과 검증 receipt로 의미 손실과 중복 재작업을 통제한다.

## Authority

```text
STS         무엇이 참인가        NarrativePackage
Human/App   무엇을 만들 것인가  DeliveryRequest
DirectorIR  어떻게 표현할까      DirectorPlan
Runtime     effect/publish owner  AssetIndex + lifecycle
Renderer    어떻게 실행할까      artifact receipt
Verifier    손실이 허용 범위인가 DeliverySet
```

## Fixed surface

- 5 durable contracts: `NarrativePackage · DeliveryRequest · DirectorPlan · AssetIndex · DeliverySet`
- 5 passes: `P0 Validate · P1 Direct · P2 Resolve Assets · P3 Render · P4 Verify + Package`
- 2 external boundaries: `AgentExecutor · MediaBackend`

## 핵심 불변식

1. Narrative meaning은 STS만 바꾼다.
2. DirectorIR의 viewer-facing `content`와 semantic-bearing asset brief는 Narrative authority에 추적 가능해야 한다.
3. Renderer는 Narrative를 읽고 의미를 새로 만들지 않는다.
4. pre-render semantic BLOCK은 renderer 호출과 DeliverySet publish를 모두 막는다.
5. PASS는 schema뿐 아니라 cross-document invariant까지 통과해야 한다.
6. 같은 canonical asset은 여러 delivery가 재사용한다.

## 완료 기준

새 delivery 추가가 STS 재실행, Narrative 재해석, 동일 asset 재생성, 무관한 delivery rebuild를 요구하지 않아야 한다.
