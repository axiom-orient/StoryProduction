# Planning Rules and Dependency Graph

## Task granularity
A task should own **one independently reviewable and repairable visual responsibility**. Good examples: one character anchor, one character variant, one recurring prop, one environment module, one key visual. Avoid a task that combines unrelated characters, props, and scenes merely to reduce generation calls.

## Anchor policy
- Recurring character with no approved reference → create `CHARACTER_ANCHOR` first.
- Recurring signature prop whose design must stay stable → `PROP_ANCHOR` first.
- A style image anchor is optional; use `STYLE_ANCHOR` only when textual StyleSpec is insufficient for consistent rendering.
- Downstream generated tasks depend on the relevant anchors **and explicitly reference them** as `TASK_OUTPUT`. Only approved `selection.json` PRIMARY outputs can resolve into a visual reference.

## Dependency vs reference
`depends_on` is an approval/order gate. It does **not** automatically make the dependency image an input. A visual input must also appear in `references` with `source_kind=TASK_OUTPUT`, a semantic role, `preserve`, and `ignore`. This keeps orchestration and image conditioning separate.

Example:
```text
CHARACTER_ANCHOR(hero) ─┐
PROP_ANCHOR(sword) ─────┼→ KEY_VISUAL(scene-12)
ENVIRONMENT_ASSET(castle)┘
```
Do not make unrelated assets depend on each other. This preserves parallelism and prevents a failure in one branch from blocking independent branches.

## Scene planning
A key visual should reference:
- exact narrative scene/state refs,
- approved character anchors/variants,
- approved recurring props,
- relevant environment asset if reuse matters,
- composition/pose refs only for their declared purpose.

Scene images are outputs, not sources of canonical reusable character parts.

## Split selection
Choose the lowest split profile that satisfies the actual consumer:
- L0: static/baked shot
- L1: swap/state-ready
- L2: interactive whole-body 2D
- L3: hero/performance close-up

Do not choose L2/L3 because it “might be useful someday.”

## PSD planning
For PSD tasks, required layer IDs must be semantic and machine-stable. Keep authoring groups human-readable, but leaf machine IDs should remain stable across repairs. A missing or merged leaf is a task failure, not an excuse to rename the contract after generation.


## VisualBrief gate
Every task must have a VisualBrief before compilation. Do not call any image executor from a task whose art direction is only a bag of adjectives.

Required reasoning order:
1. narrative function and emotional intent,
2. first read and focal hierarchy,
3. composition/camera/lens/focus behavior,
4. motivated light,
5. color/value role,
6. material/atmosphere/environment rules,
7. continuity and avoid list.

For scene/key-visual tasks, attach the relevant `color_script_beat_ids` and authority character/prop IDs. For anchors, keep the visual brief neutral enough to expose identity accurately.
