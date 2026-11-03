# QA Rubric

## Mechanical
- file exists and is non-empty
- manifest SHA-256 matches
- output type is allowed
- transparent PNG has alpha-capable color type
- PSD signature/version parse
- no duplicate machine layer IDs
- every required layer ID exists

## Identity
- same face/silhouette/proportions/distinctive marks as approved anchor
- costume/state variant changes only declared fields
- no identity contamination from pose/style refs

## Style
- shape language, line/edge treatment, shading, texture, palette/material rules obey VisualAuthority
- no per-task visual-language drift
- stylization is coherent across modeling/rendering decisions, not a last-minute filter

## Narrative
- depicted action/state matches narrative refs and protected visual facts
- no visually invented event changes causality, knowledge state, prop possession, injury/costume state, or scene meaning
- visual inventions remain cosmetic unless explicitly approved

## Composition
- first read matches `visual_brief.first_read`
- focal hierarchy is readable in the intended order
- framing, placement, depth, occlusion, gaze/action direction support the narrative function
- thumbnail read is clear; background detail does not steal focus

## Lighting / Color
- light has the declared motivation/direction/quality unless intentionally graphic
- contrast/value grouping directs attention to the intended subject
- palette follows the selected color-script beat when one is referenced
- identity-critical colors remain stable

## Material
- declared surfaces follow the project material language and task-specific material cues
- highlights, roughness/texture, wear, translucency, cloth/paper grain, or stylized equivalents are coherent
- material treatment does not silently change the project's abstraction level

## Structure
- each required semantic layer contains the intended part only
- forbidden merges are absent
- transparent/background policy is obeyed
- no unrequested baked shadow/effect

## Articulation
For L2/L3 only:
- joint overlap exists under shoulder/elbow/wrist/hip/knee/ankle as needed
- plausible planned rotation does not expose holes
- external silhouette remains clean

## Usability
- silhouette reads at intended display size
- subject is not unintentionally cropped
- negative space/use-case requirements are preserved
- no accidental text, watermark, clutter, or anatomy/object corruption that makes the asset production-unusable

## Cross-task consistency
- downstream assets use approved anchor identity/style
- recurring prop design is stable
- scene state matches narrative/character variant
- scene-to-scene color/light changes are purposeful, not random drift

A mechanical PASS never substitutes for visual QA.

## Evidence rule
Every PASS or FAIL axis must contain at least one concrete observation in `evidence`. `NA` may have no evidence. An unexplained PASS cannot be approved.

## Doctor escalation

A rejected visual review is not automatically a local repair request.

When one of the following is true, run Output Doctor before retrying:
- 2+ visual axes fail,
- any critical identity/narrative/structure claim fails,
- the primary subject/setting/action is wrong,
- a reference may have contaminated an ignored axis,
- the same failure reappears,
- it is unclear whether edit or regeneration is safer.

Doctor converts the task contract into atomic expected-vs-actual claims and preserves the failed-axis vector. See `20-doctor.md`.
