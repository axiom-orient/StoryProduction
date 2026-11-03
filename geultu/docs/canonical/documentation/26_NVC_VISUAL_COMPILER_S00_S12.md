> **ARCHIVED REFERENCE ONLY.** v4.1.0 통합 후보(geultu.integrated
> 1.0.0-candidate)의 소스 스캔 코퍼스에서 추출한 역사적 참조 자료다. 이 문서의
> 단계·노드·실패 코드·체크포인트는 현재 구현·계약의 정본이 아니며, 현재 구현의
> 계약과 경계는 각 저장소의 실제 `contracts/`, `docs/`, 소스, 테스트가 우선한다.

> **ARCHIVED REFERENCE — NOT A CURRENT IMPLEMENTATION CONTRACT.** 이 문서는 v4.1.0
> 통합 후보에서 보존한 S00–S12 시각 컴파일러 참조 자료다. 현재 구현의 계약·입력·출력·경계로
> 사용하지 않는다. 현재 실행 경계는 `NarrativeCourier`의 `ReviewedSynopsisHandoff v1` →
> `NarrativePackage v2`/`NarrativeSceneMap v2`/`NvcReleaseBundle v1` Completed release →
> `SceneMotionCompiler`의 검증된 `NarrativeSceneProjection v2`이며, 해당 계약은 각 저장소의
> `contracts/`와 `docs/BOUNDARY_NARRATIVE_COURIER.md`를 따른다. 이 문서의 S00–S12
> `narrative_package` 표기는 역사적 참조일 뿐 현재 public route가 아니다.

# [ARCHIVED] NVC Visual Compiler S00–S12

Stages: 13 / substeps: 40

| stage | owner | executor class | inputs | outputs | PASS next | FAIL |
|---|---|---|---|---|---|---|
| S00_PACKAGE_PREFLIGHT | SourceRegistry | DET, LLM | narrative_package | validated_package | S01_SEMANTIC_COMPILE | REPAIR_ROUTER |
| S01_SEMANTIC_COMPILE | SemanticCompiler | LLM, DET | validated_package | story_ir, meaning_spec | S02_DIRECTION_SYNTHESIS | REPAIR_ROUTER |
| S02_DIRECTION_SYNTHESIS | Director | LLM, DET | story_ir, meaning_spec | director_ir | S03_ADAPTATION_PLAN | REPAIR_ROUTER |
| S03_ADAPTATION_PLAN | AdaptationPlanner | LLM, DET | story_ir, meaning_spec, director_ir | adaptation_ir | S04_SCENE_PLAN | REPAIR_ROUTER |
| S04_SCENE_PLAN | ContinuityPlanner | LLM, DET | adaptation_ir, story_ir | scene_bundle_ir | S05_SHOT_PLAN | REPAIR_ROUTER |
| S05_SHOT_PLAN | CinematographyPlanner | LLM, DET | scene_bundle_ir, director_ir, meaning_spec | shot_bundle_ir | S06_ASSET_PLAN | REPAIR_ROUTER |
| S06_ASSET_PLAN | ArtDepartment | LLM, DET | shot_bundle_ir, director_ir | asset_manifest | S07_ASSET_MATERIALIZE | REPAIR_ROUTER |
| S07_ASSET_MATERIALIZE | AssetMaterializer | GEN, PROC, VLM | asset_manifest, director_ir | materialized_assets | S08_RENDER_PLAN | REPAIR_ROUTER |
| S08_RENDER_PLAN | RenderPlanner | DET, LLM | shot_bundle_ir, asset_manifest, materialized_assets | render_ir | S09_SHOT_RENDER | REPAIR_ROUTER |
| S09_SHOT_RENDER | Renderer | GEN, PROC, VLM | render_ir, materialized_assets | shot_clips | S10_COMPOSE | REPAIR_ROUTER |
| S10_COMPOSE | Composer | PROC, DET | shot_clips, director_ir, validated_package | timeline_ir, draft_master | S11_VERIFY_REPAIR | REPAIR_ROUTER |
| S11_VERIFY_REPAIR | Verifier | DET, VLM, LLM | draft_master, meaning_spec, scene_bundle_ir, shot_bundle_ir | evaluation_report | S12_PACKAGE_MASTER | REPAIR_ROUTER |
| S12_PACKAGE_MASTER | Packager | DET, PROC | draft_master, evaluation_report | master_package | terminal | REPAIR_ROUTER |

