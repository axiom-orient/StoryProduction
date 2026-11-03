> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# 텍스트 파이프라인 T00–T33 상세 계약

정본: `geultu.dev/pipeline-contract/v2` / `2.0.0`

## 불변식

- Geultu alone owns style, artifact state, approval, refinement, revision and TextMaster authority.
- Craft adapters own method, prompt and genre knowledge but cannot approve or publish.
- Every input is an exact artifact id/kind/revision/SHA-256 reference.
- Only declared direct dependencies enter a model prompt; full-history injection is forbidden.
- Refinement produces a digest-bound minimal patch; translation and silent full rewrite are forbidden.
- Front-only runs terminate at T32; full runs export exactly one GeultuStoryPackage at T33.
- ScriptForge prompt bytes and 26-node workflow are versioned source assets, not paraphrased templates.

## 전체 단계

| ID | 이름 | 실행 분류 | executor | artifact owner | 출력 | PASS 다음 |
|---|---|---|---|---|---|---|
| T00 | request_normalize | pure_transform | geultu.request.normalizer | geultu.request | normalized_request | T01 |
| T01 | rights_and_source_check | hybrid_validation | geultu.source.rights_gate | geultu.source | rights_report | T02 |
| T02 | canon_compile | hybrid_validation | geultu.canon.compiler | geultu.canon | compiled_canon | T03 |
| T03 | style_corpus_normalize | pure_transform | geultu.style.corpus_normalizer | geultu.style | normalized_style_corpus | T04 |
| T04 | style_measure | pure_transform | geultu.style.analyzer | geultu.style | style_profile | T05 |
| T05 | style_candidate_compile | pure_transform | geultu.style.compiler | geultu.style | style_candidate | T06 |
| T06 | style_candidate_compare | hybrid_review | geultu.style.comparator | geultu.style | style_comparison | T07 |
| T07 | style_approve | human_gate | human.style_owner | geultu.style | approved_style | T08 |
| T08 | genre_select | pure_transform | geultu.genre.selector | geultu.genre | genre_selection | T09 |
| T09 | genre_contract | hybrid_validation | geultu.genre.contract_compiler | geultu.genre | genre_contract | T10 |
| T10 | craft_strategy_select | deterministic_validation | geultu.text.adapter_registry | geultu.genre | craft_strategy_binding | T11 |
| T11 | craft_pack_verify_and_bind_inputs | deterministic_validation | geultu.text.adapter_verifier | geultu.text | craft_input_plan | T12 |
| T12 | story_diagnose | model_effect | selected_provider:analyst | geultu.text | story_diagnosis | T13 |
| T13 | dramatic_structure | model_effect | selected_provider:analyst+critic | geultu.text | dramatic_structure | T14 |
| T14 | image_character_world_design | model_effect | selected_provider:writer+critic | geultu.text | narrative_design | T15 |
| T15 | treatment_scene_plan | model_effect | selected_provider:writer+critic | geultu.text | treatment_scene_plan | T16 |
| T16 | draft_plan_freeze | deterministic_validation | geultu.text.plan_freezer | geultu.text | frozen_draft_plan | T17 |
| T17 | unit_drafting | model_effect | selected_provider:writer | geultu.text | unit_draft_set | T18 |
| T18 | craft_critique_and_revision | model_effect | selected_provider:critic+writer | geultu.text | craft_reviewed_draft | T19 |
| T19 | structural_review | hybrid_review | geultu.quality.structural_reviewer | geultu.quality | structural_review | T20 |
| T20 | semantic_and_canon_review | hybrid_review | geultu.quality.semantic_reviewer | geultu.quality | semantic_review | T21 |
| T21 | style_drift_review | hybrid_review | geultu.quality.style_reviewer | geultu.quality | style_review | T22 |
| T22 | locale_refinement_diagnose | hybrid_review | geultu.text.locale_refinement | geultu.refinement | refinement_diagnosis | T23 |
| T23 | polish_patch_plan | model_effect | geultu.text.locale_refinement | geultu.refinement | polish_patch | T24 |
| T24 | semantic_guard | deterministic_validation | geultu.refinement.semantic_guard | geultu.refinement | semantic_guard_report | T25 |
| T25 | polish_patch_apply | pure_transform | geultu.refinement.patch_applier | geultu.text | polished_draft | T26 |
| T26 | author_revision_review | human_gate | human.author_or_director | geultu.quality | author_review | T31 |
| T27 | revision_directive | hybrid_validation | geultu.refinement.directive_compiler | geultu.refinement | revision_directive | T28 |
| T28 | targeted_revision_apply | model_effect | selected_provider:writer | geultu.text | revised_draft | T29 |
| T29 | revision_recheck | hybrid_review | geultu.quality.revision_reviewer | geultu.quality | revision_recheck | T30 |
| T30 | revision_disposition_gate | human_gate | human.product_owner | geultu.approval | approval_decision | T31 |
| T31 | genre_completion | hybrid_validation | geultu.quality.genre_completion | geultu.quality | completion_report | T32 |
| T32 | text_master_approve | human_gate | human.text_owner | geultu.state | text_master | T33 |
| T33 | geultu_story_package_export | projection | geultu.handoff.exporter | geultu.handoff | geultu_story_package | terminal:geultu_story_package_complete |

## T00 — request_normalize

자유형 요청을 누락 없는 실행 계약으로 정규화한다.

