> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# 시각 품질 checkpoint V00–V27

V00–V27은 NVC와 경쟁하는 별도 실행 파이프라인이 아니다. NVC S00–S12 위의 제품 품질·사람 검토 checkpoint다.

| checkpoint | 이름 | NVC stage | blocking | evidence |
|---|---|---|---|---|
| V00 | narrative_package_preflight | S00_PACKAGE_PREFLIGHT | True | True |
| V01 | visual_profile | S02_DIRECTION_SYNTHESIS | False | True |
| V02 | source_segmentation | S01_SEMANTIC_COMPILE | False | True |
| V03 | visual_beat_extraction | S03_ADAPTATION_PLAN | False | True |
| V04 | scene_mapping | S04_SCENE_PLAN | False | True |
| V05 | cut_intent | S05_SHOT_PLAN | False | True |
| V06 | layout_reading_order | S05_SHOT_PLAN | False | True |
| V07 | shot_composition | S05_SHOT_PLAN | False | True |
| V08 | asset_manifest | S06_ASSET_PLAN | False | True |
| V09 | reference_grammar | S02_DIRECTION_SYNTHESIS | False | True |
| V10 | rough_board | S05_SHOT_PLAN | False | True |
| V11 | board_review | S11_VERIFY_REPAIR | True | True |
| V12 | lineart_control | S07_ASSET_MATERIALIZE | False | True |
| V13 | lineart_generation | S09_SHOT_RENDER | False | True |
| V14 | lineart_review | S11_VERIFY_REPAIR | True | True |
| V15 | tone_plan | S08_RENDER_PLAN | False | True |
| V16 | tone_application | S09_SHOT_RENDER | False | True |
| V17 | lettering_sfx | S10_COMPOSE | False | True |
| V18 | still_composition | S10_COMPOSE | False | True |
| V19 | still_proof | S11_VERIFY_REPAIR | True | True |
| V20 | still_approval | S11_VERIFY_REPAIR | True | True |
| V21 | layer_decomposition | S07_ASSET_MATERIALIZE | False | True |
| V22 | occlusion_repair | S07_ASSET_MATERIALIZE | False | True |
| V23 | layer_validation | S11_VERIFY_REPAIR | True | True |
| V24 | camera_motion | S08_RENDER_PLAN | False | True |
| V25 | web_projection | S10_COMPOSE | False | True |
| V26 | accessibility_runtime_proof | S11_VERIFY_REPAIR | True | True |
| V27 | visual_master_approval | S12_PACKAGE_MASTER | True | True |

## 규칙

- 실행 순서는 NVC stage DAG가 소유한다.
- blocking checkpoint는 evidence 없이 PASS할 수 없다.
- V20 still approval과 V27 visual master approval은 사람 결정이다.
- V11/V14/V19/V23/V26은 S11 평가·repair evidence를 소비한다.
- checkpoint mapping은 Rust source와 JSON 계약이 byte-independent parity 검사로 일치해야 한다.
