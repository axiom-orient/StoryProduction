# 글투 — 구현 분석

## Current source owners

이 문서는 현재 reachable source와 caller의 책임을 설명한다. 날짜가 붙은 실행 결과와 후속 source의 qualification은 [제품 Current](docs/IMPLEMENTATION_STATUS.md)가 소유한다. 아래의 source 구조 설명만으로 test/build/runtime 검증을 주장하지 않는다.

Changeset decision binding borrows the changeset and candidate from one loaded Project rather than
cloning both values. Candidate content and receipt stay in that snapshot through `bind_decision`;
required reviewer/revision/digest claims and the later journal transaction remain unchanged.

The 2026-10-01 recorded current-source run passed 96 Rust tests; the official gate stopped at three
rustfmt layout differences in `store/history.rs`, `store/journal.rs`, and `store/recovery.rs`. Those
imports/signature layout were then aligned without changing behavior. That 2026-10-01 record did not
include a gate after the formatting edit. The later current-source gate is recorded in [Geultu
Current](docs/IMPLEMENTATION_STATUS.md); the original dated result remains in the
[root revalidation report](../docs/research/2026-10-01-live-use-revalidation-current-tree.md).

### Authoring API and state

범위는 `authoring/mod.rs`, `api/`, `mutation/`, `read.rs`, `validation.rs`, `conformance.rs`와
story/synopsis craft 입력, Application dispatch·tool 등록, domain decide/reduce·snapshot admission의
필수 호출 경로다. Story/synopsis state는 Geultu journal에 남으며 STS의 씬 의미·release를 대신하지 않는다.

| owner | 분석·변경·유지 |
|---|---|
| `mod.rs` | Kind·Brief·Document·Task·Draft·Candidate·Decision·State·Mutation·Action의 기존 serde fields와 public names 유지. 모델은 persisted 값이며 runtime adapter 상태가 아님 |
| `api/call.rs` | Operation→Action decode와 CAS transaction, commit 뒤 packet/candidate/status 선택을 소유. 응답 선택은 private ResultView의 ID만 보관해 Draft를 포함한 Action 전체 복제 제거 |
| `api/requests.rs`, `api/tools.rs` | 폐쇄 request DTO와 public tool schema·이름·입력 제약 유지. 같은 문자열이 runtime instruction과 tool 설명에 존재해도 채널·시점의 역할이 달라 제거하지 않음 |
| `mutation/mod.rs` | Private next-state와 request fingerprint/idempotency 조합 유지. 하위 handler는 ID/hash의 `&str`, Copy enum/revision/bool을 받아 불필요한 owned-value 참조 제거 |
| `mutation/documents.rs`, `tasks.rs` | Source intake, edit CAS, 역사 binding을 유지하는 restore, source/guidance-bound prepare와 cancel. 검증한 dependency가 task input에서 빠지면 조용히 누락하지 않고 Corrupt 반환 |
| `mutation/candidates.rs`, `review.rs` | Late/cancelled candidate 보존·based_on 같은 task 제한·exact selection·별도 reviewer/hash/currentness 검사를 유지. Review의 전체 Candidate 복제와 partial merge 전에 버려지던 draft-block 복제 제거 |
| `read.rs` | Block ID index로 before/after·remove 조회, task ID set으로 has_candidates 계산. 문서/후보/블록의 기존 출력 순서·값과 채택 전 label 유지 |
| `freshness.rs` | 한 조회/결정의 immutable Project를 빌려 current document hash·guidance·dependency currentness를 재사용. Task의 캡처된 blocks는 정확한 값으로 대조하고, 부분 선택은 block index 사용. 영속 cache나 새로운 상태 권한을 만들지 않음 |
| `validation.rs`, `conformance.rs` | Text/block admission과 저장 graph/lineage checks 유지. 입력·현재 권한·저장 graph의 서로 다른 검사를 합치지 않음 |
| `story.md`, `synopsis.md` | 실제 본문 작성·원문 의미·문체·이름·관계·ending 보존, assumptions/open questions와 no-auto-approval의 작업 입력 유지 |

`Application → authoring API → Workspace transaction → domain apply/evolve → journal/snapshot`은
같은 경로다. 부분 채택은 현재 문서의 순서를 유지하고 선택 블록만 바꾸며, source/guidance가
바뀌거나 취소된 task는 채택하지 못한다. 거절은 본문을 바꾸지 않는다. 호환성 alias나 자동 migration은 public contract에 없다.
Status는 같은 task의 pending 후보들에 blocker 계산을 한 번만 수행한다. Prepare는 선택·freshness
검사를 마친 뒤 base를 복제하며, review는 같은 current snapshot의 dependency hashes를 사용한다.
공개 `stale_reasons` 경로·serde fields·reason 순서·late/cancelled 후보 보존·선택 밖 편집 보존은 유지했다.

### Generation plan and provider boundary

범위는 `generation.rs`, `generation/hash.rs`, `generation/registry/{writing,provider}.rs`,
Application generation dispatch/request/context/pack policy, tool catalog의 필수 연결과
`adapters/geultu-writing`이다. Plan generation은 canonical state를 쓰지 않으며 provider 결과도
후보 payload일 뿐 승인·본문 적용이 아니다.

| owner | 분석·변경·유지 |
|---|---|
| `generation.rs` | WritingType/Stage·request/context/plan/draft·trait의 public names와 serde shape 유지. Context 정렬은 ID를 비교할 때 복제하지 않으며 scene/timeline order→ID와 fact/thread ID 순서를 보존 |
| `generation/hash.rs`, `registry/` | Input digest, adapter/type/provider 선택과 ambiguous registration 거절, project/type/stage/revision·prompt 및 returned draft admission 유지. Provider 실패를 성공이나 빈 draft로 바꾸지 않음 |
| `application/generation.rs` | Current Project에서 context를 만들어 registry에 전달한 뒤 verified craft reference만 보강. Pack 위치·loading 구현을 독립 Application-local `packs.rs`로 분리 |
| `application/packs.rs`, `craftpack/paths.rs` | 한 번 선택한 위치를 그대로 사용. Generation의 실제 미구성과 mandatory refinement/pack 작업을 구분. 명시된 Application/environment 경로의 missing/corrupt 입력은 default pack으로 대체하지 않고 기존 loader 오류로 중단 |
| `application/tools/{generation,voice}.rs` | 오해를 주던 `tools/authoring.rs`의 generation/voice metadata를 각각의 이름으로 분리. Public tool names·schemas·description·등록 순서는 그대로 유지; 실제 `authoring.*` catalog는 authoring API owner에 유지 |
| `application/dispatch/generation.rs`, request DTO | 기존 Application→plan→selected provider→untrusted draft/candidate response 연결 유지. 실행·지속성·승인을 catalog나 type adapter에 옮기지 않음 |
| `adapters/geultu-writing` | Novel/fiction/synopsis/storyboard의 pure plan projection. 반드시 source revision을 렌더링하므로 도달하지 못하던 empty-context placeholder 제거 |

의도적 clean break는 잘못 설정한 `GEULTU_ASSETS_ROOT`에서 다른 default pack을 쓰던 경로의
제거다. Env를 설정하지 않은 경우의 기존 발견 순서와 optional no-pack 동작은 유지한다.
해시·model/provider·source authority를 합치거나 새 compatibility API를 추가하지 않는다.

### Review and decision boundary

범위는 `review/{diagnostics,proof,text}.rs`, `receipts/{subject,decision,validation,timestamp}.rs`,
Application approval·review dispatch/request/tool catalog, candidate/changeset DTO, domain review
command·reducer와 HumanDecision 계약이다. 저널·snapshot은 기존 store를 소비하며 다른 상태 owner를
만들지 않는다. Authoring workbench의 별도 Draft/Decision 흐름과 generic candidate/changeset을 합치지 않았다.

