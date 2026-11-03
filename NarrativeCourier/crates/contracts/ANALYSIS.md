# NVC contracts crate — module analysis

## Verdict·책임 경계

**IMPROVE organization / MAINTAIN wire contracts.** 이 crate는 NVC가 생산하는 artifact와 다섯 canonical delivery DTO의 Rust serialization 및 cross-field invariants를 소유한다. State, disk persistence, workflow transition은 소유하지 않는다. `lib.rs`는 schema version과 canonical crate-root type/validator public entrypoints를 소유한다.

## Module ownership

| Module | 소유 책임 | 경계 |
|---|---|---|
| `artifact.rs` | 공통 `ArtifactRef`와 path/hash 검증 | 파일 bytes를 읽거나 artifact를 게시하지 않음 |
| `narrative.rs` | `NarrativePackage`와 authority refs | STS NarrativePackage의 의미·저장 state를 새로 결정하지 않음 |
| `scene_map.rs` | DTO/validator private module composition; crate root exports canonical DTOs, validator, and `is_valid_scene_id` | owner wildcard export 없음 |
| `scene_map/types.rs` | `NarrativeSceneMap`, `SemanticManifest`, `SemanticEntry`, `SourceLocator` wire DTOs | one canonical Rust name per DTO; serde names·field shapes 유지 |
| `scene_map/validation.rs` | canonical `SCN-###` ID grammar and map identity validator orchestration | STS heading/blueprint projection and contract scene/semantic validation share one rule |
| `scene_map/validation/scenes.rs` | ordered scene IDs, scene fields, dialogue/silent, asset requirement structure | scene DTO의 구조만 검증하며 story semantic authority를 만들지 않음 |
| `scene_map/validation/semantic_manifest.rs` | semantic-manifest validation orchestration, scene lookup, occurrence coverage, and entry/map hash sequencing | preserve validation order and current scene-byte binding |
| `scene_map/validation/semantic_manifest/entries.rs` | semantic ID/kind grammar, source-locator bounds, and hash-string shape | entry-local contract only; cross-entry uniqueness and occurrence coverage remain with the parent validator |
| `synopsis_handoff.rs` | reviewed synopsis receipt와 body-bound identity | 사람 승인·작성 state를 발행하지 않음 |
| `release_bundle.rs` | Completed release bundle identity와 fixed artifact refs | Completed state를 만들거나 복구하지 않음 |
| `common.rs` | 빈 값·SHA·ID·ref uniqueness와 optional present-typed deserialization primitive | DTO별 field policy를 결정하지 않음 |
| `asset_types.rs` | asset kind, generation mode, source, request value | provider 실행이나 approval을 소유하지 않음 |
| `request.rs` | DeliveryRequest DTOs and public validator re-export facade | crate::request type/function paths remain available |
| `request/validation.rs` | request identity/ref/optional-string admission and validator composition | no rendering or capability fallback |
| `request/validation/assets.rs` | asset-input IDs, refs, mode, source path/hash/MIME | no materialization or file reads |
| `request/validation/outputs.rs` | positive fields, explicit required options and exact ratio arithmetic | generic structural admission; capability policy remains Delivery-owned |
| `director.rs` | stable DirectorPlan/ProjectionUnit DTO and crate-local public validator facade | existing type and validator import paths remain available |
| `director/validation.rs` | request-bound hashes, grammar, and validator composition | contract validation only; no narrative rewrite or render effect |
| `director/validation/assets.rs` | plan-local source provenance and asset IDs | borrowed ID set; no materializer policy |
| `director/validation/deliveries.rs` | output binding, actual unit order, lineage and checked timing | no sorting/reinterpretation of the plan |
| `asset_index.rs` | AssetIndex가 DirectorPlan AssetRequest를 정확히 한 번 materialize하는 규칙 | 자산 파일 자체를 소유하지 않음 |
| `delivery_set.rs` | Verification/DeliveryReceipt/DeliverySet aggregation | technical PASS를 human approval로 승격하지 않음 |

`NarrativePackage`, `DeliveryRequest`, `DirectorPlan`, `AssetIndex`, `DeliverySet`은 서로 다른 contract다. SceneMap, release bundle, reviewed synopsis handoff도 producer별 NVC artifact로 남는다. 하나의 polymorphic package/request/state machine으로 합치지 않는다.

## 2026-09-29 semantic DTO alias clean break — source-only

Removed five unused Rust type aliases that gave `SemanticManifest`, `SemanticEntry`, and
`SourceLocator` duplicate `Narrative*`/`*Entry` names. STS and the contract validator already use the
canonical names. The crate root continues to export those canonical DTOs and the same validator;
JSON field names, schema identity, serialized bytes, owner boundary, and validation order are
unchanged. This source edit has not been built or otherwise qualified.

