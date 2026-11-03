# NVC STS — 구현 분석

공개 artifact 계약과 공통 path·token의 정본은 [contracts](../contracts/ANALYSIS.md)와 [Runtime](../runtime/ANALYSIS.md)에 둔다.

## Verdict·책임 경계

서사 workflow·gate·Completed release의 유일한 semantic authority다. caller는 CLI/Tauri와 제한된 MCP다. 전달 포맷의 state는 delivery가 소유한다. 방향 **IMPROVE**. 상위 제품과 별도 제품으로 포장하지 않으며, 이 문서는 독립 material state/effect의 상세를 소유한다. wiring 확신도는 높고 실환경 결과는 아래 범위로 제한한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| handoff import | CLI/MCP → create_project_from_reviewed_synopsis | raw source/handoff schema·hash·approval claim | Engine/project source snapshot | 새 project 또는 integrity error | `src/engine/project/creation/reviewed_synopsis.rs` |
| agent node | run/next/agent-task/submit → Engine | current source/task token/manifest output | node durable state | pending/human gate/blocked | `src/engine/workflow/tasks/packet.rs`, `tasks/submission.rs` |
| Human gate | UI/CLI → decide | gate token·allowed decision·current evidence | human decision + Completed state | approve/revise/reject/Completed | `src/engine/workflow/decisions.rs::decide` |
| Completed resume | UI/CLI → resume | project lock·source·package·bundle existence/integrity | approved state readonly; derived bundle export | Completed 또는 explicit error | `src/engine/workflow/resume.rs::resume` |
| release consumer | CLI/SMC/delivery → release_bundle/narrative_package | Completed + raw-byte identity | release artifacts | verified package/bundle | `src/engine/release/{narrative_package,bundle}.rs` |
| manifest and read projections | CLI/Tauri/MCP → validate/list/read/view/doctor | executable manifest·current state·hash-bound artifacts | STS-owned read-only DTOs | current projection or integrity error | `src/engine/manifest.rs`, `src/engine/views/{diagnostics,catalog,project}.rs` |

## 구현 모듈 경계

