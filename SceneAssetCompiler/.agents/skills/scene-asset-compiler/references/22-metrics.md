# Optional Metric Evidence

SceneAssetCompiler core remains Python-stdlib and does not require vision-model dependencies. External metrics may emit `metric-evidence.json` as **advisory evidence**.

## What each metric is good for

### TIFA / Soft-TIFA / GenEval
Use for **atomic prompt faithfulness**: object presence, attributes, count, relations, and other decomposable claims.

This is closest to SceneAssetCompiler Doctor's claim model. TIFA explicitly decomposes a prompt into fine-grained VQA probes rather than hiding everything in one embedding score. GenEval/GenEval2 similarly evaluate atomic visual concepts and compositional relationships.

Use as: compliance evidence. Do not use as an aesthetics score.

### Grounding DINO + SAM 2
Use for:
- required object presence,
- rough position/bounding-box evidence,
- object masks/area,
- occlusion or foreground/background diagnostics.

Useful for structural evidence, not identity/style approval.

### DINOv2
Use visual features for project-calibrated anchor/asset similarity or retrieval.

Good soft evidence for "does this candidate still look like the approved visual object?" but no universal threshold should be hard-coded.

### DreamSim
Use for holistic human-like perceptual difference when comparing related images/variants. It was designed to bridge low-level LPIPS-style similarity and high-level semantic similarity, including layout/pose/color sensitivity.

Useful for edit drift and candidate comparison. Not a narrative correctness gate.

### LPIPS
Use only when two images are spatially comparable and the question is **how much did the edit change perceptually?**

Good for edit locality. Poor choice for two semantically equivalent images with different camera/pose/layout.

### CLIPScore
Useful as a broad text-image compatibility signal, but it compresses fine-grained claims and can miss contextual details. Never let CLIPScore override atomic claim failures.

### HPS / PickScore
Human-preference/aesthetic ranking signals. Use to choose between already-compliant candidates, never to decide whether a required object or story fact exists.

## No universal weighted score

Do not compute:

```text
0.3 * CLIP + 0.2 * HPS + ... = PASS
```

Aesthetic preference must not cancel a missing character, wrong setting, broken PSD structure, or narrative contradiction.

Metrics remain named evidence attached to specific purposes. Thresholds are project-calibrated and soft unless a deterministic contract explicitly defines them.