| 항목 | 값 |
|---|---|
| execution_class | pure_transform |
| executor | geultu.request.normalizer |
| semantic_authority | human_intent |
| artifact_owner | geultu.request |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| request | request_spec | external:request | False | True | request_normalize에서 external:request의 request_spec exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| normalized_request | schemas/artifacts/normalized_request.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| request_schema_valid | schema | True | execution_receipt | geultu.text.v2.T00.request_schema_valid | output | RequestSpec와 NormalizedRequest schema가 유효하다. |
| intent_preserved | deterministic_diff | True | execution_receipt | geultu.text.v2.T00.intent_preserved | output | 원래 목표·대상·제약을 삭제하거나 추가하지 않았다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-001`
- `GT-CTR-005`
- `GT-CTR-007`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T01 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T01 — rights_and_source_check

모든 source와 style corpus의 권리·출처·허용 용도를 fail-closed로 판정한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_validation |
| executor | geultu.source.rights_gate |
| semantic_authority | human_rights_attestation |
| artifact_owner | geultu.source |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| request | normalized_request | T00 | False | True | rights_and_source_check에서 T00의 normalized_request exact revision을 필수로 소비한다. |
| sources | source_bundle | external:sources | False | True | rights_and_source_check에서 external:sources의 source_bundle exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| rights_report | schemas/artifacts/rights_report.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| all_sources_classified | validator | True | execution_receipt | geultu.text.v2.T01.all_sources_classified | output | 모든 source에 소유자·용도·라이선스가 있다. |
| rights_attestation_present | human_evidence | True | approval_decision | geultu.text.v2.T01.rights_attestation_present | input:sources | 사람이 허용 범위를 확인했다. |
| no_blocking_rights_finding | review | True | rights_report | geultu.text.v2.T01.no_blocking_rights_finding | output | blocking finding이 0개다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-001`
- `GT-CTR-005`
- `GT-STY-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T02 |
| REVISION | 금지 |
| REJECT | terminal:rejected |
| PIVOT | terminal pivoted |

## T02 — canon_compile

source provenance를 유지하며 사실·인물·시간·금지사항을 canonical canon으로 만든다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_validation |
| executor | geultu.canon.compiler |
| semantic_authority | geultu.canon_policy |
| artifact_owner | geultu.canon |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| request | normalized_request | T00 | False | True | canon_compile에서 T00의 normalized_request exact revision을 필수로 소비한다. |
| rights | rights_report | T01 | False | True | canon_compile에서 T01의 rights_report exact revision을 필수로 소비한다. |
| sources | source_bundle | external:sources | False | True | canon_compile에서 external:sources의 source_bundle exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| compiled_canon | schemas/artifacts/compiled_canon.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| canon_schema_valid | schema | True | execution_receipt | geultu.text.v2.T02.canon_schema_valid | output | Canon schema가 유효하다. |
| facts_have_provenance | provenance | True | execution_receipt | geultu.text.v2.T02.facts_have_provenance | output | 모든 fact가 source ref에 연결된다. |
| contradictions_resolved | review | True | compiled_canon | geultu.text.v2.T02.contradictions_resolved | output | 미해결 blocking contradiction이 없다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-002`
- `GT-CTR-005`
- `GT-QLT-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T03 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T03 — style_corpus_normalize

권리가 확인된 글뭉치를 측정 가능한 형태로 정규화하되 내용을 style identity로 복사하지 않는다.

| 항목 | 값 |
|---|---|
| execution_class | pure_transform |
| executor | geultu.style.corpus_normalizer |
| semantic_authority | geultu.style_policy |
| artifact_owner | geultu.style |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| sources | source_bundle | external:sources | False | True | style_corpus_normalize에서 external:sources의 source_bundle exact revision을 필수로 소비한다. |
| rights | rights_report | T01 | False | True | style_corpus_normalize에서 T01의 rights_report exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| normalized_style_corpus | schemas/artifacts/normalized_style_corpus.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| corpus_nonempty | validator | True | execution_receipt | geultu.text.v2.T03.corpus_nonempty | output | 분석 가능한 sample이 1개 이상이다. |
| rights_bound | provenance | True | execution_receipt | geultu.text.v2.T03.rights_bound | output | sample마다 rights result가 있다. |
| normalization_lossless | digest | True | execution_receipt | geultu.text.v2.T03.normalization_lossless | output | 원문 bytes와 normalized mapping이 추적 가능하다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-STY-001`
- `GT-STY-002`
- `GT-CTR-004`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T04 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T04 — style_measure

말투·어투·문체의 측정 가능한 신호를 추출한다.

| 항목 | 값 |
|---|---|
| execution_class | pure_transform |
| executor | geultu.style.analyzer |
| semantic_authority | geultu.style_policy |
| artifact_owner | geultu.style |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| corpus | normalized_style_corpus | T03 | False | True | style_measure에서 T03의 normalized_style_corpus exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| style_profile | schemas/artifacts/style_profile.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| feature_schema_valid | schema | True | execution_receipt | geultu.text.v2.T04.feature_schema_valid | output | feature version과 필수 축이 존재한다. |
| content_leakage_scan_complete | validator | True | execution_receipt | geultu.text.v2.T04.content_leakage_scan_complete | output | 사건·고유문구 leakage 검사를 완료했다. |
| signal_sufficient | validator | True | execution_receipt | geultu.text.v2.T04.signal_sufficient | output | 표본량과 측정 신호가 최소 기준을 충족한다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-STY-001`
- `GT-STY-003`
- `GT-CTR-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T05 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T05 — style_candidate_compile

측정치를 역할별로 적용 가능한 문체 지침과 불변조건으로 컴파일한다.

| 항목 | 값 |
|---|---|
| execution_class | pure_transform |
| executor | geultu.style.compiler |
| semantic_authority | geultu.style_policy |
| artifact_owner | geultu.style |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| profile | style_profile | T04 | False | True | style_candidate_compile에서 T04의 style_profile exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| style_candidate | schemas/artifacts/style_candidate.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| candidate_schema_valid | schema | True | execution_receipt | geultu.text.v2.T05.candidate_schema_valid | output | candidate schema가 유효하다. |
| no_author_imitation_label | validator | True | execution_receipt | geultu.text.v2.T05.no_author_imitation_label | output | 작가명 복제 지시가 없다. |
| craft_canon_precedence | validator | True | execution_receipt | geultu.text.v2.T05.craft_canon_precedence | output | craft 형식과 canon 우선권이 명시된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-005`
- `GT-STY-003`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T06 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T06 — style_candidate_compare

문체 강도·내용 보존·유창성·누출 위험을 독립 축으로 비교한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_review |
| executor | geultu.style.comparator |
| semantic_authority | geultu.style_policy |
| artifact_owner | geultu.style |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| candidate | style_candidate | T05 | False | True | style_candidate_compare에서 T05의 style_candidate exact revision을 필수로 소비한다. |
| profile | style_profile | T04 | False | True | style_candidate_compare에서 T04의 style_profile exact revision을 필수로 소비한다. |
| corpus | normalized_style_corpus | T03 | False | True | style_candidate_compare에서 T03의 normalized_style_corpus exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| style_comparison | schemas/artifacts/style_comparison.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| axes_separated | validator | True | execution_receipt | geultu.text.v2.T06.axes_separated | output | style/content/fluency가 별도 점수와 근거를 갖는다. |
| leakage_not_blocking | review | True | style_comparison | geultu.text.v2.T06.leakage_not_blocking | output | content leakage가 blocking이 아니다. |
| comparison_evidence_complete | review | True | style_comparison | geultu.text.v2.T06.comparison_evidence_complete | output | 대표 sample과 위험이 기록된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-STY-003`
- `GT-CTR-008`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T07 |
| REVISION | T05 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T07 — style_approve

사람이 exact StyleCandidate revision을 승인하여 StyleMaster를 만든다.

