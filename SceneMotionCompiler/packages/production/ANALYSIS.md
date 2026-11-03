# SMC production mechanisms — 구현 분석

## Verdict·책임 경계

**IMPROVE / 높은 확신도(로컬 후보 경로), 낮은 확신도(실제 제작 품질).** 승인된 cinematic/visual 입력에서 motion 요청을 만들고, 외부 반환 clip을 검사하여 shot QC와 편집 후보를 조립한다. caller는 `production-prepare`, `production_prepare`, public JS 함수다. 단일 SMC 내부 material unit이며 새로운 제작 서비스·중앙 DB·승인 authority가 아니다.

실제 영상 provider를 호출하는 실행 adapter, audio renderer, final video custody는 없다. 이름이 production이라고 상용 장편 제작 완료로 읽으면 안 된다.

## Module ownership

| Module | Owner | Boundary |
|---|---|---|
| `src/index.js` | Public stage producer facade and the single file-backed `production.prepare` dispatcher | Preserves direct stage function names; owns request read, fixed kind dispatch, source recheck, and immutable publish |
| `src/production-contract.js` | Closed stage schemas, exact-field/integer guards, contract errors, artifact sealing, and common assertions | Pure contract owner; no filesystem or media observation |
| `src/visual-context.js` | NVC projection plus verified SAC pack/approval, references, and keyframes | Reads exact inputs and constructs only the context needed by motion requests |
| `src/motion-request.js` | MotionRequestDraft projection and current request recomputation | No provider invocation or media adoption |
| `src/clip-lifecycle.js` | Provider receipt binding, clip candidate observation, QC recomputation, and internal current-candidate loaders | Consumes caller-supplied receipt/media; no provider client or approval |
| `src/editorial.js` | Approved-shot-order timeline assembly and constrained OTIO projection | Emits candidate/interchange bytes only; no final video or Human release |
| `src/io.js` | Bounded project-relative file reads, size/staleness admission, safe publication, and ffprobe process observation | Owns file/process effects and feeds bounded media chunks to core's SHA-256 stream owner |

The stage-module split was included in the 2026-09-30 isolated current-source validation: the full
227-test Node suite and 10-gate SMC aggregate passed there. The follow-up in this work moves
`check`, `exact`, `integer`, and artifact sealing into this contract owner, uses core's
`sha256Prefixed` for buffered byte hashes, and delegates bounded streaming media digesting to
`canonical.js` while `io.js` retains file admission and change detection.
The white-box fixture now uses the same core hash owner. This
owner-only clean break is after that validated snapshot; its tests, build, lint, runtime, aggregate,
and manifest verification were not run under the current instruction. See the
[current real-use record](../../../docs/research/2026-09-30-real-use-validation-current.md).

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| `production.prepare` / CLI·MCP, Studio 아님 | registry → service `prepareProduction` → `prepareProductionArtifact` | exact requestPath/outputPath, root-relative files, closed stage kind | 요청 dispatch와 immutable publication | artifact ref + `artifactState`, `finalApproval:false` / error | `../application/src/command-registry.js`, `src/index.js` |
| `MotionRequestDraft` | caller → `prepareMotionRequest` | current projection, SAC pack/approval, all image hashes, shot/policy | 기존 intent의 투영; provider 실행 없음 | sealed `ShotMotionRequest` | `visualContext`, `currentRequest` |
| `ClipIngestDraft` | caller → `ingestClip` | provider receipt/request/transfer/media binding | 로컬 clip 검증, 외부 effect의 claim 수용 | `GeneratedClipCandidate` / rejection | `validateProviderReceipt`, `io.js::probeClip` |
| `ClipReviewDraft` | caller → `reviewClip` | 실제 ffprobe 재검사 + 선택적 bound reviewer evidence | QC observation/repair classification | `ShotQCReport` PASS/BLOCKED, 승인 아님 | `currentCandidate`, `reviewClip` |
| `EditorialDraft` | caller → `buildEditorialTimeline` | shot order/count, current requests/QC recompute, trims/handles/audio refs/captions | 결정적 편집 후보 | `StoryEditorialTimeline` CANDIDATE / reject | `currentQC`, timeline builder |
| `OTIOExportDraft` | caller → `exportEditorialOTIO` | timeline 재계산, cut/dissolve만, audio 없을 때 | interchange projection | OTIO JSON; unsupported는 reject | `exportEditorialOTIO` |
| local process read | probe → `execFile(ffprobe)` | root path/media SHA, MP4/MOV, no shell, bounded outputs/deadline | subprocess observation | width/height/rational fps/read frame count/audio streams | `src/io.js` |