| 모듈 | 책임·상태 owner | 외부 계약 |
|---|---|---|
| `src/lib.rs` | public STS composition and crate-root `Engine`/DTO entrypoints; DTO names are explicitly re-exported from their source owners | Runtime `Error`/`Result` are imported directly by their source owners; the contracts crate alone exports `NarrativePackage`; adding a DTO to the public root requires an explicit export |
| `src/engine.rs` | `Engine` composition and child-module wiring | `Engine` is public only at the STS crate root; redundant path getters were removed so project/asset path policy remains with RuntimePaths and path owners |
| `src/codex.rs` | Codex module composition without a `selected_generation_policy` forwarding export | config is crate-internal; runtime status and state initialization import the owner directly |
| `src/codex/config.rs` | model/reasoning/timeout policy와 executable path resolution | environment validation과 existing defaults 유지 |
| `src/codex/runtime_status.rs` | version/login bounded probe, generation-policy diagnostics, and `codex_runtime_status` projection | invalid policy is not replaced with an advertised default; readiness probe does not mutate workflow state |
| `src/codex/runner.rs` | one task's private workspace, read-only Codex process, copied inputs, image args, artifact-body UTF-8 file admission | process timeout/cancel remains Runtime-owned; candidate text is not approved |
| `src/codex/workflow.rs` | current Agent task orchestration, workflow-lock linearization, result submission, one-action and run-to-human flow | STS alone owns task transitions and submission |
| `src/codex/workflow/lease.rs` | run ID + node + task-token identity, active/cancel marker encoding, strict reads and owned cleanup | private STS execution lease; no persisted state DTO or cross-component contract |
| `src/model.rs` | private namespace declaring the project/view/workflow DTO source modules; no type re-exports | internal consumers import the defining owner path; `src/lib.rs` directly re-exports the same approved crate-root DTO names |
| `src/model/workflow.rs` | manifest, node state, source snapshot, durable State DTOs | persisted serde fields·enum names 유지 |
| `src/model/project.rs` | project config, reviewed-synopsis binding, import/create request DTOs | request schemas와 approval/source binding 유지 |
| `src/model/views.rs` | task/decision results, diagnostics, artifact and UI projections | public read/result DTO fields 유지 |
| `src/engine/manifest.rs` | load/validate/domain-identity `Engine` entrypoint facade | 기존 public method paths와 errors 유지 |
| `src/engine/manifest/validation.rs` | manifest schema/stage/node order와 produced-path dataflow | manifest order가 선행 Agent artifact·revision 범위를 결정 |
| `src/engine/manifest/validation/agent.rs` | Agent-only fields, role, prompt/principle path, inputs, outputs 검증 | prompt root와 closed token set 보존; ordered available-path set도 output uniqueness를 소유 |
| `src/engine/manifest/validation/human.rs` | Human-only fields, source binding, revision target, gate policy 검증 | earlier Agent artifact에만 gate를 결속 |
| `src/engine/manifest/identity.rs` | manifest dependency file set 순서화와 domain identity hash | workflow/prompt/principle raw bytes identity 보존 |
| `src/engine/project.rs` | project path facade and project child-module composition; calls Runtime's canonical name/path validation before creating the projects root | project creation/interchange remain STS entrypoints; the duplicate public name-validation wrapper is removed |
| `src/engine/project/creation.rs` | generic NewProject initialization, staged files, reference copy, state write and atomic directory publish | exact synopsis bytes and private verified-binding initialization sequence |
| `src/engine/project/creation/reviewed_synopsis.rs` | geultu handoff parsing, identity/raw-hash/title binding and exact-byte STS ingress | reviewed synopsis bytes remain bound; no screenplay/package ingress |
| `src/engine/project/screenplay.rs` | completed screenplay Fountain/FDX export and Fountain/FDX project import | current Completed screenplay gate and reference artifacts |
| `src/fountain.rs` | Fountain/FDX public format facade | 기존 `detect_format`·`normalize_screenplay`·export API |
| `src/fountain/types.rs` | format-independent normalized screenplay and structural summary DTOs | 기존 type fields와 enum variants |
| `src/fountain/import.rs` | format detection, import dispatch, shared structure builder | supported-format routing과 summary aggregation |
| `src/fountain/import/fountain.rs` | Fountain 1.1 subset parsing and note/boneyard normalization | input order·heading/cue rules 유지 |
| `src/fountain/import/fdx.rs` | FDX XML subset parsing and paragraph routing | current XML unescape/paragraph policy 유지 |
| `src/fountain/export.rs` | canonical screenplay to Fountain/FDX serialization | existing headings, cue markers, and XML escaping |
| `src/engine/state.rs` | state initialization and durable `state.json` load/save facade | persisted `State` fields·schema version·project identity |
| `src/engine/state/config.rs` | project config contract and synopsis/reference path resolution | supported source formats·safe relative paths·reference types |
| `src/engine/state/source_snapshot.rs` | raw-byte config/synopsis/reference fingerprint construction and freshness comparison | existing `SourceSnapshot` hash identity |
| `src/engine/state/inputs.rs` | ordered manifest input resolution and durable Agent-output/Human-feedback hash checks | producer completion and current artifact bytes |
| `src/engine/workflow.rs` | workflow submodule composition for task identity, submission, fidelity admission, Human decisions, and resume | 기존 `Engine` public methods는 child modules에서 동일 유지 |
| `src/engine/workflow/tokens.rs` | next-node resolution, task/gate token identity, agent execution contract, path labels | 기존 token의 payload/hash와 내부 조회 범위 |
| `src/engine/workflow/tasks.rs` | stable `Engine` task entrypoint facade | existing `next`/`task_text`/`task_material`/`submit` methods |
| `src/engine/workflow/tasks/packet.rs` | next-task projection and bounded prompt/input materialization | source snapshot, task token, ordered inputs, binary input handling, explicit text-file I/O/UTF-8 failure |
| `src/engine/workflow/tasks/submission.rs` | required-evidence validation, token-bound artifact commit, versioned output history | existing lock, archive, atomic-write, state/event ordering |
| `src/engine/workflow/fidelity/mod.rs` | private source-fidelity composition; exposes alignment and source-coverage owner modules only inside workflow consumers | no validator forwarding alias, new narrative authority, or cross-component surface |
| `src/engine/workflow/fidelity/source_inventory.rs` | typed SRC/REL declarations, epistemic/polarity/materiality parsing, and obligation sensitivity | Source IR remains STS-owned; parsed obligation DTOs stay private |
| `src/engine/workflow/fidelity/text.rs` | shared closed Markdown table, identifier and assignment grammar used by both admission stages | `pub(super)` helpers only; no general-purpose Markdown parser |
| `src/engine/workflow/fidelity/alignment.rs` | STEP 8D parses recovered Screenplay IR and binds each source obligation to an OBS/OREL and SCN evidence row | reads only the manifest-declared Source IR and source-blind Screenplay IR inputs |
| `src/engine/workflow/fidelity/source_coverage.rs` | STEP 0 binds each SRC/REL obligation to a unique exact synopsis span and requires lossless non-whitespace coverage before commit | reads only the approved synopsis and proposed Source IR; no narrative inference or new story authority |
| `src/engine/workflow/decisions.rs` | Human allowed actions, gate decision, revision invalidation, approval/revise/pause events | gate token·durable state owner·release publication 순서 |
| `src/engine/workflow/resume.rs` | paused resume와 Completed release bundle repair-only recovery | 승인 state bytes 보존 및 corrupt bundle fail-closed |

