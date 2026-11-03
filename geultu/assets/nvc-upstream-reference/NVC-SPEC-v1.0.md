# Narrative Visualization Compiler — Consolidated Specification v1.0


---

<!-- README.md -->

# Narrative Visualization Compiler — Contract-First Design Package v1.0

이 패키지는 **확정된 서사(Narrative Package)를 고품질 시각 콘텐츠로 컴파일하는 시스템**을 구현하기 위한 설계·계약·검증 패키지다.

## 최종 경계

- 앞단 **Narrative Development System (NDS)**: 아이디어/원문 → 시놉시스 → 서사 개발 → 최종 스토리 또는 시나리오.
- 후단 **Narrative Visualization Compiler (NVC)**: 확정된 `NarrativePackage` → 의미 IR → 연출 → 장면 → 샷 → 자산 → 렌더 → 합성 → 검증 → Master.
- NVC 정상 진입점은 **`NarrativePackage` 하나**다.
- `canonical_narrative`만 서사적 authority다. `planning_synopsis`는 planning provenance/intent evidence다.
- story와 screenplay가 둘 다 존재해도 NDS가 하나를 canonical로 선택해 넘긴다. NVC가 임의 선택하지 않는다.

## Canonical pipeline

`S00 → S01 → S02 → S03 → S04 → S05 → S06 → S07 → S08 → S09 → S10 → S11 → S12`

각 stage는 `contracts/stages/*.json`에 **입력, 출력, executor, owner, substep, hard gate, failure code, retry, next**가 machine-readable 형태로 있다.

## 검증

```bash
python scripts/audit_contracts.py
bash scripts/grep_contract_usage.sh
```

Golden fixture: `fixtures/shadow-story/`.

## 읽는 순서

1. `docs/00_EXECUTIVE_DECISION.md`
2. `docs/01_BOUNDARY_AND_AUTHORITY.md`
3. `docs/02_PIPELINE.md`
4. `docs/03_IR_AND_SCHEMAS.md`
5. `docs/04_STAGE_CONTRACTS.md`
6. `docs/05_VERIFICATION_REPAIR.md`
7. `docs/06_DIRECTOR_RENDER_ROUTING.md`
8. `docs/07_INCREMENTAL_PROVENANCE.md`
9. `docs/08_IMPLEMENTATION_PLAN.md`
10. `reports/CONTRACT_AUDIT.md`


---

<!-- docs/00_EXECUTIVE_DECISION.md -->

# 00. Executive Decision

## 판정

이전의 “이야기/시놉시스/시나리오를 별도 프로그램으로 만들 필요는 없다”는 표현은 폐기한다.

**더 좋은 구조는 Narrative Development System(NDS)과 Narrative Visualization Compiler(NVC)를 분리하는 것이다.**

```text
Idea / Source Text
      ↓
Narrative Development System
      ├─ Synopsis                 [planning artifact]
      ├─ Story development       [causality, character, setup/payoff]
      └─ Canonical Narrative     [Story OR Screenplay]
      ↓ NarrativePackage
Narrative Visualization Compiler
      ↓
Verified Visual Master
```

## 왜 이 경계가 더 좋은가

1. **목적 함수가 다르다.** NDS는 “이야기가 좋은가?”를 최적화하고, NVC는 “확정된 이야기를 의미 손실 없이 어떻게 보여줄 것인가?”를 최적화한다.
2. **시놉시스는 최종 authority가 아니다.** 압축된 계획물에는 micro-causality, 상태 변화, payoff object, blocking이 생략될 수 있다.
3. **story와 screenplay는 직렬 필수 단계가 아니다.** NDS가 작품 목적에 따라 최종 canonical representation 하나를 선택한다.
4. NVC가 synopsis를 다시 확장하면 서사 개발과 시각화가 섞이고 semantic drift가 생긴다.
5. NVC가 단일 input port를 가지므로 schema/gate/test를 고정할 수 있다.

## 호환성

raw prose/synopsis/screenplay를 직접 넣고 싶다면 별도 **Compatibility Adapter**가 `NarrativePackage`로 변환한다. Core NVC는 입력 종류별 분기 코드를 갖지 않는다.


---

<!-- docs/01_BOUNDARY_AND_AUTHORITY.md -->

# 01. Boundary & Authority Contract

