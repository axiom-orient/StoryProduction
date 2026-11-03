# ADR 0001 — Story capability envelope, no shared image core

- Status: Accepted

## Context

여러 component가 image/prompt 효과를 호출할 수 있지만 생성 mechanism과 domain authority는 서로 다르다.

## Decision

공유 runtime/core 대신 `StoryCapabilityRequest`, `StoryCapabilityJob`, `StoryCapabilityResult`만 공통 교환 계약으로 둔다. provider execution은 component/adapter가 소유한다. 모든 성공 결과는 candidate다.

## Consequences

중복되는 provider mechanism을 억지로 합치지 않으면서 workflow orchestration이 동일한 failure/cancel/hash/rights 의미를 사용할 수 있다.