## S00_PACKAGE_PREFLIGHT

| 항목 | 값 |
|---|---|
| artifact_owner | SourceRegistry |
| executor_classes | DET, LLM |
| retry.max_repairs | 0 |
| PASS next | S01_SEMANTIC_COMPILE |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| narrative_package | schemas/narrative-package.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| validated_package | schemas/narrative-package.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- JSON Schema PASS
- canonical authority unique
- target range valid
- synopsis does not materially contradict canonical hard intent

### Failure codes

- `SRC_001_SCHEMA_INVALID`
- `SRC_003_HASH_MISMATCH`
- `SRC_004_AUTHORITY_AMBIGUOUS`
- `SRC_005_SYNOPSIS_CANONICAL_CONFLICT`
- `SRC_006_TARGET_INVALID`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | validate package schema and hashes | DET | narrative_package | schema_report | schema errors == 0 | SRC_001_SCHEMA_INVALID, SRC_003_HASH_MISMATCH |
| P02 | resolve canonical authority and target policy | DET | narrative_package | authority_report | exactly one canonical narrative | SRC_004_AUTHORITY_AMBIGUOUS, SRC_006_TARGET_INVALID |
| P03 | compare planning synopsis against canonical hard intent when synopsis exists | LLM | planning_synopsis, canonical_narrative | conflict_report | no material hard-intent contradiction | SRC_005_SYNOPSIS_CANONICAL_CONFLICT |

## S01_SEMANTIC_COMPILE

| 항목 | 값 |
|---|---|
| artifact_owner | SemanticCompiler |
| executor_classes | LLM, DET |
| retry.max_repairs | 2 |
| PASS next | S02_DIRECTION_SYNTHESIS |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| validated_package | schemas/narrative-package.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| story_ir | schemas/story-ir.schema.json |
| meaning_spec | schemas/meaning-spec.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- explicit events source-traceable
- hard assertions traceable
- no invented fact marked explicit
- blocking ambiguity == 0

### Failure codes

- `SEM_001_UNRESOLVED_REFERENCE`
- `SEM_002_UNSOURCED_FACT`
- `SEM_003_SOURCE_CONTRADICTION`
- `SEM_006_EVENT_ORDER_INVALID`
- `SEM_008_AMBIGUITY_BLOCKING`
- `SEM_004_CAUSAL_LINK_LOST`
- `SEM_005_INTENTION_LOST`
- `SEM_007_HARD_ASSERTION_MISSING`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | extract entities events and source spans | LLM | validated_package | story_ir.draft | explicit claims carry evidence | SEM_002_UNSOURCED_FACT |
| P02 | resolve references and temporal event order | LLM | story_ir.draft | story_ir.resolved | references resolve and event order valid | SEM_001_UNRESOLVED_REFERENCE, SEM_006_EVENT_ORDER_INVALID |
| P03 | compile must-keep assertions and transition obligations | LLM | story_ir.resolved, canonical_narrative | meaning_spec.draft | hard source meaning covered | SEM_004_CAUSAL_LINK_LOST, SEM_005_INTENTION_LOST, SEM_007_HARD_ASSERTION_MISSING |
| P04 | deterministic provenance audit | DET | story_ir.resolved, meaning_spec.draft | story_ir, meaning_spec | all explicit items trace to canonical source | SEM_002_UNSOURCED_FACT, SEM_003_SOURCE_CONTRADICTION |

## S02_DIRECTION_SYNTHESIS

| 항목 | 값 |
|---|---|
| artifact_owner | Director |
| executor_classes | LLM, DET |
| retry.max_repairs | 2 |
| PASS next | S03_ADAPTATION_PLAN |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| story_ir | schemas/story-ir.schema.json | True |
| meaning_spec | schemas/meaning-spec.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| director_ir | schemas/director-ir.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- chosen preset production-feasible
- typography grammar defined
- motion grammar defined

### Failure codes

