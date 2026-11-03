---
name: scene-asset-compiler
description: "Use for story-to-visual asset production in Codex App: turn a synopsis, screenplay, released NarrativeCourier scene projection, or visual direction into structured visual authority and generation tasks; execute reference-free PNG tasks through the typed Scene Provider v2 or delegate reference/edit/PSD work to $imagegen; mechanically and visually review returned artifacts; repair only failed assets or layers; and publish a hash-bound AssetPack after all required tasks pass. Do not use for one-off image generation that needs no reusable asset planning/QA pipeline."
---

# SceneAssetCompiler — Codex Visual Asset Compiler

SceneAssetCompiler is a **Codex skill + workflow + deterministic pipeline**. It does not generate pixels itself.

**Use only the installed `$imagegen` tool or the typed Scene Provider v2 process adapter. Never implement or call an image API from SceneAssetCompiler.**

## Product boundary

Before production work, apply `references/13-image-generation-mastery.md`. SceneAssetCompiler owns:
- narrative-to-visual interpretation boundaries,
- Character/Style visual authority,
- reusable asset inventory,
- L0–L3 split decisions,
- reference roles and approved anchor dependencies,
- generation task/prompt compilation,
- immutable attempts and result manifests,
- mechanical + visual QA,
- minimal repair planning,
- approved selections and AssetPack publication.

`$imagegen` owns reference-aware generation, editing, cutout, and PSD work. Scene Provider owns one-shot, reference-free PNG text-to-image execution for the headless lane.

Read `references/00-positioning.md` and `references/01-workflow.md` when starting a new run.

## Canonical CLI

From the extracted Standalone bundle root (the `pwd -P` assignments make
every executable, lock, and run path absolute):

```bash
BUNDLE_ROOT="$(pwd -P)"
PRODUCT_ROOT="$BUNDLE_ROOT/SceneAssetCompiler"
SCENE_ASSET_COMPILER="$PRODUCT_ROOT/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py"
PROVIDER_BIN="$BUNDLE_ROOT/bin/scene-provider-v2"
PERFECTPIXEL_BIN="$BUNDLE_ROOT/bin/perfectpixel"
COMPONENT_LOCK="$PRODUCT_ROOT/contracts/runtime-component-lock.json"
python3 "$SCENE_ASSET_COMPILER" <command> ...
```

The runtime CLI uses Python standard library only. Contract and release verification additionally require the `jsonschema` development dependency declared in `requirements-dev.txt`; verification fails closed when it is unavailable.

## Phase A — Intake

If no run exists:
1. Create a RunRequest from `templates/run-request.json`.
2. Keep user creative direction verbatim where it constrains output.
3. If source is an SMC durable narrative projection derived from a NarrativeCourier Completed release, use `SMC_NARRATIVE_SCENE_PROJECTION`; supply the fixed projection receipt path, project root, and receipt `sha256`.
4. Run `init`.

```bash
python3 "$SCENE_ASSET_COMPILER" init request.json .scene-assets/runs/<run-id>
```

Never bypass SHA verification for an imported NarrativeCourier projection. The receipt and staged graph are copied byte-for-byte into the run's fixed `projection/` paths.

## Phase B — Visual Authority

If `authority/visual-authority.json` is missing:
1. Read the released narrative artifacts and user references.
2. Write project style invariants and one Character identity contract per recurring character.
3. Define style as a visual system: shape, edge, shading, texture, palette, light, camera, material and detail density.
4. Build a color script when scene-to-scene light/color progression carries story meaning.
5. Keep narrative facts distinct from visual inventions.
6. Do not mention GPT/imagegen/provider fields in VisualAuthority.
7. Validate:

```bash
python3 "$SCENE_ASSET_COMPILER" validate-authority <run-dir>
```

Use `references/02-contracts.md` and `references/04-qa-rubric.md`.

## Phase C — Generation Plan

Write `plan/generation-plan.json`. Every task requires `authority_refs` and a `visual_brief`; follow `references/15-shot-grammar.md`, `references/16-color-light-material.md`, and `references/17-prompt-compiler.md`.

Plan order:
1. STYLE_ANCHOR only when a visual reference anchor materially helps.
2. CHARACTER_ANCHOR for every recurring character without an already-approved identity anchor.
3. PROP_ANCHOR for recurring signature props when consistency matters.
4. Reusable character/prop/environment assets.
5. Key visuals / motion-source images.

Every task must be independently reviewable and repairable. Do not make one giant PSD/task merely to reduce task count.

