# 14. Test Strategy

> 0.12.1 current regression includes the deterministic in-process planner. AgentExecutor-specific tests below apply when that external boundary becomes reachable.

## Layer 1 — Contract

- Draft 2020-12 meta-validation.
- positive fixture.
- negative schema fixture.
- cross-field/cross-document invariant negative fixture.
- fixture hash chain.
- production JCS는 Phase 2에서 official RFC 8785 vectors로 별도 검증.

## Layer 2 — Runtime primitives

- path escape rejection.
- atomic write/hash.
- project/run lock.
- timeout/cancel/process ownership on supported platforms.
- stale/late publish rejection when delivery effects are introduced.

## Layer 3 — STS regression

- `tools/verify_contract.py`.
- Auditor unit suite.
- Rust workflow regression without assertion weakening.

## Layer 4 — DirectorIR

- unsupported/ambiguous backend.
- lineage completeness.
- FULL omission legality.
- false fact/actor/epistemic/causal/chronology/knowledge/ending BLOCK.
- dangling asset ref/duplicate ID rejection.
- deterministic plan fixture with mocked AgentExecutor.

## Layer 5 — Delivery

- same AssetIndex across outputs.
- partial rebuild only affected output.
- cancel/stale no publish.
- delivery/top-level false PASS rejection.
- exact frame count after video qualification.

## One-command target

얇은 `verify-all.sh`가 기존 verifier를 호출한다. 새 `xtask` framework는 실제 필요가 생기기 전 만들지 않는다.
