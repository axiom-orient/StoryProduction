# NVC Delivery — 구현 분석

입력·receipt 계약과 공통 process의 정본은 [contracts](../contracts/ANALYSIS.md)와 [Runtime](../runtime/ANALYSIS.md)에 둔다.

## Verdict·책임 경계

Released narrative의 deterministic delivery와 local publish/cancel authority. caller는 CLI/UI/MCP → DeliveryEngine::run/review/cancel. 방향 **MAINTAIN**. Latest full NVC gate는 rustfmt 차이로 exit 1이었지만 68 Rust tests와 앞선 Delivery/source checks, 별도 Clippy는 통과했다. 이후 Delivery planner/RunControl forwarding 제거와 서식 정리를 적용해 해당 source slice는 **NOT_RUN**이다. 외부 renderer·사람의 review는 미실행이다.

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | CLI/UI/MCP → DeliveryEngine context/run/review/load/list/cancel/read_artifact_bytes | DeliveryRequest + Completed NarrativePackage; DeliverySet + technical/human verification | 아래 owner 구분 | 정상 result 또는 명시적 오류 | `src/engine/{context,run,review,views}.rs`, `src/engine/release.rs` |

`DeliveryRequest`와 `DeliverySet`은 contracts crate가 소유한다. CLI/MCP/Tauri는 해당 crate에서 DTO를 직접 import하고, delivery crate는 자체 engine/backend/planner API만 노출한다.

Production consumers import canonical DTOs and validators from `narrative_courier_contracts`
through per-module name allowlists:

| Consumer | Explicit contract imports |
|---|---|
| `render` | `AssetIndex`, `DeliveryPlan`, `DeliveryReceipt`, `DeliveryStatus`, `DirectorPlan`, `OutputFormat`, `OutputRequest`, `PresentationStructure` |
| `planner::preflight` | `validate_director_plan`, `Coverage`, `DeliveryRequest`, `DirectorPlan`, `GenerationMode`, `ProjectionOperation` |
| `planner::plan` | `validate_delivery_request`, `validate_director_plan`, `validate_narrative_package`, `AssetRequest`, `DeliveryPlan`, `DeliveryRequest`, `DirectorPlan`, `GrammarAxis`, `NarrativePackage`, `OutputFormat`, `PresentationStructure`, `ProjectionOperation`, `ProjectionUnit` |
| `planner::plan::request_policy` | `AssetKind`, `Coverage`, `DeliveryIntent`, `DeliveryRequest`, `OutputFormat` |

These consumers no longer inherit the entire producer contract surface through wildcard imports.
This does not re-export or duplicate a contract and does not change the five DTO ownership boundary.
Tests, build, fmt, Clippy, CLI/runtime, and manifest verification were **NOT_RUN** after this
source-only import cleanup.

## 구현 모듈 경계