| owner | 분석·변경·유지 |
|---|---|
| `application/approval.rs` | Claim→exact subject binding 유지. `commit_decision`이 응답 decision을 dispatch 전에 serialize하고 같은 owned decision을 Command에 넘겨 commit 응답 조합을 한 곳에서 수행 |
| `application/dispatch/review/{candidates,changesets}.rs` | Reject/approve의 반복 decision clone·commit·후처리를 제거. Candidate subject lookup의 pending 제한과 existing public operation names 유지 |
| `application/requests/review.rs`, tool catalog | 모든 review-specific request DTO가 unknown fields를 거절해 기존 `additionalProperties:false`와 맞춤. 필드·required claim·human-only 노출 정책 유지; compatibility 입력을 추가하지 않음 |
| `domain/command/review.rs`, reducer | Pending 상태·project/content/scene revision·operation validity·subject/kind/scope·budget와 event 순서 유지. Command에서 event로 넘기는 ID의 불필요한 복제 제거 |
| `review/proof.rs` | Grounding membership을 source/evidence set에서 직접 조회해 cloned union 제거. Missing protected terms도 빌려 사용. 기존 lexical scores·임계값·findings 순서 보존 |
| `review/diagnostics.rs`, `text.rs` | Report 조립 중 scene/timeline ID를 빌려 중복 title/sentence/order를 집계. Style metric의 line vector는 한 번의 count fold로 교체. 기존 report fields·순서·dialogue/문장 판정 유지 |
| `receipts/` | Candidate JCS subject, reviewer claims, receipt raw/JCS identity, validator/version/outcome/finding mapping와 UTC timestamp issuance 유지. Receipt는 검토 근거이며 창작 품질·사람 승인 자체가 아님 |
| `contracts/human_decision.rs` | 날짜·clock·offset의 같은 두 자리 숫자 범위 primitive 사용. 한 자리 month/day 및 범위를 벗어난 offset을 더 이상 수용하지 않음. Leap-second allowance 유지; 후속 typed-contract 파트에서 Gregorian month/leap-year 범위도 처리 |
| `dispatch/review/{receipts,runs}.rs` | Read-derived validation receipt 기록과 별도 text-run budget initialization 유지. Effect가 다르므로 HumanDecision commit 경로에 합치지 않음 |

`bind_decision`의 caller claims 확인과 domain의 effect 직전 subject/kind/scope 재검사는 서로 다른
신뢰 경계다. 둘 다 유지한다. 승인만 operations를 적용하고, 거절은 canonical text를 변경하지 않는다.
MCP는 candidate subject read만 제공하며 사람 전용 decision은 local ingress에 남긴다. Actor label을
로그인 인증으로 주장하지 않는다.

Review-specific DTO는 미등록 필드와 malformed date/offset을 거절한다. Persisted fields·producer
schema·valid decision identity는 바꾸지 않는다. lexical proof의 점수나 receipt PASS를 작품 fidelity의 증명으로 확대하지 않는다.

### Voice profile and measurement

범위는 `voice.rs`, `voice/{measurements,profile,compare,guide}.rs`, model voice DTO,
Application voice request/dispatch/tool catalog와 profile activation/reducer의 필수 연결이다.
샘플 prose는 request-local이고 profile·저널에는 count/digest/guidance만 남는다. 측정 점수는
작가 정체성이나 원문 의미·품질의 승인 권한이 아니다.

| owner | 분석·변경·유지 |
|---|---|
| `application/requests/voice.rs`, dispatch | 비교 본문은 필수 `candidate_text`, object sample ID는 선택 `id` 한 경로로 정리. `candidate`/`text` 및 `sample_id` alias 제거와 unknown-field 거절. 실패할 수 없는 sample-value conversion의 Result wrapper 제거 |
| `application/tools/voice.rs` | 실제 DTO와 닫힌 schema를 맞추고 max sample count를 `MAX_VOICE_SAMPLES` owner에서 사용. 공개 operation 이름과 string/object sample의 다른 사용 역할 유지 |
| `voice/measurements.rs` | Character/line count를 기존 feature loop에 모아 중복 전체 순회 제거. Stored profile의 count 범위·disjoint character classes·punctuation/line 범위와 derived fixed-point 값의 일관성을 거절 경계에 명시 |
| `voice/profile.rs` | Sample IDs·bytes limits, length-prefixed corpus digest, summary/guidance·schema/analyzer 계약 유지. 이 파일만 사용하는 guideline builder는 private으로 축소 |
| `voice/compare.rs` | 0..100으로 이미 bounded된 변환의 가짜 100 fallback와 label 중복 clone 제거. 문장이 기준보다 짧으면 더 길게, 길면 나누도록 반대로 연결된 tip 수정. Score weights·thresholds·나머지 tips 순서 보존 |
| `voice/guide.rs` | Scene revision/raw digest와 관련 사실, 보존 규칙을 가진 copyable 결과 유지. UI용 notice와 복사 결과의 설명은 서로 다른 채널이므로 유지 |
| model/command/reducer | 같은 VoiceProfile/VoiceMeasurements/Comparison/Guide fields와 activation CAS·profile admission을 유지. 샘플 text를 저장 이벤트나 다른 component authority로 보내지 않음 |

의도적 clean break는 voice alias·미등록 필드와 count/derived values가 불일치하는 stored profile의
수용 경로 제거다. Malformed profile을 조용히 정규화·성공 처리하지 않는다. 실제 생성되는 기존
측정값·profile digest·정수 비율 방식과 prose privacy를 보존했다. `profile.get`의 저장값 read,
comparison과 guide의 사용 전 profile admission, activation/replay의 검사 역할은 다르므로 유지한다.
비교·guide는 저장된 profile의 shape와 source binding을 확인하며 자동 rewrite나 adoption을 하지 않는다.

### Writer process adapter

범위는 `adapters/geultu-writer-process`의 public API/policy·workspace/protocol·child lifecycle과
CLI/Studio registry 연결, producer-owned `GeneratedDraft` DTO다. `process.rs`로 spawn/poll/timeout/
kill/reap을 분리하고 process 종료를 관측한 뒤에도 두 output 한도를 재확인한다. `lib.rs`는
plan serialization·file-backed stdin/stdout/stderr·status/JSON decoding을 조합하며 workspace owner와
direct-child owner를 섞지 않는다. Pending candidate·Human review·canonical state는 core/caller에 남긴다.

의도적 clean break는 GeneratedDraft의 미등록 JSON field 수용 경로 제거다. Public 타입·fields,
정상 request/result bytes, policy defaults와 기존 CLI/Studio entrypoint는 유지한다. Actual writer
상세 책임과 direct-child/OS/remote 한계는 [process writer 분석](adapters/geultu-writer-process/ANALYSIS.md)을 따른다.

### Studio transport

범위는 `adapters/geultu-studio/src/{main,config,assets,http,security,routes}.rs`와 필수 smoke
contract 소비자다. HTTP input rejection과 response write failure를 분리해 malformed/auth request는
400/403으로 반환하고 전송 실패에 두 번째 응답을 쓰지 않는다. 실제 bind address를 URL·Host guard에
사용하고 unsupported Origin/port·unknown ToolCall·empty launch 설정을 거절한다. Body receive
buffer와 immutable security headers는 재사용한다.

Application이 후보/decision/commit을 소유하고 Studio는 전송·local bearer·표시만 소유한다.
새 화면이나 state owner를 추가하지 않았다. 기존 public routes/JSON fields/CSP·loopback·CSPRNG·
도구 의미를 유지하며 execute 결과 소실을 작업 취소로 바꾸지 않는다. 순차 listener/I/O timeout 한계는 [Studio 분석](adapters/geultu-studio/ANALYSIS.md)에 있다.

### Refinement proposal

범위는 `refinement/{mod,catalog,diagnose,proposal}.rs`, Application request/dispatch/tool catalog,
pack policy·checksum 소비, DomainDecision 조립과 candidate/changeset의 필수 Workspace 호출이다.
Locale archives는 byte integrity/coverage만 소비하며 그 내부 rule 실행을 구현했다고 주장하지 않는다.

