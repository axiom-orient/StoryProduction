> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# ScriptForge 26-node 상세 계약

Source archive: `c4ce79818873205e3fcc13c3b7679f6e4d9198f12d07d8cac5f74ff9c88349f0`

| node | kind | role | T stage | depends_on | output | PASS next |
|---|---|---|---|---|---|---|
| step_00 | agent | analyst | T12 | - | output/step_00_critique.md | human_step0_feedback |
| human_step0_feedback | human | human | T12 | step_00 | input/human_feedback/step_00.md | step_01 |
| step_01 | agent | analyst | T13 | step_00, human_step0_feedback | output/step_01_act_structure.md | step_02 |
| step_02 | agent | analyst | T13 | step_00, step_01 | output/step_02_beat_sheet.md | checkpoint_a_review |
| checkpoint_a_review | agent | critic | T13 | step_00, step_01, step_02 | output/checkpoint_a_review.md | checkpoint_a |
| checkpoint_a | human | human | T13 | checkpoint_a_review | - | step_03 |
| step_03 | agent | writer | T14 | step_00, step_01, step_02, checkpoint_a | output/step_03_image_system.md | step_04 |
| step_04 | agent | writer | T14 | step_00, step_01, step_02, step_03 | output/step_04_characters.md | checkpoint_b_review |
| checkpoint_b_review | agent | critic | T14 | step_00, step_03, step_04 | output/checkpoint_b_review.md | checkpoint_b |
| checkpoint_b | human | human | T14 | checkpoint_b_review | - | step_05 |
| step_05 | agent | writer | T15 | step_00, step_01, step_02, step_03, step_04, checkpoint_b | output/step_05_treatment.md | step_06 |
| step_06 | agent | writer | T15 | step_00, step_01, step_02, step_03, step_04, step_05 | output/step_06_scene_list.md | checkpoint_c_review |
| checkpoint_c_review | agent | critic | T15 | step_00, step_01, step_03, step_05, step_06 | output/checkpoint_c_review.md | checkpoint_c |
| checkpoint_c | human | human | T15 | checkpoint_c_review | - | step_07_act1 |
| step_07_act1 | agent | writer | T17 | step_00, step_03, step_04, step_05, step_06, checkpoint_c | output/step_07_first_draft_act1.md | step_07_act1_review |
| step_07_act1_review | human | human | T17 | step_07_act1 | - | step_07_act2a |
| step_07_act2a | agent | writer | T17 | step_00, step_03, step_04, step_05, step_06, step_07_act1, step_07_act1_review | output/step_07_first_draft_act2a.md | step_07_act2a_review |
| step_07_act2a_review | human | human | T17 | step_07_act2a | - | step_07_act2b |
| step_07_act2b | agent | writer | T17 | step_00, step_03, step_04, step_05, step_06, step_07_act1, step_07_act2a, step_07_act2a_review | output/step_07_first_draft_act2b.md | step_07_act2b_review |
| step_07_act2b_review | human | human | T17 | step_07_act2b | - | step_07_act3 |
| step_07_act3 | agent | writer | T17 | step_00, step_03, step_04, step_05, step_06, step_07_act1, step_07_act2a, step_07_act2b, step_07_act2b_review | output/step_07_first_draft_act3.md | step_07_act3_review |
| step_07_act3_review | human | human | T17 | step_07_act3 | - | step_07_full |
| step_07_full | agent | writer | T17 | step_07_act1, step_07_act2a, step_07_act2b, step_07_act3, step_07_act3_review | output/step_07_first_draft_full.md | step_08_diagnosis |
| step_08_diagnosis | agent | critic | T18 | step_07_full, step_03, step_06, step_00 | output/step_08_visual_revision_diagnosis.md | step_08_apply |
| step_08_apply | agent | writer | T18 | step_07_full, step_08_diagnosis, step_03, step_06, step_00 | output/final_screenplay.md | step_08_summary |
| step_08_summary | agent | writer | T18 | step_07_full, step_08_diagnosis, step_08_apply, step_03, step_06, step_00 | output/step_08_visual_revision.md | terminal:scriptforge_complete |

## step_00 — prompts/analyst/step_00_critique.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:analyst |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T12 |
| depends_on | - |

