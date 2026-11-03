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