## NDS 책임

`무엇을 이야기할 것인가`를 확정한다.

- 소재/아이디어 해석
- 주제·의도
- 시놉시스
- 플롯/인과관계
- 캐릭터 arc
- setup/payoff
- 극단적으로 짧은 단편 또는 screenplay 작성
- narrative QA
- canonical 선택

**출력:** `schemas/narrative-package.schema.json`.

## NVC 책임

`확정된 이야기를 무엇을, 어떤 매체로, 어떤 장면/샷/움직임으로 보여줄 것인가`를 결정한다.

NVC가 하면 안 되는 것:

- plot hole을 몰래 고침
- 결말을 바꿈
- synopsis와 story 사이에서 더 마음에 드는 내용을 임의 선택
- 생성 모델이 만든 설정을 source fact로 승격

## Authority matrix

| Artifact | Authority | Mutability | Conflict rule |
|---|---|---|---|
| canonical_narrative | canonical | immutable | 최우선 |
| planning_synopsis | planning evidence | immutable | hard-intent conflict면 `SRC_005` |
| user constraints | policy | immutable | canonical과 직접 충돌 시 review |
| StoryIR / MeaningSpec | derived | owner가 새 version만 생성 | canonical에 종속 |
| DirectorIR 이후 | production-derived | owner가 새 version만 생성 | semantic upstream에 종속 |

## Story vs Screenplay

둘 다 지원하되 **동시에 canonical authority를 주지 않는다**.

- `story`: 내레이션/내면/서술이 풍부하므로 S01이 event/state/meaning을 구조화한다.
- `screenplay`: scene/action/dialogue가 더 구조화되어 있으므로 그 구조를 S01에서 활용한다.

동일 작품에 story와 screenplay가 모두 있으면 NDS가 하나를 canonical로 지정한다. 다른 것은 reference가 될 수 있으나 NVC core source-of-truth에는 포함하지 않는다.

## Normalized external port

```text
NDS.out.NarrativePackage
        ↓
NVC.in.NarrativePackage
```

이 포트 하나가 시스템 경계다.


---

<!-- docs/02_PIPELINE.md -->

# 02. Canonical Pipeline

## Stage chain

| ID | Owner | Executor | Output | On PASS |
|---|---|---|---|---|
| S00_PACKAGE_PREFLIGHT | SourceRegistry | DET+LLM | validated_package | S01 |
| S01_SEMANTIC_COMPILE | SemanticCompiler | LLM+DET | StoryIR + MeaningSpec | S02 |
| S02_DIRECTION_SYNTHESIS | Director | LLM+DET | DirectorIR | S03 |
| S03_ADAPTATION_PLAN | AdaptationPlanner | LLM+DET | AdaptationIR | S04 |
| S04_SCENE_PLAN | ContinuityPlanner | LLM+DET | SceneBundleIR | S05 |
| S05_SHOT_PLAN | CinematographyPlanner | LLM+DET | ShotBundleIR | S06 |
| S06_ASSET_PLAN | ArtDepartment | LLM+DET | AssetManifest | S07 |
| S07_ASSET_MATERIALIZE | AssetMaterializer | GEN+PROC+VLM | materialized assets | S08 |
| S08_RENDER_PLAN | RenderPlanner | DET+LLM | RenderIR | S09 |
| S09_SHOT_RENDER | Renderer | GEN+PROC+VLM | shot clips | S10 |
| S10_COMPOSE | Composer | PROC+DET | TimelineIR + draft | S11 |
| S11_VERIFY_REPAIR | Verifier | DET+VLM+LLM | EvaluationReport | S12/repair |
| S12_PACKAGE_MASTER | Packager | DET+PROC | MasterPackage | DONE |

## 공통 진입 조건

```text
CAN_ENTER(stage) =
  required inputs all exist
  AND required inputs validate against declared schemas
  AND upstream required stages == PASS
  AND unresolved hard failure == 0
```

## 공통 PASS 조건

```text
PASS(stage) =
  every declared output exists
  AND output schema validation == PASS
  AND every hard_gate == true
  AND artifact write owner == artifact_owner
```

soft metric은 hard gate를 뒤집지 않는다.

## 실행 상태

`PENDING | RUNNING | PASS | FAIL_RETRYABLE | BLOCKED_UPSTREAM | NEEDS_REVIEW | TERMINAL_FAIL`

