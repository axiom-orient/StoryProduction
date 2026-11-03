# 07. Refinement and Quality

## patch 계약

모든 patch는 다음에 묶인다.

- source artifact ref와 body digest
- UTF-8 byte range
- replacement bytes
- patch intent
- protected-span policy version

## 적용 순서

`Parse → Bind Source → Validate Ranges → Detect Overlap → Check Protected Spans → Apply in descending offsets → Recompute Body → Validate Output`

범위가 겹치거나 source digest가 stale이면 전체 patch set을 거부한다. 부분 적용은 금지한다.

## 품질 검증

quality score는 승격 결정 그 자체가 아니다. hard gate validator와 advisory metric을 분리한다. hard gate는 required receipt selector에 포함되고, advisory metric은 decision context에 기록하되 자동 pass로 변환하지 않는다.
