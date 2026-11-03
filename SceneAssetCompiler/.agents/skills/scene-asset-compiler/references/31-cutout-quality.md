# GPT Cutout Quality — Prompt and Validation Only

## Boundary

The image generator performs cutout/background removal and creates the PSD path metadata.

SceneAssetCompiler does not run segmentation, matting, despill, contour tracing or path fitting in the canonical pipeline.

## Prompt priorities

For an isolated asset:
- transparent background;
- complete object extent;
- generous canvas padding;
- no cast shadow unless it is a separate requested layer;
- no background color contamination/halo;
- preserve real soft alpha on hair/fur/translucency;
- clean silhouette;
- do not crop extremities;
- no pixel/user layer mask in the PSD;
- promote Work Path to a formal Named Saved Path.

For scene assetization:
- use approved target only for pose/composition/lighting guidance;
- generate the semantic asset cleanly rather than cropping it from the target;
- `COMPLETE_OBJECT` regenerates occluded reusable objects;
- `VISIBLE_ONLY` does not hallucinate unnecessary hidden content;
- `POSE_SAFE_UNDERLAY` adds only enough hidden overlap to prevent articulation seams.

## QA

For reusable `CHARACTER_*` and `PROP_*` PNG assets, a positive per-edge margin is a planning and compile-time invariant. The default safe margin is 2% of the shorter output canvas edge, rounded up. An explicit `output.min_transparent_margin_px` overrides the default and must be positive. Prompts and capsules state the exact pixel margin, require the complete character/object extent, and forbid edge contact. Touching an edge is `ALPHA_FOREGROUND_CLIPPED`; falling below a nonzero required margin is `ALPHA_MARGIN_TOO_SMALL`.

Margin is measured from decoded nonzero-alpha pixels for direct-alpha, palette/`tRNS`, grayscale, 8-bit and 16-bit PNGs. An RGBA file header, a checkerboard preview, a fully opaque channel, or an all-254 full-canvas alpha plane is not evidence of a reusable transparent background. Reusable character/prop assets require at least one fully transparent background pixel in addition to the 2% margin and near-opaque foreground core.

SceneAssetCompiler validates:
- file is a real PSD;
- expected leaf layers exist;
- user/pixel layer masks are absent;
- required Saved Paths exist;
- path records are structurally valid;
- document clipping-path policy is correct;
- alpha/transparency and visible edge quality are inspected;
- identity/style/registration still match approved authority.

If the cutout edge is wrong, ask `$imagegen` for a targeted canvas-fit correction only: scale down, recenter, or extend the transparent canvas while preserving approved pixels and identity. Do not creative-reroll the asset. A missing preferred PSD is a packaging/format failure, not a visual failure; it may receive at most one PSD-only packaging retry that reuses locked pixels and layout without redraw, restyle, relight, recompose, or local PSD assembly.
