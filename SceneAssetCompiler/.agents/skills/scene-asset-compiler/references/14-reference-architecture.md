# Reference Architecture

## Default reference diet
Prefer the smallest sufficient set:
1. approved IDENTITY anchor,
2. STYLE anchor only when textual visual language is insufficient,
3. POSE/COMPOSITION reference only when spatial control materially matters,
4. one COSTUME/PROP reference when that object must be exact.

More references increase cross-contamination risk and review complexity. Add a reference only if it controls a distinct axis.

## Source and role are separate
For every reference record distinguish:
- `source_kind=FILE` — an explicit user/project reference copied and SHA-bound at compile time,
- `source_kind=TASK_OUTPUT` — an approved PRIMARY artifact from an upstream SceneAssetCompiler task.

Then declare its semantic role independently: IDENTITY, STYLE, POSE, COSTUME, PROP, COMPOSITION, ENVIRONMENT, or MATERIAL, plus what to preserve and ignore. A workflow dependency is not implicitly a visual reference.

Example:
```text
Image 1 — IDENTITY: preserve face geometry, silhouette, hair, marks; ignore pose/background.
Image 2 — STYLE: preserve line_weight, shading, texture, palette; ignore identity/composition/content.
Image 3 — POSE: preserve body pose only; ignore face, costume, style.
```

## Authority precedence
When references conflict:
1. Narrative/VisualAuthority facts win.
2. Approved generated anchor wins over an older candidate.
3. Role-specific preserve fields win only for their axis.
4. Prompt prose cannot silently redefine authority.

## Character sheets
Community practice strongly favors character/master sheets for consistency. SceneAssetCompiler uses them as **review and anchor artifacts**, not as an excuse to generate every production view in one low-resolution board. For high-value characters:
- lock one canonical identity anchor,
- derive additional approved views/expressions individually when needed,
- assemble a sheet for human/Codex review,
- use the most relevant approved view as the shot reference.

This preserves local repairability and resolution while retaining the consistency benefits of a sheet.
