# Image prompt contract

## 1. Authority

`ImagePromptRequest` is the public typed contract. `promptgen-core` validates it and deterministically renders the model prompt. Catalog directives may refine an outcome but cannot override task mode, references, exact text, dimensions, quality, or safety fields.

## 2. Taxonomy v4

`taxonomy.category` is exactly one of C1, C4, C5, C6, C10, or C11. `taxonomy.patterns` must be empty. Medium, composition, lighting, palette, surfaces, text, and visual style are typed fields.

## 3. Task modes

- `generate`: no edit-only reference role or change contract.
- `edit`: exactly one base image plus explicit `change_only` and `preserve`.
- `composite`: one unambiguous base and at least one additional role-bound reference plus explicit change contract.

`render_profile=pose_transfer` is a workflow category layered over `composite`; it is not a seventh visual-output taxonomy card. It requires exactly one `role=pose` reference and one `role=subject` reference. The pose reference owns visible body geometry, facing, gaze, camera, and occlusion. The subject reference owns identity, face, expression, proportions, hair, wardrobe, and rendering style. See `POSE_TRANSFER_PROMPT_SPEC.md`.

`render_profile=cinematic_storyboard` is a text-to-image C10 execution profile, not a seventh visual-output taxonomy card. It requires an illustration, no references or image text, and a typed `storyboard` with a locked layout, screen direction, identity anchors, and ordered camera/action/gaze panels. See `CINEMATIC_STORYBOARD_PROFILE.md`.

`consistency` is an optional cross-image contract. It adopts the four
provider-neutral dimensions from [ATH-MaaS/Awesome-Multi-Image-Generation](https://github.com/ATH-MaaS/Awesome-Multi-Image-Generation):
`multi_view`, `character`, `temporal`, and `semantic`. A declared dimension is
rendered as an observable QA rule, not as a claim that a particular model or
method can guarantee it. `cinematic_storyboard` requires `character`,
`temporal`, and `semantic`, and its sequence must match panel IDs. See
`MULTI_IMAGE_CONSISTENCY.md`.

Reference indices are contiguous and every reference has one responsibility.

### App icon master concept

`profile=app_icon` is category-bound to C4 and is generate-only. It requires a
non-photo medium (`graphic_design`, `illustration`, `3d`, or `mixed`), exactly
one typed subject with `count=1` in the center zone, no `text_elements`, an
opaque background, `format=png`, and exactly 1024×1024 output. Rendering states
the contract in this order: product purpose → core metaphor and component
geometry → centered composition with simple 2–3 visual planes → 32px silhouette
check. Text, letters, numerals, pseudo-writing, UI screenshots, device mockups,
rounded-square masks, app-icon frames, and watermarks are prohibited. The
result is one opaque PNG master concept only; SVG/vector/editable layers and
platform packages are non-goals.

For `profile=app_web_ui` with empty `text_elements`, the prompt describes only
visual structure, navigation, state, and component relationships for app, web,
or agent screens. It omits title/label/data/chart typography directives and
rejects readable text, numerals, pseudo-writing, invented data, and charts. A
non-empty exact-text contract preserves the existing text path.

## 4. Rendered order

A structured prompt renders compiler-owned sections in this order:

1. input references;
2. change contract;
3. pose fidelity contract when `render_profile=pose_transfer`;
4. textless cinematic storyboard contract when `render_profile=cinematic_storyboard`;
5. multi-image consistency contract when `consistency` is present;
6. scene;
7. subjects;
8. composition and space;
9. lighting, color, surfaces, exact text, constraints, and output.

User text is quoted as data and cannot create a compiler-owned section.

## 5. Exact text

Exact copy is rendered as ordered quoted lines, not machine labels:

```text
1. "URBAN"
2. "SIGNAL"
```

Line order, spelling, placement, family, size, color, and optional spelling hint stay separate. If no text is requested, the prompt explicitly excludes readable text, logos, and watermarks.

## 6. Hard constraints

Observable counts, panel counts, identity anchors, placements, and exclusions belong in subjects, composition, and `constraints.required_elements`. High-density C5/C6/C10/C11 outputs default to high quality. Validation rejects unknown categories, parallel patterns, invalid palette proportions, ambiguous references, unsafe dimensions, and conflicting change/preserve instructions before generation.

For a multi-instance subject, placement is the meaningful group region in which every
reported instance center must lie; it is not a claim of pixel-perfect packing. Use a
region large enough for the declared count, scale, spacing, and pose. If the arrangement
itself matters, state it observably (for example, a loose 3×3 cluster) instead of encoding
false precision in an extremely narrow box.

## 7. Output

The supported execution profile is `gpt-image-2`, PNG, opaque background, a declared aspect ratio, dimensions no larger than 3840 on either edge, and quality `auto|low|medium|high`.

## 8. Adapter evidence

Codex execution receives the compiled prompt as JSON data under a read-only sandbox. The receipt binds compiled prompt SHA-256 and character count to artifact SHA-256, dimensions, model parameters, allowed built-in skill reads, attempts, and fidelity checks. Exact counts and every reported subject center are rechecked against the typed request with a two-percentage-point visual-measurement tolerance; a model-reported pass cannot override that comparison. Unknown tools, arbitrary shell commands, extra artifacts, malformed PNGs, or failed hard constraints prevent publication.

The CLI's direct `openai-image-edit` adapter is intentionally narrower: it accepts
only one `task_mode=edit` request with `references[0].index=1` and
`references[0].role=base`, plus one `--reference-image FILE`. It sends one
multipart `POST /v1/images/edits` request to OpenAI with `gpt-image-2`, binds the
compiled prompt and copied reference by content hash, and publishes only one
strictly validated PNG at the declared dimensions. The API key is read only from
`OPENAI_API_KEY`; it is not placed in argv or the receipt. Generate, composite,
pose-transfer, multiple-reference, mask, Web, and MCP reference transport are
rejected rather than silently approximated.

## 9. Verification

```bash
cargo test -p promptgen-core
cargo test -p promptgen-codex
python3 scripts/renew-image-catalog.py --check
python3 scripts/audit-prompts.py --check
```
