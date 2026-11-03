# 08. Implementation Plan

## Phase 0 — Contract Kernel

1. JSON Schema registry
2. StageContract loader/meta validation
3. Artifact canonicalization + CAS
4. DAG scheduler
5. failure catalog + Repair Router
6. RunRecord/evidence store
7. golden fixture runner

**Gate:** 외부 생성 모델 없이 mocked pipeline으로 stage transition, fail-closed, repair routing이 deterministic하게 테스트되어야 한다.

## Phase 1 — Semantic Front-End

S00/S01 구현.

필수 golden tests:

- explicit source evidence coverage
- `explicit/inferred/creative` 분류
- unresolved reference
- causal/intention transition
- synopsis↔canonical material conflict fail-closed

## Phase 2 — Creative Planning

S02~S06. 아직 영상 생성하지 않는다.

완료 artifact: DirectorIR, AdaptationIR, SceneBundleIR, ShotBundleIR, AssetManifest.

## Phase 3 — Hybrid Production

S07~S10.

우선 구현 순서:

1. deterministic 2.5D compositor
2. image generation adapter
3. I2V adapter
4. Blender/3D adapter
5. audio adapter

## Phase 4 — End-to-end QA

S11/S12. VLM + deterministic checks + optional human review.

## Phase 5 — Corpus operation

156편 이상에서:

- Director preset usage history
- quality/cost/reliability metrics
- asset/cache reuse
- corpus novelty soft metric
- incremental rebuild

## 처음부터 만들지 않을 것

- microservices
- 역할 이름만 다른 20개 agent
- universal quality score
- 특정 모델명이 박힌 core schema
- 무한 self-reflection
- 모든 shot full T2V
