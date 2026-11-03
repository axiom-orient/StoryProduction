# Failure Codes

| Code | Owner | Meaning | Default repair |
|---|---|---|---|
| NO_PRIMARY_OUTPUT | Mechanical | no project-bound primary result | regenerate task |
| HASH_MISMATCH | Mechanical | result manifest no longer matches file | stop; re-manifest only if file change was intentional |
| UNEXPECTED_FORMAT | Mechanical | task output contract violated | regenerate/convert only if deterministic conversion is valid |
| PNG_INVALID | Mechanical | invalid PNG bytes | regenerate task |
| ALPHA_MISSING | Mechanical | transparent output contract violated | edit background/alpha only |
| PSD_INVALID | Mechanical | PSD cannot be parsed | regenerate this PSD task |
| DUPLICATE_LAYER_ID | Mechanical | machine IDs ambiguous | rename/restructure only duplicates |
| REQUIRED_LAYER_MISSING | Mechanical | contracted semantic layer absent | add/repair only missing layer(s) |
| PSD_FALLBACK | Mechanical WARN | task requested PSD but allowed raster fallback | acceptable only when warnings policy permits; not PSD compliant |
| IDENTITY_DRIFT | Visual | identity invariant failed | edit failed trait(s); broad collapse regenerates task |
| STYLE_DRIFT | Visual | VisualAuthority language failed | edit only failed visual-language axis |
| NARRATIVE_MISMATCH | Visual | action/state/fact contradicts story authority | correct depicted story fact only |
| COMPOSITION_FAIL | Visual | first read/framing/depth/hierarchy failed | edit composition only |
| LIGHTING_COLOR_FAIL | Visual | motivated light/value/palette failed | edit light/color only |
| MATERIAL_FAIL | Visual | declared material/texture/surface response failed | edit only failed material cue while preserving abstraction level |
| STRUCTURE_FAIL | Visual | semantic separation/merge contamination | separate failed parts only |
| POSE_SEAM | Visual | articulation overlap insufficient | extend hidden joint overlap only |
| USABILITY_FAIL | Visual | silhouette/crop/readability/production-fit problem | edit only reported usability issue |

Unknown failure classes should be recorded explicitly and routed to a narrow Codex review before repair; do not map them silently to “regenerate everything.”

## 0.5 diagnostic hypothesis codes

These are **causal hypotheses**, not mechanical failure codes:

- `PROMPT_PRIORITY_INVERSION` — lower-priority style/detail survives while core semantic contract fails.
- `REFERENCE_CONFLICT_OR_CONTAMINATION` — reference appears to control an ignored axis or contradict another reference.
- `REFERENCE_PRESSURE` — many simultaneous references correlate with identity/style/composition drift.
- `UNDERCONSTRAINED_REQUEST` — global content drift without sufficient anchors/atomic constraints.
- `OVERCONSTRAINED_REQUEST` — too many independent controls compete and hierarchy is unclear.
- `OUTPUT_CONTRACT_FAILURE` — deterministic file/format/alpha/PSD failure dominates.
- `REPEATED_FAILURE` — same gap signature survives across attempts.

Do not report these as facts unless a controlled experiment confirms them.

- `EXECUTOR_CONTEXT_CONTAMINATION` — generated pixels reflect workflow/chat/meta-project context rather than the task-scoped visual capsule. Recovery: fresh reconstruction using only `generation-capsule.md` + explicit visual references; never edit the contaminated candidate.
