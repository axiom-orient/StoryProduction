# Reconstruction Policy

Reconstruction is not "write a stronger prompt". It chooses the earliest visual boundary that must be rebuilt.

## Recovery strategies

### SURGICAL_EDIT
Use when the image is correct except for one local noncritical mismatch.

Examples:
- hand shape,
- small crop/padding error,
- one material cue,
- local joint seam.

Use previous PRIMARY as edit target and repeat every preserved invariant.

### TARGETED_EDIT
One coherent region/axis is wrong but overall scene/layout should remain.

Examples:
- lighting only,
- a prop state,
- local background object,
- one costume detail.

### TARGETED_REGENERATION
A core axis is wrong enough that editing would inherit a bad composition, but approved references/VisualBrief remain valid.

Generate a fresh candidate from the same authority with a rebuilt prompt.

### RECONSTRUCT_FROM_AUTHORITY
Use for global mismatch.

Re-read:
1. Narrative/VisualAuthority,
2. task content,
3. VisualBrief,
4. role-specific references.

Then rebuild the prompt with this priority:

```text
must-have subject / identity
→ setting + action/state
→ first-read + composition
→ must-not-have constraints
→ reference role rules
→ lighting / color / material
→ decorative style detail
```

The rejected image is evidence only. Do not inherit its accidental composition.

### REFERENCE_ABLATION
When reference contamination/conflict is suspected:
- keep the smallest role-complete set,
- remove one questionable reference or role at a time,
- regenerate a controlled candidate,
- compare only affected axes,
- keep/revert based on the gap vector.

### RESEED_ANCHOR
If a recurring identity anchor repeatedly fails, do not patch downstream scenes forever. Rebuild the anchor itself and invalidate dependent selections according to project policy.

## Repeated-failure rule

If the same `gap_signature` repeats, stop making equivalent prompts.

Change the experiment:
- remove a conflicting reference,
- simplify the prompt,
- split a task,
- reseed an anchor,
- or acknowledge provider limitation.

A third cosmetically rewritten prompt with the same controls is not a new experiment.

## Preserve rule

Every reconstruction explicitly lists passed axes to preserve.

For edits, preservation is pixel/visual intent.
For regeneration, preservation means reasserting canonical authority—not copying accidental details from a rejected candidate.

## Repeated executor-context contamination

If `EXECUTOR_CONTEXT_CONTAMINATION` repeats after a task-scoped capsule reconstruction, do **not** keep rewriting the visual prompt in the same conversation. Doctor escalates to `RESET_EXECUTION_CONTEXT`. The next attempt is marked `fresh_executor_context_required=true` and must run in a separate Codex agent/thread when available, with only `generation-capsule.md` plus the explicitly resolved images/edit target. Codex App supports agents in separate project threads; if the current runtime cannot establish a fresh thread, stop and report the gate rather than retrying in the contaminated context.