The four production workflow consumers `decisions`, `tokens`, `tasks::packet`, and
`tasks::submission` name their exact required DTOs from the private `model` composition surface;
they no longer glob-import every model re-export. `model` remains the internal canonical model
composition and `lib.rs` remains the sole public DTO entrypoint. No persisted field or public export
is changed by this source-only import cleanup; tests/build/fmt/Clippy/CLI/runtime/manifest
verification are **NOT_RUN** afterward.

`tasks::packet` propagates `read_to_string` failures for extension-classified text inputs through the
existing Runtime `Error::Io` boundary. It no longer turns invalid UTF-8, permission errors, or other
read failures into a successful placeholder prompt. This aligns task assembly with STS's fail-closed
input authority; the change is source-only and its tests/build/CLI/runtime remain **NOT_RUN**.
| `src/engine/release.rs` | release lifecycle child-module composition only | 기존 producer schemas와 release entrypoints |
| `src/engine/release/scene_map.rs` | canonical screenplay/scene IR로 STS-owned NarrativeSceneMap publication과 freshness check | `scene_map.rs::build_scene_map` authority 보존 |
| `src/engine/release/narrative_package.rs` | source-bound NarrativePackage construction, publish, and verification | package identity·raw artifact hashes 보존 |
| `src/engine/release/bundle.rs` | Completed-state NvcReleaseBundle publication and cross-artifact proof | 승인 state identity와 canonical paths 보존 |
| `src/engine/views.rs` | read-view child-module composition only | 기존 public read entrypoints |
| `src/engine/views/diagnostics.rs` | config/source/artifact/release integrity diagnosis | 오류와 경고를 기존 DTO로 반환; state 변경 없음 |
| `src/engine/views/catalog.rs` | read-only project enumeration and shared artifact-list projection | no root-directory creation or config/state error-to-title fallback; artifact output/role come from the validated manifest |
| `src/engine/views/project.rs` | review/final-screenplay/stage UI projection | Human source and checkpoint targets are hash-verified before review content is exposed; completed screenplay version must come from manifest/state |
| `src/engine/{events,paths,project_files}.rs` | durable event metadata, STS project-root guards, input filename/type policy | 내부 helper만; 새 public contract 없음 |
| `src/scene_map.rs` | screenplay·blueprint 입력을 source-hash-bound `NarrativeSceneMap`으로 묶는 facade | 기존 `build_scene_map` entrypoint와 schema/hash envelope |
| `src/scene_map/screenplay.rs` | per-scene line-state composition과 `parse_scenes` facade | 기존 scene-map 호출자와 DTO 조립 순서 유지 |
| `src/scene_map/screenplay/heading.rs` | canonical SCN heading grammar, setting/location/time parsing, time-of-day normalization; imports contracts' scene-ID owner | heading 오류 토큰·label/canonical value 유지; syntax projection only |
| `src/scene_map/screenplay/dialogue.rs` | indented/inline dialogue, speaker cue, parenthetical recognition | speaker/action/silent-scene rules 유지; prose 의미 추론 없음 |
| `src/scene_map/blueprint.rs` | scene blueprint directive·asset requirement routing and contract-owned SCN ID admission | 기존 annotation label set 유지; no local copy of scene-ID grammar |
| `src/scene_map/semantic_manifest.rs` | occurrence ID·entry hash·source locator 생성 | raw source occurrence를 보존하고 semantic approval을 주장하지 않음 |

## 실제 흐름

`verified import → source snapshot → manifest-selected task → submit(token) → Human gate(token) → final artifact checks → NarrativePackage → Completed state → NvcReleaseBundle`. release export 실패 후 `resume`는 같은 승인 state를 바꾸지 않고 누락 bundle만 복구한다.

## 현재 state·contract·I/O owner

Engine owns node status·task/gate identity·source binding·approval. skills manifest는 executable configuration이며 prompt output은 검증 전 후보다. package/map/bundle schema는 producer 계약. 다른 저장소의 손상 artifact를 임의 보정하지 않는다.

`scene_map.rs`는 raw screenplay/blueprint bytes와 output identity를 결속하고 screenplay parser, blueprint router, semantic occurrence builder를 조합하는 STS facade다. `screenplay.rs`의 `SCN-*`, heading/time, dialogue parsing은 supported syntax만 projection하며 scene-ID 문법은 contracts owner를 호출한다. `blueprint.rs`도 같은 validator를 사용해 local grammar duplication을 없앤다. 기존 label set을 scene별 directive/asset requirement로 라우팅하고, `semantic_manifest.rs`는 occurrence identity와 locator를 계산한다. 어느 단계도 prose의 사건 의미나 대사 화용을 승인하지 않는다. 인라인 `SPEAKER: line`은 지원 형식상 모호성이 남으므로 parser 결과를 별도 의미 authority로 사용하지 않는다.

