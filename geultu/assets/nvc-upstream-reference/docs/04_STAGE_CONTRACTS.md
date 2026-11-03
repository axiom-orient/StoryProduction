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
