# GPT-Generated Scene PSD — Packaging, Not Local Assembly

## Decision

The image generator creates the PSD.

SceneAssetCompiler does **not** locally:
- cut out pixels,
- trace a mask,
- construct a PSD,
- synthesize Work/Saved/Clipping paths.

SceneAssetCompiler owns only:
1. approved upstream scene/asset inputs,
2. exact PSD generation settings,
3. the `$imagegen` packaging capsule,
4. structural/visual validation of the returned PSD,
5. retry instructions for only the failed PSD property.

## Scene sequence

```text
composition candidates
→ COMPOSITION_LOCK
→ integrated target
→ TARGET_LOCK
→ optional LAYOUT_LOCK
→ one independent PLACEHOLDER_PHOTO and PLACEHOLDER_LOCK per wireframe slot
→ PLACEHOLDER_SET_LOCK
→ independent semantic asset generations
→ LAYER_LOCK per asset
→ ASSET_LOCK
→ one PSD_PACKAGING GenerationUnit
→ returned GPT-generated PSD
→ PSD structural validator
→ visual/composite Doctor
→ COMPOSITE_LOCK
```

The PSD generation unit is not allowed to redesign the scene.
For layout scenes it explicitly depends on `LAYOUT_LOCK` and
`PLACEHOLDER_SET_LOCK` as well as `ASSET_LOCK`; `scene-psd-entry-check` revalidates
those records and their artifact/review hashes before packaging.

A real editable PSD remains a real external capability. One invocation may return
`PREFERRED_OUTPUT_MISSING`; record that invocation-specific result without claiming
live success and without converting it into a global unsupported declaration.

## Canonical input to PSD packaging

- approved target render: `COMPOSITION` evidence only,
- approved semantic layer assets: actual layer sources,
- SceneCoordinateContract / locked canvas,
- exact layer IDs and z-order,
- PSDGenerationProfile.

## Required scene PSD behavior

- real Adobe PSD;
- RGB 8-bit/channel;
- embedded sRGB IEC61966-2.1;
- exact scene canvas;
- native pixel layers, no Smart Objects;
- normal layer alpha/transparency;
- no Photoshop pixel/user layer masks;
- one exact leaf layer ID per semantic asset;
- Named Saved Path `clip.<layer_id>` for every cutout layer;
- no document clipping path for a multi-layer scene PSD;
- no Work Path requirement after paths have been saved;
- no trim;
- no creative redraw/restyle/recomposition.

## Why

Adobe stores Saved Paths as document image resources and only one document clipping-path name can exist. A multi-layer scene therefore needs multiple Saved Paths, not pretend per-layer document clipping paths.
