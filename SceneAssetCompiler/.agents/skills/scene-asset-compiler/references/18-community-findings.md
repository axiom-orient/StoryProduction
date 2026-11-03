# Community / SNS Findings — Filtered Field Evidence

Community/SNS material is **field evidence**, never authority. Adopt a pattern only when it is repeatable and compatible with OpenAI guidance or established visual-production practice.

## Reddit — recurring-character production
Recent 2026 threads repeatedly converge on:
- create a strong character/model sheet or canonical anchor before a sequence,
- reuse relevant approved views every shot rather than trusting long chat memory,
- describe stable facial structure/silhouette instead of only “same person” language,
- choose a good base and repair it surgically instead of rerolling everything.

**Adopted:** CharacterAnchor, approved views, explicit identity invariants, append-only targeted edits.

## Reddit — “prompt like an art director”
Practitioners report that inventory-style noun lists produce generic work, while deliberate emotion, atmosphere, camera, lighting, texture, composition, and constraints produce more intentional results.

**Adopted:** VisualBrief. But vague “cinematic feeling” is not enough; emotional intent must be translated into concrete visual decisions.

## Threads — identity lock + controlled variation
Threads posts from AI designers repeatedly use a useful contract pattern: hold facial identity/character construction fixed while explicitly allowing pose, camera angle, framing, expression, lighting, or composition to change.

**Adopted:** `identity_invariants` + `allowed_variation`; edits state both what may change and what must remain fixed.

## Threads — tagged/role-specific references
Several creator workflows explicitly tag character/reference images for consistent scenes. The reliable lesson is not any claimed reference-count limit; it is assigning each reference a narrow job.

**Adopted:** source and role are separate. Every visual input is IDENTITY / STYLE / POSE / COSTUME / PROP / COMPOSITION / ENVIRONMENT / MATERIAL with `preserve` and `ignore`.

## X — strict turnaround / storyboard constraints
2026 X examples show that production-sheet prompts improve when they specify exact view layout, neutral pose, background, no props/text, and consistency requirements; storyboard examples explicitly enumerate shot types rather than asking for “cinematic angles.”

**Adopted selectively:** exact view/shot contracts for reference sheets and storyboards. **Not adopted:** “8K/ultra-detailed” as quality logic.

## X — separate reference assignment
A recurring video/image workflow assigns separate inputs to character identity, outfit, environment, material detail, or camera behavior and explicitly says what each controls.

**Adopted:** role isolation and source-role separation. This agrees with OpenAI's official imagegen reference guidance and image-conditioning research.

## Combined development boards
Character sheet + storyboard grids can be useful for exploration because they expose consistency failures quickly.

**Use for:** ideation/review boards.
**Do not use as canonical production assets:** one bad cell forces broad rework, individual cells have less resolution, and targeted repair becomes harder.

## Rejected / not promoted
- “8K”, “masterpiece”, “best quality”, “trending” adjective piles as a substitute for direction.
- repeating the whole prompt as a magic weighting trick.
- seeds, thread history, or session state as identity authority.
- dumping many references without role isolation.
- unverified claims about exact maximum reference counts or hidden model versions.
- generating many random alternatives after a strong base exists instead of editing the winner.
- session-specific bug workarounds as permanent architecture.

The workflow keeps these sources below official documentation, primary production sources, and reproducible research evidence.