### 정확한 입력 token

- `config.yaml`
- `@synopsis`
- `@references`
- `?영화시나리오연구_영화적묘사_markdown.md`
- `?영화 시나리오 작법 연구 개요.docx`

### 정확한 출력

`output/step_00_critique.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/analyst/system.md | 1921 | e8ee443292f5cf0131bd8570e11dce57c485ecb6d2c6f0bd4655437043c54e86 |
| task_prompt | prompts/analyst/step_00_critique.md | 4811 | 58b09e72ae0dee49bc820d39f9b02a336b42d42123840d77cbbf287df6d1c0ad |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_00.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_00.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_00.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_00.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | human_step0_feedback |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## human_step0_feedback — STEP 0 보완점 확인

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T12 |
| depends_on | step_00 |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/step_00_critique.md |  | input/human_feedback/step_00.md |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.human_step0_feedback.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.human_step0_feedback.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_01 |
| on_revision | human_step0_feedback |
| on_reject | step_01 |
| on_pivot |  |

## step_01 — prompts/analyst/step_01_act_structure.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:analyst |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T13 |
| depends_on | step_00, human_step0_feedback |

### 정확한 입력 token

- `config.yaml`
- `@synopsis`
- `output/step_00_critique.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/step_01_act_structure.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/analyst/system.md | 1921 | e8ee443292f5cf0131bd8570e11dce57c485ecb6d2c6f0bd4655437043c54e86 |
| task_prompt | prompts/analyst/step_01_act_structure.md | 3698 | 924bcd4557e28360db317dd3b1ced2c68568e895b9a83a791cd003fd062b9565 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_01.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_01.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_01.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_01.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_02 |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_02 — prompts/analyst/step_02_beat_sheet.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:analyst |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T13 |
| depends_on | step_00, step_01 |

### 정확한 입력 token

- `config.yaml`
- `output/step_00_critique.md`
- `?input/human_feedback/step_00.md`
- `output/step_01_act_structure.md`

### 정확한 출력

`output/step_02_beat_sheet.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/analyst/system.md | 1921 | e8ee443292f5cf0131bd8570e11dce57c485ecb6d2c6f0bd4655437043c54e86 |
| task_prompt | prompts/analyst/step_02_beat_sheet.md | 6097 | 2f654a1fcdc5a6effeb1fc0d15fb190bd79ca7ca0c1122c6c05025d3ed62569a |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_02.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_02.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_02.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_02.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | checkpoint_a_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## checkpoint_a_review — prompts/critic/checkpoint_a.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:critic |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T13 |
| depends_on | step_00, step_01, step_02 |

### 정확한 입력 token

- `config.yaml`
- `output/step_00_critique.md`
- `output/step_01_act_structure.md`
- `output/step_02_beat_sheet.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/checkpoint_a_review.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/critic/system.md | 2551 | aa0ded62090e41ba54d8a54dcea693923a51e903748853da2811008fd3594f66 |
| task_prompt | prompts/critic/checkpoint_a.md | 3097 | 0fbebfd032bb0a92306d92309a5f1ec385c69727f780097a9d7ca345b449398a |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.checkpoint_a_review.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.checkpoint_a_review.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.checkpoint_a_review.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.checkpoint_a_review.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | checkpoint_a |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## checkpoint_a — 구조 확정

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T13 |
| depends_on | checkpoint_a_review |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/checkpoint_a_review.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.checkpoint_a.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.checkpoint_a.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_03 |
| on_revision | step_00, step_01, step_02 |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_03 — prompts/writer/step_03_image_system.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T14 |
| depends_on | step_00, step_01, step_02, checkpoint_a |

### 정확한 입력 token

- `output/step_00_critique.md`
- `output/step_01_act_structure.md`
- `output/step_02_beat_sheet.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_03_image_system.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_03_image_system.md | 5363 | 61568eecfd672e6f45c2d695060cb711fc2db73a327fdea7b0127d82e656584a |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_03.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_03.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_03.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_03.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_04 |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_04 — prompts/writer/step_04_characters.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T14 |
| depends_on | step_00, step_01, step_02, step_03 |

### 정확한 입력 token

- `@synopsis`
- `output/step_00_critique.md`
- `output/step_01_act_structure.md`
- `output/step_02_beat_sheet.md`
- `output/step_03_image_system.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_04_characters.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_04_characters.md | 6390 | b38c3695e9d64f494fd65d2cd93a44f082836205eccd67e0cdd5c3bc1922d0de |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_04.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_04.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_04.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_04.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | checkpoint_b_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## checkpoint_b_review — prompts/critic/checkpoint_b.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:critic |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T14 |
| depends_on | step_00, step_03, step_04 |