## 실제 흐름

대표 경로: `approved source capsule → verified SAC VisualReferencePack + approval → ShotMotionRequest → 외부 provider가 만든 receipt/media 입력 → 실제 ffprobe → candidate → evidence-bound QC → 5 clips를 승인된 shot 순서로 trim → 324-frame timeline candidate → OTIO JSON`.

검사에서는 실제 ffmpeg로 **단색 합성 fixture**를 인코딩하고 실제 ffprobe로 읽었다. 5-shot binding·frame count·CLI·stale checks의 증거이지 실제 generation/character consistency/사람 승인 증거가 아니다. 실제 NVC/SMC/SAC Python·Node process가 호출되는 범위와 fixture review를 구별한다.

## 현재 state·contract·I/O owner

`contracts/schemas/production/production-v1.schema.json`(제품 root 기준)이 closed input/output 계약이다. foreign camera/transition/reference shape는 기존 SMC/SAC 원본을 `$ref`한다. receipt/hash와 semantic owner를 복제하지 않는다.

provider state는 `SUCCEEDED / FAILED / CANCELLED / OUTCOME_UNKNOWN`. SUCCEEDED도 candidate이며 media/observation 없이는 성립하지 않는다. transfer claim이 true이면 request의 permission이 true여야 한다. 이 검사 자체가 실제 네트워크 전송 차단이나 provider identity 인증은 아니다.

QC `PASS`는 결정적 media checks와 세 observed 항목이 모두 PASS일 때만 생성된다. 미검토는 `NOT_RUN`→`BLOCKED`. 입력 review는 local operator/model attestation이며 신뢰된 reviewer 서비스나 서명 증명은 없다. `currentQC`는 단순 자기 hash만 믿지 않고 현재 candidate·review 원문·probe 결과로 다시 계산한다.

`production-contract.js` owns `check`, exact-field and safe-integer guards, schema assertions, and the production artifact seal. The seal continues to use the cinematic contract's canonical content-hash owner. In-memory raw-byte identity uses core's `sha256Prefixed`; `io.js` keeps its bounded streaming digest for large media, where reading the full file into memory would violate the I/O bound.

`io.js`의 내부 `POLICY`는 JSON 8 MiB, media 2 GiB, probe 60초/1 MiB 출력 한도를 고정한다. callerless `PRODUCTION_POLICY` barrel alias는 제거했다. root/path, `SMC_FFPROBE_BIN`은 Configuration; approved frames/camera는 Intent이고 timeout policy와 섞지 않는다. `contentHash`는 정수-only canonical payload, 별도 raw file SHA는 bytes identity다. DB/cache/current pointer를 새로 만들지 않았다.

`PRODUCTION_STAGES` is a private map in `src/index.js` and selects only a declared stage producer.
Shared schema/sealing rules live in `production-contract.js`; request/clip/QC/editorial stages consume
those owner functions without moving state or approval authority into the dispatcher. Individual stage
producers remain directly callable through the existing index export names, while the dispatch map,
current-candidate loaders, and validators stay module-internal.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| motion request prepare, candidate ingest | REACHABLE | SATISFIED | PASS: 실제 public CLI·file/probe 경계; provider dispatch는 미포함 | `tests/production-editorial.test.js`(제품 root 기준) |
| clip contract QC·repair classification | REACHABLE | PARTIAL | PASS: 구조·재검사·NOT_RUN 차단; visual/model quality NOT_RUN | `reviewClip` |
| 5-shot trim/order/total timeline | REACHABLE | SATISFIED | PASS: fixture 324frames/24fps, order·trim/stale negatives | `buildEditorialTimeline` |
| cut/dissolve OTIO JSON projection | REACHABLE | PARTIAL | PASS: JSON 구조/refs; 공식 OTIO roundtrip·NLE import NOT_RUN | `exportEditorialOTIO` |
| audio plan refs·caption ranges | REACHABLE | PARTIAL | NOT_RUN: 실제 audio sync/loudness/mix; audio export는 거절 | `audioQualification`, `OTIO_AUDIO_EXPORT_UNQUALIFIED` |
| real motion generation/provider reconciliation | ABSENT_IN_SCOPE | NOT_IMPLEMENTED | NOT_RUN: provider/credential/version 미지정 | provider receipt는 입력일 뿐 |
| automatic selective repair executor | ABSENT_IN_SCOPE | NOT_IMPLEMENTED | NOT_RUN: 분류만 제공, paid retry 없음 | repairs[] |
| video final Human release/custody | ABSENT_IN_SCOPE | NOT_IMPLEMENTED | NOT_RUN: 기존 Track B HTML release와 별도 | 루트 PLAN |

