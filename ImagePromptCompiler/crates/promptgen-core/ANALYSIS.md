# promptgen-core — module ownership

## Verdict and boundary

`promptgen-core` is the deterministic semantic library shared by the image-only CLI, MCP, and Web
adapters. It owns typed request decoding, interview decisions, image schema and field invariants,
diagnostics, catalog selection, prompt rendering, and structured generation metadata. It does not own
filesystem state, network calls, provider credentials, or persistent story authority.

`lib.rs` remains the crate facade for the established `PromptKind`, compilation, image, interview,
catalog, and JSON APIs. The module split below keeps those public paths and serialized field names in
their current owners. `validate_image_request` remains public as documented; raw LUT resolution
types and helpers are implementation details rather than a second image API.

## Module map

| Module | Owner |
|---|---|
| `image/common.rs` | Image-local typed-decoding and prompt-text projections; helpers stay crate-private. `PromptLanguage` is exported from the image module as the nameable type of `ImagePromptRequest::language`. |
| `json.rs` / `json/{value,parser,decode}.rs` | Strict JSON value model, parser and typed field-decoding primitives. |
| `diagnostic.rs` | Prompt kind, compilation status, diagnostics and compilation outcome. |
| `image/model.rs` / `image/model/*.rs` | DTO facade and nested scene, visual, continuity, and request-part owners. The facade keeps only JSON helpers shared by multiple DTO modules; scene, visual, and request-part helpers stay with their single consuming owner. |
| `image/schema.rs` / `image/schema/*.rs` | Public image schema entrypoint and field/storyboard/contract schema projection. |
| `image/lut.rs` | Crate-owned resolved LUT data and selection logic; only the JSON preset listing is public, while preset names and the resolver remain internal. |
| `image/validate.rs` / `image/validate/*.rs` | Pure request validation orchestration and task, consistency, taxonomy, free-text, profile and field-specific rules. |
| `image/validate/{fields,profiles,profile_text}/mod.rs` | Restricted child-module declarations only; orchestration and sibling validators import their defining field/profile/text owners directly. |
| `image/render.rs` | Stable render-profile dispatch, shared text formatting, and the exact top-level prompt section order. |
| `image/render/scene_sections.rs` | Scene, subject, and composition section projection. |
| `image/render/capture_sections.rs` | Camera, lighting, color/LUT, and material section projection. |
| `image/render/constraints.rs` | Required/omitted elements, safety clauses, and generation-output parameters. |
| `image/render/exact_text.rs`, `profiles.rs`, `editorial_flat.rs` | Exact-copy contract, profile-specific clauses, and bounded editorial-flat rendering. |
| `image/generation_plan.rs` | Structured generation/QA metadata derived from an already valid typed request. |
| `interview/mod.rs` | Stable interview facade, image-flow dispatch and shared answer normalization. |
| `interview/contracts.rs` | Interview request, question, option, status, ready/outcome values, their JSON projections, and the matching request schema. |
| `interview/image.rs` / `interview/image/{flow,fields,questions,route_decisions,text_contract,visual_defaults}.rs` | Image interview composition entry, stateful flow, routing, question construction, and answer contracts. |
| `interview/image/builder.rs` / `builder/{camera_lighting,color,constraints,geometry,subjects,text}.rs` | Stable typed-request assembly and focused capture, color, request-constraint, spatial-composition, typed-subject, and exact-copy projections. |
| `interview/image/inference.rs` / `inference/{defaults,fields,routing}.rs` | Stable inference composition facade and caller-specific default, field, and routing projections. |
| `inference/fields/{spatial,visual_attributes,graphics}.rs` | Directly declared by `inference.rs`; scene/composition, surface/wardrobe/lighting, and text/palette inference owners with no intermediate re-export module. |
| `interview/image/inference/subjects/{parser,panels,appearance,lexicon,text,counts}.rs` | Private subject inference owners declared directly by `inference.rs`; no intermediate `subjects.rs` re-export facade. They own phrase parsing, repeated-panel identity, finish/material parsing, shared vocabulary/text matching, and quantified-count candidates. |
| `interview/image/route_decisions/mod.rs` | Private route-decision composition only; no operation re-exports. |
| `interview/image/route_decisions/route_inference/mod.rs` | Private brief-inference module composition only. |
| `interview/image/route_decisions/route_inference/{route,stated_fields,profile,content}.rs` | Language/category selection, direct stated-field extraction, typed profile inference, and category-bound content completion respectively. |
| `interview/image/route_decisions/category_contracts/mod.rs` | Private category-route module composition only. |
| `interview/image/route_decisions/category_contracts/{answer_pruning,category,profile,questions}.rs` | Route-specific stale-answer pruning, category contract data/projection, profile/medium compatibility, and explicit user questions. |
| `catalog.rs` / `catalog/*.rs` | Public catalog data/lookup and canonical family parser (`CatalogFamily::parse`), used by the MCP family filter; `model.rs` owns DTOs, `store.rs` owns source decode/index, `validate.rs` owns catalog invariants, and selection diagnostics, derived prompt metadata, and render-only English display formatting remain crate-internal. |
| `image_artifact/png.rs` / `image_artifact/png/deflate.rs` | Bounded PNG chunk/scanline and zlib/DEFLATE reading for returned artifacts. |