STEP 8D's workflow-owned alignment validator reads only its declared Source IR and source-blind
Screenplay IR inputs. It requires one row for each CORE/SUPPORT source item and protected TEXTURE
assertion, rejects duplicate or unknown IDs, checks SRC→OBS / REL→OREL typing, and binds each cited
SCN to the recovered item. Submission runs it before artifact publication; final Human release
repeats it before release effects, blocking old marker-only READY artifacts as well. READY is
blocked by UNCERTAIN rows and protected/CORE omission or drift. This proves structural evidence
binding, not that a model's interpretation is semantically true. Regression fixtures cover missing
table/coverage and invalid references. Tests/build/fmt/Clippy/CLI/runtime/manifest freshness remain
**NOT_RUN**.

The fidelity implementation is now split by input authority: `source_inventory.rs` owns typed source
obligations, `text.rs` owns the small grammar shared by both admission stages, `source_coverage.rs`
owns STEP 0 synopsis-span coverage, and `alignment.rs` owns STEP 8D reconstruction binding. This
module split is source-only; current tests/build/fmt/Clippy/CLI/runtime remain **NOT_RUN**.

Workflow decisions and token-bound submission now import STEP 8D validation directly from its alignment owner. The fidelity composition grants that implementation module visibility only within the workflow tree and no longer forwards the function from its root. Public crate exports, workflow semantics, serialized state, and artifact formats are unchanged. This internal clean break remains unverified per request.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| STS gates/import/release | REACHABLE | PARTIAL | PASS baseline: current workspace check + 60 Rust tests/fmt/clippy; latest manifest owner split is source-only / NOT_RUN; external provider/작품 검토 NOT_RUN | `src/engine/manifest/`, `project/state/views`, `workflow/*`, `release/*`, tools verifier |
| Completed repair-only resume | REACHABLE | PARTIAL | PASS: current workspace workflow regressions, including preserve/repair/fail-closed cases | tests/workflow.rs |

## 의존성·findings

**NS1.** 문제 → Completed resume의 state 재저장이 completed_state hash를 깨뜨리고 export 실패 복구가 없었다. 근거 → `workflow/resume.rs`와 `tests/workflow.rs` 회귀. 원인 → 재개와 파생 결과 복구 혼합. 영향 → 승인된 release를 consumer가 거절. 방향 → source/package를 확인하고 existing bundle은 검증만 한다. NotFound만 exact approved-state 재발행을 허용하고 corrupt/symlink bundle은 실패한다. 새 approval·rollback은 추가하지 않았다.

**NS2 · scene-map annotation 분기 — 단순화, 실행 미검증.** 문제 → `requirement_line`의 explicit label set에 있는 모든 항목이 바로 아래 asset/prop substring 조건에도 포함되어 `explicit || hinted`가 항상 asset 분류로 이어졌다. 그 결과 `requirement` 출력 분기는 도달할 수 없었다. 개선 → 현재 인식되는 asset/prop/소품/자산 label의 규칙을 하나의 힌트 판정으로 표현했다. 정식 SCN header와 supported screenplay syntax, map owner는 유지했고 새 annotation label을 추가하지 않았다. 구조 parse는 narrative meaning verification이 아니다. STS build/test를 실행하지 않았다.

## 실패·취소·재시도·복구와 남은 gap

Codex cancellation acceptance and artifact submission share `.workflow.lock`. The active lease binds
run ID, node ID, and task token; cancellation succeeds only while that exact task is current. A
matching cancel marker is checked under the same lock immediately before submit, so a late accepted
cancel cannot be reported as a completed task. Marker read errors propagate. `agent_run_is_active`
opens only an existing lock file and does not create the project directory during observation.
Delivery cancellation-marker reads also propagate non-NotFound I/O failures. These changes preserve
the existing owner-specific leases and do not add a durable cross-component contract.

초기 package/Completed/bundle은 여러 파일이므로 crash transaction 하나가 아니다. completed bundle 재생성 테스트를 실제 Rust/filesystem에서 실행해야 한다. stale task/gate token 거절·명시 pause/cancel/revise는 유지된다. cryptographic human identity를 증명하는 시스템은 아니다.

## gap·검증·근거