## 내부 substep

각 stage의 세부 실행은 stage JSON의 `substeps[]`에 고정되어 있다.

```json
{
  "step_id": "P02",
  "executor_class": "DET",
  "inputs": ["..."],
  "outputs": ["..."],
  "hard_gate": "...",
  "failure_codes": ["..."]
}
```

즉 문서상의 설명이 아니라 runtime이 읽을 수 있는 계약이다.


---

<!-- docs/03_IR_AND_SCHEMAS.md -->

# 03. IR & Schemas

## IR chain

```text
NarrativePackage
  ↓
StoryIR + MeaningSpec
  ↓
DirectorIR
  ↓
AdaptationIR
  ↓
SceneBundleIR
  ↓
ShotBundleIR
  ↓
AssetManifest
  ↓
RenderIR
  ↓
TimelineIR
  ↓
EvaluationReport
```

## NarrativePackage

NVC의 유일한 canonical input. `canonical_narrative`, optional `planning_synopsis`, adaptation policy, target, constraints를 가진다.

## StoryIR

문장 요약이 아니라 entity/event/state 변화 representation이다. `explicit`로 선언한 정보는 source span이 반드시 있어야 한다.

## MeaningSpec

### must_keep_assertions
최종 결과에 반드시 남아야 하는 사실/의미.

### transition_obligations
장면 A와 B가 둘 다 있다는 것만으로는 충분하지 않다. A→B의 `causal / intentional / emotional / consequence / temporal / revelation` 연결이 복구 가능해야 한다.

## DirectorIR

“수묵화” 같은 style label 하나가 아니다.

- visual grammar
- motion grammar
- camera grammar
- typography grammar
- transition grammar
- sound grammar
- negative rules
- production constraints

## AdaptationIR

무엇을 image/text/sound가 담당할지, 무엇을 생략했는지 기록한다. 생략은 항상 이유와 안전성 판정을 가진다.

## SceneBundleIR

```text
state_in → state_delta → state_out
```

continuity를 free-form memory에 맡기지 않는다.

## ShotBundleIR

각 shot은 narrative function, Setup, Event, Lighting, Camera, Motion, hard constraints, assets, semantic obligations를 가진다.

## AssetManifest

shot 계획이 잠긴 뒤 실제 필요한 자산만 lazy materialize한다. identity-critical 자산은 `continuity_scope`, `allowed_mutations`, `forbidden_mutations`를 가진다.

## RenderIR

모델명이 아니라 capability requirement와 backend assignment를 저장한다. 모델 교체가 StoryIR/ShotIR schema를 바꾸면 안 된다.

## TimelineIR

시간은 floating second가 아니라 `fps + frame index`로 기록한다.


---

<!-- docs/04_STAGE_CONTRACTS.md -->

# 04. Stage Contract Specification

Machine source of truth: `contracts/stages/*.json`.

## 필수 필드

- `stage_id`
- `version`
- `artifact_owner`
- `executor_classes`
- `inputs[] {kind, schema, required}`
- `outputs[] {kind, schema}`
- `preconditions[]`
- `hard_gates[]`
- `soft_metrics[]`
- `failure_codes[]`
- `retry_policy.max_repairs`
- `next.on_pass / on_fail`
- `substeps[]`

## Artifact ownership

1. 한 artifact kind/version의 write owner는 하나다.
2. downstream은 upstream artifact를 mutate하지 않는다.
3. 수정은 parent를 가리키는 새 artifact version이다.
4. Verifier는 production artifact를 수정하지 않고 EvaluationReport만 쓴다.
5. Repair Router는 수정하지 않고 root-cause owner를 재호출한다.

## Executor class

- `DET`: schema/ref/hash/DAG/media/timing 같은 결정적 검사
- `LLM`: 의미 해석, candidate synthesis
- `VLM`: 이미지/영상 관찰 및 의미·continuity 평가
- `GEN`: 이미지/영상/음성 생성
- `PROC`: 2.5D, rig, Blender, compositor, FFmpeg 계열 절차 렌더
- `HUMAN`: low-confidence hard semantic review
- `HYBRID`: tightly-coupled mixed execution

## Prompt contract

LLM/VLM 프롬프트도 `contracts/prompts/*.json`에서 다음을 고정한다.