| 모듈 | 소유 책임 | 보존 경계 |
|---|---|---|
| `src/lib.rs` | Delivery-owned engine/backend and direct-owner public entrypoints for plan, preflight, scene, and backend descriptor operations | Public crate-root backend descriptor types remain reachable; contract DTOs remain owned by `narrative-courier-contracts` |
| `src/engine.rs` | `DeliveryEngine` composition and delivery result DTO ownership, including the active `ImportedDeliveryAsset` adapter result | existing crate-root result type path remains; STS narrative authority is not owned here |
| `src/engine/context.rs` | release-bound authoring context assembled from verified package, backend descriptors, and STS scenes | package freshness is checked before exposing the projection; backend capability descriptions remain owned by `backend/discovery.rs` |
| `src/engine/review.rs` | human review transition and persisted DeliverySet update | only explicit caller decision changes `NEEDS_REVIEW`/`BLOCKED` state |
| `src/engine/views.rs` | hash-bound DeliverySet load/list projections and bounded artifact byte capture | stale package and tampered artifacts are rejected; returned preview bytes are the same bytes checked against the receipt digest |
| `src/engine/assets/mod.rs` | private module declarations for image ingress and asset materialization | no compatibility re-export surface |
| `src/engine/assets/imported_image.rs` | import image byte/format/dimension/rights validation and `DeliveryEngine::import_asset` effect | `ImportedDeliveryAsset` DTO owner stays in `engine.rs`; candidate image is hash-pinned; semantic requirements remain with STS/DirectorPlan |
| `src/engine/assets/materialize.rs` | USER/DERIVED DirectorPlan asset→content-addressed blob/AssetIndex materialization | canonical `visual/assets/blobs/` 경계·hash/size verify 유지 |
| `src/engine/storage.rs` | project-owned path boundary와 canonical JSON file I/O | project boundary와 atomic write 보존 |
| `src/engine/release.rs` | NarrativePackage artifact/current-release proof와 persisted DeliverySet contract-chain verification | stale/hash drift는 fail-closed |
| `src/engine/run.rs` | request preflight, plan/assets/render composition, final DeliverySet commit sequence | released package/current hashes guard every canonical publish |
| `src/engine/run/lease.rs` | `.delivery-active`/`.delivery-cancel`/`.delivery-publishing` lifecycle, cancellation admission, final publish ownership claim, run cleanup | Delivery-local cancel-vs-publish race and temporary marker bytes remain one owner |
| `src/planner.rs` | private child-module composition only; operation re-exports 없음 | internal callers name plan/preflight/scene owners directly |
| `src/planner/scene.rs` | canonical screenplay에서 ordered `SCN-*` scene/content 추출 | exact source slice·duplicate rejection 유지 |
| `src/planner/plan.rs` | package/request binding, deterministic asset/output projection, DirectorPlan composition and its bundle-hash input | public `build_plan`/bundle-hash root names and generated hashes stay stable |
| `src/planner/plan/request_policy.rs` | supported capability rejection and ordered `FULL`/`SELECTION`/`EXCERPT` scene selection | unsupported interpretation fails before plan projection or render effects |
| `src/planner/preflight.rs` | current screenplay에 대한 PRESERVE/lineage/asset semantic preflight | narrative 의미를 재작성하거나 승인하지 않음 |
| `src/backend.rs` | private composition facade for backend descriptor, discovery, and routing owners | crate-root exports retain the existing public names |
| `src/backend/types.rs` | `BackendOperation`, `BackendLimits`, `BackendDescriptor`, descriptor fingerprint and admission | the fingerprint binds the exact returned capability fields; all public descriptor field types are reachable from the crate root |
| `src/backend/discovery.rs` | internal and PATH-resolved renderer discovery, 5-second Runtime-managed version probes, descriptor construction | missing executable is absent; probe/runtime failures propagate; external capability is Unix-only |
| `src/backend/routing.rs` | descriptor uniqueness, render operation/format/limit admission, and exact backend selection | `OutputRequest.backend_id` is the only pin source; unsupported capability has no fallback |
| `src/render.rs` | per-output render orchestration와 atomic publication, process owner composition without type forwarding | renderer 결과는 DeliverySet commit 전 후보 |
| `src/render/document.rs` | format metadata·plan lookup·escaping·composition facade | 입력 DeliveryPlan content/lineage 유지 |
| `src/render/document/{html,svg,graph}.rs` | captured image HTML·bounded text SVG·single graph projection | unsupported input is explicit; graph semantic consumer shares the projection |
| `src/render/cache.rs` | output-local content key와 artifact receipt | backend fingerprint·asset provenance에 결속 |
| `src/render/process.rs` | `RunControl` owner, cancellation, bounded external process, stage cleanup, media probes | internal consumers import this owner directly; Runtime process primitive를 사용하며 실패를 명시 |
| `src/render/verification.rs` | artifact-byte semantic/technical verification | renderer 성공을 사람의 의미·visual 승인으로 승격하지 않음 |

PDF text extraction failure is recorded as an explicit semantic `Warn` with preview required; it
does not substitute empty text or record a semantic `Pass`. PDF delivery remains `NEEDS_REVIEW`.

Asset decoder/pixel/materialization tests stay beside `engine/assets/` owners; project-owned path
symlink coverage stays beside `engine/storage.rs`. The old `engine.rs` test bundle therefore no longer
makes asset/storage invariants appear to belong to the DeliveryEngine composition facade. The
canonical scene-parser fixture now stays beside `planner/scene.rs::extract_scenes`.

