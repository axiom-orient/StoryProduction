# 09. Critical Review

## 제거/보정한 구조적 문제

1. duplicate Narrative IR → 제거; STS authority 재사용.
2. video-centric global IR → delivery-local structure로 강등.
3. DeliveryRequest `medium + format` → `format` 하나로 축소.
4. AssetIndex global provider fingerprint → item provenance로 이동.
5. VisualGrammar 모든 축 강제 → 필요한 축만 존재.
6. CapabilityRegistry 별도 service → MediaBackend `DESCRIBE`로 통합.
7. speculative generic workflow abstraction → 실제 두 번째 consumer 전 금지.
8. schema-only authority 표현 → schema + invariant validator로 정정.
9. semantic audit 결과 lifecycle 모순 → pre-render transient evidence / final DeliverySet receipt로 분리.
10. untyped MediaBackend payload/result → operation-discriminated protocol로 고정.
11. DirectorPlan renderer ambiguity → `content`, `duration_frames`, asset source provenance 추가.

## 아직 UNKNOWN / NOT_RUN

- Rust compile/test.
- production RFC 8785 implementation correctness.
- DirectorIR 실제 controlled generation 품질.
- browser/video backend qualification.

## Stop rule

새 IR/stage/interface를 추가하려면 현재 5 contract/5 pass로 표현 불가능한 **두 개 이상의 실제 variation**을 먼저 증명한다.
