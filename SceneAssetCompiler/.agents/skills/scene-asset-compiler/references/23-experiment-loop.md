# Diagnostic Experiment Loop

When Doctor cannot confidently isolate a cause, use controlled experiments instead of prompt folklore.

```text
hypothesis
→ change one control
→ generate one candidate
→ audit affected claims
→ compare gap vector
→ keep or revert
```

## Canonical experiments

### MINIMAL_CORE_PROMPT
Keep only:
- primary subject/identity,
- setting/action,
- first read,
- hard must-not constraints,
- minimal required style.

Purpose: detect prompt priority/overconstraint problems.

### ANCHOR_ONLY_CONTROL
Approved identity anchor + neutral simple task.

Purpose: determine whether identity drift comes from the anchor/provider or from scene/reference pressure.

### REFERENCE_ABLATION
Remove one reference or one role at a time.

Purpose: identify contamination/conflict.

### STYLE_ONLY_CONTROL
Simple neutral subject + style reference only.

Purpose: test whether style reference is coherent without identity/pose pressure.

### EDIT_LOCALITY_CONTROL
Apply exactly one edit and compare to source using visual evidence and, optionally, LPIPS/DreamSim.

Purpose: detect collateral drift.

## Keep/revert rule

Keep an experiment only if:
- targeted failed claims improve,
- preserved axes do not materially regress,
- no new critical failure is introduced.

Record observations, not just "version 2 looks better".

### TASK_SCOPED_EXECUTION_CAPSULE / execution reset
If task-scoped content leaks once, rerun with the capsule as the only visual brief. If the same executor/meta-context leakage repeats, the controlled variable is no longer prompt wording: reset the executor conversation/thread. Equivalent same-context prompt rewrites are invalid experiments.
