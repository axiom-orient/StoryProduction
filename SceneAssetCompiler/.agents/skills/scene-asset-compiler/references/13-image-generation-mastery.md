# Image Generation Mastery — Production Method

SceneAssetCompiler does not treat image generation as prompt magic. The production method is:

```text
story intent
→ visual authority
→ canonical anchors
→ task-specific visual brief
→ role-isolated references
→ concise imagegen brief
→ inspect
→ evidence-based QA
→ single-axis edit
→ lock approved result
```

## 1. Purpose before aesthetics
Every image must answer: **why does this frame exist?** Write `narrative_function` and `first_read` before style adjectives. A hero shot, identity sheet, reveal, threat beat, and environment establishing image need different framing even inside one style.

## 2. Design a hierarchy, not a pile of details
Choose at most three focal priorities. The first read must survive thumbnail size. Background detail, texture, particles, props, and text are subordinate unless they carry story information.

## 3. Lock identity and visual language before scene production
Recurring character/prop assets need approved anchors. Downstream shots use approved selections, never chat memory or unapproved candidates.

## 4. Separate reference roles
Use a small set of references. Label every input: IDENTITY, STYLE, POSE, COSTUME, PROP, COMPOSITION. State both `preserve` and `ignore`. A pose image must not donate a face, costume, or style.

## 5. Brief like a cinematographer / art director
Prefer concrete decisions over prestige tags:
- who/what at what moment,
- what reads first,
- where the subject sits in the frame,
- foreground/midground/background,
- camera angle/perspective,
- where light physically comes from,
- hard/soft quality and contrast,
- dominant/accent colors and value grouping,
- material cues.

`cinematic, masterpiece, 8k, beautiful lighting` is weaker than a real shot decision.

## 6. Let story drive light and color
Lighting and framing follow character/emotion and scene function. Color is planned across scenes, not randomly optimized per image. Use `color_script` to preserve project language while allowing narrative progression.

## 7. Style is a system, not an artist name
Describe shape abstraction, edge treatment, shading, texture, palette, materials, camera behavior, and detail density. This is more controllable, more portable across models, and less brittle than naming a specific artist.

## 8. Generate the simplest correct base first
Do not solve identity, complicated action, elaborate environment, typography, FX, and PSD topology in one first shot unless the task truly requires them. Anchor first, then derive.

## 9. Edit winners; do not reroll passing decisions
Once a result has strong identity/composition, keep it and make one targeted change. Repeat invariants on every edit. Full regeneration is reserved for broad collapse, not a small defect.

## 10. Treat output as production evidence
A persuasive image is not automatically a usable asset. Validate the actual file, alpha, layer IDs, crop, semantic separation, identity, visual hierarchy, narrative state, lighting/color, and articulation before approval.

## Evidence tiers
1. OpenAI official image-generation guidance and official Codex `$imagegen` skill.
2. Primary production sources (Pixar color scripts/world design; Roger Deakins/ASC on character, framing, motivated light).
3. Open-source research patterns (IP-Adapter/InstantID/StoryDiffusion: separate identity/image conditioning and preserve long-range identity).
4. Community/SNS practice (Reddit/OpenAI forum): used only when it agrees with higher tiers or exposes repeatable failure modes.

Community “magic prompt” claims, seed folklore, adjective stacks, and unsupported capability claims are not promoted into SceneAssetCompiler contracts.