| owner | 분석·변경·유지 |
|---|---|
| `diagnose.rs` | 한 char array를 네 규칙과 run scanner가 공유한다. 정렬의 code clone과 모든 accepted span 재탐색 제거. Char offsets·첫 overlap winner·mechanical replacement를 보존하고, nonmatching word pair 뒤의 다음 단어를 건너뛰던 진단 누락 수정 |
| `catalog.rs`, policy/catalog DTO | Catalog 자체의 SHA256SUMS coverage와 실제 parse bytes의 digest를 확인한다. 지원 output_authority·catalog ID·archive name을 명시적으로 수용하고 unknown policy를 실행하지 않음 |
| `proposal.rs` | Full command ID + domain-separated SHA-256으로 64-character changeset transaction ID 유도. Caller command ID와 pack metadata, pure repair decision은 effect 전에 처리. Patched text/command의 불필요한 clone과 오래 유지하던 JSON body 제거 |
| Workspace publication boundary | 두 transaction 뒤 실제 stored candidate/changeset가 요청한 pending proposal과 일치하는지 읽는다. ID-only transaction replay가 다른/missing/decided artifacts를 성공 결과로 반환하지 않음 |
| Candidate binding | Replay 관측에서는 최초 write의 creation timestamp만 제외한다. Human review의 `candidate_subject`는 여전히 전체 stored receipt를 hash에 포함한다. 저장값·approval subject를 정규화하거나 수정하지 않음 |
| public/API surface | 두 refinement request DTO는 unknown fields를 거절한다. 외부 finding/replacement를 받던 callerless `apply_replacements` export를 제거하고 proposal-internal 적용만 유지. 기존 tool names/result fields·repair decision factory 유지 |
| approval/budget | Pending candidate·changeset만 만든다. Canonical text 적용과 budget 사용은 기존 `changeset.approve` 및 exact HumanDecision/scene/project guards에 남김 |

의도적 clean break는 clipped `.cs` command derivation, raw replacement helper export,
미등록 request fields와 unsupported/unbound catalog 수용 경로 제거다. 기존 workspace records는
삭제·migration하지 않으며 옛 `.cs` ID를 새 알고리즘으로 번역하지 않는다. 기존 pending artifacts는
공개 review/decision 경로로 별도 다룬다. Candidate/changeset publication은 여전히 두 transaction이며
부분 완료·crash 원자성 한계는 남는다. 실패를 성공 proposal로 바꾸지 않는다.

공개 rule findings·char offsets·same-language mechanical patch·reason-code order와 producer fields는
보존한다. Human approval과 budget admission 없이는 canonical text를 변경하지 않는다.

### Craftpack loading and integrity

범위는 `craftpack/{mod,loader,nodes,paths,status,reference}.rs`, `integrity/{parse,verify}.rs`,
Application pack/status 연결과 같은 checksum/text boundary를 소비하는 refinement loader다.
Pack은 계획·knowledge 입력의 checksum snapshot이며 narrative authority나 승인자/실행기 아니다.

| owner | 분석·변경·유지 |
|---|---|
| `integrity/parse.rs` | 기존 text/binary record syntax와 case-insensitive digest는 유지. Duplicate paths·NUL·native prefix·internal dot/empty components를 거절해 normalized package-relative paths만 수용 |
| `integrity/verify.rs` | Archive bytes를 64 KiB chunk로 hash해 whole-file allocation 제거. 두 loader가 쓰는 `read_verified_text`가 실제 parse bytes를 verified digest에 다시 결속. Entry symlink/parent/regular-file admission 공유; unlisted files는 여전히 무시 |
| `craftpack/loader.rs` | 선택한 directory를 canonical path로 고정하고 craft/workflow parse bytes를 공통 read로 가져옴. API/workflow version, node ID/uniqueness, prompt asset count/hash와 agent prompt coverage를 거절 경계에 명시 |
| `craftpack/nodes.rs` | Unknown kind/fields와 문서 순서 보존. WorkflowNode serialization이 원 manifest의 flat `kind` body와 일치하도록 고쳐 잘못된 external enum tag를 제거 |
| `CraftPack`, `RefinementPack` | Externally writable verified-state fields를 닫고 read-only accessor 제공. Public constructor 없이 loader가 만든 snapshot을 소비; 실제 status directory는 loaded CraftPack 내부에 유지 |
| `craftpack/status.rs`, reference | Caller가 임의 directory를 주어 verified status를 표시하는 API와 infallible Result wrapper 제거. `status()`는 loaded snapshot의 JSON projection이며 새 live check가 아님. Existing plan reference fields 유지 |
| Application / existing fixtures | Status call과 externally read fields를 accessor로 연결한다. Tool operation/JSON fields, license·redistribution·source archive metadata, optional/mandatory pack 정책을 유지한다. |

의도적 clean break는 public verified-pack struct construction/field mutation, `status(path)` signature,
외부 tag 형태의 workflow serialization과 ambiguous/unsupported/uncovered pack input의 수용 경로
제거다. 기존 bundled pack·archive·SHA256SUMS·legal 자료·workspace는 수정하지 않았다.
동일 filesystem entry를 삭제하거나 복사본을 만들지 않았고 compatibility wrapper도 없다.

Checksum reads와 manifest parse snapshot은 분리된 관측이다. 새 guard가 atomic multi-file custody나
adversarial ancestor-replacement의 완전한 방어를 뜻하지 않으며 오래 보관한 snapshot의 live
freshness를 자동 보증하지 않는다. 필요한 caller는 다시 load한다.

## Visual package admission

범위는 `visual.rs`, `visual_run.rs`, `story_package.rs`의 projection consumer,
`application/{requests,dispatch,tools}/visual.rs`, `domain/command/visual.rs`, 기존 tests와
현재 visual 문서다. Geultu의 local draft/admission만 다루며 downstream owner를 합치지 않는다.

| owner | 분석·개선 |
|---|---|
| Local plan | 중간 paragraph 배열과 truncate의 전체 재순회를 제거. 현재 scene/content revision, deterministic shots와 빈 scene의 title fallback 유지 |
| Package admission | Inline body만 받는 폐쇄 DTO로 변경. 항상 실패하던 bare `package_id`/resolver 제거. Self identity 이후 기존 export owner의 현재 projection과 source refs를 대조 |
| Application commit | 읽은 project revision의 CAS로 admission 기록. Command-ID replay 뒤에도 stored run/receipt가 요청과 일치하는지 확인하여 무관한 replay를 성공 응답으로 표시하지 않음 |
| Domain boundary | Initial READY/revision 0, empty outputs/no pending effect, current content revision 및 admission-validator PASS receipt를 요구. Receipt subject 일치와 conflict/idempotency 경계 유지 |
| 사용자 entrypoint | Local paragraph draft와 source-bound READY 준비를 설명. Inline package를 required schema에 명시하고 NVC execution/release를 별도 owner action으로 안내 |

Required upstream receipt와 route input refs의 실제 approval/source 결속은 여전히 downstream
handoff 범위다. Local PASS는 Human adoption이나 NVC completion이 아니다. Producer schema와 integration byte pins, persisted event shape는 바꾸지 않는다.

## Domain command and reducer

범위는 `domain.rs`, `domain/{command,reducer,validation}.rs`와 여섯 command owner module이다.
Command/Event의 persisted serde 계약과 public `decide/evolve` facade는 유지하며 각 기능의
기존 review·CAS·budget 정책을 owner에 둔다. Typed contract 내부는 다음 독립 파트다.