- `DIR_001_MEDIUM_SEMANTIC_MISMATCH`
- `DIR_002_RENDERER_UNAVAILABLE`
- `DIR_003_TEXT_GRAMMAR_UNDEFINED`
- `DIR_004_MOTION_GRAMMAR_UNDEFINED`
- `DIR_005_PRODUCTION_INFEASIBLE`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | generate 3-5 medium candidates from meaning and production constraints | LLM | story_ir, meaning_spec | direction_candidates | candidate count and required grammars complete | DIR_003_TEXT_GRAMMAR_UNDEFINED, DIR_004_MOTION_GRAMMAR_UNDEFINED |
| P02 | hard feasibility filter against capability registry | DET | direction_candidates, capability_registry | feasible_candidates | at least one feasible candidate | DIR_002_RENDERER_UNAVAILABLE, DIR_005_PRODUCTION_INFEASIBLE |
| P03 | pairwise rank semantic fit and lock direction | LLM | feasible_candidates, meaning_spec | director_ir | winner explains semantic rationale | DIR_001_MEDIUM_SEMANTIC_MISMATCH |

## S03_ADAPTATION_PLAN

| 항목 | 값 |
|---|---|
| artifact_owner | AdaptationPlanner |
| executor_classes | LLM, DET |
| retry.max_repairs | 2 |
| PASS next | S04_SCENE_PLAN |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| story_ir | schemas/story-ir.schema.json | True |
| meaning_spec | schemas/meaning-spec.schema.json | True |
| director_ir | schemas/director-ir.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| adaptation_ir | schemas/adaptation-ir.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- hard assertions coverage == 100%
- hard transition coverage == 100%
- all omissions justified

### Failure codes

- `ADP_001_MANDATORY_BEAT_OMITTED`
- `ADP_002_REDUNDANT_SCENE`
- `ADP_003_UNSAFE_COMPRESSION`
- `ADP_004_UNJUSTIFIED_CREATIVE_INSERTION`
- `ADP_005_HARD_OBLIGATION_UNCOVERED`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | segment event graph into candidate visual beats | LLM | story_ir, meaning_spec | beat_candidates | every hard obligation appears in >=1 candidate | ADP_005_HARD_OBLIGATION_UNCOVERED |
| P02 | assign representation mode text/image/sound and scene economy | LLM | beat_candidates, director_ir | scene_candidates | each scene has narrative function | ADP_002_REDUNDANT_SCENE |
| P03 | optimize scene set under duration and coverage constraints | DET | scene_candidates, meaning_spec, target | adaptation_ir.draft | hard coverage 100% and duration feasible | ADP_001_MANDATORY_BEAT_OMITTED, ADP_003_UNSAFE_COMPRESSION |
| P04 | audit omissions and creative insertions | DET | adaptation_ir.draft | adaptation_ir | all omissions justified and creative additions explicit | ADP_004_UNJUSTIFIED_CREATIVE_INSERTION |

## S04_SCENE_PLAN

| 항목 | 값 |
|---|---|
| artifact_owner | ContinuityPlanner |
| executor_classes | LLM, DET |
| retry.max_repairs | 2 |
| PASS next | S05_SHOT_PLAN |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| adaptation_ir | schemas/adaptation-ir.schema.json | True |
| story_ir | schemas/story-ir.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| scene_bundle_ir | schemas/scene-ir.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- adjacent state compatible
- immutable entity mutation justified
- world rules valid

### Failure codes

- `SCN_001_STATE_IN_OUT_MISMATCH`
- `SCN_002_ILLEGAL_ENTITY_MUTATION`
- `SCN_003_TIME_DISCONTINUITY`
- `SCN_004_LOCATION_DISCONTINUITY`
- `SCN_005_WORLD_RULE_VIOLATION`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | materialize state_in/state_delta/state_out per scene | LLM | adaptation_ir, story_ir | scene_state_draft | each scene has complete state transition | SCN_001_STATE_IN_OUT_MISMATCH |
| P02 | check adjacent state compatibility and world invariants | DET | scene_state_draft | continuity_report | all hard state constraints valid | SCN_001_STATE_IN_OUT_MISMATCH, SCN_002_ILLEGAL_ENTITY_MUTATION, SCN_005_WORLD_RULE_VIOLATION |
| P03 | emit scene bundle | DET | scene_state_draft, continuity_report | scene_bundle_ir | continuity report PASS | SCN_003_TIME_DISCONTINUITY, SCN_004_LOCATION_DISCONTINUITY |

## S05_SHOT_PLAN