- authoritative inputs
- allowed inference
- forbidden mutation
- output schema
- hard requirements
- evaluation rubric

프롬프트 자연어 문구는 바뀔 수 있지만 **schema/hard requirement를 프롬프트가 덮어쓸 수 없다.**


---

<!-- docs/05_VERIFICATION_REPAIR.md -->

# 05. Verification, Failure, RCA Repair

## 검증 계층

### Structural
schema, IDs, refs, DAG, decode, fps, duration.

### Semantic
hard assertion 100%, hard transition recoverability 100%, contradiction 0.

### Continuity
character/world/prop/time/location state.

### Production invariants
예: `NO_HUMAN_SHADOW`, source-exact text, fixed object count, precise camera path.

### Soft quality
미학, 감정, novelty, pacing. hard gate를 override하지 않는다.

## Repair ownership

| Failure family | Root owner |
|---|---|
| SRC | S00 |
| SEM | S01 |
| DIR | S02 |
| ADP | S03 |
| SCN | S04 |
| SHT | S05 |
| AST | S06 |
| GEN | S07 |
| RND planning | S08 |
| MOT / render | S09 |
| CMP | S10 |
| QAV | 원인이 처음 발생한 upstream stage / uncertain이면 HUMAN |
| PKG | S12 |

## Bounded repair

`최초 실행 + StageContract.max_repairs`. budget 소진 시 `SYS_005_RETRY_BUDGET_EXHAUSTED`.

무한 reflection loop를 만들지 않는다.

## Semantic recoverability

transition obligation마다 질문과 accepted meaning을 가진다. 최종 visual segment를 기반으로 evaluator가 답하게 하고 의미 등가성을 평가한다.

- high-confidence equivalent → PASS
- contradiction/non-recoverable → FAIL
- low confidence → **NEEDS_REVIEW**

low confidence를 자동 PASS로 승격하지 않는다.


---

<!-- docs/06_DIRECTOR_RENDER_ROUTING.md -->

# 06. Director & Render Routing

## Genre lookup 금지

`SF → 우주선`, `사극 → 수묵화` 같은 1:1 룩업은 쓰지 않는다.

```text
theme / narrative function
        ↓
medium affordance candidates
        ↓
production feasibility
        ↓
pairwise semantic ranking
        ↓
DirectorIR lock
```

corpus novelty는 tie-breaker일 뿐 semantic fit보다 우선하지 않는다.

## 기본 motion modes

- `LAYERED_2_5D`: 배경 고정, 부분 모션, 패럴랙스
- `RIGGED_2D`: 종이 인형/마리오네트/그림자극
- `STOP_MOTION`: 프레임 기반 인형극 미학
- `GEN_I2V`: anchor/reference 기반 유기 motion
- `GEN_T2V`: identity/continuity 부담이 낮은 자유 shot
- `SCENE_3D`: 정확한 geometry/camera/miniature stage
- `HYBRID`: deterministic core + generative organic motion

## 가장 중요한 routing rule

**material hard invariant가 uncontrolled stochastic generator 하나에 의존하면 설계 실패다.**

예: “최초 그림자 이전 인간에게 그림자가 없어야 한다”는 prompt 희망사항으로 두지 않고 character layer + deterministic compositor 또는 검증 가능한 geometry로 구현한다.

## Director presets

`directors/*.json`에는 예시로 living parchment, marionette, 16mm film, analog sci-fi를 넣었다. preset은 style prompt가 아니라 6개 grammar와 supported motion modes를 포함한다.


---

<!-- docs/07_INCREMENTAL_PROVENANCE.md -->

# 07. Incremental Build & Provenance

## Content-addressed artifact

권장:

```text
JCS canonical JSON (RFC 8785)
       ↓
SHA-256
       ↓
artifact_id
```

runtime timestamp는 artifact content hash에서 제외하고 RunRecord에 둔다.

## Invalidation

- canonical narrative 변경 → S01 이후 invalidate
- MeaningSpec 변경 → S02 이후 semantic-dependent artifacts invalidate
- DirectorIR 변경 → S03 이후 invalidate, semantic IR reuse
- 한 shot 변경 → 해당 shot assets/render/timeline/eval만 invalidate
- 한 asset 변경 → `consumers[]` shot만 invalidate

## Provenance