For v0.10 capture/genre/role separation and wireframe-to-placeholder-photo staging,
use the optional `render_contract` described in `references/33-render-contracts.md`.
STYLE preserve fields use that reference's safe structured allowlist. Layout scenes
must lock every placeholder and seal `PLACEHOLDER_SET_LOCK` before layer generation;
run `scene-psd-entry-check` before PSD packaging.

Use split rules in `references/06-split-profiles.md`.

Compile:

```bash
python3 "$SCENE_ASSET_COMPILER" compile-plan <run-dir>
```

Compilation rejects duplicate tasks, unknown dependencies, cycles, missing required layers, authority mismatch, missing required character/prop anchor references, and color-script mismatches.

## Phase D — Ready queue and generation

Check:

```bash
python3 "$SCENE_ASSET_COMPILER" ready <run-dir>
```

For each ready task:

```bash
python3 "$SCENE_ASSET_COMPILER" begin <run-dir> <task-id>
```

Then choose one executor lane:

### Built-in `$imagegen` lane

Use this lane for any attempt with resolved references, an edit target, cutout repair, or PSD output.

Then:
1. Read `attempt.json`. Treat `generation-capsule.md` as the **only visual request to hand to `$imagegen`**. `prompt.md` is an auditable full explanation, not the generation payload.
2. Read `attempt.json.resolved_references`. `depends_on` is only an approval/order gate; only explicit `references` become image inputs.
3. Read and follow the installed `$imagegen` skill.
4. Call `$imagegen` once for this asset/variant using `generation-capsule.md` verbatim plus only the resolved reference images/edit target. Do not include SceneAssetCompiler/Doctor/workflow commentary in the visual request. If output is PSD, `$imagegen` itself must perform cutout/layering/path creation according to the embedded PSDGenerationProfile; SceneAssetCompiler must not locally assemble or vectorize the PSD.
5. Inspect the candidate.
6. Move/copy selected project-bound result into that attempt's `outputs/`.
7. Record the real execution metadata using `record-execution`; the submitted prompt must be an attempt-local regular file with a canonical relative path and stable hash. Absolute, parent-traversing, symlinked, external, or later-modified prompt evidence is invalid. Use the literal `UNKNOWN` only for fields the execution surface did not expose. `EXECUTOR_RECORDED` is an auditable declaration, not provider proof.
8. Never overwrite a previous attempt or its execution receipt.
9. Do not fabricate PSD/layer support. Respect task fallback policy.

### Typed Scene Provider lane

Use this lane only when the attempt is reference-free `GENERATE`, the preferred output is PNG, and the task size is supported by the installed Scene Provider.

```bash
RUN_DIR="$PRODUCT_ROOT/.scene-assets/runs/<run-id>"
python3 "$SCENE_ASSET_COMPILER" execute-scene-provider "$RUN_DIR" <task-id> <attempt-no> \
  --provider-bin "$PROVIDER_BIN" \
  --perfectpixel-bin "$PERFECTPIXEL_BIN" \
  --component-lock "$COMPONENT_LOCK" \
  --quality medium \
  --dry-run

python3 "$SCENE_ASSET_COMPILER" execute-scene-provider "$RUN_DIR" <task-id> <attempt-no> \
  --provider-bin "$PROVIDER_BIN" \
  --perfectpixel-bin "$PERFECTPIXEL_BIN" \
  --component-lock "$COMPONENT_LOCK" \
  --quality medium
```

The command first writes an immutable dispatch journal, starts the provider in a dedicated process group, records its PID/PGID custody, then sends `generation-capsule.md` verbatim exactly once. It saves the returned PNG atomically, records execution, creates the result manifest, and runs mechanical review. It rejects references, edits, PSD/WebP, unsupported sizes, nonempty output directories, and already-dispatched attempts before dispatch. Timeout cleanup while the controller is alive performs a live leader/PGID check immediately before best-effort group signalling, then observes leader reap and group survival. POSIX does not make that check and signal atomic, so never describe this as exact identity-safe teardown. A controller/host crash is still fail-closed for redispatch but does not have automatic orphan reaping; do not signal a recorded numeric PID/PGID without independently proving process identity. `OUTCOME_UNKNOWN` is never retry authority; reconstruct a new attempt only after external resolution.

Use `references/03-generation-handoff.md` and `references/34-scene-provider-executor.md` for the active Scene Provider boundary.

## Phase E — Postflight

For the `$imagegen` lane, after outputs exist:

```bash
python3 "$SCENE_ASSET_COMPILER" record-execution <run-dir> <task-id> <attempt-no> execution-draft.json
python3 "$SCENE_ASSET_COMPILER" manifest <run-dir> <task-id> <attempt-no>
python3 "$SCENE_ASSET_COMPILER" review <run-dir> <task-id> <attempt-no>
```

