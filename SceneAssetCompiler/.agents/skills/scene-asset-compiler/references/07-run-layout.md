# Run Layout and Immutability

```text
run/
├─ request.json
├─ run.json
├─ inputs/
├─ projection/
│  ├─ narrative-scene-projection.v3.json       # fixed NVC receipt bytes
│  └─ generations/<id>/scene-graph.json       # receipt-selected graph bytes
├─ authority/visual-authority.json
├─ plan/generation-plan.json
├─ tasks/
│  └─ TASK-x/
│     ├─ task.json                 # immutable
│     ├─ selection.json            # approved pointer only
│     └─ attempts/
│        ├─ 0001/
│        │  ├─ attempt.json
│        │  ├─ prompt.md
│        │  ├─ outputs/
│        │  ├─ result.json
│        │  ├─ mechanical-review.json
│        │  └─ visual-review.json
│        └─ 0002/...
└─ publish/
   ├─ blobs/<sha256>.<ext>
   ├─ asset-pack.json
   └─ MANIFEST.sha256

For an `SMC_NARRATIVE_SCENE_PROJECTION` run, the separate SMC release surface
adds `handoff/` only after asset release approval:

```text
handoff/
├─ asset-pack.v2.json
├─ asset-pack-audit.v1.json
├─ binding-manifest.v1.json
├─ scene-asset-release-approval.v1.json
└─ scene-motion-asset-handoff.v2.json       # final consumer pointer
```

The generic `publish/asset-pack.json` is not interchangeable with the final
SMC handoff. The receipt, graph, source/runtime paths, and every handoff JSON
are project-relative and hash-checked; a symlink, traversal path, non-regular
file, or changed byte fails closed.
```

Attempt directories are append-only. `selection.json` may only point to an attempt that has mechanical PASS + visual APPROVED. Publication re-hashes selected files. Status is reconstructed from files; no hidden in-memory state is authoritative.

## Diagnostic artifacts per attempt

```text
attempts/0001/
├─ attempt.json
├─ prompt.md
├─ outputs/
├─ result.json
├─ mechanical-review.json
├─ visual-review.json
├─ output-audit.json          # expected vs actual atomic claims
├─ metric-evidence.json       # optional, advisory
├─ doctor-report.json         # deterministic diagnosis
└─ reconstruction-plan.json  # next-attempt boundary
```

These files belong to the rejected/inspected attempt and remain immutable evidence after the next attempt is created.

`reconstruct` creates a sibling `attempts/0002/`; it never mutates `0001` output.