`parent_ids`로 내부 lineage를 완전히 보존한다.

C2PA는 외부 배포 시 provenance를 전달하기 위한 선택적 packaging layer다. **semantic correctness 검증을 대체하지 않는다.**

## Event transport

분산 worker로 갈 때 CloudEvents를 transport envelope로 사용할 수 있으나 domain semantics는 NVC schema가 소유한다.


---

<!-- docs/08_IMPLEMENTATION_PLAN.md -->

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


---

<!-- docs/09_RESEARCH_BASIS.md -->

# 09. Research Basis

2026-08-19 기준. 연구를 그대로 제품 법칙으로 복제하지 않고, 각 근거가 어떤 설계 결정을 지지하는지 제한해서 사용한다.

## Narrative development separation

- **A Decomposed Approach to Well-Crafted Screenwriting with LLMs** — outline→rich narrative prose와 prose→screenplay formatting을 분리. NDS 내부의 단계 분리 및 NDS/NVC 책임 분리에 참고.
  - https://arxiv.org/abs/2510.23163
- **Narrative Keyframing for Generative Creative Writing** — planning constraints와 생성 prose 사이에 명시적 control points. synopsis/outline을 planning artifact로 보는 근거.
  - https://arxiv.org/abs/2608.10337

## Long-form narrative state

- **STAGE: A Full-Screenplay Benchmark for Reasoning over Evolving Narrative Structures** — 장편 screenplay에서 character/event/relation/knowledge state를 provenance-linked backbone으로 추적.
  - https://arxiv.org/abs/2601.08510
- **Character-Grounded Multi-Agent Story Generation / Atlas** — graph 기반 world-state consistency 검사.
  - https://arxiv.org/abs/2607.00918

## Storyboard / multi-shot continuity

- **STAGE: Storyboard-Anchored Generation for Cinematic Multi-shot Narrative** — shot별 start/end anchor pair와 multi-shot memory.
  - https://arxiv.org/abs/2512.12372
- **DreamShot** — context-aware storyboard와 multi-reference role identity conditioning.
  - https://arxiv.org/abs/2604.17195
- **StoryBlender** — global canonical assets와 shot-specific variables 분리, continuity memory graph, editable 3D grounding.
  - https://arxiv.org/abs/2604.03315
- **Continuity-Aware Narratives via Visual Agentic Storyboarding** — character/environment/scene transition consistency를 explicit concern으로 처리.
  - https://arxiv.org/abs/2604.13452


## Semantic trajectory / diagnostic evaluation

- **KathaTrace: Diagnosing Semantic Trajectory Collapse in Generated Visual Narratives** — 장면이 개별적으로 자연스럽고 인물/배경이 일관돼도 장면 사이의 원인·의도·결과 의미가 이미지에서 복구되지 않을 수 있음을 별도 실패로 정의. `transition_obligations`와 recoverability gate의 직접 근거.
  - https://arxiv.org/abs/2607.01312
- **DirectorBench** — long-form video 품질을 단일 aggregate score 대신 script/visual/audio/cross-modal/stability의 checkpoint-level diagnosis로 평가. stage별 hard/soft gate 분리의 근거.
  - https://arxiv.org/abs/2605.30090
- **Stable Cinemetrics** — professional video controls를 Setup, Event, Lighting, Camera의 계층적 taxonomy로 구조화. ShotIR의 cinematic control decomposition 근거.
  - https://arxiv.org/abs/2509.26555

## Compiler architecture

- **MLIR Developer Guide / Pass Management** — valid IR input → transformation pass → valid IR output, pass별 verifier. NVC의 StageContract/IR verifier 구조에 직접 대응.
  - https://mlir.llvm.org/getting_started/DeveloperGuide/
  - https://mlir.llvm.org/docs/PassManagement/

## Machine contracts / provenance

- JSON Schema Draft 2020-12: https://json-schema.org/draft/2020-12
- RFC 8785 JCS: https://datatracker.ietf.org/doc/rfc8785/
- CloudEvents: https://github.com/cloudevents/spec
- C2PA 2.4: https://spec.c2pa.org/specifications/specifications/2.4/index.html

## 적용 경계

서사인지, 영화 언어인지, 생성 모델 품질인지, 소프트웨어 계약인지 평가 대상이 다르다. 하나의 “이론”으로 전부 설명하지 않는다. 각 이론은 자기 계층에서만 사용한다.


