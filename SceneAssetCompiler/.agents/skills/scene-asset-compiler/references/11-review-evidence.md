# Visual Review Evidence Standard

A visual review must record **observed evidence**, not a bare “looks good.”

## Identity PASS evidence examples
- face shape, eye color, hair silhouette and brass earring match approved anchor
- body proportions remain compact/athletic; no age or species drift

## Style PASS evidence examples
- same clean ink edge treatment and two-step cel shading as style anchor
- palette stays within muted blue-gray family; no photoreal material drift

## Structure PASS evidence examples
- left upper/lower arm are visibly independent semantic parts in returned PSD
- foreground occluder is separate; character color layer contains no baked floor shadow

## Articulation PASS evidence examples
- elbow/knee overlap extends under adjacent segment; no transparent seam at planned bend direction
- hair-back is independently movable and does not contain torso pixels

## FAIL evidence
Name the exact invariant/layer/region and what is wrong. This evidence feeds the targeted repair prompt. Avoid vague labels such as “bad quality,” “off,” or “try again.”

## Narrative / composition / lighting-color / material / usability evidence
- narrative: depicted action/state and prop possession match the cited story beat; no extra causal event appears
- composition: the declared first read is dominant at thumbnail scale and the second/third focal priorities remain subordinate
- lighting/color: the stated source direction and value grouping are visible; selected color-script beat is preserved
- material: cloth/metal/skin/paper/etc. show the declared surface behavior without changing the abstraction level
- usability: silhouette/crop/negative space/readability fit the intended asset/runtime use

## Required evidence rule
Every axis marked PASS or FAIL must have at least one concrete evidence line. Only NA may be empty. Approval fails closed if a decided axis has no evidence.

For wireframe layout review, `artifact_sha256` is mandatory and must equal both `layout_evidence.output_sha256` and the current SHA-256 of the sole PRIMARY result artifact. Approval re-hashes every result artifact immediately before selection. Later task-status and publish checks re-hash selected artifacts again, so stale review metadata cannot preserve approval after bytes drift or disappear.

## Atomic divergence evidence

For OutputAudit, evidence must describe observable pixels, not the reviewer's theory.

Good:
- "No white dog is visible; the two largest figures are an adult and child."
- "Sun disk touches the horizon; orange sky reads as sunset."
- "Required `char.hero.arm_l.lower` layer is absent from PSD inventory."

Bad:
- "The prompt was ignored."
- "The model got confused."
- "Reference 2 probably dominated."

The first group is observation. The second group is hypothesis and belongs in Doctor, with confidence and evidence.