`tools/verify_contract.py`와 `tools/verify_delivery_source.py`는 현재 private module graph 전체를 읽으며,
NVC `verify-all.sh --require-rust`에서 static checks와 56개 Rust tests/fmt/clippy가 통과했다.
이 증거는 local runtime/fixture coverage이며 외부 Codex 호출이나 실제 작가의 의미·문체 승인을
대신하지 않는다.

[상위 ANALYSIS](../../ANALYSIS.md) · [제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md). source 근거 경로는 이 분석 단위 기준이다. PASS는 표시한 검사에만 적용하며 PUBLIC_LIBRARY/REACHABLE은 실행 성공을 뜻하지 않는다.

## STS 입력 경계 파트 — 소스 정리 완료, 실행 미검증

범위는 project creation/reviewed-synopsis ingress, NewProject/ReferenceInput DTO와 필수 source config
consumer다. 일반 NewProject의 caller-supplied reviewed_synopsis를 제거하고 전용 ingress만 private
captured-input을 조립한다. Producer schema와 handoff identity는 기존 contracts owner를 소비한다.

검증한 handoff bytes를 경로에서 다시 복사하지 않고 그대로 reference에 저장한다. Direct synopsis도
trim/newline normalization 없이 exact bytes를 저장한다. Binding summary 조립은 한 owner가 맡고,
load_config는 stored synopsis/title와 retained approval receipt/hash/identity를 대조한다. Synopsis,
references와 config는 project-root regular-file resolver를 사용하며 duplicate reference names를 거절한다.
Reference reading은 기존 25 MiB 한도를 실제 bounded read에도 적용한다.

Staging directory는 exclusive create로 소유권을 확보한다. 기존 temp를 삭제/재사용하지 않으며 own
staging build 실패만 cleanup한다. Synopsis persisted-byte 대조를 rename 전에 마친다. Directory rename
뒤 project-view 오류는 성공으로 바꾸지 않는다. Atomic no-replace against arbitrary external writers,
fd-based adversarial filesystem custody와 자동 project rollback을 추가했다고 주장하지 않는다.
기존 config binding shape와 지속 데이터는 보존한다. Tests/build/fmt/lint/runtime 및 manifest
qualification은 **NOT_RUN**이다.

## STS workflow 입력/snapshot — 소스 정리 완료, 실행 미검증

범위는 state input/source-snapshot owners, state admission과 필수 packet/token asset consumers다.
입력 종류는 canonical path의 prefix로 추정하지 않고 manifest token으로 판정한다. Optional input은
실제 NotFound만 생략하며 permission, wrong type, symlink, output/feedback hash mismatch는 거절한다.
PathBuf identity로 중복을 제거하고 synopsis/references 순서와 빈 reference 목록 의미를 유지한다.

Durable output의 producer state는 checked lookup으로 읽고 feedback은 input/human_feedback/ 안에서만
수용한다. Project/asset 파일 모두 regular-file resolver를 통해 읽는다. Source fingerprints는 canonical
project 안의 relative path만 기록하며 absolute-path fallback을 제거한다. STEP 0 completed/version
기록이 있는데 snapshot이 없으면 오류다. State/snapshot DTO는 unknown fields를 거절하며 state.json
자체도 project-root regular-file boundary로 읽는다.

Packet의 prompt/principles와 task token asset 소비는 동일 resolver에 연결한다. Summary-only input의
project-relative 판정도 canonical project root를 사용한다. Input hash 검사와 소비 사이의 atomic
fd custody, 외부 source mutation의 완전한 차단 및 provider cancellation을 추가했다고 주장하지 않는다.
Tests/build/fmt/lint/runtime과 manifest qualification은 **NOT_RUN**이다.

## STS submission/human-gate 전이 — 소스 정리 완료, 실행 미검증

범위는 workflow task submission/version history, human decisions/invalidation과 token/packet의 필수
allowed-decision consumer다. Manifest node는 빌려 사용하고 응답의 필요한 ID만 복제한다. Private
submission의 세 번째 empty-body guard와 human_allowed의 unused node argument를 제거했다.

Submission/human approval의 version 및 revise count를 checked arithmetic으로 effect 전에 계산한다.
Revision target/range의 in-memory invalidation도 feedback write 전에 수행하여 잘못된 범위가 파일을
먼저 생성하지 않게 한다. Paused project는 resume 전 decision을 거절한다. Final-release source와
output/feedback parents/targets는 regular project boundary로 수용한다.

Hash-verified artifact를 소비할 때는 검증 owner가 반환한 canonical path를 final approval, review,
artifact read, NarrativePackage 구성까지 전달한다. Gate-token source hash도 regular-file resolver를
사용해 project 경계 정책을 공유한다.

