# Prompt Audit Report

## 결론

- Catalog entries: **6**
- Catalog directives: **18**
- Catalog reference model payloads: **6**
- Runtime matrix contract: **6 × 2 languages × 2 profiles = 24 cases**
- `editorial_flat` contract: **1 valid entry projections / 5 explicit rejections per language**
- Reference text contracts: **0 sample-copy / 0 intentional blank-zone / 6 no-copy**
- Concrete catalog-owned test cases: **6**
- Validator mutation proofs: **5/5 detected**

`structured`는 선택 entry의 모든 directive를 투영한다. `editorial_flat`은 C1 자체를 profile-owned 규칙으로 투영하고, editorial/look 계열의 첫 directive만 투영하며, 나머지는 렌더 전에 거절한다. 의도된 생략은 완전성 결함으로 계산하지 않는다.

## 검수 표면

| Surface | Source of truth | Leakage boundary | Completeness contract | Structure boundary |
|---|---|---|---|---|
| Structured image compiler | catalog + typed request | no ID/path/hash/directive key/intent | every selected directive | semantic slot labels only |
| Editorial-flat compiler | catalog + typed request | no raw slug or Korean compiler label in English | profile-owned projection | unsupported families fail closed |
| Codex image adapter | compiled prompt + output parameters | JSON data boundary | one prompt + all parameters | fixed execution contract owns headings |
| Catalog reference generator | catalog directives | metadata outside payload | every directive exactly once | fenced payload only |

## Mutation proof

| Defect class | Deliberate mutation | Detected |
|---|---|---:|
| leakage | inject entry.source_path | PASS |
| completeness | remove/duplicate a directive and remove catalog-owned audit copy | PASS |
| language | inject untranslated Hangul into English payload | PASS |
| structure | append an empty label and inject an authoring-mode directive | PASS |
| determinism | change the second render | PASS |

## Entry-by-entry projection matrix

| ID | Family | Directives | Reference text | Structured | editorial_flat | Generated payload |
|---|---|---:|---|---|---|---|
| C1 | category | 3 | none | all directives | VALID: profile-owned C1 projection | audited |
| C4 | category | 3 | none | all directives | REJECTED by profile contract | audited |
| C5 | category | 3 | none | all directives | REJECTED by profile contract | audited |
| C6 | category | 3 | none | all directives | REJECTED by profile contract | audited |
| C10 | category | 3 | none | all directives | REJECTED by profile contract | audited |
| C11 | category | 3 | none | all directives | REJECTED by profile contract | audited |

## 증거

- Runtime exhaustive test: `crates/promptgen-core/src/image/render.rs::every_catalog_entry_compiles_to_a_clean_prompt`
- Profile validation: `crates/promptgen-core/src/image/validate.rs::validate_render_profile`
- Catalog authoring-source and payload audit: `scripts/audit-prompts.py`
- Deterministic payload generator: `scripts/generate-catalog-image-prompts.py`
- Synchronized renewal gate: `scripts/renew-image-catalog.py --check`