Delivery request/output segment checks call Runtime's `validate_path_segment` owner directly. The
duplicate `safe_segment` wrappers in render and engine storage were removed; project-root containment
and filesystem path admission remain Delivery storage responsibilities.

## 실제 흐름

validate → project lock/source reverify → immutable request identity → plan → asset/render staging → verify artifact/current inputs → active→publishing ownership claim → delivery-set marker

## 현재 state·contract·I/O owner

delivery request/state/control files는 DeliveryEngine, source semantic meaning은 STS. FULL/SELECTION/EXCERPT/PRESERVE가 지원 계약; SUMMARY/MOOD/locale conversion은 자동 fallback하지 않는다. renderer configuration은 effect 선택.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 공개 surface | REACHABLE | PARTIAL | Last executed full Rust workspace gate reached 68 passing tests; current owner-path/format source-only edits are NOT_RUN. Prior public `deliver` CLI fixture reached `NEEDS_REVIEW`; real provider/human review is NOT_RUN | `src/engine/run.rs`, `src/engine/assets/materialize.rs`, `src/engine/release.rs` |
| 독립 호출 library 확장 | PUBLIC_LIBRARY | UNKNOWN | NOT_RUN: 외부 caller 미조사 | public exports/상위 composition |

## 의존성·findings

**ND1.** 문제 → renderer 성공과 visual semantic PASS를 구분해야 한다. 실제 raster/video 경계는 기술검사만으로 사람 검토를 대체하지 않으며 dependency가 없는 환경에서 mock 결과는 production 증거가 아니다. 근거 → `src/engine/run.rs::run/review/cancel/claim_publish_ownership`. 원인 → 환경·검토 증거를 구현 wiring과 혼동할 위험. 영향 → caller가 작업 완료/승인/복구 가능성을 잘못 해석할 수 있다. 방향 → 현재 ownership/NEEDS_REVIEW 경계를 유지하고 real renderer 및 cancel-vs-publish 경합을 실제 환경에서 검증한다.

## 실패·취소·재시도·복구와 남은 gap

동일 requestId/다른 hash는 거절. final marker 전 partial artifact는 완료가 아니며 재사용도 source/artifact hash 재검증을 거친다. cancel은 active ownership claim과 경합하며 이미 publishing이면 무조건 취소 성공으로 보고하지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)

## Current source-only slice — backend discovery and routing

Descriptor fingerprints now serialize the same `operations` value that the caller receives. Render routing
requires the `RENDER` operation, rejects zero dimension multiples, and counts compatible descriptors without
allocating/cloning a temporary vector. ImageMagick's version probe is reused for its image and video
descriptors. The renderer dispatches on the selected backend ID and format together, so a pinned descriptor
cannot silently execute another format's hard-coded renderer. `MATERIALIZE` remains a separate protocol
capability and is not accepted as a render route. Public descriptor fields and protocol schemas are
unchanged; `BackendLimits` is now reachable from the crate root like the other public descriptor types.
`route_output` now takes its only backend pin from `OutputRequest.backend_id`; the redundant pin parameter
has no compatibility overload. Probe failures remain explicit errors. Regression tests were added;
tests/build/fmt/lint/runtime/provider and manifest qualification are **NOT_RUN**. Next: remaining NVC consumer
and tool surfaces.

## Current source-only slice — verified artifact bytes for preview consumers

`DeliveryEngine::read_artifact_bytes` reloads the current package/set contract chain, resolves the artifact
through Delivery's project-owned path boundary, bounds the read, and checks the SHA-256 of the exact byte
buffer returned to its caller. Tauri preview consumes that buffer and does not reopen the receipt path.
Producer DTOs, schemas, and persisted identity remain unchanged. Tests/build/fmt/lint/native UI/runtime and
manifest qualification are **NOT_RUN**. Next: Delivery run/cancel/publish lifecycle.

## Current source-only slice — persisted contract and release admission

`verify_released_package` compares the supplied object with the canonical package before verifying the current
artifact set once. Persisted request/plan/index/set chain checks remain in `load_contract_set`; receipt artifact
integrity is now one shared owner used by load/list/review/run while list still filters stale-package records
before checking their output receipts. Review re-reads the committed set and output artifacts before returning a
successful result. Existing package and Delivery DTO schemas/identity algorithms are unchanged.
Tests/build/fmt/lint/runtime/renderer and manifest qualification are **NOT_RUN**. Next: Delivery run/cancel/
review effect lifecycle.

