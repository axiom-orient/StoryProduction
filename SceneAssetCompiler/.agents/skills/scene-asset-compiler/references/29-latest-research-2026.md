# Latest Research Notes — 2026-08-24

## Multi-subject generation

Recent 2026 research continues to show that single-call multi-subject generation is not solved:

- **MultiBind (2026-03)** isolates cross-subject failures including drift, swap, dominance and blending, and evaluates face, appearance, pose and expression separately.
- **MultiCompose (2026-08)** reports identity degradation and attribute misalignment when several personalized concepts are activated together; it improves results using explicit spatial separation/masks and per-concept composition.
- **CRAFT (2026-08)** emphasizes correct per-subject attention routing — "where to look" — for personalized multi-subject generation.
- **Beyond Facial Consistency (2026-07)** argues that face-only similarity is insufficient; holistic appearance cues must also remain consistent.

SceneAssetCompiler consequence: solve composition separately, use role-bound references, keep holistic identity invariants, and generate editable semantic assets independently where practical.

## Layer-native design

- **Qwen-Image-Layered (2025-12)**: semantic RGBA decomposition improves editability, but released weights are primarily image→layers and explicitly have limited text→multi-layer control.
- **LASAGNA (2026-01)**: jointly modeling background/transparent foreground improves cross-layer visual effects, showing that shadows/reflections are composition relationships rather than simple cutout properties.
- **UniWorld-Design (2026-08)**: treats semantic RGBA layers as atomic units and reports complete-object layer generation rather than visible-pixel partitions.
- **OmniPSD (2025-12)**: demonstrates text/image-to-PSD as a research direction, but SceneAssetCompiler still prefers deterministic assembly from independently approved layers because it is easier to validate and selectively repair.

## Codex/OpenAI image generation

The official Codex `$imagegen` skill says to use the built-in image generation tool by default, inspect the actual output, make one targeted change at a time, and for multi-asset requests loop separate built-in calls. That maps directly to GenerationUnits.

A current Codex App issue (2026-08-23) reports installations where the skill is present but the built-in `image_gen` tool is not exposed. SceneAssetCompiler therefore treats image generation availability as an executor capability and fails closed instead of silently changing provider/API semantics.
