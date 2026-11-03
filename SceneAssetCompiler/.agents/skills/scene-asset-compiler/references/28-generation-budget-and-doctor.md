# Generation Budget + Doctor Recovery Boundaries

## Search budget

Generation count is a **ceiling**, not a target. Stop when a lock has an approvable candidate.

Typical complex PSD scene:

```text
composition exploration            3–5
integrated target                  1–3
environment/backplate              1–3
each recurring character           1–4
each important prop                1–3
foreground/occluders               1–2
effects/light helpers              1–3
targeted repairs                   2–8
layer reconstruction repairs       1–8
```

20–40+ image-executor calls for one scene can be rational; generating 40 images without lock/stop logic is not.

## Doctor failure families

`AUTHORITY`
: Contradicts narrative/Character/Style authority. Regenerate from authority.

`ANCHOR_DRIFT`
: Identity/prop appearance diverges from approved anchor. Edit or regenerate only that asset with the anchor.

`LAYOUT_DRIFT`
: Loses locked framing/blocking. Use the approved composition lock. Reopen composition only if the lock itself was wrong.

`LAYER_CONSTRUCTION`
: Alpha, full extent, semantic isolation, registration or underlay is wrong. Regenerate only that layer.

`CROSS_LAYER_HARMONY`
: Individually valid layers disagree on perspective, scale, light direction or color temperature. Regenerate the smallest conflicting layer against the locked target.

`COMPOSITE_DELTA`
: Deterministic layer flatten no longer matches the approved scene target closely enough. Identify responsible layers; never rerun every layer by default.

`EXECUTOR`
: Image-executor capability/context/file handoff failed. Reset executor context/capability. Do not mutate creative authority.

## Reconstruction ladder

```text
same pixels, surgical edit
→ same unit, new candidate
→ regenerate one semantic asset
→ reopen one scene lock
→ rebuild integrated target
→ only then revisit scene intent
```

Use the earliest boundary that can plausibly repair the evidence.