Inference composition, defaults, and routing now import only their enumerated parent/helper symbols;
wildcard imports no longer make future image-interview additions implicit dependencies.

Image model leaf modules also import only their JSON and sibling-owner dependencies. `model.rs` keeps
shared u8 decoding and optional-string projection; scene serialization owns `optional_fields`, visual
DTOs own optional unsigned-number and float projection, and `ImageGenerationParameters` owns its
aspect-ratio reduction. DTO fields, serialized keys/order, numeric bounds, and decode errors are
unchanged.

Typed-subject and request-constraint builder leaves import answer, inference, geometry, route-contract,
and JSON owners directly; the aggregate builder remains the composition point, not their implicit
dependency namespace.

The exact-copy, profile-contract, and editorial-flat render leaves import only their model values and
render/catalog helpers from the owners that define them. Render dispatch and section order stay in
`render.rs`; prompt text, profile clauses, budget policy, and selection behavior remain unchanged.

Interview request assembly now names its answer/request, inference, JSON, and child-projection owners
explicitly. Typed-subject and request-constraint projections import only their geometry, inference,
route-contract, answer, and JSON dependencies; the builder facade no longer supplies them implicitly.

`interview/image.rs` now keeps only its route constants and `run` entrypoint. Its inference, question,
route, text-contract, and visual-default imports are owned by their actual flow consumers rather than
re-exported from the image facade. Interview tests import only the helpers and request values they use.

Image-schema composition now imports only the schemas it assembles, while field and storyboard
projections import only the shared schema constructors they call. The JSON Schema remains owned by the
same public `image_schema` entrypoint.

## 2026-09-30 Catalog family clean break — source-only

`CatalogFamily::parse` remains public because the MCP catalog family filter calls the core-owned parser;
it accepts only the eight canonical values emitted by the embedded catalog. Short forms and synonym
spellings were unused compatibility inputs and are no longer accepted.
`family` remains a grouping policy for supporting-direction cardinality and render-profile behavior;
primary-result routing continues to use `tier_1`. The embedded catalog and serialized fields are
unchanged. Tests/build/quality/provider execution were not run after this edit.

## Interview boundary

`InterviewRequest::from_json` rejects unknown fields, unsupported kinds, blank briefs, surrounding
whitespace in answer keys, and non-string answers. `InterviewOutcome` keeps request JSON, the typed
request value and compilation together; `ready()` yields them only for a ready outcome. `interview_schema`
describes the same request surface. `run_interview` delegates the currently supported image flow without
adding a second session or persistence owner.

One existing API edge remains explicit: `InterviewOutcome::completed` accepts an optional typed request
and can therefore construct `Ready` with `request_value=None`. `ready()` fails closed and returns `None`
unless request JSON, typed request, and compilation are all present. Callers should consume `ready()`
rather than infer executable readiness from the status string alone. Making the constructor require a
typed request would change its public API, so this structural move does not alter that contract.

The source-only refactor moves the interview wire/value contract out of `interview/mod.rs` while that
module remains the public facade. `promptgen_core::interview::{InterviewRequest, InterviewOutcome,
InterviewStatus, interview_schema}` and crate-root exports remain at their current paths. `InterviewReady`
and `InterviewRequestValue` remain available through the public `interview` module.

Brief inference and category-specific route policy are separated behind the existing private
`route_decisions` facade. The image flow's step order and public interview/question contracts are kept
at their existing owners; these structural moves have **NOT_RUN** tests/build/fmt/clippy/source-snapshot
qualification.

## Typed subject request projection — source-only