| 항목 | 값 |
|---|---|
| execution_class | human_gate |
| executor | human.style_owner |
| semantic_authority | human_style_decision |
| artifact_owner | geultu.style |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| candidate | style_candidate | T05 | False | True | style_approve에서 T05의 style_candidate exact revision을 필수로 소비한다. |
| comparison | style_comparison | T06 | False | True | style_approve에서 T06의 style_comparison exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| approved_style | schemas/artifacts/approved_style.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| exact_subject_approved | human_decision | True | approval_decision | geultu.text.v2.T07.exact_subject_approved | input:candidate | approval subject ID/kind/revision/digest가 candidate와 일치한다. |
| approval_evidence_present | human_evidence | True | approval_decision | geultu.text.v2.T07.approval_evidence_present | input:candidate | comparison 또는 sample evidence가 있다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-APR-004`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T08 |
| REVISION | T05 |
| REJECT | terminal:rejected |
| PIVOT | terminal pivoted |

## T08 — genre_select

요청·canon·StyleMaster에 맞는 닫힌 장르를 선택한다.

| 항목 | 값 |
|---|---|
| execution_class | pure_transform |
| executor | geultu.genre.selector |
| semantic_authority | human_intent |
| artifact_owner | geultu.genre |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| request | normalized_request | T00 | False | True | genre_select에서 T00의 normalized_request exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | genre_select에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | genre_select에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| genre_selection | schemas/artifacts/genre_selection.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| genre_supported | validator | True | execution_receipt | geultu.text.v2.T08.genre_supported | output | 선택 장르가 registry에 존재한다. |
| rationale_present | validator | True | execution_receipt | geultu.text.v2.T08.rationale_present | output | 선택 근거와 제외 장르가 기록된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T09 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T09 — genre_contract

장르 완료 조건·독자 약속·결말 의무를 실행 가능한 계약으로 고정한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_validation |
| executor | geultu.genre.contract_compiler |
| semantic_authority | human_product_intent |
| artifact_owner | geultu.genre |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| selection | genre_selection | T08 | False | True | genre_contract에서 T08의 genre_selection exact revision을 필수로 소비한다. |
| request | normalized_request | T00 | False | True | genre_contract에서 T00의 normalized_request exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | genre_contract에서 T02의 compiled_canon exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| genre_contract | schemas/artifacts/genre_contract.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| completion_requirements_nonempty | validator | True | execution_receipt | geultu.text.v2.T09.completion_requirements_nonempty | output | 장르별 완료 항목이 1개 이상이다. |
| ending_obligation_explicit | review | True | genre_contract | geultu.text.v2.T09.ending_obligation_explicit | output | 결말 공개/폐쇄 의무가 명확하다. |
| audience_promise_explicit | review | True | genre_contract | geultu.text.v2.T09.audience_promise_explicit | output | 대상 독자와 약속이 명시된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-005`
- `GT-QLT-003`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T10 |
| REVISION | T09 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T10 — craft_strategy_select

장르에 따라 native synopsis/short-story/novel/serial-novel adapter 또는 exact ScriptForge adapter를 명시적으로 선택한다.

| 항목 | 값 |
|---|---|
| execution_class | deterministic_validation |
| executor | geultu.text.adapter_registry |
| semantic_authority | geultu_genre_policy |
| artifact_owner | geultu.genre |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| genre_contract | genre_contract | T09 | False | True | craft_strategy_select에서 T09의 genre_contract exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | craft_strategy_select에서 T07의 approved_style exact revision을 필수로 소비한다. |
| pack_registry | verified_prompt_pack | external:pack_registry | True | True | craft_strategy_select에서 external:pack_registry의 verified_prompt_pack exact revision을 선택적으로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| craft_strategy_binding | schemas/artifacts/craft_strategy_binding.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| single_pack_selected | validator | True | execution_receipt | geultu.text.v2.T10.single_pack_selected | output | 하나의 pack ID/version/digest가 선택된다. |
| provider_capability_selected | validator | True | execution_receipt | geultu.text.v2.T10.provider_capability_selected | output | 필요 model capability가 명시된다. |
| no_silent_fallback | validator | True | execution_receipt | geultu.text.v2.T10.no_silent_fallback | output | 대체 provider 자동 전환이 없다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-001`
- `GT-CTR-011`
- `GT-CFT-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T11 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T11 — craft_pack_verify_and_bind_inputs

선택된 adapter의 version, capability, prompt/source digest, license, declared input frontier를 검증하고 고정한다.

| 항목 | 값 |
|---|---|
| execution_class | deterministic_validation |
| executor | geultu.text.adapter_verifier |
| semantic_authority | craft_pack_source |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| binding | craft_strategy_binding | T10 | False | True | craft_pack_verify_and_bind_inputs에서 T10의 craft_strategy_binding exact revision을 필수로 소비한다. |
| pack | verified_prompt_pack | external:selected_pack | False | True | craft_pack_verify_and_bind_inputs에서 external:selected_pack의 verified_prompt_pack exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| craft_input_plan | schemas/artifacts/craft_input_plan.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| source_manifest_verified | digest | True | execution_receipt | geultu.text.v2.T11.source_manifest_verified | output | 모든 declared file의 size/hash가 일치한다. |
| workflow_contract_verified | validator | True | execution_receipt | geultu.text.v2.T11.workflow_contract_verified | output | node·output·input·gate·revision edge가 유효하다. |
| prompt_coverage_complete | validator | True | execution_receipt | geultu.text.v2.T11.prompt_coverage_complete | output | 모든 agent node가 exact prompt bytes를 갖는다. |
| license_policy_recorded | provenance | True | execution_receipt | geultu.text.v2.T11.license_policy_recorded | output | pack license와 배포 정책이 기록된다. |
| direct_input_bindings_frozen | provenance | True | execution_receipt | geultu.text.v2.T11.direct_input_bindings_frozen | output | 각 craft node의 필수 token이 exact artifact/file ref에 결합되고 undeclared context가 없다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-001`
- `GT-CFT-002`
- `GT-CFT-003`
- `GT-CFT-004`
- `GT-CTR-011`
- `GT-CFT-007`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T12 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T12 — story_diagnose

원본/시놉시스의 강점·결함·확장 위험을 독립 Analyst 관점으로 진단한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:analyst |
| semantic_authority | selected_craft_adapter |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| plan | craft_input_plan | T11 | False | True | story_diagnose에서 T11의 craft_input_plan exact revision을 필수로 소비한다. |
| pack | verified_prompt_pack | external:selected_pack | False | True | story_diagnose에서 external:selected_pack의 verified_prompt_pack exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | story_diagnose에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | story_diagnose에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| story_diagnosis | schemas/artifacts/story_diagnosis.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| subworkflow_nodes_complete | execution_receipt | True | provider_receipt | geultu.text.v2.T12.subworkflow_nodes_complete | output | step_00과 human_step0_feedback가 계약대로 완료된다. |
| diagnosis_schema_valid | schema | True | execution_receipt | geultu.text.v2.T12.diagnosis_schema_valid | output | 강점·약점·질문·causal risk가 존재한다. |
| feedback_bound | execution_receipt | True | execution_receipt | geultu.text.v2.T12.feedback_bound | output | 사람 피드백이 있으면 exact digest로 다음 입력에 결합된다. ScriptForge subworkflow receipt가 내부 사람 결정을 exact gate source와 함께 포함한다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-005`
- `GT-CFT-007`
- `GT-PRV-001`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-PRV-004`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T13 |
| REVISION | T12 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

### Subworkflow

- `step_00`
- `human_step0_feedback`

## T13 — dramatic_structure

막 구조·러닝타임·비트시트를 설계하고 독립 Critic과 사람에게 확정받는다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:analyst+critic |
| semantic_authority | craft_pack.scriptforge |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| diagnosis | story_diagnosis | T12 | False | True | dramatic_structure에서 T12의 story_diagnosis exact revision을 필수로 소비한다. |
| genre_contract | genre_contract | T09 | False | True | dramatic_structure에서 T09의 genre_contract exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | dramatic_structure에서 T02의 compiled_canon exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| dramatic_structure | schemas/artifacts/dramatic_structure.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| subworkflow_nodes_complete | execution_receipt | True | provider_receipt | geultu.text.v2.T13.subworkflow_nodes_complete | output | step_01, step_02, checkpoint_a_review가 완료된다. |
| checkpoint_a_approved | execution_receipt | True | execution_receipt | geultu.text.v2.T13.checkpoint_a_approved | output | checkpoint A exact review revision이 승인된다. ScriptForge subworkflow receipt가 내부 사람 결정을 exact gate source와 함께 포함한다. |
| causal_spine_complete | review | True | dramatic_structure | geultu.text.v2.T13.causal_spine_complete | output | 막 전환과 핵심 beat 인과가 닫힌다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-005`
- `GT-CFT-006`
- `GT-CFT-007`
- `GT-PRV-002`
- `GT-QLT-001`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T14 |
| REVISION | T13 |
| REJECT | terminal:rejected |
| PIVOT | terminal pivoted |

