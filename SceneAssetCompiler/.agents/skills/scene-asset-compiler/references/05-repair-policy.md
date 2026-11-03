# Targeted Repair Policy

The default is **edit the smallest failed scope**. Never regenerate all passing assets because one item fails.

- alpha/background failure → targeted canvas-fit correction only for reusable transparent assets (scale down, recenter, or extend the transparent canvas; preserve approved pixels and identity); if any content/visual failure coexists, separate it before creating a canvas-fit repair
- one identity invariant drift → edit that identity feature only
- broad identity collapse → regenerate that task only, anchored to approved identity
- style drift → edit style only; preserve identity/pose/structure
- missing PSD layer → add/correct only missing semantic layer if supported; otherwise regenerate that task's PSD only
- merged parts → separate those parts only
- articulation seam → extend hidden internal overlap only; do not change visible silhouette
- corrupt/unopenable artifact → regenerate that task only
- preferred PSD missing → one bounded PSD-only packaging retry from an approved visual-review raster substrate bound to its exact SHA; require the execution receipt to exactly match the attempt retry lock; never redraw, restyle, relight, recompose, or assemble a PSD locally; accept only one PRIMARY PSD with a contained `SUBMITTER_ATTESTED`, hash-bound submitted comparison attestation binding substrate/PSD/PNG render-back hashes and declaring exact zero-delta metrics; treat reviewer/tool strings and receipt hashes only as integrity/audit metadata, never authenticated provenance or proof that an external tool ran; stop after the second miss while retaining both attempts' previews/evidence

Every repair creates a new immutable attempt and retains the failed attempt as evidence.

## Doctor vs direct repair

Use direct `repair` only when all are true:
- failure is already understood,
- scope is clearly local,
- previous composition/identity/setting should be preserved,
- no reference conflict is suspected,
- failure has not repeated.

Use Doctor when any are true:
- core subject/identity/setting/narrative is wrong,
- multiple axes fail,
- reference contamination is plausible,
- the same failure repeats,
- a local edit could preserve the wrong global composition.

A global failure must not be converted into a giant list of local edits. Reconstruct from the earlier canonical boundary.