| 항목 | 값 |
|---|---|
| artifact_owner | CinematographyPlanner |
| executor_classes | LLM, DET |
| retry.max_repairs | 2 |
| PASS next | S06_ASSET_PLAN |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| scene_bundle_ir | schemas/scene-ir.schema.json | True |
| director_ir | schemas/director-ir.schema.json | True |
| meaning_spec | schemas/meaning-spec.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| shot_bundle_ir | schemas/shot-ir.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- each shot renderable
- hard obligations assigned
- hard invariants enforceable

### Failure codes

- `SHT_001_UNRENDERABLE_SHOT`
- `SHT_002_CAMERA_UNDEFINED`
- `SHT_003_ACTION_AMBIGUOUS`
- `SHT_004_OBLIGATION_UNASSIGNED`
- `SHT_005_INVARIANT_UNENFORCEABLE`
- `SHT_006_ASSET_REFERENCE_DANGLING`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | derive shot purpose and setup/event/lighting/camera | LLM | scene_bundle_ir, director_ir | shot_draft | every shot has narrative purpose and camera | SHT_002_CAMERA_UNDEFINED, SHT_003_ACTION_AMBIGUOUS |
| P02 | route motion mode and encode hard constraints | LLM | shot_draft, director_ir | shot_motion_draft | hard invariant has enforceable strategy | SHT_005_INVARIANT_UNENFORCEABLE |
| P03 | assign meaning obligations and required asset refs | DET | shot_motion_draft, meaning_spec | shot_bundle_ir | hard obligations assigned and refs syntactically valid | SHT_004_OBLIGATION_UNASSIGNED, SHT_006_ASSET_REFERENCE_DANGLING, SHT_001_UNRENDERABLE_SHOT |

## S06_ASSET_PLAN

| 항목 | 값 |
|---|---|
| artifact_owner | ArtDepartment |
| executor_classes | LLM, DET |
| retry.max_repairs | 2 |
| PASS next | S07_ASSET_MATERIALIZE |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| shot_bundle_ir | schemas/shot-ir.schema.json | True |
| director_ir | schemas/director-ir.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| asset_manifest | schemas/asset-manifest.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- all shot asset refs resolved
- continuity scopes defined
- identity-critical forbidden mutations defined
- no unused expensive asset

### Failure codes

- `AST_001_MISSING_REFERENCE`
- `AST_002_IDENTITY_SPEC_INCOMPLETE`
- `AST_003_CONTINUITY_SCOPE_INVALID`
- `AST_004_HIDDEN_AREA_REQUIRED`
- `AST_005_RIGHTS_PROVENANCE_MISSING`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | collect unique asset requirements from shots | DET | shot_bundle_ir | asset_requirements | all shot refs represented | AST_001_MISSING_REFERENCE |
| P02 | define identity continuity hidden-area and mutation rules | LLM | asset_requirements, director_ir | asset_manifest.draft | identity-critical assets fully specified | AST_002_IDENTITY_SPEC_INCOMPLETE, AST_004_HIDDEN_AREA_REQUIRED |
| P03 | remove unused assets and validate rights/continuity scope | DET | asset_manifest.draft | asset_manifest | no dangling/unused asset and scopes valid | AST_003_CONTINUITY_SCOPE_INVALID, AST_005_RIGHTS_PROVENANCE_MISSING |

## S07_ASSET_MATERIALIZE

| 항목 | 값 |
|---|---|
| artifact_owner | AssetMaterializer |
| executor_classes | GEN, PROC, VLM |
| retry.max_repairs | 2 |
| PASS next | S08_RENDER_PLAN |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| asset_manifest | schemas/asset-manifest.schema.json | True |
| director_ir | schemas/director-ir.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| materialized_assets | schemas/artifact-envelope.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- required assets exist/decode
- identity/style/world hard checks PASS
- hashes recorded

### Failure codes

- `GEN_001_IDENTITY_DRIFT`
- `GEN_002_STYLE_DRIFT`
- `GEN_003_STATE_VIOLATION`
- `GEN_004_REFERENCE_IGNORED`
- `GEN_005_OUTPUT_CORRUPT`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | generate or procedurally build required anchors/assets | GEN | asset_manifest, director_ir | candidate_assets | all required outputs produced | GEN_005_OUTPUT_CORRUPT |
| P02 | verify identity style and world-state constraints | VLM | candidate_assets, asset_manifest | asset_eval | hard asset checks PASS | GEN_001_IDENTITY_DRIFT, GEN_002_STYLE_DRIFT, GEN_003_STATE_VIOLATION, GEN_004_REFERENCE_IGNORED |
| P03 | hash and register accepted assets | DET | candidate_assets, asset_eval | materialized_assets | accepted asset hashes recorded | GEN_005_OUTPUT_CORRUPT |

