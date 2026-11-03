# v0.10 Render Contracts

`render_contract` is an optional task-level contract. It keeps capture, genre, and
semantic responsibility orthogonal:

- `capture_profile`: `NONE` or `PHOTOREAL`.
- `genre_profile`: `PROJECT_STYLE`, `PHOTOREAL`, `JAPANESE_MANGA`, `FANTASY`, or
  `PICTURE_BOOK`.
- `semantic_role`: `SCENE`, `BACKGROUND`, `OBJECT`, or `CHARACTER`.

`PHOTOREAL` genre requires `PHOTOREAL` capture. An optional `wireframe_layout` is
allowed only for `SCENE` or `BACKGROUND` and must contain at least one slot. Each
slot has a stable ID, normalized bounds, intended aspect ratio, content intent,
normalized focal point, crop-safe inset, z-order, and layout tolerance.

JSON Schema is the structural gate. `validate_render_contract` is the canonical
semantic gate for relationships Draft 2020-12 cannot express directly: bounds must
remain inside the normalized canvas, focal points must remain inside their slot,
opposing crop-safe insets cannot exceed the slot, and slot IDs must be unique. The
release tests intentionally include schema-pass/runtime-fail cases for these rules.

Wireframe contracts may declare the exact `label_guide_policy` enum `REQUIRED`,
`PRESERVE_IF_PRESENT`, or `FORBIDDEN`; omission defaults to `PRESERVE_IF_PRESENT`
without waiving evidence. Every wireframe visual review records one structured
layout-evidence entry per declared slot: source and output SHA-256 bindings,
observed bounds, rotation, exact z-order, focal/crop result, separate source/output
label and guide states, and tolerance verdict. `PRESERVE_IF_PRESENT` requires the
source/output states to match; the other policies constrain the output state.
Evidence is bound to the attempt's resolved source and returned output hashes;
an otherwise positive layout result is rejected when its measured evidence does
not satisfy the contract. Policy-present and policy-omitted wireframes both require
this evidence. Non-layout v0.9 review payloads retain their legacy compatibility path.

The prompt compiler emits deterministic capture, genre, role, reference-boundary,
and layout lines. A STYLE reference may preserve only these structured axes:
`line_weight`, `ink`, `value_grouping`, `screentone`, `palette`, `edge_language`,
`shading`, `texture`, `material_response`, `lighting_language`, `camera_language`,
`detail_density`, and `brushwork`. Its `ignore` entries remain freeform. Preserve
and ignore cannot overlap, and subject, face, identity, layout, composition,
content, pose, costume, and environment terms are invalid STYLE preserve axes.

For a layout task, the scene graph stages `LAYOUT_LOCK`, one independent
`PLACEHOLDER_PHOTO` unit per slot, one reviewed `PLACEHOLDER_LOCK` per slot, and a
sealed `PLACEHOLDER_SET_LOCK`. Semantic `LAYER` and `PSD_PACKAGING` units depend on
both layout locks. Asset sealing and PSD entry re-hash the layout, every placeholder,
their reviews, and the aggregate lock; missing or drifted evidence fails closed.
The placeholder photos are generated independently by the eligible selected executor; SceneAssetCompiler never
creates pixels or assembles a PSD locally.

CLI lifecycle: `scene-lock ... LAYOUT_LOCK`, `scene-placeholder-lock`,
`scene-seal-placeholders`, `scene-layer-lock`, `scene-seal-assets`, then
`scene-psd-entry-check` before handing the PSD packaging capsule to `$imagegen`.

A real editable PSD remains a real requested capability. A particular invocation
may return `PREFERRED_OUTPUT_MISSING`; that is a format/package failure, not a
visual failure, and is never a global unsupported declaration or live-success
proof. The runtime permits at most one PSD-only packaging retry after that
result. Its raster substrate may be a primary or preview output, but must have an
APPROVED visual review bound to that exact SHA; wireframe layout evidence must bind
the same output. The retry execution receipt exactly matches the attempt retry lock,
including retry index, maximum, substrate SHA, and preservation flags. Accepting a
returned retry PSD requires exactly one PRIMARY PSD plus a contained, hash-bound
submitted comparison attestation with `evidence_level=SUBMITTER_ATTESTED`, binding
substrate, PRIMARY PSD, and PNG render-back hashes. Its declared metrics must state
canvas and alpha match, pixel exactness, zero changed pixels, and zero maximum channel
delta. SceneAssetCompiler validates internal consistency and current file/hash binding only; the
reviewer/tool strings and receipt hash are integrity/audit metadata, not authenticated
provenance or proof that an external tool ran. SceneAssetCompiler does not render the PSD locally,
and actual external PSD/render-back comparison remains `[UNVERIFIED]`. A second missing
PSD stops the packaging path while retaining both attempts' previews and evidence.

Legacy v0.9 tasks remain valid with the exact ten visual-review axes. A task with
an explicit render contract adds required, non-`NA` `genre_fidelity`, and adds
non-`NA` `layout_fidelity` when a wireframe layout is present.