## 공개 surface·보존 계약

`lib.rs` exports the version/schema IDs and canonical DTO/validator entrypoints. DTO fields, declaration
order, `serde` rename/default/skip/`deny_unknown_fields`, validator errors, hash identity, and path
constraints stay with their owner modules. Consumers use the crate-root contract entrypoint, not
private module paths or duplicate type aliases.

`NarrativeSceneMap` validator는 scene field invariants와 semantic-manifest lineage를 별도 private modules로 나눴다. `scene_map.rs`→`validation.rs`→child validators composition만 추가했고 public `validate_narrative_scene_map` entrypoint와 same contract owner를 유지한다. Source-only diff; no schema/wire changes.

## Current source-only slice — lowercase SHA-256 grammar

Raw SHA-256 and prefixed hash/identity values now share one private lowercase-hex predicate. NarrativeSceneMap `map_id`, reviewed-synopsis `handoff_id`, release-bundle `bundle_id`, and semantic-entry hash fields delegate to the same prefix-plus-digest owner; each validator retains its existing field-specific error and body-identity check. DTO names, JSON schemas, serialized bytes, validation order, and crate-root exports are unchanged. Tests/build/fmt/lint/CLI/runtime/manifest qualification are **NOT_RUN**.

## Optional DTO values preserve schema types — source-only

Every `Option<T>` contract field uses the shared present-typed decoder: an absent field retains its
Serde default, a present value must deserialize as `T`, and `null` is rejected rather than normalized
to absence. This covers artifact and asset MIME, asset-request source/consistency, projection-unit
content/timing, reviewed-synopsis reason, semantic source-locator character/dialogue index, and
DeliveryRequest/OutputRequest option fields. The producer schemas already describe those values as
non-null types, so schema bytes and valid serialized values are unchanged. `invariants.rs` adds
representative explicit-null regression cases; they were not executed. Tests/build/fmt/lint/schema/
CLI/runtime/manifest qualification are **NOT_RUN**.

## Current·미확인

The prior NVC workspace gate and root integration gate describe the graph before the latest private
STS and semantic-manifest owner edits. The semantic-manifest entry grammar now lives in a private
child module; occurrence uniqueness, coverage, and hash sequencing remain in the existing validator
facade. No schema, DTO, state owner, or public validator path moved. These latest source edits have
not been qualified by build, tests, lint, CLI, or runtime.

## AssetIndex identity/provenance admission — source-only closure

The contracts validator binds AssetIndex to the exact supplied DirectorPlan JCS hash and its
NarrativePackage hash, rejects duplicate/invalid plan asset IDs before map construction, and requires
each materialized asset ID to equal its source request ID. This prevents a complete source-request
set from disguising renamed or swapped materialized identities. Wire DTOs and producer schema bytes
remain unchanged. Provider fingerprint policy stays outside the generic contract.

Delivery's internal-copy:v3 owns persisted index admission: full INDEX-plan identity, concrete
materializer fingerprint, USER/DERIVED source hash and declared MIME, canonical digest/codec blob path,
and bounded captured actual image bytes. The one read_materialized_image owner serves persisted
admission and HTML representation; repeated metadata/hash reads and the old read_render_image name
are removed. The unimplemented non-image copy branch is retired instead of materializing assets that
no output can represent. Release composition delegates to this owner and drops hash comparisons
already guaranteed by the AssetIndex validator. Historical user files are retained; prior materializer
indexes need an explicit new request rather than a compatibility/fallback branch. Source guard
consumers reflect current full identities, immutable-blob failure, and captured image admission but
were not executed. Tests/build/fmt/lint/runtime/media probes and manifest qualification are **NOT_RUN**.

## DeliverySet receipt/status aggregation — source-only closure

DeliverySet::aggregate_status is the single nonempty, failure-first aggregate owner consumed by the
contract validator, Delivery run, and Human review. The validator binds the exact supplied request
JCS/narrative hashes and maps each receipt to its requested format and pinned backend. Receipt lineage
must be nonempty and contain every required request reference; findings must contain meaningful text.
Any failed verification layer requires BLOCKED, and BLOCKED requires an explicit layer failure.
Existing false-PASS guards remain; warnings continue to require review. The producer DeliverySet schema
now expresses nonempty lineage/findings, failure/BLOCKED equivalence, and total aggregate precedence.
DTO field shapes and schema identifiers are unchanged; no DeliverySet schema copy is registered in
the integration byte-equality registry. Historical blueprint fixtures/schemas remain reference inputs.