---

<!-- docs/10_CRITICAL_REVIEW.md -->

# 10. Critical Review

## 1. NVC가 synopsis를 직접 발전시키는 구조

**폐기.** canonical pipeline에서는 NDS가 완성 서사를 책임진다.

## 2. 자연어 agent relay

**축소.** 역할명보다 IR + StageContract를 authority로 둔다.

## 3. 중간 IR

**유지·강화.** semantic drift, call-site ambiguity, retry ownership을 줄인다.

## 4. Director 다양성

**유지.** genre→style 고정이 아니라 meaning→affordance→feasibility.

## 5. 모든 장면 생성 영상

**폐기.** 2.5D/rig/3D/procedural/generative를 shot별로 route한다.

## 6. 캐릭터 consistency만 검사

**불충분.** causal/intention/emotional/consequence trajectory까지 hard semantic QA.

## 7. 마지막 단계에서만 QA

**폐기.** 각 pass 직후 verifier를 둔다.

## 8. universal score

**폐기.** semantic/continuity/technical hard gates와 aesthetic soft metric을 분리한다.

## 9. 전체 재생성

**폐기.** artifact dependency graph 기반 incremental rebuild.

## 남은 구조적 위험

1. LLM semantic parser가 자신있게 틀릴 수 있음.
2. VLM evaluator와 generator의 correlated blind spot.
3. preset library가 반복되어 corpus가 획일화될 수 있음.
4. backend capability registry가 stale해질 수 있음.
5. hard semantic evaluator가 불확실한데 자동 PASS할 위험.

대응 원칙은 `fail-open 금지`, `evidence`, `NEEDS_REVIEW`, `root-cause repair`다.


---

<!-- docs/11_TRACEABILITY_MATRIX.md -->

# 11. Requirement → Contract → Gate Matrix

| Requirement | Contract | Gate |
|---|---|---|
| 최종 narrative authority 하나 | NarrativePackage.canonical_narrative | S00 |
| synopsis는 planning evidence | SynopsisArtifact.purpose | S00 |
| 원문 traceability | SourceSpan | S01 |
| explicit/inferred/creative 분리 | origin enum | S01 |
| 인과/의도 보존 | transition_obligations | S03 + S11 |
| world consistency | state_in/delta/out | S04 |
| shot renderability | ShotIR | S05 |
| hard invariant enforcement | hard_constraints → RenderIR.invariants | S05/S08/S09 |
| asset 누락 방지 | asset_refs ↔ AssetManifest | S06 |
| identity drift 방지 | forbidden_mutations + asset eval | S06/S07 |
| backend 교체 가능 | CapabilityProfile/RenderIR | S08 |
| 정확한 timing | TimelineIR frame index | S10 |
| semantic final QA | EvaluationReport | S11 |
| RCA repair | failure namespace + repair stage | runtime/S11 |
| bounded retries | max_repairs | runtime |
| incremental rebuild | artifact graph/consumers | scheduler |
| 배포 provenance | parent_ids + optional C2PA | S12 |


---

<!-- docs/12_CALL_GRAPH_AND_PORTS.md -->

# 12. Call Graph & Ports

## External call graph

```text
NDS
 └─ emitNarrativePackage()
       ↓
NVC.accept(package)
 └─ S00
    └─ S01
       └─ S02
          └─ S03
             └─ S04
                └─ S05
                   └─ S06
                      └─ S07
                         └─ S08
                            └─ S09
                               └─ S10
                                  └─ S11
                                     ├─ PASS → S12
                                     └─ FAIL → RepairRouter(root_owner)
```

## Core ports

### `NarrativePackagePort`
- input: `narrative-package.schema.json`
- write owner: NDS
- read owner: S00+
- mutation: forbidden

### `ArtifactStorePort`
- put immutable artifact
- get by artifact_id
- list parents/consumers
- no in-place update

### `ExecutorPort`
- execute(StageContract, input artifact IDs)
- returns RunRecord + output artifacts/failures

### `CapabilityRegistryPort`
- query task/control/limit
- S02 uses feasibility
- S08 uses exact backend assignment

### `EvaluationPort`
- deterministic and VLM evaluations
- must return evidence and confidence