| owner | 분석·개선 |
|---|---|
| Command dispatch | Exhaustive match가 명령별 typed function을 직접 호출한다. 여섯 owner의 전체 Command 재분기와 도달 불가능한 wrong-owner 오류 경로를 제거. DTO를 새로 복제하거나 owner를 합치지 않음 |
| Narrative admission | Bulk import 내 중복 scene ID를 conflict로 거절하여 last-write-wins 소실 제거. 실제 stored scene ID와의 충돌 검사 유지 |
| Operation validation | Scene/fact/timeline/thread 중복 판정은 borrowed IDs를 사용. Artifact별 의미·source revision·evidence 경계 유지 |
| Receipt record | Consumed receipt를 event로 이동하여 불필요한 clone 제거. Immutable receipt의 identical replay는 그대로 유지 |
| Event reduction | 신규 scene/fact/timeline/thread/candidate/changeset/visual plan과 approved additions는 insert-new 정책을 공유. Existing records 덮어쓰기를 corrupt로 거절하며 receipt/decision의 identical replay 정책과 구분 |
| Revision authority | Project/content/scene revision의 saturating 성공을 제거하고 checked increment로 한계 도달을 거절. Visual command의 source revision 계산도 overflow를 거절. Transaction consumer는 이미 append 전 pure evolve를 수행 |

기존 stored data를 자동 수정·삭제하거나 migration하지 않는다. 이미 중복 ID나 overflow를 포함한
journal은 조용히 재생하는 대신 오류가 드러날 수 있다. No-op/idempotency, 별도 Human decision,
FAIL/cancel/outcome_unknown 의미를 유지한다.

## Typed contracts

범위는 `contracts/`의 모든 typed contract·digest·lexical helper와 필수 handoff/refinement
consumer다. Producer schema 파일 및 integration/consumer byte pins는 바꾸지 않았다.

| owner | 분석·개선 |
|---|---|
| Contract facade | ArtifactRef는 private `artifact.rs`, flat package는 private `story_package.rs`가 정의. 기존 root exports 유지. Export projection 실행과 wire contract 책임은 별개 |
| Shared grammar | Caller-owned prefix + 64 lowercase hex grammar를 공유. Unicode string bounds는 최대값 초과까지의 제한된 scan. Raw byte hash와 canonical JSON hash 의미는 유지 |
| Language | Story package는 subtag 2..8, reviewed synopsis는 1..8을 producer별로 명시. Handoff/refinement consumer의 기존 1..8 규칙 유지 |
| Versions | 정확히 세 개의 nonempty numeric components를 요구. Validator suffix와 selector numeric-only version은 각 contract가 소유. `1`/`1.2` 수용 제거 |
| Optional fields | 선택적이지만 nonnullable인 reason/path/min_version/provider_attempts는 missing과 null을 구분. Nullable effect fields와 pending fields는 계약상 null 수용을 유지 |
| Run/decision enums | Text/visual run status와 DomainDecisionKind를 SCREAMING_SNAKE_CASE로 직렬화. WAITING_HUMAN/REPAIR_REQUIRED/RECOVERY_REQUIRED/HUMAN_REQUIRED의 schema token 유지; 잘못된 밑줄 없는 token compatibility는 없음 |
| Pending payloads | Text pending_decision은 object/null만 validate. Visual pending_effect의 임의 JSON을 typed EffectReceipt로 교체하고 schema의 IDs/digests/lengths/failure code를 validate. COMMITTED/FAILED/UNKNOWN_OUTCOME을 별도로 보존 |
| Human timestamp | Date/clock Vec allocations 제거. Gregorian month/leap-year day bounds와 lowercase t/z 허용. Leap second 60은 유지하되 외부 leap-second schedule 확인은 수행하지 않음 |
| Reviewed synopsis | Raw synopsis SHA-256 grammar를 직접 판정하여 임시 prefixed String 제거. Approval의 exact digest/revision binding과 canonical handoff identity 유지 |

EffectReceipt는 기존 schema의 구현이며 외부 effect 실행기나 성공 변환이 아니다. 잘못된 version,
null, enum token 및 pending payload를 기존보다 엄격히 거절하므로 해당 malformed 입력에 대한
자동 migration은 없다. 기존 사용자 workspace는 자동 수정하지 않는다.

## Admission budget and snapshot

범위는 `admission.rs`, `admission/{budget,snapshot}.rs`와 필수 consumer인 reducer budget event,
Workspace load/transaction, Application run.open 및 해당 tool metadata다.

| owner | 분석·개선 |
|---|---|
| Budget | Pure `revision_remaining_after_spend`가 positive units, terminal status와 checked remaining subtraction을 소유. Command admission과 event reduction이 같은 policy를 소비. Run project 소속도 거절 경계에 명시 |
| Snapshot shape | Mutable error closure 제거. Contract map의 key identity와 embedded validate를 한 pass에서 처리. Authoring의 독립 conformance owner 유지 |
| Snapshot lineage | Project revision/event count equality, applied-command revision 범위, text/visual run project identity, candidate future-source 및 changeset-candidate source binding, visual plan scene/source와 voice profile shape를 거절 경계에 추가. Stale 후보/plan은 보존 |
| Workspace load | Snapshot 없는 full replay나 기존 snapshot 이후 tail replay를 마친 Project에도 conformance 적용. Existing snapshot admission과 append 전 deterministic validation은 유지 |
| run.open replay | Project lock 아래 journal의 최초 TextRunOpened payload와 동일 command request를 대조. 다른 command/payload reuse는 conflict. Application은 run DTO clone을 제거하고 현재 stored run을 반환해 차감된 예산을 되돌려 표시하지 않음 |
| 사용자 계약 | Run을 참조하는 edit/adoption의 1 revision-unit 비용과 terminal/exhausted 거절을 명시. Pivot budget은 기록만 되고 spending command가 없음을 tool description에 명시 |

이는 내부 engine Project gate이며 project-state wire schema 전체 구현이 아니다. Snapshot body와
모든 이전 journal bytes의 cryptographic equivalence를 증명하거나 no-op command의 snapshot loss를
복구한다고 주장하지 않는다. Persistent user data를 자동 수정·삭제하지 않는다.

## Reviewed synopsis handoff and Markdown transfer

범위는 `store/handoff.rs`, `markdown.rs`, Workspace transfer request/dispatch/tool schemas,
CLI handoff command와 필수 integration/test consumer다. Whole-project scene export와 adopted
workbench synopsis는 서로 다른 source authority로 유지한다.

| owner | 분석·개선 |
|---|---|
| Reviewed handoff | Producer DTO/approval/identity 조립을 한 private helper로 통합. Whole-project export는 리뷰한 project revision과 raw Markdown SHA-256을 필수로 받아 lock 아래 대조. Optional authority claims/default review를 폐기 |
| Immutable authoring handoff | Existing receipt의 project/title/path/reason까지 current authority와 대조하고 manifest/body는 regular files만 재사용. Symlink와 incomplete body를 덮어쓰거나 성공으로 변환하지 않음. Failed publication은 자동 repair하지 않음 |
| Filesystem admission | lstat 기반 entry presence로 dangling symlink도 존재하는 입력으로 처리. Permission/I/O 오류를 missing으로 바꾸지 않음. 기존 actor/language/receipt identity와 실제 body bytes 결속 유지 |
| Markdown transfer | Fenced code 안의 headings를 text로 유지. Duplicate scene suffix는 base ID에서 유도하여 누적 suffix 제거. Index/order arithmetic은 checked 처리. Application import는 첫 order 1 및 project revision CAS로 읽은 ordering을 commit |
| Public entrypoint | Transfer DTO의 unknown fields 거절. CLI는 PROJECT_ID ACTOR_ID REVISION SHA256 LANGUAGE [REASON]만 받으며 authority claim을 자동 생성하지 않음. Tool description은 whole-project/adopted synopsis 범위를 구분 |
| 필수 consumer | 기존 Rust test callers와 CLI integration E2E source를 새 필수 입력으로 맞춤. README/current canonical 문서 및 해당 checksum metadata 갱신 |

두 public handoff files의 pair crash atomicity, immutable publication의 외부 writer race 및 자동
recovery는 보장하지 않는다. 기존 workspace와 handoff는 자동 수정·삭제하지 않는다.

## Application tool catalog and admission

범위는 `application.rs`, `application/{dispatch,support,tools}.rs`, 공통 request/schema helpers,
전체 tool owner catalog와 registry handler의 필수 visibility consumer다. 기능별 상태/효과 owner는
그대로 유지하며 이 파트에서 새 workflow나 provider를 만들지 않는다.

