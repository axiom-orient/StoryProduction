# Output Doctor — Input/Output Divergence

SceneAssetCompiler Doctor exists for one question:

> **What changed between the approved input contract and the actual generated pixels, and what is the smallest correct recovery boundary?**

Do not collapse this into one similarity score.

## 1. Evidence layers

Doctor evidence is ordered:

1. actual artifact bytes + hash
2. mechanical review
3. atomic visual claims in `output-audit.json`
4. role-specific reference observations
5. optional metric evidence
6. visual-review axis verdicts
7. causal hypothesis

A hypothesis never outranks direct visual evidence.

## 2. Atomic claim model

`audit-template` decomposes the task into individually inspectable claims from:
- subject/action,
- narrative function,
- first read,
- environment/continuity/avoid rules,
- Character identity invariants,
- style forbidden drift,
- protected visual facts,
- acceptance criteria,
- required layer IDs.

Codex inspects the actual image and marks each claim:
- `PASS`
- `MINOR`
- `MAJOR`
- `NA`

`UNKNOWN` means the audit is incomplete and Doctor must not run.

## 3. Gap vector

Doctor keeps divergence per axis:

```text
identity
style
narrative
composition
lighting_color
material
structure
articulation
usability
```

Do not average them into one magic score. A perfect watercolor style does not compensate for the requested dog being absent.

## 4. Scope

### LOCAL
One or two noncritical mismatches. Existing image is fundamentally correct.

Typical recovery: `SURGICAL_EDIT`.

### REGIONAL
One important region/axis is wrong, or several minor issues combine.

Typical recovery: `TARGETED_EDIT` or `TARGETED_REGENERATION`.

### GLOBAL
Primary subject, identity, narrative fact, required structure, or several major axes are wrong.

Typical recovery: `RECONSTRUCT_FROM_AUTHORITY`.

Do not try to inpaint an entirely wrong scene into the requested scene merely because it is cheaper than regeneration.

## 5. Causal hypotheses

Doctor may suggest, never assert without evidence:

- `PROMPT_PRIORITY_INVERSION` — style/decorative traits survived while core semantic claims failed.
- `REFERENCE_CONFLICT_OR_CONTAMINATION` — a reference leaked an ignored axis or two references fight.
- `REFERENCE_PRESSURE` — too many image conditions plausibly compete.
- `UNDERCONSTRAINED_REQUEST` — core content drift with no strong anchor/atomic constraints.
- `OVERCONSTRAINED_REQUEST` — too many simultaneous claims and references; model has no clear priority.
- `OUTPUT_CONTRACT_FAILURE` — format/alpha/PSD mechanics failed.
- `REPEATED_FAILURE` — the same gap signature survives multiple attempts.

Hypothesis confidence is advisory. Recovery decisions are driven primarily by failed claims and scope.

## 6. Doctor commands

```bash
python3 "$SCENE_ASSET_COMPILER" audit-template <run-dir> <task-id> <attempt-no>
# Codex fills output-audit.json after inspecting real output
python3 "$SCENE_ASSET_COMPILER" validate-audit <run-dir> <task-id> <attempt-no>
python3 "$SCENE_ASSET_COMPILER" doctor <run-dir> <task-id> <attempt-no>
python3 "$SCENE_ASSET_COMPILER" reconstruction-plan <run-dir> <task-id> <attempt-no>
```

Read `doctor-report.json` before deciding whether to edit or regenerate.

## Executor context contamination

If the generated image contains workflow dashboards, code, JSON, project names, QA panels, or other meta-context not present in the visual contract, record `EXECUTOR_CONTEXT_LEAKAGE:` in `global_observations`. Treat this as an execution-boundary failure, not ordinary artistic drift. Reconstruct from authority with the task-scoped `generation-capsule.md` as the only visual brief.