### Subworkflow

- `step_01`
- `step_02`
- `checkpoint_a_review`
- `checkpoint_a`

## T14 — image_character_world_design

이미지 시스템·인물·세계·setup/payoff를 함께 설계하고 checkpoint B에서 확정한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:writer+critic |
| semantic_authority | craft_pack.scriptforge |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| structure | dramatic_structure | T13 | False | True | image_character_world_design에서 T13의 dramatic_structure exact revision을 필수로 소비한다. |
| diagnosis | story_diagnosis | T12 | False | True | image_character_world_design에서 T12의 story_diagnosis exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | image_character_world_design에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | image_character_world_design에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| narrative_design | schemas/artifacts/narrative_design.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| subworkflow_nodes_complete | execution_receipt | True | provider_receipt | geultu.text.v2.T14.subworkflow_nodes_complete | output | step_03, step_04, checkpoint_b_review가 완료된다. |
| checkpoint_b_approved | execution_receipt | True | execution_receipt | geultu.text.v2.T14.checkpoint_b_approved | output | checkpoint B가 exact review에 대해 승인된다. ScriptForge subworkflow receipt가 내부 사람 결정을 exact gate source와 함께 포함한다. |
| design_consistent | review | True | narrative_design | geultu.text.v2.T14.design_consistent | output | 인물 변화·공간·이미지 모티프가 구조와 모순되지 않는다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-005`
- `GT-CFT-006`
- `GT-PRV-002`
- `GT-QLT-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T15 |
| REVISION | T14 |
| REJECT | terminal:rejected |
| PIVOT | terminal pivoted |

### Subworkflow

- `step_03`
- `step_04`
- `checkpoint_b_review`
- `checkpoint_b`

## T15 — treatment_scene_plan

트리트먼트와 씬 리스트를 생성하고 집필 전 checkpoint C를 통과한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:writer+critic |
| semantic_authority | craft_pack.scriptforge |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| design | narrative_design | T14 | False | True | treatment_scene_plan에서 T14의 narrative_design exact revision을 필수로 소비한다. |
| structure | dramatic_structure | T13 | False | True | treatment_scene_plan에서 T13의 dramatic_structure exact revision을 필수로 소비한다. |
| genre_contract | genre_contract | T09 | False | True | treatment_scene_plan에서 T09의 genre_contract exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | treatment_scene_plan에서 T02의 compiled_canon exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| treatment_scene_plan | schemas/artifacts/treatment_scene_plan.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| subworkflow_nodes_complete | execution_receipt | True | provider_receipt | geultu.text.v2.T15.subworkflow_nodes_complete | output | step_05, step_06, checkpoint_c_review가 완료된다. |
| checkpoint_c_approved | execution_receipt | True | execution_receipt | geultu.text.v2.T15.checkpoint_c_approved | output | checkpoint C가 승인된다. ScriptForge subworkflow receipt가 내부 사람 결정을 exact gate source와 함께 포함한다. |
| scene_coverage_complete | review | True | treatment_scene_plan | geultu.text.v2.T15.scene_coverage_complete | output | treatment의 사건·beat가 scene list에 매핑된다. |
| continuity_ledger_complete | review | True | treatment_scene_plan | geultu.text.v2.T15.continuity_ledger_complete | output | 인물·시간·공간·소품 상태가 추적된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-005`
- `GT-CFT-006`
- `GT-PRV-002`
- `GT-QLT-001`
- `GT-QLT-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T16 |
| REVISION | T15 |
| REJECT | terminal:rejected |
| PIVOT | terminal pivoted |

### Subworkflow

- `step_05`
- `step_06`
- `checkpoint_c_review`
- `checkpoint_c`

## T16 — draft_plan_freeze

승인된 구조·scene list·StyleMaster와 unit order를 immutable plan으로 고정한다.

| 항목 | 값 |
|---|---|
| execution_class | deterministic_validation |
| executor | geultu.text.plan_freezer |
| semantic_authority | geultu_state_policy |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| scene_plan | treatment_scene_plan | T15 | False | True | draft_plan_freeze에서 T15의 treatment_scene_plan exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | draft_plan_freeze에서 T07의 approved_style exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | draft_plan_freeze에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| genre_contract | genre_contract | T09 | False | True | draft_plan_freeze에서 T09의 genre_contract exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| frozen_draft_plan | schemas/artifacts/frozen_draft_plan.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| input_refs_exact | digest | True | execution_receipt | geultu.text.v2.T16.input_refs_exact | output | 모든 upstream ref가 exact revision/digest다. |
| unit_order_total | validator | True | execution_receipt | geultu.text.v2.T16.unit_order_total | output | unit/act 순서가 누락·중복 없이 완전하다. |
| plan_digest_frozen | digest | True | execution_receipt | geultu.text.v2.T16.plan_digest_frozen | output | plan digest가 실행 key에 포함된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-003`
- `GT-CTR-004`
- `GT-CTR-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T17 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T17 — unit_drafting