| owner | 분석·개선 |
|---|---|
| Registered catalog | Static ToolDefinition과 owning handler를 함께 등록하고 OnceLock으로 보관. 별도 tool-name dispatch 목록과 authoring prefix 우회 제거. 기존 transport 순서와 public ToolDefinition shape 유지 |
| Human entry policy | Registry에 human-only 분류를 결속. Whole-project reviewed synopsis approval도 MCP 목록/호출에서 배제. Public is_human_only_tool은 같은 등록부를 조회하며 CLI/Studio의 별도 human entry는 유지 |
| Root input admission | Registered schema의 최상위 properties/required와 object shape를 actual call에서 수용. No-argument tool의 stray inputs와 unknown fields를 묵인하지 않음. Nested constraints와 domain policies는 기존 DTO/owner가 처리; full JSON Schema executor를 주장하지 않음 |
| Composition | open은 from_workspace constructor를 재사용. with_writing_adapters는 configured registry에 추가하여 이미 설정된 providers/adapters가 사라지는 경로 제거. Duplicate adapter admission은 기존 registry policy 유지 |
| Shared support | ProjectRef는 폐쇄 DTO. Generated ID의 byte별 임시 String collection을 단일 hex formatting으로 바꾸며 같은 16-hex suffix 의미 유지 |
| Catalog schema construction | Human-decision metadata 조립의 silent empty-map fallback와 extra claims Map clone 제거. Preflight의 두 owner를 definition/handler tuple로 명시. Handler visibility는 Application 내부로만 제한 |

Cache는 정적인 metadata만 보관하며 workspace/current state나 draft 결과를 캐시하지 않는다.
사용자 data와 component ownership, candidate/review 분리 및 failure semantics를 유지한다.

## CLI and MCP entrypoints

범위는 `crates/geultu-cli/src/{main,cli,mcp}.rs`, `cli/commands.rs`, 기존 MCP fixture consumer와
해당 문서다. CLI와 MCP는 Application/public Workspace만 소비하며 narrative authority를 갖지 않는다.

| owner | 분석·개선 |
|---|---|
| CLI parsing | Global workspace/writer options는 command 전에만 해석. 뒤의 option-like text는 positional로 보존하며 --pretty와 literal --를 명시. Empty paths, unknown globals, orphan writer args와 excess positional args를 거절. Argument move로 불필요 clone 제거 |
| Command admission | Arity/unknown command를 workspace open 전에 처리. Help/version은 workspace/provider를 열지 않음. Usage의 중복 첫줄/고정 제품 version 제거; handoff claim과 literal 옵션 설명 통합 |
| Output | JSON/help/version/raw export stdout write 오류를 반환. Export 응답의 missing Markdown을 빈 성공으로 표시하지 않음. 기존 text/JSON output 의미·exit codes 유지 |
| Scene ordering | 공개 typed Workspace projection으로 max order를 읽으며 silent filter/default JSON decoding 제거. Positive next order와 checked increment로 overflow 거절; ordering read+add의 concurrency CAS는 추가하지 않음 |
| MCP session | New → AwaitingInitialized → Ready를 구분. 유효 initialize 없이 알림만 보내서 tools를 여는 경로 제거. Protocol version/capabilities/clientInfo shape와 string/integer IDs/object params를 수용하며 repeated initialize 거절 |
| RPC outcome | Success serialization 실패는 internal error로 반환하고 null 성공 fallback 제거. Borrowed params와 error text를 사용. LSP-style shutdown/exit branch 제거; stdin EOF가 세션 종료 |
| Consumer/policy | Doctor protocol 목록은 transport constant를 사용. Existing initialize fixture에 필수 params를 맞춤. Human-only filtering은 Application registry로 유지하며 whole-project handoff 승인도 제외 |

Protocol negotiation의 supported-version 선택은 유지하며 새 transport나 외부 provider cancellation을
만들지 않는다. 알림은 응답/동기 provider 취소 보장이 없음을 명시한다. 기존 사용자 workspace와
외부 output files를 자동 수정·삭제하지 않는다.

## Writing plan adapter

범위는 `adapters/geultu-writing` 전체 source/public entrypoint와 CLI/Studio registration의 필수
consumer 연결이다. 네 builtin type이 core generation input을 pure plan으로 투영하며 state나
external writer effect를 소유하지 않는다.

| owner | 분석·개선 |
|---|---|
| Public facade | PromptAdapter/builtin_adapters export 경로는 유지하고 private adapter/builtins/prompt module로 책임 분리 |
| Adapter plan | Type mismatch를 기존대로 거절. Requested/default stage는 borrow하여 plan ownership 시 한 번 복제. Input digest/source content revision/craftpack None 및 output format 유지 |
| Builtin configuration | Novel/fiction/synopsis/storyboard의 ordered registration, IDs/default stages와 craft instructions를 한 전용 owner에 보존 |
| Prompt renderer | 최종 String에 context와 instructions를 직접 기록. 전체 context intermediate와 field별 format allocations 제거. 현재 source revision, scene/fact/timeline/thread/guideline 의미 유지 |
| Draft boundary | Source prose/parameter values를 data로 다루는 output rule 명시. Canonical claim·approval·provider execution은 생성하지 않음. JSON parameter serialization 실패는 오류로 전파 |

Provider data transfer의 field projection을 확장하거나 원고/workspace를 자동 수정하지 않는다.

## 2026-09-29 source-only approval-boundary clean break

`candidate.subject` is the sole read path for an engine-computed pending candidate ArtifactRef.
Candidate rejection and changeset approval/rejection now require actor ID plus that exact subject's
revision/digest; claim-less decision construction and the refinement-only exception are removed.
The local Studio collects a reviewer label and submits the displayed subject. The Application owns
MCP human-only classification, which now excludes all candidate/changeset decisions from MCP. The
persisted Event stream remains readable; every newly constructed review Command carries a decision.
Tests/build/fmt/clippy/CLI/MCP/Studio verification were not run for this source graph.

### Workspace and journal

범위는 `store.rs`와 `store/`의 project·transaction·journal·recovery·history·file I/O·lock·handoff,
그리고 그 결과를 노출하는 CLI다. `journal_entries`가 JSON/schema/command ID/sequence/blank-line
admission을 소유하며 replay, snapshot reconciliation, authoring history가 그 iterator를 소비한다.
Workspace metadata의 schema admission도 `Workspace::metadata` 한 곳에서 init/open/후속 read에
적용한다. Snapshot은 동일 wire fields를 유지하는 generic DTO로 read 때 Project를 소유하고
write 때 같은 Project를 빌려 전체 복제를 피한다. handoff directory의 반복 type check는 하나로
정리했다. CLI init은 실제 model schema version을 반환하며 commands owner를 직접 실행한다.

Lock, transaction append, snapshot/index write와 source-bound handoff는 별도 effect·approval 역할이
있어 유지한다. Journal 손상은 세 소비자에서 `Corrupt`로 분류하고 line/sequence 근거를 남긴다.
기존 handoff pair·multi-event crash 원자성은 보장 범위에 포함되지 않는다.

## Verdict·책임 경계

Geultu는 local authoring의 canonical state와 candidate/review record를 소유한다. Public callers are CLI, MCP, Studio and Rust library consumers. Generated drafts remain reviewable proposals; NarrativeCourier owns the separate STS screenplay and release authority. Source-level owner details follow below. Dated gate results, synthetic fidelity outcome and unqualified scope are owned by [제품 Current](docs/IMPLEMENTATION_STATUS.md); the 2026-09-30 report is a historical run, not qualification for later source.