### 정확한 입력 token

- `output/step_00_critique.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/checkpoint_b_review.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/critic/system.md | 2551 | aa0ded62090e41ba54d8a54dcea693923a51e903748853da2811008fd3594f66 |
| task_prompt | prompts/critic/checkpoint_b.md | 3014 | aa9ca743e5067be10b3c5c352a91e0240dbafe39c9c75278304f0e2e6223457f |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.checkpoint_b_review.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.checkpoint_b_review.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.checkpoint_b_review.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.checkpoint_b_review.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | checkpoint_b |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## checkpoint_b — 시각 전략 및 인물 확정

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T14 |
| depends_on | checkpoint_b_review |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/checkpoint_b_review.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.checkpoint_b.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.checkpoint_b.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_05 |
| on_revision | step_03, step_04 |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_05 — prompts/writer/step_05_treatment.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T15 |
| depends_on | step_00, step_01, step_02, step_03, step_04, checkpoint_b |

### 정확한 입력 token

- `config.yaml`
- `@synopsis`
- `@references`
- `output/step_00_critique.md`
- `output/step_01_act_structure.md`
- `output/step_02_beat_sheet.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/step_05_treatment.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_05_treatment.md | 10903 | a05adf1ea7ea5595a2d337e2663cda7b981a68522c3d45ca7e82aec14615be30 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_05.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_05.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_05.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_05.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_06 |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_06 — prompts/writer/step_06_scene_list.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T15 |
| depends_on | step_00, step_01, step_02, step_03, step_04, step_05 |

### 정확한 입력 token

- `output/step_00_critique.md`
- `output/step_01_act_structure.md`
- `output/step_02_beat_sheet.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `output/step_05_treatment.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/step_06_scene_list.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_06_scene_list.md | 4815 | 2bc1f0bbfd88376489915f04533cbaaf5d835db83101d49344d45687cfc1b3d4 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_06.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_06.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_06.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_06.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | checkpoint_c_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## checkpoint_c_review — prompts/critic/checkpoint_c.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:critic |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T15 |
| depends_on | step_00, step_01, step_03, step_05, step_06 |

### 정확한 입력 token