Submitted artifact는 trailing-space/newline normalization 없이 exact bytes를 기록하며 persisted hash를
실제 submitted bytes에 대조한다. Feedback도 generated receipt body의 expected hash와 비교한다.
Archive는 regular existing output을 기록 hash와 대조하고 create_new로 충돌 시 overwrite/suffix fallback
없이 거절한다. History parent의 symlink/non-directory와 feedback collision도 실패로 전파한다.
State/save/release bundle 실패는 성공으로 변환하지 않는다.

Artifact/history/state/release를 하나의 crash-atomic transaction으로 만드는 기능이나 부분 발행의
자동 복구는 추가하지 않았다. 실패 시 partial/orphan files가 남고 explicit recovery가 필요할 수 있다.
Tests/build/fmt/lint/runtime 및 manifest qualification은 **NOT_RUN**이다.

## STS fidelity/Source IR validation — 소스 정리 완료, 실행 미검증

범위는 workflow/fidelity의 text, source inventory, source coverage와 alignment owners 전체다.
Source coverage와 STEP 8D의 의미/입력 책임은 그대로 분리한다. Shared table parser는 단일 pipe의
invalid slice를 거절하고 canonical separator를 최소 세 dash와 optional colon으로 판정한다. Coverage/
alignment는 separator를 생략하기 전에도 각 4/5-column shape를 요구한다.

Source metadata는 중복 assignment와 empty/multiword atom을 거절하여 first-wins/trailing-word 수용을
제거했다. 기존 enum vocabulary·case handling과 protected/CORE·SRC/REL/OBS/OREL/SCN 의미는 유지한다.
Unmapped Markdown heading은 synopsis의 complete source line에만 매칭하며 narrative 내부의 동일
substring을 formatting으로 덮지 않는다. Coverage range를 정렬·병합하여 source-sized bitmap과
겹치는 span의 반복 byte marking을 제거하고 첫 non-whitespace uncovered offset을 보존한다.

Alignment는 borrowed row index와 scene citation union을 사용하여 entire row/scene-ID 복제를 줄였다.
Duplicate detection은 첫 duplicate에서 중단한다. Source authority, uncertain/CORE drift READY 거절과
별도 Human adoption은 유지한다. Actual screenplay quality나 원문의 의미 충실도를 이 기계적 grammar
변경으로 증명하지 않는다. Tests/build/fmt/lint/runtime 및 manifest qualification은 **NOT_RUN**이다.


## STS manifest/domain identity — 소스 정리 완료, 실행 미검증

범위는 manifest load/Agent/Human/cross-node validation, domain identity와 필수 DTO/path consumer다.
Stage/node ID는 filesystem-safe bounded segment로 수용하고 duplicate/conflicting markers, input/
principle/summary 선언을 거절한다. Summary-only inputs는 실제로 선언된 earlier output만 받는다.
Human checkpoint의 revision targets는 runtime이 직접 revise하는 step_NN Agent와 일치하며 duplicate
steps와 artifact_review/checkpoint field 혼합을 거절한다. Stage outputs도 실제 produced output을
참조해야 한다. Borrowed ID/path sets와 earlier revision-step index로 복제/반복 manifest scan을 줄였다.

Manifest/asset 수용은 STS의 lexical path grammar와 regular-file resolver를 공유한다. Optional assets는
실제 NotFound만 생략하고 permissions/symlinks/wrong types는 오류다. Identity는 현재 workflow bytes를
읽어 supplied typed manifest와 대조한 뒤 그 same bytes를 hash한다. Ordered domain file set에는 prompt/
principle과 실제 존재하는 static asset inputs를 포함한다. Project source/output/reference는 이 domain
bundle에 합치지 않는다. Canonical workflow의 기존 file order/hash serialization 의미는 유지한다.

파일들을 하나의 atomic filesystem snapshot으로 고정하거나 외부 writer를 차단하는 custody를 추가하지
않았다. Producer schema/pins와 persistent projects는 수정하지 않았다. Tests/build/fmt/lint/runtime 및
manifest qualification은 **NOT_RUN**이다.

## STS SceneMap projection — 소스 정리 완료, 실행 미검증

범위는 SceneMap facade, blueprint, screenplay/heading/dialogue, semantic occurrence projection과
필수 release SceneMap publish/load consumer다. Blueprint는 malformed/duplicate SCN headings,
scene 없이 나온 recognized annotation과 empty asset description을 거절한다. Blueprint SCN은
실제 screenplay scene에 연결되어야 하며 unknown annotations를 묵인해 버리지 않는다.

Screenplay는 duplicate scene ID를 먼저 거절하며 active cue의 연속 indented dialogue/parenthetical
처리를 보존해 첫 줄 이후의 대사를 action으로 오인하지 않는다. Inline dialogue와 silent scene 의미는
유지한다. Blueprint current ID는 borrow하며 scene-map identity의 이미 빈 map_id를 다시 주입하던
wrapper를 제거했다. Semantic occurrence ID/hash 및 optional directive의 명시적 소비 의미는 유지한다.