`builder.rs` retains normalized input decisions and final request-key ordering. Per-subject output objects
now have one private owner in `builder/subjects.rs`, which projects typed identity, count, action, appearance,
features, face/gaze/hair, placement, pose, and scale. The helper consumes the existing `ParsedSubject` value;
that type is re-exported only within the `interview::image` visibility boundary. JSON field names, array order,
category policy, and serialized values are intended to remain unchanged. No request/schema/provider contract
or interview transition moved. Tests, build, fmt/Clippy, prompt snapshots, and source-snapshot qualification
are **NOT_RUN** after this source-only slice.

## Composition geometry projection — source-only

`builder/geometry.rs` now owns the image's spatial composition bundle: viewpoint, camera angle, depth-layer
description, and text-aware negative space, alongside existing subject placement, repeated-panel placement,
and composition-balance helpers. `builder.rs` still owns when this bundle is selected and where its values
are serialized in the request object. Category/profile branching and the field sequence are preserved by
construction; tests/build/fmt/Clippy and prompt/source snapshot qualification are **NOT_RUN** after this move.

## Color request projection — source-only

`builder/color.rs` owns palette fallback and area allocation, photo-LUT admission, and the ordered color
constraint object. `builder.rs` invokes palette projection before lighting and assembles the final color object
after lighting, preserving the former pure-calculation sequence and existing top-level `color` field position.
Palette ordering, LUT eligibility, wording, and serialized keys are retained from the inline projection.
Tests/build/fmt/Clippy and prompt/source snapshot qualification are **NOT_RUN** after this move.

## Interview request constraints — source-only

`builder/constraints.rs` assembles the typed request's safety tier, excluded elements, original-character
policy, and category-specific required elements into the existing `constraints` object. It consumes the
already selected category, subject description, repeated-panel state, and safety tier; it does not own or
change the inference policy that selected those values. This is distinct from `image/render/constraints.rs`,
which writes prompt text from a completed typed request. The request object keys and required-element order
are intended to remain unchanged. Tests/build/fmt/Clippy and prompt/source snapshot qualification are
**NOT_RUN** after this move.

The request-constraint owner also formats the existing newline/semicolon-separated exclusion answer as a
JSON string array. That projection helper now stays beside the request field that consumes it instead of
being re-exported through inference defaults. Trimming, separators, empty-item filtering, and JSON element
order remain the same by construction; this source-only ownership adjustment is **NOT_RUN**.

## Structured prompt section owners — source-only

The structured renderer keeps `render_image_prompt` and `render.rs` as its existing internal entry and ordering owner. Scene/subject/composition, camera/lighting/color/material, and constraints/output section bodies moved behind three private section owners. `render_structured_prompt` still concatenates them in the same explicit order, with the exact-text section between composition and capture. Existing profile, exact-text, and editorial-flat owners remain. This source move does not change request/schema/provider contracts. Tests, build, fmt/Clippy, prompt snapshots, and source-identity qualification were not run under the current user instruction; previous workspace evidence predates this graph.

## Subject inference forwarding-facade removal — source-only

The established `inference.rs` composition facade still exposes the function set used by the image
builder. It now declares the six private files under `inference/subjects/` directly; the intermediate
`subjects.rs` re-export layer is removed. The subowners still separate phrase parsing, repeated-panel
identity, appearance terms, vocabulary, replacement, and counts. Tests, build, fmt/Clippy, prompt-flow
snapshots, and source-identity qualification are **NOT_RUN** after facade removal.

## Brief field inference owners — source-only

`inference/fields.rs` remains the facade imported by `inference.rs`. Scene and composition parsing,
surface/wardrobe/lighting extraction, and typography/palette parsing now live in private owners. The
existing exported function names and route caller sequence remain at their prior facade path. No
keyword set, search precedence, value normalization, or diagnostic contract was intentionally changed.
Tests, build, fmt/Clippy, prompt-flow snapshots, and source-identity qualification are **NOT_RUN** after
this move; prior evidence predates the field split.

## Gaps and evidence boundary

Only image interviews are supported; other `PromptKind` values are rejected. Core compilation and
interview results are deterministic values, not provider success, image quality, user approval, or story
state. The current source-only interview contract and route-decision splits have **NOT_RUN** crate
tests/build/fmt/clippy or provider/runtime qualification. Previous 263-test and quality-gate results
apply to the source graph before these module moves.

Raw SHA-256 buffered-byte projection is owned by `image_artifact/sha256.rs::digest_hex`.
Codex/OpenAI adapters call it directly; its streaming `Sha256` remains available for copy-bound
hashing. No provider state or execution was added to core. This change is NOT_RUN.
