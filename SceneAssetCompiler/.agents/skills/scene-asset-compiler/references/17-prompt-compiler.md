# Prompt Compiler Rules

## Control plane vs image brief
JSON contracts are the source of truth. The generation capsule sent to the selected executor is a **concise projection**, not a dump of every project document.

OpenAI guidance favors clear 1–3 sentence requests or short labeled sections. SceneAssetCompiler therefore compiles only high-salience decisions:

```text
Use case / asset
Narrative function / emotional intent
Primary moment
First read / focal hierarchy
Composition / camera / lens / focus depth
Lighting
Color/value
Materials / atmosphere
Style visual language + project light/camera/material rules
Identity/reference invariants
Structure/output
Avoid / hard constraints
```

## Why not raw JSON-only
Structure improves machine validation, but JSON syntax itself does not make the image model more visually intelligent. Codex reads validated JSON; the selected executor receives the distilled visual brief plus exact invariants.

## Specificity rule
- Already precise user direction → normalize, do not embellish.
- Vague direction → add only decisions required to make a usable asset.
- Never invent extra characters, story beats, props, slogans, palettes, or brand details merely to make a prompt sound rich.

## Edits
Every edit repeats:
- change only failed axis,
- preserve passing axes,
- keep approved identity/anchors,
- keep unmentioned composition/style/structure unchanged.

Prefer one targeted edit over a rewritten full prompt.

## Reconstruction compilation

A reconstructed prompt is not the rejected prompt plus louder adjectives.

For `RECONSTRUCT_FROM_AUTHORITY` / `TARGETED_REGENERATION`:
1. discard accidental visual decisions from rejected pixels,
2. rebuild from task authority snapshot + VisualBrief,
3. move critical MUST_HAVE/MUST_NOT_HAVE claims earlier,
4. reduce the reference diet to role-complete minimum,
5. keep decorative style/detail after subject/setting/action/composition,
6. include diagnostic experiment instructions only when one variable is deliberately being tested.

For `SURGICAL_EDIT` / `TARGETED_EDIT`:
- name exact edit target and SHA,
- repeat preserved axes,
- change one failed region/axis,
- prohibit collateral change explicitly.
