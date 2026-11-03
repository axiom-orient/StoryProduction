# SAC RenderContract — implementation analysis

## Owner and boundary

SceneAssetCompiler owns the optional `render_contract` semantics used by generation units,
wireframe slots, visual-review evidence, and deterministic prompt guidance. The JSON Schema at
`docs/contracts/render-contract.schema.json` remains the producer-owned structural contract;
runtime code adds cross-field and evidence-to-attempt checks. This contract does not approve a
candidate, produce pixels, or claim that visual measurements are truthful without bound evidence.

## Module ownership

| Module | Responsibility | Public status |
|---|---|---|
| `scene_asset_compilerlib/render_contract_validation.py` | Profiles, semantic role, style-reference separation, label/guide policy, layout-slot geometry and cross-field constraints | Canonical semantic validator |
| `scene_asset_compilerlib/render_layout_evidence.py` | Evidence-only label/guide states and verdicts, slot evidence shape, exact slot coverage, tolerance/focal/crop checks, optional source/output hash binding to the attempt | Evidence validator; never an approval writer |
| `scene_asset_compilerlib/render_contract_prompt.py` | `prompt_lines`: stable ordered prompt projection for validated capture/genre/role/reference/layout constraints | Pure prompt projection |
| `contract_primitives.py` | Converts RenderContract-owned exceptions to SAC `ContractError` at the consumer boundary | Shared SAC contract glue |
| `request_contracts.py`, `review_contracts.py` | Own task-level RenderContract normalization and visual-review binding respectively | Request/review owners |
| `scene_plan.py`, `scene_units.py`, `prompts.py`, `visual_review_template.py` | Import semantic validation or prompt projection from their owner modules | Direct imports; no `scene.py` or RenderContract forwarding module |

## Runtime paths

`validate_render_contract` rejects unsupported fields and invalid profile combinations, then
validates each wireframe slot's normalized bounds, aspect ratio, focal point, crop inset, z-order,
and allowed tolerances. `validate_style_reference_separation` permits style references to control
only declared style axes and rejects overlap between preserve/ignore declarations.

`validate_layout_evidence` compares each observed slot to the contract, rejects missing/duplicate
slots and false `PASS` verdicts, and checks source/output hashes against `attempt.json` and
`result.json` when the caller supplies `attempt_dir`. `require_pass` means every required evidence
verdict must be `PASS`; it does not certify an image or replace human visual review.

`prompt_lines` first validates the contract and style-reference separation, then emits stable,
ordered natural-language constraints. It does not read image files or call any provider. The
`render_contract.py` forwarding module and duplicate prompt-function aliases were removed; callers
import directly from validation, evidence, prompt, request, review, and primitive owners. The former
aggregate `contracts.py` re-export module was removed; command-line entrypoints remain unchanged.

## Current refactor status

The former 594-line module is now three responsibility owners with direct imports from consumers.
Request, review, and error-adaptation responsibilities remain with their owner modules. The aggregate
import module is gone; producer schema identity, serialized fields, CLI, and human approval boundary
did not change. This source-only clean break has not been tested, built, linted, or run through a
manifest verifier.