## Current source-only slice — run/cancel/review effect lifecycle

Capability, screenplay/package identity, planning and semantic preflight precede the lease. The active→cancel
and active→publishing rename remains the linearization point. Lease IDs use process plus nanosecond identity;
marker reads are bounded and reject non-regular files. A persisted set is not reported as success until its
owned publish markers are removed. Renderer cache outputs are revalidated, private staging directories are
exclusive, and successful receipts require owned stage cleanup. Delivery review re-reads the persisted set and
artifact bytes after its decision write. Request state and renderer output semantics are unchanged.
Tests/build/fmt/lint/runtime/renderer qualification are **NOT_RUN**. Next: MCP/CLI/Tauri Delivery consumers.

## Current source-only slice — project path and renderer staging

Delivery no longer keeps a lexical-only `project_path` wrapper. Package artifacts, persisted request/plan/index/
set files, materialized assets, rendered targets, HTML-embedded assets, and lease markers resolve through the
canonical project-owned path boundary. HTML hashes and embeds one captured asset buffer. Renderer staging and
run directories use exclusive creation; a reused content-addressed target is verified before reuse. Owned
stage cleanup and post-publish lease cleanup errors remain failed outcomes. No Delivery contract DTO or identity
algorithm changed. Tests/build/fmt/lint/runtime/renderer qualification are **NOT_RUN**. Next: request admission/
planner.

## Delivery request admission/planner — 소스 정리 완료, 실행 미검증

범위는 planner request policy, scene extraction, deterministic plan/preflight와 필수 Engine context/run
admission consumer다. Planner는 supplied screenplay UTF-8 bytes의 raw hash를 NarrativePackage artifact에
대조한다. Scene text는 trim 없이 exact byte partitions로 보존하며 malformed SCN heading을 묵인하지
않는다. 기존 parser fixture의 expected text만 새 보존 계약에 맞췄고 실행하지 않았다.

Selection/Excerpt candidate plan도 exact requested SCN order/coverage를 수용한다. Asset requests는
requested provenance/kind/role/refs/mode/source/hard constraints와 정확히 일치하며 unit의 scene-specific
asset bindings도 대조한다. USER/DERIVED source 존재는 contracts validator가 소유하고, preflight는
지원 mode와 request-exact provenance만 판단한다. USER/DERIVED 외 modes는 supported capability로 수용하지 않는다.
Timed scene count/order는 checked u32 conversion과 bounded range로 계산한다. Source scene ID set은
borrow하여 duplicate 판정의 clone을 제거했다.

Engine context의 reread screenplay bytes도 package raw hash에 결속한다. Run은 cache/effects 이전에
capability policy를 수용하고 pure plan/preflight 후 lease를 연다. Director bundle v2는 raw source binding,
lossless scene partitions와 exact requested sequence/provenance를 명시하여 이전 planner receipt와
같은 identity로 광고하지 않는다. Five DTO schemas와 STS narrative ownership은 유지한다.
Actual renderer output fidelity, Human review와 external filesystem custody는 증명하지 않는다. 기존
persistent requests/outputs는 수정·삭제하지 않았다. Tests/build/fmt/lint/runtime/provider 및 source
manifest qualification은 **NOT_RUN**이다.

## Current source-only slice — persisted contract and release admission

`verify_released_package` compares the supplied object with the canonical package before verifying that
canonical artifact set once. Persisted request/plan/index/set chain checks remain owned by `load_contract_set`.
Receipt-byte integrity has one shared verifier for load/list/review/run; list still filters stale-package sets
before checking receipt outputs. Human review re-reads the committed set and output artifacts before returning
success. Producer DTO schemas, request/package identity algorithms, and persisted project state are unchanged.
Tests/build/fmt/lint/runtime/renderer and manifest qualification are **NOT_RUN**. Next: Delivery run/cancel/
review effect lifecycle.

## Delivery persisted contract/release admission — 소스 정리 완료, 실행 미검증