Release publish/load는 하나의 current SceneMap builder를 사용한다. Captured screenplay/blueprint
bytes를 stored producer hash와 대조해 hashing과 file reread 사이의 source drift를 거절한다. JSON
publication의 newline concat copy와 load error의 blanket conversion을 제거했다. Producer schemas,
public SceneMap version와 contract identity algorithm은 바꾸지 않는다. Atomic multi-file custody나
실제 서사/대사 품질을 이 변경으로 증명하지 않는다. Tests/build/fmt/lint/runtime 및 manifest
qualification은 **NOT_RUN**이다.

## STS NarrativePackage/release bundle — 소스 정리 완료, 실행 미검증

범위는 release package/bundle owners와 필요한 workflow state equality consumer다. Artifact ref는
검사 뒤 파일을 다시 hash하여 다른 bytes를 참조하지 않고 stored producer digest를 사용한다. SceneMap
bytes는 validated map DTO와 대조한 same captured bytes로 raw proof를 만든다. Untracked artifact helper의
약한 root admission 및 blanket load-error conversion을 제거한다.

Bundle 발행은 전체 workflow Completed와 모든 Human gate의 approve 기록을 요구한다. Captured
persisted State는 supplied current state와 정확히 일치해야 하며 package도 current STS package 및
captured DTO에 결속한다. State/package/map hash는 captured bytes에서 계산하고 package hash의
중복 file reads를 제거했다. 발행 뒤 current bundle 수용 경로가 state/package/map proof를 함께
대조한다. Package와 bundle의 SceneMap proof도 동일해야 한다. Pretty JSON newline은 same Vec에
추가하여 concat copy를 제거했다.

Schema/version/identity algorithms와 persistent project state shape는 유지한다. Publication 실패를
Completed success로 바꾸거나 candidate package 파일만으로 release를 인정하지 않는다. Multi-file
atomic custody와 외부 writer 차단은 별도 한계로 남는다. Tests/build/fmt/lint/runtime 및 manifest
qualification은 **NOT_RUN**이다.

## STS read views/diagnostics — 소스 정리 완료, 실행 미검증

범위는 views catalog/project/diagnostics, 공통 durable text projection과 packet의 필수 review-action
consumer다. Doctor는 hash I/O를 None으로 바꾸지 않고 output/feedback authority owner의 실제 오류를
보고한다. Source snapshot 수용을 항상 호출하여 missing-after-STEP-0도 진단한다. Pending/orphan
파일과 release artifacts는 regular file만 warning이며 symlink/unsafe path/I/O는 errors다.

Project view는 이미 읽은 manifest/state로 doctor를 구성하여 별도 재로드 frame 혼합을 줄인다.
Artifact/final/review text는 captured bytes를 stored producer hash에 대조한다. Paused 또는 integrity
failure에는 actionable review를 제공하지 않으며 revision limit에 도달한 revise는 UI/NextPacket의
공통 action projection에서 제외한다. Invariant/backend gate policy는 기존 owner가 그대로 소유한다.

Stage progress는 manifest nodes를 한 번 집계하고 빈 stage를 completed로 만들지 않는다. Review body의
불필요한 clone을 제거한다. Project listing은 reserved hidden staging directories만 생략하며 실제
project load 오류를 missing으로 숨기지 않는다. Updated-at tie는 name으로 정렬한다. Public DTO shape와
source/state authority는 유지한다. Tests/build/fmt/lint/runtime/browser 및 manifest qualification은
**NOT_RUN**이다.

## STS Codex runner/lifecycle — 소스 정리 완료, 실행 미검증

범위는 Codex config, runtime-status probe, one-obligation runner, run/cancel lease와 필수 State policy
consumer다. Generation policy·bounded timeout·explicit executable 경로는 private workspace 생성 전에
수용한다. Missing environment의 기존 default는 유지하지만 invalid UTF-8·empty binary path·invalid
persisted policy를 default로 바꾸지 않는다. Model/reasoning 선택과 process-group cancellation은
기존 owner와 설정 범위를 유지한다.

Task workspace는 exclusive-create이며 Unix에서 0700이다. 충돌한 기존 경로를 재사용하거나
삭제하지 않고 자신이 만든 workspace만 정리한다. Final artifact는 regular UTF-8 file이어야 하며,
empty 판정에만 trim을 사용하고 author output bytes는 그대로 submission에 넘긴다. Clock/config/
launch/timeout/encoding 오류는 실제 Result로 반환해 설치·인증 안내와 구분한다. Ignored local
AGENTS.md 생성·상수와 filename fallback은 제거했다.