| 단위 | 판정·이유 |
|---|---|
| `crates/geultu` | 독립 제품 코어. Application, domain, store, contracts, review/refinement/voice/visual을 함께 소유한다. |
| `crates/geultu/src/model.rs` + `src/model/` | 안정된 `geultu::model::*` facade 아래 persisted scene/content, candidate/change, visual, voice, workspace, report DTO를 private modules로 나눈다. Project field·serde 계약과 canonical state owner는 바꾸지 않는다. |
| `adapters/geultu-writing` | [독립 분석](adapters/geultu-writing/ANALYSIS.md): 네 built-in writing type의 pure plan adapters. 별도 canonical state/provider effect/lifecycle이 없다. |
| `crates/geultu-cli` | 상위 포함: `src/main.rs` maps process exit/error output; private `cli.rs` owns CLI errors, global options, JSON/text I/O and output projection; `cli/commands.rs` owns built-in commands and Application/provider composition; `mcp.rs` remains the stdio JSON-RPC adapter. CLI and MCP still reach the same Application. |
| `adapters/geultu-writer-process` | [독립 effect 분석](adapters/geultu-writer-process/ANALYSIS.md): `lib.rs` composes public ProcessWriter/policy/protocol; private `process.rs` owns direct-child lifecycle, and `workspace.rs` owns private files and bounded capture reads. |
| `adapters/geultu-studio` | [독립 transport/UI 분석](adapters/geultu-studio/ANALYSIS.md): loopback HTTP 보안·framing과 임시 browser 편집·review presentation을 소유하며 canonical state는 Application/Workspace가 소유한다. |
| `docs/canonical`, `assets`, `vendor` | 계약/지식 입력·외부 코드 경계. 실행용 schema와 fixture, vendored build 입력은 보존한다. |


## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| CLI/MCP tool·workspace 지정 | CLI/MCP → `Application` facade → `application/dispatch.rs` → owner별 handler | `application/requests/mod.rs`의 tool DTO, project/object revision, command_id | domain `decide/evolve`; store journal | JSON result 또는 typed error | `crates/geultu/src/application.rs`, `application/dispatch/`, `application/requests/mod.rs`, `store.rs`, `domain.rs`, `domain/` |
| project/scene/fact/timeline/thread 쓰기 | Application → `Workspace::transact` | expected revision·id·text 한도 | `store/lock.rs` project lock, `store/transaction.rs` commit coordination, `store/journal.rs` append/replay | snapshot/index 및 revision | `store/transaction.rs::transact`, `store/journal.rs`, `store/recovery.rs` |
| review/candidate/changeset | tool → `application/dispatch/review.rs` facade → receipts/candidates/changesets/runs adapters → review/domain approval | `candidate.subject` ArtifactRef; required actor/source revision/digest claims; expected project and operation revision | project journal | candidate subject, bound human decision, or typed stale/conflict error | `application/dispatch/review/{receipts,candidates,changesets,runs}.rs`, `application/requests/mod.rs`, `application/approval.rs`, `domain/command/review.rs`, `domain/reducer.rs`, `review/` |
| generation.prepare/generate·provider 등록 시 | Application → `application/dispatch/generation.rs` → `generation/registry.rs` → WritingType/GenerationProvider | brief/parameters/plan, 내용 한도 | plan은 writing adapter owner, 외부 효과는 provider owner | untrusted draft·source revision/hash | `generation.rs` DTO/trait facade, `generation/registry.rs` stable registry, `generation/registry/writing.rs` plan preparation, `generation/registry/provider.rs` external draft admission, `generation/hash.rs` identity |
| voice/refinement/craftpack | Application → owner별 dispatch handler → 각 domain module | `application/requests/mod.rs` 입력, 샘플·pack·rule별 validation | project 기록과 content-addressed 입력 | 진단·guide·후보, 자동 승인 없음 | `application/dispatch/voice.rs`, `application/dispatch/refinement.rs`, `application/requests/mod.rs`, `voice/{profile,measurements,compare,guide}.rs`, `refinement/{catalog,diagnose,proposal}.rs` |
| reviewed synopsis export | Application → `export_reviewed_synopsis` | actor/language, expected revision/hash | `store/handoff.rs` file publication, producer schema | synopsis Markdown와 handoff JSON | `store/handoff.rs`, `contracts/reviewed_synopsis.rs` |
| visual.plan/admit_package·선택 | Application → `application/dispatch.rs` → `application/dispatch/visual.rs` → visual validator | required inline `GeultuStoryPackage`, producer identity/current-source binding, caller route-shape claim | Geultu-local visual plan/run state and structural receipt | source-bound READY run; no NVC execution, Completed release, or human-approval claim | `application/dispatch/visual.rs`, `visual_run.rs`, `story_package.rs` |
| workspace/project recovery·명시 호출 | Application → recover methods | workspace/project id·remove_lock 옵션 | journal replay, rebuildable projection | 복구 상태 또는 lock/schema 오류 | `store/recovery.rs::recover_project/recover_index` |

## 실제 흐름

대표 쓰기 경로: `CLI call scene.set_text → Application DTO → Workspace::transact → project lock → journal/snapshot load → domain::decide → pure evolve/command-id/schema preflight → append_envelopes/fsync → snapshot → workspace index → JSON result`.

생성 경로: `generation.prepare → WritingType plan → provider 선택 → generation.generate → 외부 draft → 후보 생성 → changeset → approve 시 revision 재검사 → canonical mutation`. 생성 자체와 승인은 다른 쓰기 경계다.

handoff 경로: `synopsis.export_reviewed_handoff → 현재 project export → 원문 SHA-256 → ReviewedSynopsisHandoff identity → synopsis.md + handoff JSON → NVC import`. 소비자는 원문 bytes를 다시 검증한다. `story.export`와 `visual.admit_package`는 별도 공개 capability이며 이 main chain에서 호출되지 않는다고 dead로 판단하지 않는다.

## Model value module ownership

`model.rs`는 public re-export facade이고 type 정의는 `model/{scene,candidates,visual,voice,workspace,reports}.rs`가 소유한다. 기존 `geultu::model::Type` 경로와 constant 위치, fields, serde attributes/order, `Project::empty` 초기값을 유지한다. 이 module map은 public type paths와 persisted Project contract를 설명한다.

The public `integrity` surface exposes only fail-closed `verify_sums`; `parse_sums` and its transient
`ChecksumEntry` stay private to `integrity::parse`. Files are hashed incrementally, pack-local
`SHA256SUMS` and each listed path are admitted without symlink traversal, and the exact UTF-8
manifest/catalog text consumed by Serde is re-read and compared with its captured SHA-256. Missing
entries map to `not_found`, unsafe paths keep `invalid_input`, and unrelated I/O errors remain I/O
errors. An explicitly supplied external sums path remains caller-selected. Craftpack/Refinement never
receive the parser DTO.

Domain Command/Event definitions, reducer, validation, and narrative/review dispatch consumers now
name the exact values imported from the model facade. The public facade remains the sole stable
`geultu::model::Type` path; internal consumers no longer inherit future model additions through a
wildcard import.

`application::support::parse` is the single crate-internal Value-to-DTO decoder for both Application
dispatch and Authoring API. Authoring now imports that owner directly instead of using a second
`application::parse_request_value` forwarding path; the support module remains inaccessible to
external crates.

## 현재 state·contract·I/O owner