- `config.yaml`
- `output/step_00_critique.md`
- `output/step_01_act_structure.md`
- `output/step_03_image_system.md`
- `output/step_05_treatment.md`
- `output/step_06_scene_list.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/checkpoint_c_review.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/critic/system.md | 2551 | aa0ded62090e41ba54d8a54dcea693923a51e903748853da2811008fd3594f66 |
| task_prompt | prompts/critic/checkpoint_c.md | 3492 | 7847c4c651b36d3e17be1204a53b4ee6874b7e1faf2b28473582f8b281426deb |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.checkpoint_c_review.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.checkpoint_c_review.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.checkpoint_c_review.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.checkpoint_c_review.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | checkpoint_c |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## checkpoint_c — 집필 전 최종 리뷰

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T15 |
| depends_on | checkpoint_c_review |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/checkpoint_c_review.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.checkpoint_c.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.checkpoint_c.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act1 |
| on_revision | step_05, step_06 |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act1 — prompts/writer/step_07_first_draft.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T17 |
| depends_on | step_00, step_03, step_04, step_05, step_06, checkpoint_c |

### 정확한 입력 token

- `@synopsis`
- `output/step_00_critique.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `output/step_05_treatment.md`
- `output/step_06_scene_list.md`
- `?prompts/writer/step_07_examples.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_07_first_draft_act1.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_07_first_draft.md | 15266 | af02ae39a642222425ddc61601cb179bab3c69c20d64d7c7635a2e60abd14b15 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_07_act1.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_07_act1.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_07_act1.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_07_act1.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act1_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act1_review — STEP 7 1막 점검

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T17 |
| depends_on | step_07_act1 |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/step_07_first_draft_act1.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.step_07_act1_review.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.step_07_act1_review.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act2a |
| on_revision | step_07_act1 |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act2a — prompts/writer/step_07_first_draft.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T17 |
| depends_on | step_00, step_03, step_04, step_05, step_06, step_07_act1, step_07_act1_review |

### 정확한 입력 token

- `@synopsis`
- `output/step_00_critique.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `output/step_05_treatment.md`
- `output/step_06_scene_list.md`
- `output/step_07_first_draft_act1.md`
- `?prompts/writer/step_07_examples.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_07_first_draft_act2a.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_07_first_draft.md | 15266 | af02ae39a642222425ddc61601cb179bab3c69c20d64d7c7635a2e60abd14b15 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_07_act2a.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_07_act2a.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_07_act2a.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_07_act2a.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act2a_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act2a_review — STEP 7 2막 전반 점검

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T17 |
| depends_on | step_07_act2a |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/step_07_first_draft_act2a.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.step_07_act2a_review.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.step_07_act2a_review.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act2b |
| on_revision | step_07_act2a |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act2b — prompts/writer/step_07_first_draft.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T17 |
| depends_on | step_00, step_03, step_04, step_05, step_06, step_07_act1, step_07_act2a, step_07_act2a_review |

### 정확한 입력 token

- `@synopsis`
- `output/step_00_critique.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `output/step_05_treatment.md`
- `output/step_06_scene_list.md`
- `output/step_07_first_draft_act1.md`
- `output/step_07_first_draft_act2a.md`
- `?prompts/writer/step_07_examples.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_07_first_draft_act2b.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_07_first_draft.md | 15266 | af02ae39a642222425ddc61601cb179bab3c69c20d64d7c7635a2e60abd14b15 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_07_act2b.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_07_act2b.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_07_act2b.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_07_act2b.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act2b_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act2b_review — STEP 7 2막 후반 점검

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T17 |
| depends_on | step_07_act2b |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/step_07_first_draft_act2b.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.step_07_act2b_review.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.step_07_act2b_review.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act3 |
| on_revision | step_07_act2b |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act3 — prompts/writer/step_07_first_draft.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T17 |
| depends_on | step_00, step_03, step_04, step_05, step_06, step_07_act1, step_07_act2a, step_07_act2b, step_07_act2b_review |

### 정확한 입력 token

- `@synopsis`
- `output/step_00_critique.md`
- `output/step_03_image_system.md`
- `output/step_04_characters.md`
- `output/step_05_treatment.md`
- `output/step_06_scene_list.md`
- `output/step_07_first_draft_act1.md`
- `output/step_07_first_draft_act2a.md`
- `output/step_07_first_draft_act2b.md`
- `?prompts/writer/step_07_examples.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_07_first_draft_act3.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_07_first_draft.md | 15266 | af02ae39a642222425ddc61601cb179bab3c69c20d64d7c7635a2e60abd14b15 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_07_act3.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_07_act3.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_07_act3.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_07_act3.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_act3_review |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_act3_review — STEP 7 3막 점검

| 항목 | 값 |
|---|---|
| kind | human |
| execution_class | human_gate |
| executor | human |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |
| macro_stage | T17 |
| depends_on | step_07_act3 |

### Gate source/결정

| source | allowed | optional output |
|---|---|---|
| output/step_07_first_draft_act3.md |  | None |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| source_revision_exact | digest | scriptforge-v4.step_07_act3_review.source_revision_exact | gate_source | gate source가 현재 producer output revision과 일치한다. |
| human_decision_recorded | human_decision | scriptforge-v4.step_07_act3_review.human_decision_recorded | gate_source | actor, rationale, exact subject, action이 기록된다. |

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-CFT-005`
- `GT-CFT-006`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_07_full |
| on_revision | step_07_act3 |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_07_full — prompts/writer/step_07_first_draft.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T17 |
| depends_on | step_07_act1, step_07_act2a, step_07_act2b, step_07_act3, step_07_act3_review |

### 정확한 입력 token

- `output/step_07_first_draft_act1.md`
- `output/step_07_first_draft_act2a.md`
- `output/step_07_first_draft_act2b.md`
- `output/step_07_first_draft_act3.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/step_07_first_draft_full.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/writer/step_07_first_draft.md | 15266 | af02ae39a642222425ddc61601cb179bab3c69c20d64d7c7635a2e60abd14b15 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_07_full.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_07_full.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_07_full.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_07_full.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_08_diagnosis |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_08_diagnosis — prompts/critic/step_08_visual_revision.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:critic |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T18 |
| depends_on | step_07_full, step_03, step_06, step_00 |