동결 plan에 따라 막/단위별 초고를 쓰고 각 act human review를 통과한 뒤 합본한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | craft_pack.scriptforge+approved_style |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| plan | frozen_draft_plan | T16 | False | True | unit_drafting에서 T16의 frozen_draft_plan exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | unit_drafting에서 T07의 approved_style exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | unit_drafting에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| scene_plan | treatment_scene_plan | T15 | False | True | unit_drafting에서 T15의 treatment_scene_plan exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| unit_draft_set | schemas/artifacts/unit_draft_set.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| all_act_nodes_complete | execution_receipt | True | provider_receipt | geultu.text.v2.T17.all_act_nodes_complete | output | act1, act2a, act2b, act3와 full seam review가 완료된다. |
| all_act_reviews_approved | execution_receipt | True | execution_receipt | geultu.text.v2.T17.all_act_reviews_approved | output | 4개 act review가 exact draft revision을 승인한다. ScriptForge subworkflow receipt가 내부 사람 결정을 exact gate source와 함께 포함한다. |
| unit_order_preserved | validator | True | execution_receipt | geultu.text.v2.T17.unit_order_preserved | output | 합본이 동결 unit order와 일치한다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-005`
- `GT-CFT-006`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-QLT-001`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T18 |
| REVISION | T17 |
| REJECT | terminal:rejected |
| PIVOT | terminal pivoted |

### Subworkflow

- `step_07_act1`
- `step_07_act1_review`
- `step_07_act2a`
- `step_07_act2a_review`
- `step_07_act2b`
- `step_07_act2b_review`
- `step_07_act3`
- `step_07_act3_review`
- `step_07_full`

## T18 — craft_critique_and_revision

선택된 craft adapter의 독립 review와 targeted revision을 완료하고 Geultu review 경계로 exact draft를 인계한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:critic+writer |
| semantic_authority | selected_craft_adapter |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| drafts | unit_draft_set | T17 | False | True | craft_critique_and_revision에서 T17의 unit_draft_set exact revision을 필수로 소비한다. |
| design | narrative_design | T14 | False | True | craft_critique_and_revision에서 T14의 narrative_design exact revision을 필수로 소비한다. |
| scene_plan | treatment_scene_plan | T15 | False | True | craft_critique_and_revision에서 T15의 treatment_scene_plan exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | craft_critique_and_revision에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| craft_reviewed_draft | schemas/artifacts/craft_reviewed_draft.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| diagnosis_before_apply | execution_receipt | True | provider_receipt | geultu.text.v2.T18.diagnosis_before_apply | output | step_08_diagnosis가 apply보다 먼저 완료된다. |
| revision_applied | execution_receipt | True | provider_receipt | geultu.text.v2.T18.revision_applied | output | step_08_apply 결과가 final screenplay artifact다. |
| revision_summary_complete | review | True | craft_reviewed_draft | geultu.text.v2.T18.revision_summary_complete | output | step_08_summary가 변경·잔여 위험을 기록한다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CFT-007`
- `GT-PRV-002`
- `GT-PRV-003`
- `GT-QLT-001`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T19 |
| REVISION | T18 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

### Subworkflow

- `step_08_diagnosis`
- `step_08_apply`
- `step_08_summary`

## T19 — structural_review

CraftPack과 독립된 Geultu 구조 검토로 인과·장면 기능·setup/payoff를 평가한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_review |
| executor | geultu.quality.structural_reviewer |
| semantic_authority | geultu_quality_policy |
| artifact_owner | geultu.quality |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | structural_review에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| structure | dramatic_structure | T13 | False | True | structural_review에서 T13의 dramatic_structure exact revision을 필수로 소비한다. |
| scene_plan | treatment_scene_plan | T15 | False | True | structural_review에서 T15의 treatment_scene_plan exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| structural_review | schemas/artifacts/structural_review.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| review_coverage_complete | review | True | structural_review | geultu.text.v2.T19.review_coverage_complete | output | 모든 unit/scene이 검토됐다. |
| findings_locatable | schema | True | execution_receipt | geultu.text.v2.T19.findings_locatable | output | finding이 scene/span과 requirement에 연결된다. |
| independent_reviewer | execution_receipt | True | execution_receipt | geultu.text.v2.T19.independent_reviewer | output | Writer와 다른 reviewer/provider 또는 사람 검토다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-008`
- `GT-QLT-001`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T20 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T20 — semantic_and_canon_review

원고의 사실·시간·인물 상태·사건 인과를 canon과 대조한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_review |
| executor | geultu.quality.semantic_reviewer |
| semantic_authority | geultu_canon_policy |
| artifact_owner | geultu.quality |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | semantic_and_canon_review에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | semantic_and_canon_review에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| structural_review | structural_review | T19 | False | True | semantic_and_canon_review에서 T19의 structural_review exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| semantic_review | schemas/artifacts/semantic_review.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| canon_matrix_complete | review | True | semantic_review | geultu.text.v2.T20.canon_matrix_complete | output | canon fact와 원고 근거가 대조된다. |
| timeline_state_checked | review | True | semantic_review | geultu.text.v2.T20.timeline_state_checked | output | 시간·인물·소품 state가 검증된다. |
| findings_locatable | schema | True | execution_receipt | geultu.text.v2.T20.findings_locatable | output | 모든 finding에 근거가 있다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-QLT-002`
- `GT-CTR-008`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T21 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T21 — style_drift_review

승인된 글투의 강도·내용 보존·유창성을 독립 축으로 평가한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_review |
| executor | geultu.quality.style_reviewer |
| semantic_authority | geultu_style_policy |
| artifact_owner | geultu.quality |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | style_drift_review에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | style_drift_review에서 T07의 approved_style exact revision을 필수로 소비한다. |
| semantic_review | semantic_review | T20 | False | True | style_drift_review에서 T20의 semantic_review exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| style_review | schemas/artifacts/style_review.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| three_axes_reported | review | True | style_review | geultu.text.v2.T21.three_axes_reported | output | style strength, content preservation, fluency가 분리된다. |
| role_scope_respected | validator | True | execution_receipt | geultu.text.v2.T21.role_scope_respected | output | 분석/critic 문서가 작품 문체로 오염되지 않는다. |
| drift_findings_locatable | schema | True | execution_receipt | geultu.text.v2.T21.drift_findings_locatable | output | drift가 span/unit에 연결된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-STY-004`
- `GT-CTR-008`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T22 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T22 — locale_refinement_diagnose

