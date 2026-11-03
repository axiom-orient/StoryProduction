# SAC task and review contracts — implementation analysis

## Owner and authority

SceneAssetCompiler owns the contracts for its run request, visual authority, generation plan,
visual review, and packaging-preservation evidence. Their producer schemas remain under
`SceneAssetCompiler/docs/contracts/`; these validators add cross-field, lineage, and persisted-byte
checks where schema shape alone is insufficient. Contract validation can reject or bind evidence;
it does not approve an asset or certify a human visual judgment.

## Module ownership

| Module | Responsibility |
|---|---|
| `scene_asset_compilerlib/contract_primitives.py` | `ContractError`, required/unknown-field helpers, hash shape, and conversion from RenderContract-owned errors to SAC contract errors |
| `scene_asset_compilerlib/request_contracts.py` | RunRequest, VisualAuthority, GenerationPlan, task-level RenderContract normalization, canvas dimensions, and transparent-margin policy |
| `scene_asset_compilerlib/packaging_preservation.py` | PSD packaging metrics, comparison-receipt path/hash/lineage and attempt-output byte binding |
| `scene_asset_compilerlib/review_contracts.py` | VisualReview lineage, prompt/render snapshots, artifact hash and layout evidence checks |

CLI and workflow callers import request, review, and shared error-primitives directly from their
owner modules. RenderContract semantics, layout evidence, and prompt projection also use their
dedicated owners. The aggregate `contracts.py` re-export module is removed. The primitive module owns
only helpers and exception adaptation; it does not own a parallel workflow state.

## Caller paths

`workspace.py` validates the RunRequest before staging and validates the VisualAuthority and
GenerationPlan at their respective transitions. `prompts.py` obtains task-level render semantics
and applies style-reference separation. `repair.py`, `publish.py`, and the CLI consume visual-review
validation from `review_contracts.py`; mechanical review imports packaging-preservation validation
directly from `packaging_preservation.py`. The
review validator binds `artifact_sha256` and layout evidence to the attempt when the caller supplies
`attempt_dir`, while the publish/approval owner remains distinct.

## Current refactor status

`request_contracts.py`, `review_contracts.py` and `packaging_preservation.py` have separate owners;
the removed `contracts.py` facade is not restored. Packaging owns exact metrics, comparison receipt
path/hash/lineage and output binding. Producer schemas, error tokens, and approval lifecycle are
unchanged. Tests/build/lint/runtime/manifest verification are NOT_RUN. Manifest generation updates
metadata only.