Do not repeat these three commands after `execute-scene-provider`; that command already writes the immutable execution receipt, result manifest, and mechanical review.

Mechanical review binds the execution receipt to the capsule and output hashes; decodes direct, indexed, grayscale, 8-bit and 16-bit PNG alpha to prove real transparency, exact canvas, foreground core and reusable-asset margin; and checks PSD canvas, identifiable sRGB ICC profile, native pixel-leaf layer inventory, raw/RLE/ZIP channel payload structure, masks/Smart Objects, and nonempty Saved/Work/Clipping Path structure. It does not approve semantic quality or claim decoded PSD alpha/path alignment without render-back evidence.

Create visual rubric:

```bash
python3 "$SCENE_ASSET_COMPILER" visual-template <run-dir> <task-id> <attempt-no>
```

Visually inspect actual output against the task's acceptance snapshot, VisualBrief, and approved references. Fill every decided QA axis in `visual-review.json` with concrete evidence. Photoreal profiles require a non-NA `photorealism` verdict covering camera/depth, light/contact, anatomy, materials, and natural imperfections. Then validate:

For a wireframe contract, bind `artifact_sha256` and `layout_evidence.output_sha256` to the same sole PRIMARY output. Approval re-hashes every result artifact from disk before writing `selection.json`; a stale mechanical/visual PASS cannot approve missing or changed bytes.

```bash
python3 "$SCENE_ASSET_COMPILER" validate-visual <run-dir> <task-id> <attempt-no>
```

Follow `references/04-qa-rubric.md`.

## Phase F — Approve or diagnose

If mechanical PASS + visual APPROVED:

```bash
python3 "$SCENE_ASSET_COMPILER" approve <run-dir> <task-id> <attempt-no>
```

This creates the task's `selection.json`. Downstream tasks use only selected outputs as generated anchors. Task status and publication independently re-hash selected artifacts; missing or changed bytes are not `APPROVED`.

If either review fails, do not immediately reroll. Build an atomic input/output audit:

```bash
python3 "$SCENE_ASSET_COMPILER" audit-template <run-dir> <task-id> <attempt-no>
# inspect actual output; fill expected vs actual claims in output-audit.json
python3 "$SCENE_ASSET_COMPILER" validate-audit <run-dir> <task-id> <attempt-no>
python3 "$SCENE_ASSET_COMPILER" doctor <run-dir> <task-id> <attempt-no>
python3 "$SCENE_ASSET_COMPILER" reconstruction-plan <run-dir> <task-id> <attempt-no>
```

Doctor classifies the divergence as LOCAL / REGIONAL / GLOBAL and chooses the earliest recovery boundary. For diagnosed recovery:

```bash
python3 "$SCENE_ASSET_COMPILER" reconstruct <run-dir> <task-id> <attempt-no>
```

This creates a new immutable attempt. It uses an edit target only for surgical/local recovery; global failures regenerate from canonical authority instead of inheriting the rejected composition. Re-select the executor from the new attempt using Phase D: only a reference-free PNG `GENERATE` attempt may use Scene Provider; edits, references, cutouts, WebP, and PSD stay with `$imagegen`. Then repeat review. If `attempt.json.repair.fresh_executor_context_required=true`, do not execute image generation in the current thread: use a separate Codex agent/thread with only `generation-capsule.md` and resolved visual inputs, or stop and report that executor-context reset is required.

`repair` remains a fast path for already-understood single-axis failures. Prefer Doctor when the cause/scope is unclear, multiple axes fail, or the same failure repeats.

Follow `references/05-repair-policy.md`, `references/20-doctor.md`, `references/21-reconstruction.md`, and `references/23-experiment-loop.md`.

## Phase G — Publish

Inspect status:

```bash
python3 "$SCENE_ASSET_COMPILER" status <run-dir>
```

Publish only when every required task is APPROVED. If `RunRequest.review_policy.human_final_review` is true, first record explicit final approval:

```bash
python3 "$SCENE_ASSET_COMPILER" human-approve <run-dir> --reviewer "<name>" --note "<optional note>"
```

Then publish:

```bash
python3 "$SCENE_ASSET_COMPILER" publish <run-dir>
```

Publication re-hashes selected outputs and creates content-addressed blobs, `asset-pack.json`, and `MANIFEST.sha256`.

The generic `publish` command and its v1 `asset-pack.json` remain a standalone SceneAssetCompiler workflow. They are not the SceneMotionCompiler boundary. For a released scene projection, prepare the producer-owned SceneAssetPack v2 and the four requirement bindings with:

```bash
python3 "$SCENE_ASSET_COMPILER" scene-release prepare <run-dir> <scene-release-request.json>
python3 "$SCENE_ASSET_COMPILER" scene-release approve <run-dir> --reviewer <human-id>
python3 "$SCENE_ASSET_COMPILER" scene-release validate <run-dir>
python3 "$SCENE_ASSET_COMPILER" scene-release publish <run-dir>
```

These commands write only the stable `handoff/asset-pack.v2.json`, `handoff/asset-pack-audit.v1.json`, `handoff/binding-manifest.v1.json`, `handoff/scene-asset-release-approval.v1.json`, and final `handoff/scene-motion-asset-handoff.v2.json` files. Each runtime asset must explicitly declare `kind`, `rights`, and `sourceRegionId`; the consumer never infers them. The v2 handoff pins the projection's SceneGraph, source semantic manifest, and projected semantic manifest hashes.

The scene-release boundary is trusted-local and single-writer: producer and
consumer re-check project-relative regular files, raw hashes, sizes, MIME
declarations, and supported image/PSD signatures. It does not authenticate a
hostile local writer or reviewer identity. Cryptographic signatures, key
ownership, and signer authenticity are **NOT PROVIDED** by these receipts;
approval binds only the exact four-hash subject.

## Non-negotiable invariants

- JSON contracts > prompt prose > chat context.
- Approved anchors > unapproved candidates.
- One task owns one independently repairable asset responsibility.
- Attempts are append-only; never overwrite evidence.
- Every returned output requires an immutable execution receipt bound to capsule/input/output hashes.
- Image generation/editing uses only `$imagegen` or the typed Scene Provider lane; SceneAssetCompiler never contains an image API client.
- Mechanical PASS never substitutes for visual QA.
- Codex visual QA is mandatory; optional human final review follows RunRequest policy.
- Repair scope is minimal.
- AssetPack contains only approved selections.
- Record failed, unavailable-in-this-attempt, and unexposed execution facts explicitly; never convert `UNKNOWN` into a success claim or one failed PSD return into a global unsupported claim.
- GPT/$imagegen is the only PSD producer. Local PSD assembly/cutout/path synthesis is outside the canonical product boundary.

## Progressive reference map

- `references/00-positioning.md` — product boundary
- `references/01-workflow.md` — end-to-end phases
- `references/02-contracts.md` — authority/source-of-truth map
- `references/03-generation-handoff.md` — executor selection and exact `$imagegen` boundary
- `references/04-qa-rubric.md` — mechanical + visual gates
- `references/05-repair-policy.md` — selective retry rules
- `references/06-split-profiles.md` — L0–L3 separation rules
- `references/07-run-layout.md` — filesystem state/immutability
- `references/08-codex-playbook.md` — operational agent behavior
- `references/09-plugin-roadmap.md` — later Codex plugin packaging
- `references/10-planning-rules.md` — task/anchor/dependency planning
- `references/11-review-evidence.md` — evidence standard
- `references/12-failure-codes.md` — failure → repair mapping
- `references/34-scene-provider-executor.md` — headless Scene Provider v2 prerequisites, limits, execution, and recovery


## Image-generation craft references

- `references/13-image-generation-mastery.md` — highest-level production method and evidence hierarchy
- `references/14-reference-architecture.md` — reference diet, role isolation, character-sheet policy
- `references/15-shot-grammar.md` — narrative function, first read, composition/camera/blocking
- `references/16-color-light-material.md` — color scripts, motivated light, material cues
- `references/17-prompt-compiler.md` — JSON control plane → concise executor capsule
- `references/18-community-findings.md` — filtered Reddit/forum/SNS field evidence and rejected folklore
- `references/19-production-recipes.md` — character/prop/environment/scene/stylized/PSD production recipes
- `references/20-doctor.md` — atomic input/output divergence diagnosis
- `references/21-reconstruction.md` — edit vs regenerate vs anchor/reference reset policy
- `references/22-metrics.md` — optional TIFA/DreamSim/DINOv2/LPIPS/etc evidence roles
- `references/23-experiment-loop.md` — one-control experiment → keep/revert loop

- `references/30-path-and-clipping-path.md` — GPT-generated PSD path policy
- `references/31-cutout-quality.md` — prompt-only cutout quality and validation
- `references/32-gpt-psd-generation-profile.md` — exact copy/paste PSD generation settings
- `references/33-render-contracts.md` — v0.10 capture, genre, role, layout, and review contracts