선택 locale pack의 원문 규칙으로 어색함을 진단하되 텍스트를 직접 수정하지 않는다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_review |
| executor | geultu.text.locale_refinement |
| semantic_authority | geultu_refinement_policy |
| artifact_owner | geultu.refinement |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | locale_refinement_diagnose에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| structural | structural_review | T19 | False | True | locale_refinement_diagnose에서 T19의 structural_review exact revision을 필수로 소비한다. |
| semantic | semantic_review | T20 | False | True | locale_refinement_diagnose에서 T20의 semantic_review exact revision을 필수로 소비한다. |
| style | style_review | T21 | False | True | locale_refinement_diagnose에서 T21의 style_review exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| refinement_diagnosis | schemas/artifacts/refinement_diagnosis.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| locale_policy_selected | validator | True | execution_receipt | geultu.text.v2.T22.locale_policy_selected | output | 입력 locale과 장르에 맞는 policy다. |
| protected_spans_complete | validator | True | execution_receipt | geultu.text.v2.T22.protected_spans_complete | output | 고유명사·수치·날짜·인용·법문·code가 보호된다. |
| diagnosis_only | validator | True | execution_receipt | geultu.text.v2.T22.diagnosis_only | output | 이 단계는 text를 변경하지 않는다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-005`
- `GT-REF-004`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T23 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T23 — polish_patch_plan

진단을 source-digest-bound PolishPatch로 계획한다. 전체 재작성과 번역은 금지한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | geultu.text.locale_refinement |
| semantic_authority | geultu_refinement_policy |
| artifact_owner | geultu.refinement |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | polish_patch_plan에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| diagnosis | refinement_diagnosis | T22 | False | True | polish_patch_plan에서 T22의 refinement_diagnosis exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | polish_patch_plan에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| polish_patch | schemas/artifacts/polish_patch.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| source_digest_bound | digest | True | execution_receipt | geultu.text.v2.T23.source_digest_bound | output | patch가 정확한 draft digest에 결합된다. |
| operations_minimal | review | True | polish_patch | geultu.text.v2.T23.operations_minimal | output | 진단 근거 없는 구간을 변경하지 않는다. |
| ranges_nonoverlap | validator | True | execution_receipt | geultu.text.v2.T23.ranges_nonoverlap | output | UTF-8 range가 정렬·비중첩이다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-REF-001`
- `GT-REF-002`
- `GT-REF-003`
- `GT-REF-004`
- `GT-PRV-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T24 |
| REVISION | T22 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T24 — semantic_guard

patch 적용 전 의미·보호 구간·장르 형식의 불변성을 검증한다.

| 항목 | 값 |
|---|---|
| execution_class | deterministic_validation |
| executor | geultu.refinement.semantic_guard |
| semantic_authority | geultu_canon_policy |
| artifact_owner | geultu.refinement |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | semantic_guard에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| patch | polish_patch | T23 | False | True | semantic_guard에서 T23의 polish_patch exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | semantic_guard에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| genre_contract | genre_contract | T09 | False | True | semantic_guard에서 T09의 genre_contract exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| semantic_guard_report | schemas/artifacts/semantic_guard_report.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| source_digest_matches | digest | True | execution_receipt | geultu.text.v2.T24.source_digest_matches | output | patch source digest와 draft가 일치한다. |
| protected_spans_untouched | validator | True | execution_receipt | geultu.text.v2.T24.protected_spans_untouched | output | 보호 구간을 건드리지 않는다. |
| meaning_preserved | semantic_check | True | semantic_guard_report | geultu.text.v2.T24.meaning_preserved | output | 사실·주장·사건·시점·결말 의무가 보존된다. |
| format_preserved | validator | True | execution_receipt | geultu.text.v2.T24.format_preserved | output | 장르 형식을 깨지 않는다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-REF-001`
- `GT-REF-002`
- `GT-REF-003`
- `GT-REF-004`
- `GT-REF-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T25 |
| REVISION | T23 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T25 — polish_patch_apply

검증된 patch를 단일 순회로 적용하고 새 digest를 만든다.

| 항목 | 값 |
|---|---|
| execution_class | pure_transform |
| executor | geultu.refinement.patch_applier |
| semantic_authority | geultu_refinement_policy |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | craft_reviewed_draft | T18 | False | True | polish_patch_apply에서 T18의 craft_reviewed_draft exact revision을 필수로 소비한다. |
| patch | polish_patch | T23 | False | True | polish_patch_apply에서 T23의 polish_patch exact revision을 필수로 소비한다. |
| guard | semantic_guard_report | T24 | False | True | polish_patch_apply에서 T24의 semantic_guard_report exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| polished_draft | schemas/artifacts/polished_draft.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| guard_passed | validator | True | execution_receipt | geultu.text.v2.T25.guard_passed | output | SemanticGuard verdict가 pass다. |
| apply_single_pass | execution_receipt | True | execution_receipt | geultu.text.v2.T25.apply_single_pass | output | 정렬된 patch를 O(N+K)로 적용한다. |
| output_digest_recorded | digest | True | execution_receipt | geultu.text.v2.T25.output_digest_recorded | output | 출력 bytes SHA-256가 기록된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-REF-001`
- `GT-REF-002`
- `GT-REF-003`
- `GT-REF-004`
- `GT-REF-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T26 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T26 — author_revision_review

작가/감독 관점에서 전체 작품을 다시 읽고 accept·revise·pivot·reject를 결정한다.

| 항목 | 값 |
|---|---|
| execution_class | human_gate |
| executor | human.author_or_director |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.quality |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| draft | polished_draft | T25 | False | True | author_revision_review에서 T25의 polished_draft exact revision을 필수로 소비한다. |
| structural | structural_review | T19 | False | True | author_revision_review에서 T19의 structural_review exact revision을 필수로 소비한다. |
| semantic | semantic_review | T20 | False | True | author_revision_review에서 T20의 semantic_review exact revision을 필수로 소비한다. |
| style | style_review | T21 | False | True | author_revision_review에서 T21의 style_review exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| author_review | schemas/artifacts/author_review.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| whole_work_reviewed | human_evidence | True | approval_decision | geultu.text.v2.T26.whole_work_reviewed | input:draft | 부분 문장뿐 아니라 전체 흐름을 검토했다. |
| recommendation_explicit | schema | True | execution_receipt | geultu.text.v2.T26.recommendation_explicit | output | accept/revise/pivot/reject 중 하나다. |
| blocking_findings_explicit | review | True | author_review | geultu.text.v2.T26.blocking_findings_explicit | output | blocking과 nonblocking이 분리된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-CTR-008`
- `GT-QLT-001`
- `GT-QLT-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T31 |
| REVISION | T27 |
| REJECT | terminal:rejected |
| PIVOT | T30 |

## T27 — revision_directive

blocking finding을 최소한의 구체적 수정 명령과 acceptance test로 변환한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_validation |
| executor | geultu.refinement.directive_compiler |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.refinement |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| review | author_review | T26 | False | True | revision_directive에서 T26의 author_review exact revision을 필수로 소비한다. |
| recheck | revision_recheck | prior:T29 | True | True | revision_directive에서 T29의 revision_recheck exact revision을 선택적으로 소비한다. |
| current_draft | polished_draft | T25 | True | True | revision_directive에서 T25의 polished_draft exact revision을 선택적으로 소비한다. |
| previous_revision | revised_draft | prior:T28 | True | True | revision_directive에서 T28의 revised_draft exact revision을 선택적으로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| revision_directive | schemas/artifacts/revision_directive.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| targets_bounded | validator | True | execution_receipt | geultu.text.v2.T27.targets_bounded | output | scene/unit/span target가 유한하다. |
| preserve_change_separated | validator | True | execution_receipt | geultu.text.v2.T27.preserve_change_separated | output | 보존할 것과 바꿀 것이 분리된다. |
| acceptance_tests_explicit | validator | True | execution_receipt | geultu.text.v2.T27.acceptance_tests_explicit | output | 각 target에 확인 방법이 있다. |
| budget_available | validator | True | execution_receipt | geultu.text.v2.T27.budget_available | output | revision budget가 남아 있다. |

### 입력 group 규칙

- {"description": "현재 실행이 사용하는 원고 revision은 두 후보 중 정확히 하나다.", "id": "exactly_one_current_draft", "rule": "exactly_one", "slots": ["current_draft", "previous_revision"]}
- {"description": "이전 revised_draft를 사용할 때는 해당 revision_recheck가 반드시 존재한다.", "id": "revision_requires_recheck", "rule": "requires", "slots": ["previous_revision", "recheck"]}

### 실패 code

- `GT-REF-006`
- `GT-REF-007`
- `GT-CTR-005`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T28 |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | T30 |

## T28 — targeted_revision_apply

CraftPack의 제작 지식과 승인 StyleMaster 아래에서 directive만 적용한다.

| 항목 | 값 |
|---|---|
| execution_class | model_effect |
| executor | selected_provider:writer |
| semantic_authority | craft_pack+approved_style |
| artifact_owner | geultu.text |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| directive | revision_directive | T27 | False | True | targeted_revision_apply에서 T27의 revision_directive exact revision을 필수로 소비한다. |
| draft | polished_draft | T25 | True | True | targeted_revision_apply에서 T25의 polished_draft exact revision을 선택적으로 소비한다. |
| previous_revision | revised_draft | prior:T28 | True | True | targeted_revision_apply에서 T28의 revised_draft exact revision을 선택적으로 소비한다. |
| style | approved_style | T07 | False | True | targeted_revision_apply에서 T07의 approved_style exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | targeted_revision_apply에서 T02의 compiled_canon exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| revised_draft | schemas/artifacts/revised_draft.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| directive_scope_only | review | True | revised_draft | geultu.text.v2.T28.directive_scope_only | output | 지시 밖의 기능·내용을 변경하지 않는다. |
| lineage_preserved | provenance | True | execution_receipt | geultu.text.v2.T28.lineage_preserved | output | 이전 revision과 directive가 provenance에 있다. |
| digest_changed | digest | True | execution_receipt | geultu.text.v2.T28.digest_changed | output | 실제 수정이면 digest가 달라진다. |

### 입력 group 규칙

- {"description": "현재 실행이 사용하는 원고 revision은 두 후보 중 정확히 하나다.", "id": "exactly_one_current_draft", "rule": "exactly_one", "slots": ["draft", "previous_revision"]}

### 실패 code

- `GT-PRV-002`
- `GT-PRV-003`
- `GT-REF-006`
- `GT-REF-007`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T29 |
| REVISION | T27 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |

## T29 — revision_recheck

acceptance test·회귀·잔여 finding을 독립적으로 재검사한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_review |
| executor | geultu.quality.revision_reviewer |
| semantic_authority | geultu_quality_policy |
| artifact_owner | geultu.quality |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| directive | revision_directive | T27 | False | True | revision_recheck에서 T27의 revision_directive exact revision을 필수로 소비한다. |
| draft | revised_draft | T28 | False | True | revision_recheck에서 T28의 revised_draft exact revision을 필수로 소비한다. |
| canon | compiled_canon | T02 | False | True | revision_recheck에서 T02의 compiled_canon exact revision을 필수로 소비한다. |
| style | approved_style | T07 | False | True | revision_recheck에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| revision_recheck | schemas/artifacts/revision_recheck.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| all_tests_evaluated | review | True | revision_recheck | geultu.text.v2.T29.all_tests_evaluated | output | directive의 모든 acceptance test 결과가 있다. |
| regression_scan_complete | review | True | revision_recheck | geultu.text.v2.T29.regression_scan_complete | output | 구조·canon·style 회귀가 검사된다. |
| recommendation_explicit | schema | True | execution_receipt | geultu.text.v2.T29.recommendation_explicit | output | accept/revise/pivot/reject가 명시된다. |

### 입력 group 규칙

- 없음

### 실패 code

- `GT-QLT-001`
- `GT-QLT-002`
- `GT-STY-004`
- `GT-REF-006`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T30 |
| REVISION | T27 |
| REJECT | terminal:rejected |
| PIVOT | T30 |

## T30 — revision_disposition_gate

재검토된 exact draft revision을 수락·재수정·거절·피벗 중 하나로 결정한다.

| 항목 | 값 |
|---|---|
| execution_class | human_gate |
| executor | human.product_owner |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.approval |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| review | revision_recheck | T29 | True | True | pivot_gate에서 T29의 revision_recheck exact revision을 선택적으로 소비한다. |
| author_review | author_review | T26 | False | True | pivot_gate에서 T26의 author_review exact revision을 필수로 소비한다. |
| current_draft | revised_draft | T28 | True | True | pivot_gate에서 T28의 revised_draft exact revision을 선택적으로 소비한다. |
| polished_draft | polished_draft | T25 | True | True | pivot_gate에서 T25의 polished_draft exact revision을 선택적으로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| approval_decision | schemas/artifacts/approval_decision.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| decision_exact_subject | human_decision | True | approval_decision | geultu.text.v2.T30.decision_exact_subject | one_of_inputs:current_draft,polished_draft | 결정이 현재 draft exact ref에 결합된다. |
| preserved_superseded_explicit | provenance | True | execution_receipt | geultu.text.v2.T30.preserved_superseded_explicit | output | 보존/폐기 artifact가 명시된다. |
| no_in_place_rewind | validator | True | execution_receipt | geultu.text.v2.T30.no_in_place_rewind | output | pivot은 현재 run을 terminal 처리하고 새 run으로 시작한다. |

### 입력 group 규칙

- {"description": "현재 실행이 사용하는 원고 revision은 두 후보 중 정확히 하나다.", "id": "exactly_one_current_draft", "rule": "exactly_one", "slots": ["current_draft", "polished_draft"]}

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-004`
- `GT-REF-007`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T31 |
| REVISION | 금지 |
| REJECT | terminal:rejected |
| PIVOT | terminal:pivoted |