Active/cancel markers는 regular-file·bounded 4096-byte read·closed identity DTO로 확인한다.
Agent-run lock, workflow lock, current task-token 재검사와 cancellation linearization을 보존하며
paused/Human 경계를 넘지 않는다. Run outcome와 lease cleanup을 한 경계에서 조합해 original
execution/submit failure를 성공이나 cleanup failure로 숨기지 않는다. Workspace/lease cleanup
실패도 성공 artifact 반환을 막고, 이미 committed artifact가 있으면 그 사실과 cleanup 오류를
함께 보고한다. Completed resume의 검증된 NarrativePackage/Bundle 복구는 기존 recovery owner가
소유한다.

Marker lstat/read와 소비 사이 adversarial race, final artifact allocation 한도 및 complete provider
capability certification은 별도 검증 범위다. External Codex/model semantic quality와 실제 사용자
workspace/markers는 이번 정리에서 실행·삭제·수정하지 않았다. Tests/build/fmt/lint/runtime/provider
qualification은 **NOT_RUN**이며, 문서 중복 통합과 source manifest 검증이 이를 대체하지 않는다.

## STS Fountain/FDX interchange — 소스 정리 완료, 실행 미검증

범위는 format dispatch/types/import/export와 필수 Engine screenplay interchange consumer다.
FDX substring scanner를 workspace에 이미 있는 quick-xml 0.41 parser로 교체했다. 하나의 FinalDraft/
Content 구조, closing tags와 supported Paragraph types를 수용하고 incomplete XML, DOCTYPE와 unknown
paragraph approximation을 거절한다. Entity/numeric reference/CDATA를 처리하며 Character와 연속
Parenthetical/Dialogue를 한 block으로 보존한다. 이미 괄호가 있는 parenthetical을 다시 감싸지 않는다.

Fountain은 heading/transition 뒤 같은 block의 본문을 버리지 않으며 unterminated boneyard는 오류다.
Summary builder는 중복 heading String을 count로 바꾸고 unknown setting을 interior로 표시하지 않는다.
FDX export의 parenthetical은 actual character context에서만 분류하며 XML escaping의 반복 전체 String
replacements를 한 pass로 정리했다. Existing public format APIs와 supported vocabulary는 유지한다.

Engine import는 기존 reference bound를 사용하고 exclusive staging/own cleanup errors를 명시한다.
Export는 Completed release proof와 actual screenplay bytes hash를 수용하고 unsafe target을 거절한다.
Raw source reference는 그대로 보관하며 normalized format references가 STS narrative authority를 만들지
않는다. Complete Fountain/FDX compatibility, character-level lossless roundtrip, arbitrary XML schema
compliance 또는 actual writing quality를 증명하지 않는다. Existing dependency/lock metadata와 source
manifest만 갱신하며 tests/build/fmt/lint/runtime/manifest qualification은 **NOT_RUN**이다.


## STS DOCX export — 소스 정리 완료, 실행 미검증

범위는 DOCX document/OPC publication, 공통 screenplay syntax tokens와 필수 Fountain/FDX 및 Engine
export consumers다. 공통 parser는 `screenplay_text.rs`가 소유하며 DOCX 안의 parser/Token forwarding을
제거했다. Borrowed text tokens와 direct char_indices scan으로 String/char-array 복제를 줄이고 active
indented dialogue의 짧은 발화를 새 speaker로 오인하던 분류를 정리했다. Format projection은 narrative
semantic authority를 소유하지 않는다.

DOCX는 최종 document XML을 한 String에 직접 구성하며 paragraph/escape intermediate strings를
제거한다. XML 1.0에 기록할 수 없는 characters는 삭제하지 않고 거절한다. Static relationships/content
 types/styles는 private `docx/assets.rs`로 분리하고 기존 스타일/OPC path를 유지했다.

완성 ZIP은 exclusive staged file에서 finalize/sync한 후 Runtime atomic publish로 발행한다. 기존
output을 먼저 truncate하지 않으며 regular-file target을 수용한다. Own staging cleanup 실패를 원래
publication failure와 함께 보고한다. Engine export는 모든 포맷과 같은 Completed screenplay source
proof를 소비한다. Parent sync 실패는 발행 뒤에도 오류이며 old-output rollback을 주장하지 않는다.

Word 실제 렌더, 파일 roundtrip 및 multi-writer filesystem custody는 미검증이다. Existing test source를
syntax owner로 옮겼고 실행하지 않았다. Tests/build/fmt/lint/runtime/DOCX viewer와 source-manifest
qualification은 **NOT_RUN**이다.