| concern / 값의 종류 | authoritative owner | 비고 |
|---|---|---|
| project state·revision·command id / State | domain + journal | snapshot/index는 projection. 여러 event를 한 command로 쓸 때 crash atomicity는 별도 문제다. |
| workspace lifecycle / Composition | `store.rs` | absolute root, workspace schema metadata, state paths, stable `Workspace` facade, and crate-private millisecond timestamps for durable records |
| project creation/read / Persistence projection | `store/project.rs` | seed event/snapshot/index commit and project list/load entrypoints |
| command commit / Effect orchestration | `store/transaction.rs` | lock → idempotency/decision/reducer/schema preflight → append → snapshot → index; post-append failure keeps committed revision identity |
| Command/Event public DTO·content classification / Contract | `domain.rs` | Public type paths remain `geultu::domain::{Command, Event, EventEnvelope}`. `Event::affects_content_revision` owns which stored events advance narrative content revision. |
| command decision router / State transition | `domain/command.rs` | Exhaustive `Command` variant routing; public `domain::decide` path is preserved. |
| narrative command decisions | `domain/command/narrative.rs` | Pure scene/fact/timeline/thread checks and Events; canonical `Project` remains immutable. |
| candidate/changeset decisions | `domain/command/review.rs` | Source/revision checks, human-decision binding, and review Events. |
| visual command decisions | `domain/command/visual.rs` | Visual plan source binding and package-admission receipt checks. |
| voice, receipt, run decisions | `domain/command/{voice,receipts,run}.rs` | Profile validation, receipt identity, and text-run project binding. |
| event reducer / State transition | `domain/reducer.rs` | Applies Event to a candidate Project value and asks the Event owner whether it advances content revision; persistence commit remains in `Workspace`. |
| domain invariants / Validation | `domain/validation.rs` | Shared scene/evidence/change-operation validation used by command decisions and replay reduction; `validate_id` is the crate-private lexical ID owner used by persistence, authoring, generation, and voice-profile IDs, not an external contract. |
| lock lifecycle / Effect | `store/lock.rs` | Exclusive lock file 생성·수명·release; destructive recovery는 Workspace 명령 소유다. |
| file encoding·atomic write / Effect | `store/file_io.rs` | JSON/byte write, parent sync, path absolutize, handoff directory check; canonical state는 소유하지 않는다. |
| journal envelope·append·replay / Persistence contract | `store/journal.rs` | EventEnvelope schema/sequence/project identity 확인과 journal 기반 replay; `Workspace`는 commit을, `store/recovery.rs`는 snapshot/index reconciliation을 조정한다. |
| cross-owner synopsis handoff / Publication | `store/handoff.rs` | Geultu owns synopsis bytes and approval record; the module never creates an NVC release. |
| authoring revision history / Read projection | `store/history.rs` | Journal events를 읽어 authoring document history를 구성하며 current authoring state는 domain이 소유한다. |
| review diagnostics / Proof·Measurement | `review/diagnostics.rs`, `review/proof.rs`, `review/text.rs` | Pure reports; candidate/source bytes와 canonical Project state를 수정하지 않는다. |
| receipt issuance / Contract adapter | `receipts/subject.rs`, `receipts/validation.rs`, `receipts/timestamp.rs` | Engine findings를 canonical receipt contracts로 변환하며 journal state를 변경하지 않는다. `text_master_ref`, validator version, RFC3339 timestamp builder는 receipt/export 조합 전용 crate-internal helpers이며 public re-export하지 않는다. |
| receipt / human-decision schemas / Wire contract | `contracts/artifact.rs` owns ArtifactRef; `contracts/story_package.rs` owns the flat package; `contracts/effects.rs` owns typed external-outcome receipts; `contracts/receipts.rs` owns ValidationReceipt; `contracts/human_decision.rs` owns HumanDecision and DecisionKind; `contracts/validate.rs` owns reusable ASCII-hex, prefix+64 lowercase-hex, and producer-parameterized language-tag predicates | Each ID prefix, minimum subtag width, and field meaning stays with its contract owner; voice profile keeps its raw-digest casing semantics; schemas and approval state remain distinct; public values export from `contracts` root, with no receipts-module re-export |
| raw bytes / canonical JSON identity / Digest | `contracts/digest.rs` | `sha256_hex` hashes exact supplied bytes; `jcs_sha256` separately canonicalizes JSON under RFC 8785. Callers own which byte stream or canonical value defines each artifact identity. |
| human decision binding / Authority bridge | `receipts/decision.rs` | Candidate subject, expected revision/digest, actor, scope를 확인한 뒤 domain이 기록할 decision을 만든다. |
| authoring public model/API / Contract·Transport | `authoring/mod.rs`, `authoring/api/mod.rs` | `call`·`tools` façade와 `api/requests.rs` DTO owner remain; shared Value→DTO error mapping uses Application support. `api/tools.rs` owns authoring schemas only. Cross-domain human-only exposure policy is implemented by Application itself, outside the tool-definition catalog. |
| authoring mutation / State transition | `authoring/mutation/mod.rs` | `apply`는 fingerprint/idempotency와 Action routing만 조정한다. `documents.rs`, `tasks.rs`, `candidates.rs`, `review.rs`가 같은 canonical State의 action-specific next-state transitions를 소유하며 journal commit은 계속 `Workspace`가 한다. |
| authoring read views / Projection | `authoring/read.rs` | task packet, candidate diff, status, current document export를 구성한다. |
| authoring freshness·conformance / Validation | `authoring/validation.rs`, `authoring/conformance.rs` | source/guidance freshness, dependency shape, candidate lineage를 확인한다. |
| craft-pack location / Configuration | `craftpack/paths.rs` | Explicit config, environment, repository discovery order; does not own pack content. |
| craft-pack verification·load / Integrity | `integrity/{parse,verify}.rs`, `craftpack/loader.rs`, `craftpack/nodes.rs` | Pure sums/path parsing is separate from streamed filesystem digests; pack-local checksum and listed paths use component-wise symlink/regular-file admission before verified text parsing. Loader validates workflow version/IDs and rejects agent prompts absent from verified pack files. |
| craft-pack projections / Read projection | `craftpack/status.rs`, `craftpack/reference.rs` | Status and verified generation-plan citation; no canonical narrative write. |
| text-run budget gate / Admission policy | `admission/budget.rs` | Rejects a run-referenced spend before the domain emits text-changing events. |
| snapshot conformance / Persistence gate | `admission/snapshot.rs` | Fail-closed Project shape and embedded contract checks called by Workspace snapshot read/write. |
| Application composition·public facade / Composition·Contract | `application.rs` | Workspace, generation registry, explicit craft-pack path configuration, stable `Application`/`ToolDefinition` facade. It no longer hosts policy/helper implementations. | `Application`, `ToolDefinition`, `Application::tool_definitions()`와 tool 순서를 보존하며 free function `tool_definitions()`는 parent-module helper로 한정한다. |
| generation preparation·pack policy / Configuration·Policy | `application/generation.rs`, `application/packs.rs` | Project-derived generation context, retained-location optional craft enrichment and mandatory pack/refinement loading. | craft-pack 부재 시 generation은 유지하고, 발견된 pack의 무결성 오류는 생략하지 않는다. |
| approval claim binding / Authority adapter | `application/approval.rs` | Changeset→candidate binding, required decision claims, and mandatory result projection; no refinement-only or claim-less branch. | 결정 기록은 `Workspace` command path에 남고, adapter는 승인 state를 쓰지 않는다. |
| request/transaction utilities / Shared support | `application/support.rs` | The single Value→DTO request decoder used by Application and Authoring, Workspace transaction result serialization, command ID selection, and process-local ID generation. | Authoring request DTOs stay in `api/requests.rs`; parsing error mapping stays with Application input support. `Workspace` retains journal commit authority and `generated_id` remains private to owner dispatch. |
| Application routing·tool schema / Dispatch·Contract | `application/dispatch.rs`, `application/dispatch/{workspace,generation,voice,narrative,review,refinement,visual}.rs`, `application/dispatch/review/{receipts,candidates,changesets,runs}.rs`, `application/requests/mod.rs`, `application/requests/{common,workspace,generation,narrative,voice,review,visual,refinement}.rs`, `application/tools.rs`, `application/tools/{bootstrap,generation,voice,project_transfer,narrative,review,preflight,refinement,visual}.rs` | `review.rs` owns route selection and the shared candidate lookup; candidate handlers expose `candidate.subject`, and decision DTO schemas require reviewer and exact artifact claims. MCP delegates the human-only filter to Application. |
| text/sample 한도 / Constant·Policy | `model::MAX_TEXT_BYTES`와 `voice.rs`의 corpus limits | core document/brief text는 16 MiB, voice sample은 2 MiB × 최대 32개·corpus 총 20 MiB, Studio/MCP JSON transport는 128 MiB envelope다. Voice cap은 `voice/profile.rs`에서 재검증하며 transport cap을 canonical prose limit으로 취급하지 않는다. |
| voice profile/measurement/compare/guide / Behavior | `voice/{profile,measurements,compare,guide}.rs` | `model::VoiceProfile`과 project journal이 active profile의 canonical state를 소유한다. Profile/sample ID는 `domain::validation::validate_id` owner를 직접 사용하고, raw corpus digest의 64-byte 길이와 대소문자 허용은 voice profile owner가 결정한다. |
| generator/craftpack 선택 / Configuration·Policy | application composition and `generation/registry.rs` facade; child `registry/writing.rs` / `registry/provider.rs` | `generation/hash.rs::context_digest` binds the exact preparation input used by the built-in writing adapter; the unused plan-hash helper was removed. Drafts are not canonical state. |
| human approval / State·intentional contract | domain decision 기록 | Every public candidate/changeset reject or approval requires an exact candidate subject and a separate human decision; MCP cannot invoke these transitions. |
| handoff schema / Constant·contract | `contracts/reviewed-synopsis-handoff-v1.schema.json` | NVC의 복제본은 consumer snapshot. |
| cwd/program/token / Configuration | CLI/process/Studio | 원문 도메인 정책과 분리. Feature Flag를 새로 만들 필요 없음. |