## T31 — genre_completion

장르 계약의 모든 요구를 작품 근거와 함께 판정한다.

| 항목 | 값 |
|---|---|
| execution_class | hybrid_validation |
| executor | geultu.quality.genre_completion |
| semantic_authority | geultu_genre_policy |
| artifact_owner | geultu.quality |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| genre_contract | genre_contract | T09 | False | True | genre_completion에서 T09의 genre_contract exact revision을 필수로 소비한다. |
| draft | revised_draft | T28 | True | True | genre_completion에서 T28의 revised_draft exact revision을 선택적으로 소비한다. |
| polished_draft | polished_draft | T25 | True | True | genre_completion에서 T25의 polished_draft exact revision을 선택적으로 소비한다. |
| reviews | revision_recheck | T29 | True | True | genre_completion에서 T29의 revision_recheck exact revision을 선택적으로 소비한다. |
| revision_disposition | approval_decision | T30 | True | True | revised_draft 경로가 사람의 accept 결정을 통과했음을 증명한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| completion_report | schemas/artifacts/completion_report.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| every_requirement_evaluated | review | True | completion_report | geultu.text.v2.T31.every_requirement_evaluated | output | 모든 completion requirement에 pass/fail/evidence가 있다. |
| blocking_findings_zero | review | True | completion_report | geultu.text.v2.T31.blocking_findings_zero | output | blocking finding이 0개다. |
| output_format_valid | validator | True | execution_receipt | geultu.text.v2.T31.output_format_valid | output | 장르 형식과 길이 계약이 유효하다. |

