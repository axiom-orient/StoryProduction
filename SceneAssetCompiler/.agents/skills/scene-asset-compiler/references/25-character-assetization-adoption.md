# Character Assetization — Adopted No-Rigging Contract

This is the distilled architecture from the attached `character_assetization_final_package_v1`.

## Adopted

- Assetization is the product; rigging is downstream and optional.
- Freeze one canonical visual identity before generating variants.
- Separate semantic identity authority, coordinate authority and release pixel authority.
- State variants are redrawn/generated independently; do not rotate/crop a flattened illustration into large pose changes.
- Normalize once into a fixed master coordinate system, then freeze anchors.
- PSD is an authoring container; aligned PNGs + contract + manifest/hashes are release truth.
- Runtime must not repair registration, missing pixels or seams.
- Validate canvas, alpha, baseline/anchors, crop, identity and seams before advancing.
- Failed state blocks its own downstream work; do not mass-produce past a failed gate.

## Generalized for SceneAssetCompiler

The attached package's `2048×3072`, five body states and seven head states are a concrete product profile, not universal constants. SceneAssetCompiler makes them contract fields.

### Strategies

`WHOLE_CHARACTER`
: Whole-state swaps. Default when only a few poses/expressions are required.

`BODY_HEAD`
: Complete body poses below a fixed head socket + complete head-expression states. This is the attached package's strongest minimal reusable design.

`SEMANTIC_LAYERS`
: Independently editable body/garment/accessory parts. Use only when PSD editing or later interaction needs the extra degrees of freedom. Rigging is still out of scope.

## Character Master Space vs Scene Space

**Character Master Space** owns reusable identity coordinates: master canvas, baseline, body center and optional neck/head socket.

**Scene Space** owns one shot: scene canvas, per-character placement, z-order and occluders.

Never mutate the reusable character contract because one scene needs a different placement. Record an explicit scene placement instead.

## Occlusion policy

Every generated semantic asset declares one policy:

- `VISIBLE_ONLY` — only visible pixels are required; suitable for static backplates/occluders.
- `COMPLETE_OBJECT` — regenerate the complete object, including naturally hidden extent; required when the object may be moved/reused.
- `POSE_SAFE_UNDERLAY` — regenerate only the hidden margin required to prevent articulation/seam holes.

This avoids both failure modes: flattened-pixel decomposition with holes and unnecessary hallucination of every hidden surface.

## Recommended no-rigging character PSD

```text
character_master.psd
├─ 00_GUIDES
├─ 10_CANONICAL
├─ 20_WHOLE_STATES       # WHOLE_CHARACTER
├─ 30_BODY_STATES        # BODY_HEAD
├─ 40_HEAD_STATES        # BODY_HEAD
├─ 50_SEMANTIC_PARTS     # SEMANTIC_LAYERS when required
├─ 60_PROPS_ACCESSORIES
├─ 80_QA
└─ 90_REFERENCE
```

All release states use the same master canvas and are never trimmed before validation.