범위는 released package admission, persisted contract set과 필수 load/list/review/storage consumers다.
Delivery는 canonical package의 JCS identity만으로 release를 인정하지 않는다. Existing STS release-bundle
DTO와 captured package/state raw bytes, SceneMap proof 및 Completed/approved final gate/source snapshot
hash를 대조한다. STS semantic state를 만들거나 재해석하지 않으며 release proof를 read-only로 소비한다.
Package files에는 SceneMap도 포함하고 verification reread bytes의 drift를 거절한다.

Persisted request/plan은 current package와 current Director bundle을 대상으로 해야 하며 exact semantic
preflight를 다시 소비한다. 기존 hash-chain/asset/artifact checks는 유지한다. Load/review는 stale chain을
거절하고 current-only list는 명시적 STALE_RESULT만 제외한다. 다른 parsing/integrity/I/O 오류를 성공으로
바꾸지 않는다. Existing vertical fixture는 simulated Completed proof metadata로만 갱신했고 실행하지 않았다.

Storage는 regular JSON files/declared project directory를 요구하며 noncanonical relative paths와 symlink
root/components, nondirectory parents를 거절한다. Redundant ancestor exists fallback을 제거하여 실제
I/O 오류를 missing으로 묵인하지 않는다. 기존 사용자 requests/outputs를 수정·삭제하거나 migration하지
않았다. Five delivery DTO schemas와 producer pins는 유지한다. Tests/build/fmt/lint/runtime 및 source
manifest qualification은 **NOT_RUN**이다.

## Delivery assets/import/materialization — 소스 정리 완료, 실행 미검증

범위는 image ingress, USER/DERIVED materializer, Delivery-local source/blob helper와 required persisted
index consumer다. Source는 held regular-file handle로 읽고 image의 기존 16 MiB 한도를 metadata와 실제
bounded read에 함께 적용한다. Materialization은 captured bytes를 expected source hash에 대조하고
same bytes로 publish하여 별도 reread drift를 제거한다. Pixel bounds는 positive dimensions도 요구한다.

Import와 materialization/reuse가 기존 PNG/JPEG/WEBP codec admission을 공유한다. Actual codec의 MIME와
extension을 사용하며 declared image MIME mismatch, animated/invalid inputs를 거절한다. Non-image
bytes는 generic binary metadata만 유지하고 image로 추정하지 않는다. Content-addressed blob은 existing
regular bytes/hash와 일치할 때만 재사용하고 damaged cache를 자동 덮어쓰지 않는다.

Generated IMG/INDEX IDs의 digest 절단을 제거하여 full source/plan hash를 사용한다. Name의 approval
fallback를 제거하며 실제 UTF-8 filename만 표시한다. Materializer v2 fingerprint로 changed policy를
구분하고 stored index도 같은 fingerprint/native-image proof를 수용한다. Plan identity는 effects 전에
대조한다. Five DTO schemas/pins와 provider별 owner는 유지한다.

Persisted assets/cache를 이번 작업에서 수정·삭제하거나 migration하지 않았다. Full decoder resource
qualification, non-image allocation ceiling 및 atomic adversarial filesystem custody는 미검증이다.
Tests/build/fmt/lint/runtime/provider와 source-manifest qualification은 **NOT_RUN**이다.


## Delivery run/cancel/publish lifecycle — 소스 정리 완료, 실행 미검증

범위는 run/cancel/publish ownership, active-run guard와 required commit/retry admission이다.
짧은 Delivery control lock이 marker begin/cancel/claim/cleanup을 직렬화한다. Long-running project lock과
분리하여 취소가 rendering 전체 lock을 기다리지 않는다. Active marker를 읽고 나중에 다른 run의
marker를 rename하는 cooperating-handler race를 줄이며 ownership과 publish/cancel winner를 수용한다.
Control path/marker shape를 실제 project-owned regular boundary로 수용하고 foreign cleanup은 거절한다.

Run은 root와 existing request/terminal marker를 명시적으로 수용한다. Request 없이 set만 있는 상태를
자동 overwrite하지 않는다. Publish 직전에는 package files만 아니라 full current STS release proof를
다시 소비한다. DeliverySet ID는 full digest를 사용하고 missing backend route는 오류다. Final set write
뒤 current loader의 contract/artifact proof와 requested set equality를 대조한다. Explicit lease finish는
fallible cleanup을 보고하고 committed-then-cleanup failure를 성공으로 바꾸지 않는다.