## 기능 현황

| 기능 | 연결 상태 | 구현 범위와 남은 경계 | 근거 |
|---|---|---|---|
| journal 편집·review·changeset | REACHABLE | transaction/revision/review behavior is implemented; multi-event crash recovery remains open | `store/transaction.rs::transact`, `domain/command/review.rs`, `domain/reducer.rs` |
| WritingType/GenerationProvider | PUBLIC_LIBRARY | registry selects exact project/revision plans; external provider output is unqualified | `generation/registry.rs::GenerationRegistry`, registry tests, `generation.rs` public export |
| voice·craftpack·refinement·visual | REACHABLE | local contract/admission behavior; real authored-text quality remains unqualified | `application/dispatch/` owner handlers |
| reviewed synopsis producer | REACHABLE | Unicode/path/hash/lock admission; two-file crash atomicity remains open | `contracts/reviewed_synopsis.rs`, `store/handoff.rs` |
| story.export | REACHABLE | public package producer; external caller coverage is unknown | `export_geultu_story_package`; `text_master_ref` remains crate-private for package/receipt identity composition |
| 전역 NVC/SMC 실행·승인 | ABSENT_IN_SCOPE | Geultu does not own downstream execution or approval | 공개 Application surface |

## 의존성·findings

**G1 · commit과 projection 실패의 혼동.** 문제 → journal 반영 뒤 index/metadata 오류가 일반 실패가 됐다. 근거 → `store/transaction.rs::transact`, `store/project.rs::create_project`; 원인 → commit 이후 effect에 일반 `?` 사용. 영향 → 반영된 명령의 재시도 위험. 동작 → 기존 `CommittedButSnapshotFailed { revision, message }`로 결과를 구분하고 pure evolve·snapshot schema 검사를 append 전에 한다. Index-lock/recovery test coverage와 실행 날짜는 [Current](docs/IMPLEMENTATION_STATUS.md)에 기록한다. 다중-event crash prefix는 아직 남는다.

**G2 · handoff schema/runtime alignment.** 문제 → schema 길이와 플랫폼 독립 상대경로 조건이 충분히 반영되지 않았다. 근거 → `contracts/reviewed_synopsis.rs::validate`와 producer schema. 영향 → producer가 consumer에서 거절될 handoff를 발행할 수 있다. 현재 동작은 Unicode 문자 수 기준 한도와 NUL/절대·역슬래시·parent traversal 거부를 적용한다. Public schema bytes는 유지한다. 실행 범위와 날짜는 [Current](docs/IMPLEMENTATION_STATUS.md)에 둔다.

**G3 · mutable handoff pair — 부분 개선.** 문제 → `synopsis.md`와 JSON을 고정 경로에 따로 쓴다. 근거 → `store/handoff.rs::export_reviewed_synopsis`. 원인 → export를 별도 artifact transaction으로 모델링하지 않음. 영향 → 중단 시 불일치와 과거 export 덮어쓰기. 방향 → project lock과 존재 선검사로 협력 writer를 직렬화했다. immutable export history/manifest 전환은 public path·보존 정책 결정이 필요하며 아직 구현하지 않았다.

**G4 · journal command publication and recovery qualification.** 현재 `append_envelopes`는
기존 NDJSON prefix를 bounded streaming copy하고 command event 묶음을 같은 파일 형식으로
atomic replace한다. 초기 생성은 첫 ProjectCreated의 id/title/command id를 대조하며, 새 기록과
idempotent 재개 모두 snapshot·인덱스를 공통 writer에서 복구한 뒤 응답한다. 초기 생성은 첫
ProjectCreated의 id/title/display_label/summary/command id를 대조한다. title은 작품 제목이자
Markdown 내보내기 제목으로 유지하고, 선택 metadata `display_label`은 목록에서만, `summary`는
작업 지시의 참고 문맥으로만 쓴다. 예전 이벤트·snapshot에서 빠진 두 선택 필드는 `None`으로
재생되며 새 metadata 이벤트는 content_revision을 바꾸지 않는다. 새 쓰기의 부분 batch 노출을
방지하는 소스 변경이며 journal 전체 크기에 비례하는 I/O 비용이 있다.
과거 부분 command·손상 데이터와 no-op command id의 snapshot 소실은 자동 해석하지 않는다.
기존 event/schema/history/command-id 형식을 보존한다. 남은 durability·fault-injection qualification은 [Current](docs/IMPLEMENTATION_STATUS.md)와 [PLAN](docs/PLAN.md)을 따른다.

**G5 · registry/plan identity — 이전 graph에서 수정·검증 완료.** 문제 → 중복 등록 first-match 및 외부 WritingAdapter의 다른 project/revision plan을 허용했다. 근거 → `generation/registry/writing.rs::prepare`, `registry/provider.rs::generate`. 원인 → 공개 확장 경계의 authority 검증 누락. 영향 → 다른 provider나 stale context 사용. 방향 → 선택 identity 중복과 request/context/plan project·type·revision·adapter·explicit stage 불일치를 effect 전에 거절한다. `add_* -> ()`와 opaque `input_hash` 계약은 유지했다. 기존 registry tests는 모듈 이동 전 graph의 근거이며 현재 split 뒤 재실행하지 않았다.

**G6 · 수정 후보의 기준 판본 — 수정, 실행 검증 미실행.** 문제 → `Candidate.based_on`을 저장하지만 `candidate_view`의 before 블록은 늘 최초 task base에서 읽었다. 직접 수정한 후보를 다시 제출하면 reviewer에게 실제 부모 후보와 다른 변경 집합을 보여줄 수 있다. 변경 → `based_on`을 지정한 후보는 같은 task의 부모 candidate를 비교 기준으로 사용하고, 부모가 없거나 다른 task면 corrupt로 처리한다. `ensure_conformance`는 저장된 후보 계보의 부모 존재·task 동일성·순환 부재를 확인한다. 후보 본문·채택 의미·public field는 바꾸지 않았다. build/test와 journal 호환 검사는 실행하지 않았다.

## 실패·취소·재시도·복구와 남은 gap

project revision과 source content revision 재검사는 stale 변경 채택을 막는다. lock 강제 제거는 사용자의 명시적 복구 요청이며 살아 있는 writer가 없다는 사전조건이 필요하다. 자동 retry/lock expiry/전역 async 취소는 없다. snapshot/index 실패 시 journal을 기준으로 복구해야 한다. 2026-10-05 공식 owner gate와 Studio scratch 검증의 정확한 범위는 [제품 Current](docs/IMPLEMENTATION_STATUS.md)에 있다. 실제 사용자 원고·사람의 채택·외부 provider 결과, 실제 browser navigation 취소, 배포 host는 그 gate로 qualification되지 않는다. 공개 process·HTTP의 추가 gap은 하위 분석에 있다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.
