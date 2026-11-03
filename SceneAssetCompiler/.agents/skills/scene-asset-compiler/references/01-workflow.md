# End-to-End Workflow

## Phase 0 — Intake
Create `RunRequest`. Import a released NarrativeCourier scene projection when supplied and verify the fixed receipt plus every copied graph SHA-256 before using it.

## Phase 1 — Visual Authority
Codex reads narrative authority and user direction, then writes `authority/visual-authority.json`. Character identity and style become explicit invariants. Do not put provider/model details here.

## Phase 2 — Asset Planning
Codex writes `plan/generation-plan.json`. Each task has one asset responsibility, split profile, exact semantic layer contract, references, acceptance criteria, and dependency IDs. Recurring characters require `CHARACTER_ANCHOR` unless the user supplied an approved identity reference.

## Phase 3 — Compile
`compile-plan` validates IDs/dependencies/cycles and creates immutable `task.json` files.

## Phase 4 — Ready Queue
`ready` returns only tasks whose dependencies have approved selections. Dependencies only gate readiness. Visual inputs are separate explicit references; `TASK_OUTPUT` references resolve only approved PRIMARY selections. Independent ready tasks may be processed in any order, with one selected executor call per asset/variant.

## Phase 5 — Attempt
`begin` creates immutable `attempts/NNNN/` with `attempt.json`, `prompt.md`, and empty `outputs/`. Select the Scene Provider only for reference-free PNG `GENERATE`; otherwise use `$imagegen` with only `attempt.json.resolved_references`. Never overwrite an earlier attempt.

## Phase 6 — Execution receipt and mechanical QA
For `$imagegen`, after files are placed in `outputs/`, run `record-execution`, `manifest`, and `review` with observed metadata. For the Scene Provider, `execute-scene-provider` writes its dispatch journal, receipt, result, and mechanical review itself; do not repeat those commands. Literal `UNKNOWN` is valid only when the outcome cannot be established, and the same dispatched attempt must never be retried. Mechanical QA never claims semantic quality.

## Phase 7 — Visual QA
`visual-template` creates the rubric. Codex visually inspects actual outputs against identity/style/narrative/composition/lighting-color/material/photorealism/structure/articulation/usability criteria, records evidence, and sets APPROVED/REJECTED. `photorealism` may be `NA` only outside the `PHOTOREAL_*` profiles.

## Phase 8 — Doctor / Decision
If both mechanical PASS and visual APPROVED: `approve` writes `selection.json`.

If rejected, first decompose input/output divergence with `audit-template` → `validate-audit` → `doctor`. Doctor chooses LOCAL / REGIONAL / GLOBAL scope. `reconstruction-plan` decides whether the next attempt should surgically edit the prior PRIMARY, regenerate a task from canonical authority, ablate references, or reseed an anchor. `reconstruct` creates the new immutable attempt. Use direct `repair` only when a single failure and its correction are already unambiguous.

## Phase 9 — Publish
`publish` requires every required task to have an approved selection. If the RunRequest enables human final review, `human-approve` must exist first. Publication verifies hashes again, copies selected artifacts to content-addressed `publish/blobs/`, and emits `asset-pack.json` plus `MANIFEST.sha256`.

## Phase 10 — SceneMotionCompiler release handoff (NVC projection only)

This is a separate release surface from the generic `publish` command. It is
used only after `init` has imported the fixed
`projection/narrative-scene-projection.v3.json` from a released
`SMC_NARRATIVE_SCENE_PROJECTION` source produced by SMC from a NarrativeCourier Completed release. It never reparses screenplay or
synopsis text.

`scene-release prepare <run-dir> <request.json>` accepts a
`SceneAssetReleaseRequest v1`. The request must contain `createdAt`, explicit
`kind`, `rights`, and `sourceRegionId` on every asset and every requirement,
plus source/runtime artifact references with raw SHA-256, byte size, MIME, and
project-relative paths. Requirement IDs are derived from the imported graph:
`CHARACTER_SPRITE:<character-id>`, `ENVIRONMENT_PLATE:<environment-id>`,
`PROP_CUTOUT:<prop-id>`, and `SHOT_KEYFRAME:<shot-id>`. One asset may be
referenced by several unique requirement IDs when its declared metadata is
identical for each binding.

The producer validates the declared MIME against the lower-case path suffix
and the observed bytes before it creates release artifacts. Runtime derivatives
must have real PNG/JPEG/WebP signatures. Source artifacts may be those raster
formats or PSD/PSB; `image/vnd.adobe.photoshop` is accepted only for a 26-byte
`8BPS` header with PSD/PSB version and valid dimensions/depth. A filename or
MIME declaration alone is never accepted as provenance.

The producer verifies all bytes and writes these exact files:

```
handoff/asset-pack.v2.json
handoff/asset-pack-audit.v1.json
handoff/binding-manifest.v1.json
handoff/scene-asset-release-approval.v1.json
handoff/scene-motion-asset-handoff.v2.json
```

Run `scene-release approve` with a human reviewer, then `validate`, then
`publish`. The approval subject is exactly
`{sceneGraphHash, assetPackHash, assetPackAuditHash, bindingManifestHash}`.
The final handoff is the only production input accepted by
SceneMotionCompiler's `compile-scene-html` command. Runtime derivatives are
limited downstream to real PNG/JPEG/WebP bytes; PSD remains source lineage and
is never treated as an HTML runtime asset. The generic v1 `asset-pack.json`
from Phase 9 remains a standalone SceneAssetCompiler workflow and is not an SMC handoff.
