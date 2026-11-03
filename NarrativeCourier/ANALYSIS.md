# NarrativeCourier — 구현 분석

## 현재 분석·개선 범위

Cinematic JSON and storyboard Markdown publication use the same immutable byte publisher in
`tools/cinematic_contracts.py`. Storyboard export and full-package assembly share current source,
board-hash and approved-decision checks; full coverage remains package-specific. Audit input
extractors are imported from `sts_audit_inputs` by their actual consumers, without implicit CLI
re-exports. These source-only edits were later covered by the 2026-10-01 NVC gate recorded below;
real external writer, renderer, and Human approval runtime remain unqualified.

Runtime process deadline 파트는 소스 정리를 마쳤다. Child 종료와 stdin delivery의 timeout이
한 시작 시각을 사용한다. Process-group·cancel·capture의 소유권과 기존 오류 분류는 보존하며
실행 검증은 NOT_RUN이다. 상세는 [runtime 분석](crates/runtime/ANALYSIS.md)을 따른다.

STS 입력 경계 파트의 소스 정리를 마쳤다. 일반 NewProject의 review binding injection과 direct
synopsis newline normalization을 제거했다. 검증한 handoff bytes를 private ingress가 직접 저장하며
config 재수용은 retained receipt와 current synopsis/title/binding을 대조한다. Project-root regular-file
resolution, duplicate references, bounded 25 MiB reads와 exclusive staging ownership을 연결했다.
Producer contracts와 Geultu/NVC authority는 분리하며 tests/build/fmt/lint/runtime은 NOT_RUN이다.
상세는 [STS 입력 분석](crates/sts/ANALYSIS.md#sts-입력-경계-파트--소스-정리-완료-실행-미검증)을 따른다.

STS workflow 입력/snapshot 파트의 소스 정리를 마쳤다. Manifest token으로 output/feedback을
분류하고 optional의 실제 누락만 생략한다. Regular-file project/asset resolver, checked producer
state, PathBuf deduplication과 relative source fingerprints를 연결했다. STEP 0 이후 missing snapshot과
unknown state fields를 거절한다. Packet/token은 같은 asset resolver를 소비하며 tests/build/fmt/lint/
runtime은 NOT_RUN이다. 상세는 [STS 분석](crates/sts/ANALYSIS.md)을 따른다.

STS submission/human-gate 상태 전이 파트의 소스 정리를 마쳤다. Checked counters/range admission을
effect 전으로 옮기고 paused decision bypass를 제거했다. Submitted bytes와 feedback hash를 실제
request body에 결속하며 version history/feedback의 충돌과 잘못된 file parents를 거절한다. Borrowed
manifest nodes와 단일 allowed-decision helper를 사용한다. Partial publication/state failure의 crash gap은
남으며 tests/build/fmt/lint/runtime은 NOT_RUN이다. 

STS fidelity/Source IR validation 파트의 소스 정리를 마쳤다. Table/separator shape와 unique metadata
assignment를 폐쇄 문법으로 수용하고 unmapped heading은 complete source line으로 제한했다.
Coverage range merge와 borrowed alignment index/citations로 bitmap 및 반복 복제를 제거했다.
Source/STEP 8D/Human authority는 보존하며 tests/build/fmt/lint/runtime은 NOT_RUN이다.


STS manifest/domain identity 파트의 소스 정리를 마쳤다. 실제 revisable steps/stage outputs와 closed
Agent/Human metadata를 수용하고 borrowed dataflow index를 사용한다. Domain identity는 supplied
manifest와 현재 workflow를 대조하며 regular asset files 및 selected static asset inputs를 결속한다.
Source/output authority는 별개로 유지하고 tests/build/fmt/lint/runtime은 NOT_RUN이다.


STS SceneMap projection 파트의 소스 정리를 마쳤다. Blueprint declarations와 실제 screenplay
scene 연결을 수용하고 malformed/duplicate/unbound annotations를 거절한다. Active cue의 여러 줄
대사 처리를 정리하고 publish/load의 current builder를 통합했다. Captured bytes는 stored authority
hash에 결속하며 identity/schema와 producer ownership은 유지한다. Tests/build/fmt/lint/runtime은
NOT_RUN이며

STS NarrativePackage/release bundle 파트의 소스 정리를 마쳤다. Stored artifact hashes와 captured
state/package/map bytes를 결속하고 전체 Completed·Human approval proof를 수용한다. 발행 후에도
같은 bundle proof를 대조하며 반복 hash reads/newline concat/blanket I/O conversion을 제거했다.
Schema와 authority boundary는 유지하고 tests/build/fmt/lint/runtime은 NOT_RUN이다.


STS read views/diagnostics 파트의 소스 정리를 마쳤다. Doctor의 silent hash-error conversion과 missing
snapshot 검사 누락을 제거하고 durable text를 captured authority bytes에 결속한다. Paused/integrity
failure/max revisions의 action projection과 empty-stage progress를 정리했다. Project errors는 실제 오류로
반환하며 tests/build/fmt/lint/runtime/browser는 NOT_RUN이다. 

STS Codex runner/lifecycle 파트의 소스 정리를 마쳤다. Config/probe 오류를 default나 설치 상태로
뭉개지 않고, persisted policy와 marker/file admission을 강화했다. Private workdir ownership과 exact
artifact bytes를 유지하고 ignored AGENTS/filename fallback을 제거했다. Execution/submit/lease cleanup
결과를 함께 보고하며 committed-then-cleanup failure를 명시한다. Tests/build/fmt/lint/runtime/provider는
NOT_RUN이다.

STS Fountain/FDX interchange 파트의 소스 정리를 마쳤다. Existing XML parser를 사용해 FDX 구조/
문단을 수용하고 Character/Dialogue 누락과 unsupported paragraph fallback을 제거했다. Fountain의
block-tail 소실과 unterminated boneyard, format summary와 export escaping을 정리했다. Engine staging/
cleanup과 Completed source proof를 연결했으며 tests/build/fmt/lint/runtime은 NOT_RUN이다.


STS DOCX export 파트의 소스 정리를 마쳤다. 공통 borrowed screenplay parser와 static OPC assets를
분리하고 direct XML construction을 사용한다. Final ZIP은 own exclusive staging에서 완성하여 발행하며
기존 output을 먼저 truncate하지 않는다. Engine의 Completed source proof와 cleanup failure semantics를
연결했다. Tests/build/fmt/lint/runtime/viewer는 NOT_RUN이다. 

Delivery request admission/planner 파트의 소스 정리를 마쳤다. Planner/context의 raw screenplay를
package hash에 결속하고 exact scene partitions와 requested sequence를 수용한다. Candidate assets의
provenance/unit binding과 capability policy를 닫고 pure preflight를 lease 전으로 옮겼다. Director bundle
v2로 changed planner semantics를 구분하며 tests/build/fmt/lint/runtime은 NOT_RUN이다.


Delivery persisted contract/release admission 파트의 소스 정리를 마쳤다. STS release bundle의 raw
state/package/map proof와 Completed/approval/source snapshot을 소비하고 cached chain을 current
package/planner에 결속한다. Storage root/JSON/path admission을 강화하며 stale list filtering과 actual
errors를 구분한다. Existing fixture만 새 simulated proof에 맞췄고 tests/build/fmt/lint/runtime은 NOT_RUN이다.


Delivery assets/import/materialization 파트의 소스 정리를 마쳤다. Held source bytes와 limits/hash를
결속하고 immutable blobs를 implicit repair 없이 수용한다. Actual image codec/MIME, full generated IDs와
materializer v2 policy를 stored-index consumer까지 연결했다. Provider/state ownership은 유지하며
 tests/build/fmt/lint/runtime은 NOT_RUN이다. 

Delivery run/cancel/publish lifecycle 파트의 소스 정리를 마쳤다. Short control lock으로 marker
coordination을 묶고 retry/terminal marker admission을 닫았다. Publish 전 full STS release proof와
write 후 set/contract/artifact proof를 대조하며 committed cleanup failure를 명시한다. Full set digest와
checked backend lookup을 사용한다. Tests/build/fmt/lint/runtime/race는 NOT_RUN이다.


Delivery backend discovery/routing의 소스 정리를 마쳤다. Descriptor/fingerprint·discovery·routing
owner를 분리하고 request의 backend_id를 유일한 pin으로 사용한다. Render capability와 중복 등록을
수용하며 실제 probe 실패를 누락으로 숨기지 않는다. Version probe는 codec qualification이 아니다.
Tests/build/fmt/lint/runtime은 NOT_RUN이다. 

Delivery renderer execution/cache/artifact admission의 소스 정리를 마쳤다. Failed cache의 삭제·재생성
fallback을 제거하고 v2 key, pre-publication semantic admission, published digest binding과 cleanup
error reporting을 연결했다. Image format·video container/codec/FPS 및 numeric bounds를 수용한다.
Tests/build/fmt/lint/runtime은 NOT_RUN이다.

Delivery document composition 파트의 소스 정리를 마쳤다. HTML/SVG/graph owner를 분리하고
bounded captured images·request role alt text·single graph projection·conservative SVG layout을 연결했다.
Unsupported asset omission과 small-canvas minimum fallback을 제거하고 planner/bundle v3 및 source
guard consumer를 갱신했다. Tests/build/fmt/lint/runtime은 NOT_RUN이다.

AssetIndex identity/provenance admission 파트의 소스 정리를 마쳤다. Generic contract는 exact
plan/narrative hashes·unique request IDs·item/request identity를 수용한다. Delivery internal-copy:v3는
source hash/MIME·full index ID·canonical blob·bounded captured image를 소유하고 HTML과 persisted
consumer가 같은 image reader를 사용한다. Schemas/pins는 유지하며 tests/build/fmt/lint/runtime은
NOT_RUN이다.

DeliverySet receipt/status aggregation 파트의 소스 정리를 마쳤다. Nonempty failure-first aggregate를
contract/run/review가 함께 사용하며 exact request hash·format/backend·required lineage·failure findings를
수용한다. Producer schema도 같은 상태 규칙으로 갱신하고 Human review commit equality를 연결했다.
Tests/build/fmt/lint/schema validator/runtime은 NOT_RUN이다.

DirectorPlan request binding/order/timing 파트의 소스 정리를 마쳤다. Generic validator는 exact
request hashes·actual array order·checked frame budget를 수용하고 asset/delivery 책임을 분리했다.
Deterministic builder/preflight는 같은 structure/timing owner와 full PLAN identity를 소비한다. Bundle
v4, u32 producer schema bounds와 기존 fixture/source consumer를 갱신했으며 tests/build/fmt/lint/runtime은
NOT_RUN이다.

DeliveryRequest output-option/input surface 파트의 소스 정리를 마쳤다. Request asset/output
validators와 typed present-option decoding을 분리하고 empty adaptation·unsupported geometry/timing/
format fallback을 제거했다. Producer schema의 default/bounds, existing UI request omission/numeric
admission과 bundle v5를 연결했다. Tests/build/fmt/lint/schema validators/UI/runtime은 NOT_RUN이다.


Artifact/reference와 NarrativePackage wire boundary 파트의 소스 정리를 마쳤다. Artifact MIME의
present-null fallback을 제거하고 nvc2 body identity·named artifact inventory 및 bundle identity를
producer contract owner로 통합했다. STS 생성과 Delivery captured JSON proof consumer를 갱신했으며
schema bytes/integration pins는 유지한다. Tests/build/fmt/lint/runtime은 NOT_RUN이다.

NVC app-config root resolution/entrypoint options 파트의 소스 정리를 마쳤다. Fallible shared root
resolution, lazy platform selection과 one-CWD relative anchoring을 연결했다. Empty/invalid selected
paths와 duplicate/missing MCP flags를 거절하고 CLI/MCP consumer 및 configuration-free help를
갱신했다. Tauri desktop lifecycle은 별도 유지한다. Tests/build/fmt/lint/runtime은 NOT_RUN이다.


NVC CLI command admission/dispatch 파트의 소스 정리를 마쳤다. Closed typed Invocation을
app-config/Engine 전에 수용하고 STS/Delivery handler의 raw option parsing 및 unknown-success
fallback을 제거했다. Arity·duplicate/value/decision·stdout errors를 보존하며 human token과 STS
Completed package 경계를 유지한다. Tests/build/fmt/lint/CLI/runtime은 NOT_RUN이다.

NVC MCP tool contract/dispatch 파트의 소스 정리를 마쳤다. Closed parameter DTO·typed optional
values·literal export format과 producer-derived DeliveryRequest tool schema를 연결했다. Domain 실패는
is_error로 보존하며 worker/serialization outcome 경계를 명시한다. Human gate와 owner 분리는
유지하고 tests/build/fmt/lint/schema generation/protocol/runtime은 NOT_RUN이다.

NVC Tauri command ingress/application lifecycle 파트의 소스 정리를 마쳤다. Registered commands를
shared blocking-worker로 연결하고 picker cancellation·regular/size/UTF-8 boundary·deferred explicit
export path·project-bound filename을 정리했다. UI picker consumers와 verified preview/MIME admission을
갱신했으며 native UI/worker runtime은 NOT_RUN이다.

기존 NVC UI workflow state/request consumer 파트의 소스 정리를 마쳤다. Project/read/preview
responses를 current UI scope에 결속하고 mutation·refresh outcomes와 unknown freshness를 분리했다.
Stale approval selection·native DEMO injection·reference shape aliases를 제거하고 raw synopsis와
package-bound staging을 유지했다. UI/runtime/race/visual proof는 NOT_RUN이다.

기존 NVC graph-preview parser/layout/result consumer 파트의 소스 정리를 마쳤다. Closed producer
sequence와 immutable parsed model을 수용하고 linear layout, SVG-unit gestures, pointer cancellation,
keyboard focus/selection 및 explicit raw-result recovery를 연결했다. Dependency approximation·
defaulted fields·recreated graph toggles를 제거했으며 JS/UI/runtime/visual/AX/performance는 NOT_RUN이다.


NVC Runtime document extraction 파트의 소스 정리를 마쳤다. Held bounded bytes를 plain/PDF/DOCX
consumer에 연결하고 namespace/body/unique-part DOCX owner를 분리했다. Newline normalization과
silent entity loss를 제거하고 unsupported/incomplete structures를 명시적으로 거절한다. Actual
document fidelity·decoder resource behavior·runtime은 NOT_RUN이다.

Runtime paths/project-root composition 파트의 소스 정리를 마쳤다. Canonical portable relative
paths·bounded IDs·nonempty one-CWD roots를 Runtime owner로 모으고 duplicate lexical checks를 제거했다.
Fallible STS constructors와 CLI/MCP/Tauri/fixture consumers를 연결했으며 I/O containment는 각 effect
owner에 유지한다. Tests/build/fmt/lint/platform/runtime은 NOT_RUN이다.

Runtime durable storage/publication/lock primitives 파트의 소스 정리를 마쳤다. Store facade 아래
digest/publication/lock owners를 분리하고 captured file identity/size, exclusive owned staging 및
post-rename durability errors를 연결했다. Lock의 phantom directory creation과 unsupported-platform
write effects를 제거했으며 public paths는 유지한다. Storage/crash/race/runtime은 NOT_RUN이다.


Runtime token/JCS identity grammar 파트의 소스 정리를 마쳤다. Separate serialization hash를
제거하고 schema-adapted safe-integer canonical profile을 Rust/Python에 연결했다. STS typed v2
task/gate identities는 domain/snapshot/node/review actions와 scoped UTF-8 file paths를 결속한다.
Prior token fallback은 없으며 RFC vectors/cross-language/runtime은 NOT_RUN이다.

Runtime process stdin/output/cancellation lifecycle 파트의 소스 정리를 마쳤다. Nonblocking stdin과
child/cancel/deadline/capture monitor를 한 loop로 묶고 custody/cleanup error를 보존한다. Private
capture budget·bounded reads·pre-effect platform refusal 및 renderer stdio isolation을 연결했으며
process/race/runtime/performance는 NOT_RUN이다.

## Verdict·책임 경계

완성된 서사의 semantic authority를 만드는 STS와, 이를 변형 없이 전달하는 delivery를 조합한 독립 제품이다. caller는 CLI·Tauri UI·MCP와 SMC다. 방향은 **IMPROVE**, STS와 delivery의 ownership 분리는 **MAINTAIN**. STS manifest, screenplay parser, state/task persistence, reviewed-synopsis ingress, SceneMap validation, task packet/submission, Delivery planner와 render lifecycle가 private owners로 나뉘었다. 2026-10-01 기록에서 `./verify-all.sh --require-rust`는 check/test/fmt/clippy와 73 Rust tests를 통과했고, 512-entry source manifest도 검증됐다. Public owner-chain과 83-call text package는 합성 QA 자료와 simulated Human gates로 기계적으로 완료됐다. 실제 사용자 원고·실제 model writing·사람 adoption·외부 renderer는 **NOT_RUN**이다. [현재 root 재검증 보고서](../docs/research/2026-10-01-live-use-revalidation-current-tree.md)에 command와 범위를 기록했다.

| 단위 | 판정·이유 |
|---|---|
| `crates/sts` | [독립 분석](crates/sts/ANALYSIS.md): task/gate token·사람 결정·Completed·release 소유; DTO imports name project/views/workflow source owners while the crate root keeps its explicit public list. |
| `crates/delivery` | [독립 분석](crates/delivery/ANALYSIS.md): request·renderer·cancel/publish·delivery-set 소유; deterministic request capability/coverage policy는 plan projection과 분리. |
| `crates/runtime` | [독립 effect 분석](crates/runtime/ANALYSIS.md): root `Error`/`Result` contract 아래 document/path/process/lock·token·store owner modules. Path grammar belongs to `paths`; locks, hashes and durable file effects belong to `store`. Callers import from the defining owner. |
| `crates/contracts` | [독립 분석](crates/contracts/ANALYSIS.md): crate-root public facade 아래 producer-owned DTOs와 contract validators. DeliveryRequest/DirectorPlan DTO paths stay stable while their cross-field validators have private child owners; workflow state 없음. |
| `crates/app-config` | [독립 분석](crates/app-config/ANALYSIS.md): CLI와 MCP가 공유하는 NVC application roots와 explicit→environment→platform→repository precedence. 서사·workflow state를 소유하지 않음. |
| `crates/mcp`, `src-tauri`, `ui` | 같은 engines로 연결되는 transport/composition. [MCP 분석](crates/mcp/ANALYSIS.md): `tools.rs` owns the single `#[tool_router]` and engine adapters, private `tools/screenplay.rs` owns screenplay import/export effect handling, `server.rs` owns server metadata/stdio lifecycle, `params.rs` owns typed DTOs, and `roots.rs` parses MCP flags before delegating shared defaults to `crates/app-config`; `lib.rs` preserves public reexports. Tauri `src-tauri/src/lib.rs` owns `AppState`, app path/resource setup, and the exact command allowlist; `commands/sts.rs` adapts file selection and STS project/artifact commands; `commands/delivery.rs` adapts asset selection and DeliveryEngine commands. `src-tauri/src/delivery_preview.rs` remains the UI-only preview DTO, bounded artifact read, and encoding owner over DeliveryEngine's `DeliverySet`. CLI and Tauri remain separate adapters to the same engines. |
| `tools/` | [폴더 분석](tools/ANALYSIS.md): static contract CLI and private Tauri wiring owner, SceneMap v2 fixture facade and contract/shape/JCS owners, source-blind screenplay audit, cinematic artifacts, text-board facade, raw digest primitive, source manifest and optional renderer smoke. Helpers do not replace STS authority or the five canonical delivery DTOs. |
| `crates/authoring` + native CLI `storyboard`/`animatic` | NVC text-board/frame-direction public CLI. Project FD/source lock, strict JSON/JCS/immutable bytes, semantic coverage/render, task preparation, candidate/review/publication, CLI dispatch를 분리하며 state·schema authority는 계속 NVC contracts/STS 소유. |
| `crates/sts/src/fountain*` | Fountain/FDX import normalization과 export serialization을 분리한 format adapter; semantic STS authority를 새로 만들지 않음. |
| `benchmarks`, STS skills·manifest | 입력·실행용 Markdown. 단순 과거 보고로 삭제하지 않는다. |

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| import-reviewed-synopsis | CLI → `create_project_from_reviewed_synopsis` | producer schema, raw synopsis hash, handoff identity/title | STS project initialization | bound source snapshot, 새 workflow | `crates/sts/src/engine/project/creation/reviewed_synopsis.rs`, CLI |
| run/next/agent-task/submit | CLI/UI/MCP → STS Engine | task token·source snapshot·node outputs | STS node state | next action·blocked/errors | `crates/sts/src/engine/workflow/tasks/packet.rs`, `tasks/submission.rs` |
| decide/resume/cancel | adapters → STS Engine | gate token·명시적 사람 결정 | STS durable state | Completed 또는 revise/pause/failure | `crates/sts/src/engine/workflow/decisions.rs::decide`, `workflow/resume.rs::resume` |
| manifest/project/artifact projection·doctor | CLI/Tauri/MCP → STS Engine | current manifest/state·artifact hash | read-only diagnostics and views | current project/artifact DTO or integrity error | `crates/sts/src/engine/manifest.rs`, `project.rs`, `views/{diagnostics,catalog,project}.rs` |
| narrative_package/release_bundle/export | adapters/SMC → STS export | Completed 및 current source hashes | STS release files | exact raw-byte bundle·Fountain/FDX | `engine/release/{scene_map,narrative_package,bundle}.rs`, screenplay interchange in `engine/project/screenplay.rs` |
| deliver/cancel | CLI/UI/MCP → DeliveryEngine | typed request·current package·request hash | delivery-local run and publish ownership | delivery-set 또는 incomplete/cancel/error | `crates/delivery/src/engine/run.rs` |
| context/review/list/load | adapters → DeliveryEngine | current package·artifact identity | delivery contract/artifact read and human review | current delivery view or integrity error | `crates/delivery/src/engine.rs`, `crates/delivery/src/engine/release.rs` |
| import_asset | UI/operator → DeliveryEngine | exact image bytes·rights·Completed NarrativePackage | delivery-owned imported asset/index input | hash-pinned ImportedDeliveryAsset or explicit error | `crates/delivery/src/engine/assets/imported_image.rs`; response DTO is owned by `engine.rs` |
| cinematic continuity/plan/approve/inspect | operator CLI → cinematic.py/contracts | Completed raw bundle + explicit draft/current bible/review hash | NVC cinematic artifacts·immutable file publication | bible/plan/approval; 미확정 visual 유지 | `tools/ANALYSIS.md` |
| existing-screenplay audit | maintainer/operator → `sts_audit.py` | local synopsis/screenplay bytes, bounded format extraction, pinned 3-stage audit workflow | local audit output directory only; screenplay reconstruction remains source-blind | IR/audit JSON + report, or explicit error | `tools/sts_audit.py`, `sts_audit_inputs.py`, `sts_audit_results.py` |
| storyboard prepare/task/submit/review/export/cancel/scenes/bundle | operator/`storyctl text` → native storyboard CLI | Completed release, current semantic occurrence refs, exact board/review hashes | NVC-owned task/board/review files under shared STS workflow lock | candidate, approval receipt, rendered text/package artifacts or `not_applied`/`outcome_unknown` | `crates/authoring/src/storyboard.rs` |
| UI startup/shutdown·MCP | Tauri command groups / rmcp tools → engines | config path·schema | `lib.rs` owns app state/setup/handler registration; `commands/sts.rs` and `commands/delivery.rs` own adapter effects | UI/tool responses | `src-tauri/src/{lib.rs,commands/*.rs,delivery_preview.rs}`, `crates/mcp` |

## 실제 흐름

대표 main chain: `geultu raw handoff → CLI import-reviewed-synopsis → STS source binding → node task generation → submit(token) → next human gate → decide(gateToken,approve) → final artifacts → Completed state → release-bundle export → SMC raw-byte verification`.

독립 delivery 경로: `deliver(request) → lock/source verification → immutable request → director plan → assets/render → technical verification → current source 재검사 → publish ownership claim → delivery-set commit → caller output`. 이 결과는 서사 원문을 수정하지 않는다. visual review는 별도 사람 action이다.

cinematic 경로: `Completed raw bundle → exact continuity extraction → operator/agent draft → deterministic plan validation → explicit operator review → approval → SMC v3`. 현 구현은 proposer가 아니라 draft 검증/봉인이다. 기존 delivery planner는 영화 연출 권한을 갖지 않는다.

텍스트 콘티 경로: `Completed release → current occurrence-bound task → exact-scene draft → full source/hash validation → separate human review → approved export/bundle`. Native CLI는 `crates/authoring`으로 위임하고 하위 module은 project/source, codec/schema, board/render/diff, task, publication, CLI 책임만 나눈다. 이는 기존 NVC owner 경계 안의 구조 변경이며 STS/board authority를 integration으로 이동하지 않는다. Python storyboard/animatic 공개 실행기와 중복 구현은 native producer parity 검증·caller 갱신 뒤 제거했다. 독립 cinematic Python 기능은 보존한다.

## 현재 state·contract·I/O owner

STS owns semantic state; DeliveryEngine owns delivery-local request/lifecycle; runtime owns effect primitives but no story decisions; contracts owns schema. token은 transient UI 선택 상태가 아니라 source/node fingerprint 계약이다. renderer configuration은 optional dependency이고 model/tool timeout은 effect policy다. Completed, visual review, delivery-set 존재, SMC release pointer는 서로 다른 state다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| handoff ingress·STS human gates·Completed | REACHABLE | PARTIAL | PASS: contract 정적 검사60; NOT_RUN: Rust·실제 agent | `engine.rs`, `tools/verify_contract.py` |
| Completed resume·release export | REACHABLE | PARTIAL | NOT_RUN: 승인 state bytes 보존·누락 bundle 복구 회귀 | `crates/sts/tests/workflow.rs` |
| delivery plan/render/review/cancel | REACHABLE | PARTIAL | PASS: source/wiring 검사109; NOT_RUN: 실제 renderer·Rust | delivery 분석 |
| cinematic producer·approval | REACHABLE | PARTIAL | PASS: SMC cinematic18에서 실제 Python CLI; rich continuity·native integration 제외 | [tools 분석](tools/ANALYSIS.md) |
| scene-map v2·audit | REACHABLE | SATISFIED | PASS: schema fixture/negative 검사·audit14; 의미 품질 보증 아님 | `tools/verify_narrative_scene_map_v2.py`, `test_sts_audit.py` |
| Tauri·MCP·Fountain/FDX·DOCX | REACHABLE | UNKNOWN | NOT_RUN: native/toolchain·실제 I/O | `src-tauri`, `crates/mcp`, STS format modules |
| Runtime process lifecycle | PUBLIC_LIBRARY | PARTIAL | NOT_RUN: exited leader/blocked stdin 회귀 추가 | runtime 분석 |

## 의존성·findings

**N1 · 승인된 Completed의 resume — 수정, 실행 검증 남음.** 문제 → resume가 Completed state를 다시 기록하고 누락된 release bundle을 복구하지 못했다. 근거 → `crates/sts/src/engine.rs::resume`; 원인 → 실행 재개와 승인 결과의 파생 산출물 복구를 구별하지 않음. 영향 → completed_state raw hash drift 또는 유효한 승인 후 handoff 부재. 방향 → lock 아래 source와 NarrativePackage를 검증한 뒤 기존 bundle은 검증만 하고, 정말 없는 bundle만 동일 승인 state로 재발행한다. state bytes·승인은 바꾸지 않으며 손상 bundle을 덮어쓰지 않는다. 세 회귀는 `NOT_RUN`이다.

**N2 · process leader와 stdin 수명주기 — 수정, 제한 남음.** 문제 → leader가 종료된 뒤 descendant가 stdin을 잡고 있으면 scoped join이 무기한 대기할 수 있었다. 근거 → `crates/runtime/src/process.rs`. 원인 → leader exit를 전체 process effect의 종료로 간주. 영향 → timeout/cancel 이후에도 caller가 복귀하지 않음. 방향 → stdin 완료를 같은 deadline/cancel 경계에서 수신하고 group liveness를 보고 TERM/KILL·wait한다. 별도 session으로 도주한 descendant, OS uninterruptible I/O, capture 출력 quota는 아직 보장하지 않는다.

**N3 · 정적 gate의 증명 범위.** 문제 → lexical source assertions의 PASS를 runtime 결과로 읽을 수 있다. 근거 → 두 `tools/verify_*`의 string checks. 원인 → 문서가 과거 build/runtime 숫자를 Current와 혼합. 영향 → Rust 변경 미검증을 숨김. 방향 → 이번 정적60/109과 audit14 결과만 기록하고 과거 완료 보고를 통합했다. Runtime의 일반 stdin 주석이 STS-domain 어휘 gate에 걸린 경우 역할 명칭을 바로잡았으며 검사는 약화하지 않았다.

## 실패·취소·재시도·복구와 남은 gap

STS token mismatch/source drift는 fail-closed. Delivery는 요청 ID reuse mismatch 거절과 final commit marker로 재시도한다. provider/renderer 실패나 visual NEEDS_REVIEW는 배포 완료가 아니다. 기존 benchmark 결과는 이번 runtime 증거가 아니다. MSRV·rmcp·Tauri 선언은 manifest 사실이며 최신 외부 호환성을 증명하지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

## CLI command admission and dispatch — source inspection

The CLI arguments owner defines the closed command enum, command-specific positional arity, allowed
required options, and typed step/duration values. Full argv admission completes before app-config or
Engine access. Unknown commands, repeated options, extra positional values, empty required values,
missing tokens, invalid numeric values, and invalid review decisions fail explicitly. A literal --
terminates option parsing; --name=VALUE preserves option-looking text. Help remains configuration-free
and rejects stray arguments. Reviewed-synopsis import now requires its documented PROJECT instead of
an implicit empty-name fallback; the STS public request contract remains separate.

STS and Delivery execution owners receive one admitted Invocation and no longer repeat raw option
loops or discard trailing arguments. Delivery still consumes STS's Completed package API; semantic
Human decisions remain CLI-only and exact-token-bound. The project-relative submission file admission
is retained. Handoff root conversion refuses non-UTF-8 values rather than using lossy display text for
the UTF-8 public DTO. Shared stdout serialization/writing propagates write/flush failure instead of
panicking through print macros; an output failure does not claim that an already-applied effect was
rolled back. Source-guard readers include the new arguments owner and current package consumer calls
but were not executed. No project/CLI invocation or data mutation was run. Tests/build/fmt/lint/CLI/
runtime and source-manifest qualification are **NOT_RUN**.

## Tauri command ingress and application lifecycle — source inspection

All registered STS/Delivery file, probe, and domain-effect commands use async adapters with one
blocking-worker owner. Engine handles and owned inputs cross that boundary; no second workflow state
is introduced. Worker join errors retain the operation name and possible already-applied effects.
Domain failure/cancellation remains an error or an explicit picker None, never an empty successful
selection. Reference/image picker consumers now preserve existing UI selections on None. Reference
and synopsis selection share regular non-symlink, 25 MiB, and strict UTF-8 path/name admission instead
of lossy request paths or fabricated filename defaults. Core ingress still owns the final captured
reference bytes; picker metadata does not qualify later file stability.

App setup loads the packaged manifest before creating the app-data directory. Downloads resolution
and export-directory effects are deferred to export; a path failure is explicit and does not silently
select another export destination or prevent unrelated project startup. DOCX filenames include the
validated project identity to avoid clobbering another project with the same title. Export path UTF-8
is checked before effects. Delivery context/review/preview and run resolve Completed authority inside
the worker. Preview requires actual MIME metadata and consumes the existing verified-byte API.
Command identifiers, registration, public producer DTOs and ownership are preserved; picker Option
returns and their existing UI consumers form the intentional clean break. No new product screen was
added. Source-guard consumers track current package/preview owner calls but were not run. No native
app, dialog, export, user project or runtime effect was executed. Tests/build/fmt/lint/native UI/race/
runtime and manifest qualification are **NOT_RUN**.

## UI workflow state and request consumers — source inspection

Project choice, global project-list/runtime reads, delivery context/list, delivery preview, and artifact
list/read responses are correlated with their current UI scope and latest request. Project changes
clear derived delivery/artifact projections and invalidate pending reads; mutations block project
switching and duplicate submissions. A new or failed preview clears the prior review selection before
any approval action. Delivery generation cannot overwrite a later user-selected result, and review
serialization protects its specific receipt selection. These are ephemeral UI correlation tokens,
not another workflow or approval authority. STS and Delivery still own every durable effect.

Mutation results are presented before optional list refresh. Saved decision/delivery/review plus a
failed follow-up read is reported separately from execution failure. Failed current-project reload
marks the view unavailable and disables mutation without fabricating a new backend state. Runtime
query failure remains unknown rather than asserting the CLI is absent. Cancellation says requested
until the owner reports its outcome. Browser samples are explicitly labeled; Native empty-project
run no longer injects DEMO, and sample mode cannot execute effects or Human decisions.

NewProject passes the entered synopsis/title unchanged after nonempty admission. Positive u32 input
checks have one UI owner. Native reference records have a closed shape; drag/drop explicitly maps
paths instead of legacy file/path/string coercion. Invalid replacement selection preserves existing
files, and late picker/reset responses cannot overwrite a newer form. Image staging retains project/
package provenance, uses unapproved-reference wording, and requires render-capable backend metadata;
ambiguous backend selection must be resolved explicitly. Delivery read failure clears stale approval
controls and reports unknown records. Startup failures do not prevent independent successful reads.

Existing screen hierarchy, graph renderer, input IDs and Tauri command identifiers are retained; no
new product screen was created. No browser/native UI, JS checker, test, provider or project mutation
was executed. Source correlation is not runtime race/accessibility proof. Tests/build/lint/JS syntax/
UI interaction/visual/race/runtime and source-manifest qualification are **NOT_RUN**.

## Graph-preview parser, layout, and result consumer — source inspection

Graph preview admits the actual producer shape: closed schema-version/node/edge objects, unique
nonempty unit IDs, unique nonempty canonical refs, preserved string/null content, and exact ordered
sequence edges. Missing refs/content/kind are no longer fabricated. Frozen parsed models carry private
admission identity; layout cannot consume an unparsed mutable substitute. One linear sequence layout
replaces repeated dependency-depth relaxation, sorting, and cyclic/disconnected approximation. Existing
mechanical fixtures have complete producer-valid node fields so they still target dangling-edge and
single-node layout cases; no tests were executed or new runtime claims made.

The initial SVG viewport preserves readable unit dimensions instead of fitting the whole potentially
long sequence into a tiny overview. Pan/wheel coordinates are transformed through the SVG screen CTM,
not mixed with CSS pixels. Node selection and background pan have separate input roles; primary pointer
identity, cancellation and capture-loss cleanup are explicit. Per-view arrow IDs avoid document-global
collisions. Selection uses direct element ownership and aria-pressed; keyboard arrows/Home/End move
focus and reveal the focused unit. Heading snippets, full tooltip identity and selected body retain
source content; summary counts units/links rather than assuming one scene per node. Actual font,
resize, gesture, accessibility and frame behavior remain unqualified.

The existing JSON result consumer reports parser/render failures with their cause while retaining raw
JSON. Graph/raw switching reuses one rendered view and preserves its selected detail/pan state instead
of recreating it. No durable graph/schema authority or new product screen is introduced. Producer wire
bytes and registered contract pins are unchanged. Tests/build/lint/JS syntax/native/browser/visual/
accessibility/performance/runtime and source-manifest qualification are **NOT_RUN**.