### 입력 group 규칙

- {"description": "현재 실행이 사용하는 원고 revision은 두 후보 중 정확히 하나다.", "id": "exactly_one_current_draft", "rule": "exactly_one", "slots": ["draft", "polished_draft"]}
- {"description": "revised_draft를 완료 검사하면 대응 revision_recheck가 필수다.", "id": "revised_draft_requires_review", "rule": "requires", "slots": ["draft", "reviews"]}
- {"description": "revised_draft를 완료 검사하면 T30 사람 수락 결정이 필수다.", "id": "revised_draft_requires_disposition", "rule": "requires", "slots": ["draft", "revision_disposition"]}

### 실패 code

- `GT-QLT-003`
- `GT-QLT-001`
- `GT-QLT-002`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T32 |
| REVISION | T27 |
| REJECT | terminal:rejected |
| PIVOT | T30 |

## T32 — text_master_approve

사람이 completion evidence가 있는 exact 최종 원고를 TextMaster로 승인한다. Front-only 실행은 여기서 완료된다.

| 항목 | 값 |
|---|---|
| execution_class | human_gate |
| executor | human.text_owner |
| semantic_authority | human_creative_decision |
| artifact_owner | geultu.state |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| completion | completion_report | T31 | False | True | text_master_approve에서 T31의 completion_report exact revision을 필수로 소비한다. |
| draft | revised_draft | T28 | True | True | text_master_approve에서 T28의 revised_draft exact revision을 선택적으로 소비한다. |
| polished_draft | polished_draft | T25 | True | True | text_master_approve에서 T25의 polished_draft exact revision을 선택적으로 소비한다. |
| style | approved_style | T07 | False | True | text_master_approve에서 T07의 approved_style exact revision을 필수로 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| text_master | schemas/artifacts/text_master.v2.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| completion_passed | validator | True | execution_receipt | geultu.text.v2.T32.completion_passed | input:completion | CompletionReport verdict가 pass다. |
| exact_text_approved | human_decision | True | approval_decision | geultu.text.v2.T32.exact_text_approved | one_of_inputs:draft,polished_draft | 승인 subject가 최종 text exact ref다. |
| evidence_present | human_evidence | True | approval_decision | geultu.text.v2.T32.evidence_present | one_of_inputs:draft,polished_draft | completion report와 주요 review가 evidence다. |

### 입력 group 규칙

- {"description": "현재 실행이 사용하는 원고 revision은 두 후보 중 정확히 하나다.", "id": "exactly_one_current_draft", "rule": "exactly_one", "slots": ["draft", "polished_draft"]}

### 실패 code

- `GT-APR-001`
- `GT-APR-002`
- `GT-APR-003`
- `GT-APR-004`
- `GT-QLT-003`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | T33 |
| REVISION | T27 |
| REJECT | terminal:rejected |
| PIVOT | T30 |

## T33 — geultu_story_package_export

TextMaster를 내용 변경 없이 Geultu가 소유하는 유일한 canonical flat `GeultuStoryPackage v1`로 변환한다. T33의 schema는 `contracts/geultu-story-package.schema.json` 하나이며 별도 envelope TARGET shape은 없다. NVC의 `NarrativePackage`는 NarrativeCourier에서 별도로 발행되며 이 저장소의 출력이 아니다.

| 항목 | 값 |
|---|---|
| execution_class | projection |
| executor | geultu.handoff.exporter |
| semantic_authority | geultu_text_authority |
| artifact_owner | geultu.handoff |

### 정확한 입력

| slot | artifact kind | source | optional | exact revision | purpose |
|---|---|---|---|---|---|
| text_master | text_master | T32 | False | True | T32에서 승인된 exact TextMaster를 소비한다. |

### 정확한 출력

| artifact kind | schema |
|---|---|
| geultu_story_package | contracts/geultu-story-package.schema.json |

### 통과 조건

| condition | evidence | blocking | evidence kind | validator | subject | description |
|---|---|---|---|---|---|---|
| text_master_exact | digest | True | execution_receipt | geultu.text.v2.T33.text_master_exact | output | canonical narrative content와 source TextMaster content가 byte-identical하다. |
| single_canonical_authority | schema | True | execution_receipt | geultu.text.v2.T33.single_canonical_authority | output | GeultuStoryPackage에는 정확히 하나의 canonical narrative authority가 존재한다. |
| geultu_story_schema_valid | schema | True | execution_receipt | geultu.text.v2.T33.geultu_story_schema_valid | output | geultu-story-package.schema.json을 통과한다. |

### 입력 group 규칙

- 없음

### 실패 code

- `HOF_001_TEXT_MASTER_INVALID`
- `HOF_002_NARRATIVE_PACKAGE_INVALID`
- `ART_003_DIGEST_MISMATCH`

### 다음 단계 진입

| 결과 | 다음 |
|---|---|
| PASS | terminal:geultu_story_package_complete |
| REVISION | 금지 |
| REJECT | terminal rejected |
| PIVOT | terminal pivoted |