## S08_RENDER_PLAN

| 항목 | 값 |
|---|---|
| artifact_owner | RenderPlanner |
| executor_classes | DET, LLM |
| retry.max_repairs | 1 |
| PASS next | S09_SHOT_RENDER |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| shot_bundle_ir | schemas/shot-ir.schema.json | True |
| asset_manifest | schemas/asset-manifest.schema.json | True |
| materialized_assets | schemas/artifact-envelope.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| render_ir | schemas/render-ir.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- each job has capable backend
- hard invariants not delegated to uncontrolled stochastic path
- job DAG acyclic

### Failure codes

- `RND_001_NO_CAPABLE_BACKEND`
- `RND_002_CAPABILITY_MISMATCH`
- `RND_003_NONDETERMINISTIC_INVARIANT`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | compile shot constraints into renderer capability requirements | DET | shot_bundle_ir, asset_manifest | render_requirements | every shot has capability requirements | RND_002_CAPABILITY_MISMATCH |
| P02 | select backend with hard capability match before quality/cost | DET | render_requirements, capability_registry | backend_assignments | all jobs have capable backend | RND_001_NO_CAPABLE_BACKEND |
| P03 | reject uncontrolled stochastic paths for material hard invariants | DET | backend_assignments, shot_bundle_ir | render_ir | every hard invariant has deterministic or verified control path | RND_003_NONDETERMINISTIC_INVARIANT |

## S09_SHOT_RENDER

| 항목 | 값 |
|---|---|
| artifact_owner | Renderer |
| executor_classes | GEN, PROC, VLM |
| retry.max_repairs | 2 |
| PASS next | S10_COMPOSE |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| render_ir | schemas/render-ir.schema.json | True |
| materialized_assets | schemas/artifact-envelope.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| shot_clips | schemas/artifact-envelope.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- outputs decode
- shot hard invariants PASS
- continuity tolerance PASS

### Failure codes

- `RND_004_RENDER_TIMEOUT`
- `RND_005_BACKEND_ERROR`
- `MOT_001_MOTION_INVARIANT_VIOLATION`
- `MOT_002_BACKGROUND_DRIFT`
- `MOT_003_TEMPORAL_DISCONTINUITY`
- `MOT_004_LOOP_SEAM_VISIBLE`
- `MOT_005_CAMERA_TRAJECTORY_VIOLATION`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | execute render jobs | GEN | render_ir, materialized_assets | candidate_clips | jobs complete within limits | RND_004_RENDER_TIMEOUT, RND_005_BACKEND_ERROR |
| P02 | check per-shot motion and continuity invariants | VLM | candidate_clips, shot_bundle_ir | clip_eval | hard motion checks PASS | MOT_001_MOTION_INVARIANT_VIOLATION, MOT_002_BACKGROUND_DRIFT, MOT_003_TEMPORAL_DISCONTINUITY, MOT_004_LOOP_SEAM_VISIBLE, MOT_005_CAMERA_TRAJECTORY_VIOLATION |
| P03 | decode/hash/register accepted clips | DET | candidate_clips, clip_eval | shot_clips | all accepted clips decode and hash | RND_005_BACKEND_ERROR |

## S10_COMPOSE

| 항목 | 값 |
|---|---|
| artifact_owner | Composer |
| executor_classes | PROC, DET |
| retry.max_repairs | 1 |
| PASS next | S11_VERIFY_REPAIR |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| shot_clips | schemas/artifact-envelope.schema.json | True |
| director_ir | schemas/director-ir.schema.json | True |
| validated_package | schemas/narrative-package.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| timeline_ir | schemas/timeline-ir.schema.json |
| draft_master | schemas/artifact-envelope.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- frame-exact timeline
- text/audio policy collision == 0
- duration/fps/resolution valid

### Failure codes

