# ADR 0002 — SMC projection source naming

- Status: Accepted

## Context

SAC RunRequest의 기존 `NVC_RELEASED_SCENE_PROJECTION`은 실제 projection producer가 SMC인데 NVC가 owner인 것처럼 보이게 했다.

## Decision

cross-boundary source kind를 `SMC_NARRATIVE_SCENE_PROJECTION`으로 clean-break한다. NVC는 upstream narrative authority이고 SMC가 durable projection의 producer다.

## Consequences

schema, SAC runtime validator, SMC handoff producer/consumer copy, examples/tests가 동일한 새 이름을 사용한다. 구 이름의 compatibility fallback은 두지 않는다.
