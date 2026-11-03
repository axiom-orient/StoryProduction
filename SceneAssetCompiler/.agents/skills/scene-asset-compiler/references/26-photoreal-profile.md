# Photoreal Prompt Profile — Executable and Stage-Aware

These profiles are compiled by `scene_asset_compilerlib.prompts.photoreal_profile_lines`; they are no longer documentation-only. The compiler binds camera, focus/depth, motivated light, anatomy, materials, natural imperfections, and per-character identity before stage-specific constraints.

## High-value principles

- Declare believable capture provenance: who took the image and why.
- Use motivated existing light rather than default studio fill.
- Prefer an in-progress action to a catalog pose.
- Specify anatomy **and** action logic for visible hands.
- Preserve mild asymmetry, skin/hair/material irregularity and lived-in cues.
- Pair important negatives with a positive concrete alternative.
- Generate 3–5 meaningfully varied scene candidates before rewriting everything.
- On failure, repair the failing axis only.

## Stage profiles

### `PHOTOREAL_SCENE`
Use realistic capture provenance, motivated ambient light, natural moment, plausible phone/camera imperfection and mildly imperfect framing when appropriate.

### `PHOTOREAL_CHARACTER_ANCHOR`
Keep skin/hair/eye/material realism and believable anatomy, but remove accidental crop, heavy motion blur, dramatic occlusion and unstable perspective. Identity readability outranks snapshot imperfection.

### `PHOTOREAL_LAYER_ASSET`
Keep identity, anatomy, material cues and **scene-matched light direction/color relationship**. Remove random framing defects, background clutter and irreversible blur/noise that damages alpha edges or reuse.

A photoreal scene prompt must not be blindly reused for a transparent production layer.

## Review gate

Every `PHOTOREAL_*` task must receive a decided `photorealism` verdict. Review camera/perspective/depth, shared light and contact cues, anatomy/action logic, material response, identity/attribute binding, and natural imperfections separately from general style. A glossy or cinematic look is not proof of physical plausibility.

## Multiple people

Do not apply one gaze/posture instruction to everyone. Bind gaze/action/wardrobe separately per subject. Prefer composition exploration before loading all identity references. This is reinforced by recent multi-subject research showing cross-subject attribute misbinding.

## Positive constraints

Instead of only `do not use perfect framing`, state the intended alternative: `horizon tilted 1–2 degrees, subject left of center`.

Instead of only `do not smooth skin`, state: `visible pores, fine hair and slight local redness appropriate to the stated age`.

## Candidate variation

For scene exploration vary, in order: blocking → camera position → shot size → lighting. Do not create near-duplicates that differ only in light.