- `CMP_001_TIMELINE_INVALID`
- `CMP_002_TEXT_TIMING_COLLISION`
- `CMP_003_AUDIO_TIMING_COLLISION`
- `CMP_004_FRAME_RATE_MISMATCH`
- `CMP_005_DURATION_OUT_OF_RANGE`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | compile frame-exact visual/text/audio tracks | PROC | shot_clips, director_ir, validated_package | timeline_ir.draft | all referenced clips resolve | CMP_001_TIMELINE_INVALID |
| P02 | validate timing collision fps resolution and duration | DET | timeline_ir.draft | timeline_report | all technical timeline gates PASS | CMP_002_TEXT_TIMING_COLLISION, CMP_003_AUDIO_TIMING_COLLISION, CMP_004_FRAME_RATE_MISMATCH, CMP_005_DURATION_OUT_OF_RANGE |
| P03 | render draft master | PROC | timeline_ir.draft, timeline_report | timeline_ir, draft_master | draft decodes and timeline hash recorded | CMP_001_TIMELINE_INVALID |

## S11_VERIFY_REPAIR

| 항목 | 값 |
|---|---|
| artifact_owner | Verifier |
| executor_classes | DET, VLM, LLM |
| retry.max_repairs | 2 |
| PASS next | S12_PACKAGE_MASTER |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| draft_master | schemas/artifact-envelope.schema.json | True |
| meaning_spec | schemas/meaning-spec.schema.json | True |
| scene_bundle_ir | schemas/scene-ir.schema.json | True |
| shot_bundle_ir | schemas/shot-ir.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| evaluation_report | schemas/evaluation-report.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- hard assertions == 100%
- hard transition recoverability == 100% or NEEDS_REVIEW
- hard contradictions == 0
- technical decode PASS
- rights != FAIL

### Failure codes

- `QAV_001_TRANSITION_UNRECOVERABLE`
- `QAV_002_SOURCE_CONTRADICTION`
- `QAV_003_CONTINUITY_FAILURE`
- `QAV_004_HARD_GATE_FAILED`
- `QAV_005_LOW_CONFIDENCE_REVIEW`
- `QAV_006_RIGHTS_GATE_FAILED`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | run structural technical and deterministic invariant checks | DET | draft_master, timeline_ir | det_eval | technical hard gates PASS | QAV_004_HARD_GATE_FAILED |
| P02 | evaluate semantic assertions continuity and transition recoverability | VLM | draft_master, meaning_spec, scene_bundle_ir, shot_bundle_ir | semantic_eval | hard semantic checks PASS or low confidence -> review | QAV_001_TRANSITION_UNRECOVERABLE, QAV_002_SOURCE_CONTRADICTION, QAV_003_CONTINUITY_FAILURE, QAV_005_LOW_CONFIDENCE_REVIEW |
| P03 | compile evaluation report and route failures to root owner | DET | det_eval, semantic_eval | evaluation_report | every FAIL has repair_stage or terminal reason | QAV_004_HARD_GATE_FAILED, QAV_006_RIGHTS_GATE_FAILED |

## S12_PACKAGE_MASTER

| 항목 | 값 |
|---|---|
| artifact_owner | Packager |
| executor_classes | DET, PROC |
| retry.max_repairs | 0 |
| PASS next | terminal |
| FAIL | REPAIR_ROUTER |

### 정확한 입력

| kind | schema | required |
|---|---|---|
| draft_master | schemas/artifact-envelope.schema.json | True |
| evaluation_report | schemas/evaluation-report.schema.json | True |

### 정확한 출력

| kind | schema |
|---|---|
| master_package | schemas/artifact-envelope.schema.json |

### Preconditions

- all required upstream artifacts exist and validate

### Hard gates

- S11 hard_gate_status == PASS
- master decodes
- content hash matches
- lineage complete

### Failure codes

- `PKG_001_MEDIA_DECODE_FAILURE`
- `PKG_002_LINEAGE_INCOMPLETE`
- `PKG_003_HASH_MISMATCH`
- `PKG_004_MANIFEST_INCOMPLETE`

### Substeps

| ID | purpose | executor | inputs | outputs | hard gate | failures |
|---|---|---|---|---|---|---|
| P01 | verify S11 hard_gate_status PASS | DET | evaluation_report | package_gate | hard_gate_status == PASS | PKG_004_MANIFEST_INCOMPLETE |
| P02 | package master lineage hashes reports and optional external provenance | PROC | draft_master, evaluation_report, artifact_lineage | master_package | package decodes and manifest complete | PKG_001_MEDIA_DECODE_FAILURE, PKG_002_LINEAGE_INCOMPLETE, PKG_003_HASH_MISMATCH, PKG_004_MANIFEST_INCOMPLETE |