## 실패·취소·재시도·복구

immutable output은 동일 bytes 재발행만 허용하며 충돌을 덮어쓰지 않는다. source/read set이 바뀌면 다시 생성·검토해야 한다. partial provider result나 unknown state에 media를 채택하지 않는다. `OUTCOME_UNKNOWN`은 기존 job 조회/조정이 먼저다. `CANCELLED`는 종단이며 재요청에는 새 operator 권한이 필요하다. 실패 후보를 자동 재시도하거나 검토 결과를 자동 승인하지 않는다.

library에는 `AbortSignal`이 있고 출발 전/게시 전 검사와 ffprobe 취소를 지원한다. CLI/MCP transport cancellation을 end-to-end signal로 전달하는 경로는 아직 없다. timeline/QC 재검사 단계 전체에 단일 취소 deadline이 전달되는 것도 미완료다. disk/fsync 오류·외부 path-swap에 대해 root-relative 검사만으로 OS sandbox를 주장하지 않는다.

## Findings·gap·검증·근거

**PM1 — self-hash만으로 QC 승격 금지.** 문제 → JSON을 고쳐 hash만 맞추면 PASS처럼 보일 수 있음. 근거 → `currentQC`/`currentRequest`. 원인 → hash는 authority가 아님. 영향 → 미검토 clip의 편집 진입. 방향 → 원본 current sources·probe·review evidence에서 재구성하여 동일성 검사.

**PM2 — 영수증 재채택 시 권한 재검증.** 문제 → ingest 이후 후보 재사용에서 transfer claim이 빠질 수 있었음. 근거 → `validateProviderReceipt`를 ingest/currentCandidate 양쪽에서 호출. 원인 → 최초 validation을 재채택 보증으로 오인. 영향 → 바뀐 receipt의 제한 우회. 방향 → 매 boundary에서 같은 owner validator를 사용.

**PM3 — OTIO는 원본 media를 소유하지 않음.** 문제 → project-relative 경로를 중첩 output에서 읽으면 media가 달라짐. 근거 → `target_url`을 검증한 실제 경로의 `file:` URI로 변환. 원인 → artifact scope와 interchange path base 혼동. 영향 → NLE에서 offline media. 방향 → 로컬 유효 URI, 이식 시 media relink/portable pack은 별도 작업. 공식 format 호환은 아직 NOT_RUN.

**PM4 — 편집 후보를 release로 오인 금지.** 문제 → 기존 HTML custody와 새 timeline은 다른 artifact 집합. 근거 → 기존 release 7개 입력과 신규 package의 `finalApproval:false`. 원인 → 제품 이름을 하나의 완성 경로로 오인. 영향 → lineage·승인·media 보관 누락. 방향 → [루트 PLAN](../../../docs/PLAN.md)의 별도 video release subject/retention 결정을 먼저 완료.

**PM5 — transition/audio 지원의 한계.** 문제 → 계약 enum 전체를 renderer가 구현한 것은 아님. 근거 → cut/cross-dissolve만 OTIO 허용, audio 있으면 거절, HTML CSS는 제한적 camera preview. 영향 → unsupported 효과를 성공으로 간주할 위험. 방향 → capability를 명시하고 조용한 fallback 금지.