Load composition removes request/narrative checks now guaranteed by the producer contract while
retaining plan/index/set chain proofs. Human review still requires current release/artifact admission
and cannot approve failed technical/semantic results; post-write admission additionally requires the
persisted set to equal the intended reviewed set. No approval or user project mutation was executed.
Tests/build/fmt/lint/schema validator/runtime and source-manifest qualification are **NOT_RUN**.

## DirectorPlan request binding/order/timing — source-only closure

The producer contract binds exact DeliveryRequest JCS/narrative hashes before plan admission.
Private asset-request and delivery-unit validators have distinct owners below director/validation;
the public validator and DTO paths remain stable. Unit order must equal the actual vector position
from zero rather than merely form a sortable contiguous set. Global identity checks subsume the old
per-output duplicate pass. Timed frame sums use checked u64 arithmetic and explicit requested duration;
video requires TIMED structure and timed delivery requires requested FPS. Producer schema order/frame
fields now have their Rust u32 upper bounds. The existing dangling-asset fixture uses the actual
request hash so its source setup still targets the intended invariant; it was not executed.

Deterministic Delivery construction and preflight share FrameAllocation and the output-structure owner.
Preflight requires the exact deterministic frame distribution and rejects static FPS/duration options
instead of ignoring them. Full PLAN digest replaces the clipped identity, and Director bundle v4 binds
identity/order/timing policy. Generic contracts remain distinct from deterministic planner capability
policy. Load composition removes hash comparisons now guaranteed by the DirectorPlan validator while
retaining set/plan/index chain proofs. Source-guard file consumers include the new timing owner; no guard
was run. Stored prior-bundle plans remain explicit stale results; user files are retained. Tests/build/
fmt/lint/schema validators/runtime/media probes and manifest qualification are **NOT_RUN**.

## DeliveryRequest output-option/input surface — source-only closure

Request validation now composes private asset-input and output-field owners. Positive scalar checks
are separate from presence requirements, eliminating repeated video positivity checks; rational FPS
errors describe positive numerator/denominator rather than incorrectly claiming a 1/1 minimum. Exact
aspect-ratio arithmetic and source/path/ref admission remain intact. Present audience/locale must be
nonempty. Request optional fields and AssetSource.mime deserialize only their actual declared type;
missing fields remain omitted defaults, while explicit null is no longer silently treated as absence.
The producer schema adds nonempty adaptation values and Rust u32 upper bounds, and removes the unused
ko-KR default annotation. Optional JSON types already disallow null; wire field names remain unchanged.

Deterministic capability policy rejects unimplemented formats and HTML/JSON/PDF raster geometry, plus
FPS/duration on static outputs, before discovery/lease/materialization effects. Supplied adaptation
values are explicit unsupported capabilities; empty values have no fallback. Redundant generation-mode
checks stay with the generic contract, and timing allocation consumes admitted options without a
second static-option policy. Director bundle v5 records this clean break; prior bundle results remain
stale without migration. Existing UI request construction omits an unselected backend directly and
checks positive safe u32 integers before serializing numeric input, replacing blanket null deletion.
No new product screen was added. CLI/MCP/Tauri continue through the typed request boundary. Schema
identifiers and registered integration pins are unchanged; no user project/data was altered. Tests/
build/fmt/lint/schema validators/CLI/UI/runtime and source-manifest qualification are **NOT_RUN**.

## Artifact/reference and NarrativePackage wire boundary — source-only closure

ArtifactRef.mime uses the same present-typed optional decoder as the request boundary: omission remains
valid, explicit null is rejected according to the existing schema. NarrativePackage admission now
requires its existing nvc2_<lowercase SHA-256> grammar and exact serialized body identity with package_id
omitted. The contracts owner exposes derived_id and an immutable named artifact inventory consumed
by both contract validation and Delivery file admission. STS package construction delegates identity
to that public contract entrypoint rather than reproducing the JSON field-removal rule.

NvcReleaseBundle likewise owns derived_id, requires its schema-declared nvc2 package ID, and binds
package_sha256 to the package artifact reference. STS bundle generation uses the same contract method.
A private object-body identity primitive removes duplicate serialization/hash grammar without merging
producer DTOs or state owners. SMC already consumes the same omitted-field nvc2 identity; its schema
copy and integration pins require no byte change. Producer schema bytes, field names, paths, and valid
serialized IDs are preserved. Malformed IDs/unbound bodies have no compatibility branch.

Delivery JSON storage offers one captured value/bytes read; canonical package and completed-state
admission hash the same bytes that were parsed, replacing duplicate open/parse sequences. The source
guard and existing simulated fixtures use current identity entrypoints/grammar; no guard or fixture
was executed. Existing user projects were not read or modified. Filesystem replacement outside the
Story-owned lock model remains unqualified. Tests/build/fmt/lint/schema validators/CLI/runtime and
source-manifest qualification are **NOT_RUN**.