Control lock은 cooperating processes의 coordination이며 external filesystem attackers의 atomic custody,
renderer remote effects 취소 또는 multi-output transactional rollback을 증명하지 않는다. Error unwinding의
Drop cleanup은 best-effort로 남는다. 기존 사용자 data/markers를 이번 작업에서 삭제·수정하지 않았다.
Tests/build/fmt/lint/runtime/cancellation-race 및 source manifest qualification은 **NOT_RUN**이다.


## Delivery backend discovery/routing — source-only closure

Descriptor shape/fingerprint, executable discovery, and output routing have separate private owners.
Routing admits unique valid descriptors, requires RENDER and positive dimension multiples, and consumes
only OutputRequest.backend_id. The separate pin parameter is retired without a compatibility overload.
Borrowed single-pass selection and a single ImageMagick probe remove duplicate work. PATH absence is
distinct from lookup, launch, timeout, exit, UTF-8, and empty-version failures; those errors propagate.
Version discovery describes declared implementation capabilities; it does not qualify renderer codecs
or actual output quality. Existing routing fixtures consume the new signature and fingerprint grammar.
Tests/build/fmt/lint/runtime/probes and manifest qualification are **NOT_RUN**.

## Delivery renderer execution/cache/artifact admission — source only

Existing content-addressed cache failures now propagate; render no longer removes a failed artifact
and silently regenerates it. New outputs use the artifact-admission-v2 build-key namespace without
a legacy reuse branch. Semantically blocked stage results cannot publish; the published artifact
digest must equal the admitted stage digest. Cancellation is checked across cache return and publication.
Stage ownership is acquired only after exclusive creation. Normal success/error paths explicitly
combine render and cleanup outcomes; Drop remains a best-effort unwind backstop. Video frame counting
uses a wider counter and requires explicit unit duration. Image probe responses require exact UTF-8
measurements and the requested PNG/WEBP format. Video probes reject truncated numeric values and
require MP4/H.264, positive measurements, and the requested rational FPS, dimensions, and frame count.
Graph semantics compare the full generated JSON value; HTML content must remain inside its ordered
unit section. PDF extraction warnings retain the actual error and still require Human review.

This closes the source scope across render orchestration, cache identity, process inspection/cleanup,
and artifact admission consumers. No existing user artifacts were removed or migrated. Producer DTOs
and schema pins are unchanged. External executable identity still relies on the configured PATH/version
discovery; hostile external filesystem replacement and renderer quality are not qualified by this edit.
Tests/build/fmt/lint/runtime/media probes and manifest qualification are **NOT_RUN**.

## Delivery document composition — source-only closure

HTML, SVG text layout, and graph projection now have separate private representation owners below
render/document. The facade retains format metadata, plan lookup, and one-pass escaping. HTML uses
bounded captured image reads from the existing Delivery asset owner, matching actual MIME, stored
size, and SHA before embedding the same bytes. Borrowed indexes replace repeated asset/request scans;
image alternative text consumes the source-bound request role. Standalone semantic main markup, valid
inherited typography, and narrow-width padding keep the document body usable without added screen chrome.
Graph publication and semantic admission share graph_value instead of serializing and reparsing the
expected value. SVG wrapping no longer collects per-line character vectors or forces a minimum text
capacity when the requested dimensions have no readable area. It preserves terminal blank lines, uses
monospace/space-preserving markup and a conservative layout budget, and explicitly refuses unsupported
XML/layout characters and attached assets rather than silently discarding them. Font shaping and actual
renderer clipping remain unqualified; the layout budget is not a measured glyph guarantee.

Required planner admission now rejects non-image assets and requires HTML for image attachments before
effects. Director bundle v3 binds that policy. Document-projection-v3 replaces the previous artifact cache
namespace with no legacy reuse branch; existing user files are retained. Render and source-guard consumers
are updated for the split, and the guard was not executed. No new product screen or narrative authority
was introduced; five durable DTOs and schema pins remain unchanged. Tests/build/fmt/lint/runtime/browser/
media probes and source-manifest qualification are **NOT_RUN**.