### `RepairRouterPort`
- input: Failure
- output: root stage + invalidation set
- never mutates artifacts itself

## Grep rule

구현이 생기면 CI에서 다음을 강제한다.

1. schema filename literal은 Registry 외 직접 참조 금지.
2. failure code literal은 FailureCatalog 외 직접 정의 금지.
3. stage ID literal은 StageRegistry 외 직접 정의 금지.
4. output kind producer는 하나만 존재.
5. public NVC input 함수는 `NarrativePackage` 하나만 받음.

이 패키지에는 실제 runtime source가 아직 없으므로 “실제 구현 call site grep”은 할 수 없다. 대신 `scripts/grep_contract_usage.sh`가 현재 문서/계약 패키지 전체의 schema/stage/failure reference를 정적 열거하며, 구현 단계에서 동일 검사를 source tree까지 확장해야 한다.


---

<!-- reports/CONTRACT_AUDIT.md -->

# Contract Audit Report

Date: 2026-08-19

## Result

**AUDIT: PASS**

Validated package contents:

- 22 JSON Schemas
- 13 canonical stages
- 40 stage substeps
- 8 versioned PromptSpecs
- 4 Director preset examples
- 76 registered failure codes
- 22 runtime invariants
- 2 external port contracts
- 8 shadow-story golden fixture artifacts

## Checks executed

1. JSON Schema Draft 2020-12 meta-validation.
2. Every stage JSON against `stage-contract.schema.json`.
3. Every declared stage input/output schema path exists.
4. Every stage/substep failure code exists in the central failure catalog.
5. Every substep failure is declared by its parent stage.
6. Every `next.on_pass` target resolves.
7. Stage graph is acyclic and all 13 stages are reachable from S00.
8. Every produced artifact kind has a single producer stage.
9. Artifact catalog schema references resolve.
10. Every PromptSpec validates; stage/output schema references resolve.
11. Every Director preset validates.
12. Both external port contracts validate and reference existing schemas.
13. Runtime invariant IDs are unique.
14. Golden fixture artifacts validate against their schemas.
15. Every golden ShotIR asset reference resolves to AssetManifest.
16. Every fixture meaning obligation reference resolves.
17. Every hard transition obligation is covered in AdaptationIR and ShotIR.
18. No unused golden fixture asset remains.
19. Static grep report emitted for stage IDs, schema refs and failure codes.

## Defect found and corrected during audit

S01 internal substep P03 could raise:

- `SEM_004_CAUSAL_LINK_LOST`
- `SEM_005_INTENTION_LOST`
- `SEM_007_HARD_ASSERTION_MISSING`

but the parent S01 stage initially omitted those three codes from `failure_codes[]`. The parent contract was corrected and the complete audit rerun to PASS.

## Important limit

There is **no NVC runtime source repository yet**, so it is impossible to truthfully claim that actual implementation call sites were grepped. This package instead audits all current schema/stage/prompt/director/failure references and provides `scripts/grep_contract_usage.sh`. When implementation begins, CI must run the same registry/reference audit over the source tree and enforce the call-site rules in `docs/12_CALL_GRAPH_AND_PORTS.md`.

## Command

```bash
python scripts/audit_contracts.py
bash scripts/grep_contract_usage.sh
```


---

<!-- HANDOFF.md -->

# Handoff

## Canonical decision

앞단 NDS가 synopsis와 최종 story/screenplay를 만든다. NVC의 유일한 정상 입력은 canonical narrative 하나를 담은 `NarrativePackage`다.

## 구현 시작점

- `schemas/narrative-package.schema.json`
- `schemas/stage-contract.schema.json`
- `contracts/stages/`
- `contracts/prompts/`
- `catalogs/failure-codes.json`
- `fixtures/shadow-story/`
- `scripts/audit_contracts.py`

## Phase 0 Done

1. schema meta-valid
2. stage contract valid
3. prompt/director contract valid
4. dangling schema/failure/next ref 0
5. stage cycle 0
6. output kind multi-owner 0
7. fixture schema validation PASS
8. fixture asset/obligation ref PASS
9. hard failure가 downstream PASS로 누출되지 않는 runtime test
10. failure가 root owner로 route되는 runtime test

문서 완료는 구현 완료가 아니다. 이 패키지는 **구현 가능한 계약 기준선**이다.