### 정확한 입력 token

- `output/step_07_first_draft_full.md`
- `output/step_03_image_system.md`
- `output/step_06_scene_list.md`
- `output/step_00_critique.md`
- `?input/human_feedback/step_00.md`
- `?영화시나리오연구_영화적묘사_markdown.md`

### 정확한 출력

`output/step_08_visual_revision_diagnosis.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/critic/system.md | 2551 | aa0ded62090e41ba54d8a54dcea693923a51e903748853da2811008fd3594f66 |
| task_prompt | prompts/critic/step_08_visual_revision.md | 10268 | 0b338f3cd9c19919de82e773b930e79113c168f9cccbaee4d15e1cfa80cfec33 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_08_diagnosis.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_08_diagnosis.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_08_diagnosis.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_08_diagnosis.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_08_apply |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_08_apply — prompts/critic/step_08_visual_revision.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T18 |
| depends_on | step_07_full, step_08_diagnosis, step_03, step_06, step_00 |

### 정확한 입력 token

- `output/step_07_first_draft_full.md`
- `output/step_08_visual_revision_diagnosis.md`
- `output/step_03_image_system.md`
- `output/step_06_scene_list.md`
- `output/step_00_critique.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/final_screenplay.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/critic/step_08_visual_revision.md | 10268 | 0b338f3cd9c19919de82e773b930e79113c168f9cccbaee4d15e1cfa80cfec33 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_08_apply.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_08_apply.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_08_apply.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_08_apply.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | step_08_summary |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |

## step_08_summary — prompts/critic/step_08_visual_revision.md

| 항목 | 값 |
|---|---|
| kind | agent |
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | scriptforge_v4_craft |
| artifact_owner | geultu.text |
| macro_stage | T18 |
| depends_on | step_07_full, step_08_diagnosis, step_08_apply, step_03, step_06, step_00 |

### 정확한 입력 token

- `output/step_07_first_draft_full.md`
- `output/step_08_visual_revision_diagnosis.md`
- `output/final_screenplay.md`
- `output/step_03_image_system.md`
- `output/step_06_scene_list.md`
- `output/step_00_critique.md`
- `?input/human_feedback/step_00.md`

### 정확한 출력

`output/step_08_visual_revision.md`

### Prompt asset fidelity

| 역할 | 경로 | bytes | SHA-256 |
|---|---|---|---|
| system_prompt | prompts/writer/system.md | 6721 | e649264dcb1dbfa913f9cdba605a9128ca3c3653f0f3b87ab8c326ef18317d1a |
| task_prompt | prompts/critic/step_08_visual_revision.md | 10268 | 0b338f3cd9c19919de82e773b930e79113c168f9cccbaee4d15e1cfa80cfec33 |

### 통과 조건

| condition | evidence | validator | subject | description |
|---|---|---|---|---|
| prompt_assets_verified | digest | scriptforge-v4.step_08_summary.prompt_assets_verified | output | system/task prompt bytes가 source manifest와 일치한다. |
| declared_inputs_resolved | provenance | scriptforge-v4.step_08_summary.declared_inputs_resolved | output | 필수 input token이 exact artifact/file binding으로 해결된다. |
| provider_receipt_valid | execution_receipt | scriptforge-v4.step_08_summary.provider_receipt_valid | output | provider exit 0, non-empty output, prompt/output digest가 기록된다. |
| durable_output_valid | schema | scriptforge-v4.step_08_summary.durable_output_valid | output | 선언 output path의 artifact schema와 digest가 유효하다. |

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음

| 결과 | route |
|---|---|
| on_pass | terminal:scriptforge_complete |
| on_revision |  |
| on_reject | terminal:rejected |
| on_pivot |  |
