# SceneMotionCompiler — 구현 분석

## 현재 분석·개선 범위

Application service composes `TRACK_B_HANDLER_IMPLEMENTATIONS` directly instead of defining
three forwarding async functions. The canonical command registry still maps each implementation,
and the service still owns input admission, authenticated Studio approval restrictions and result
or error envelopes. The latest docs-only test scope and results, and their limits, are in
[Current](docs/IMPLEMENTATION_STATUS.md).

이미 열린 실행 구성 변경도 소스 정리를 마쳤다. MCP는 root/project만 받고 현재 SMC 이름의
server composition을 직접 호출한다. CLI/Studio/Application의 무효 `inputPaths` config와 custody의
`applicationConfigPath` alias를 제거하고 retired 입력은 거절한다. Verified handoff에서 파생하는
실제 보호 경로, receipt와 native process/session authority는 보존한다. Tests/build/runtime은
별도 qualification이다. 최신 검사 범위는 [Current](docs/IMPLEMENTATION_STATUS.md)에 둔다. 상세는
[MCP](apps/mcp-server/ANALYSIS.md)·[store](packages/store/ANALYSIS.md)를 따른다.

## Verdict·책임 경계

NVC의 승인된 narrative·cinematic plan을 durable scene projection으로 바꾸고 SAC 자산을 검증하여 motion HTML/evidence/release를 만드는 독립 제품이다. caller는 CLI/MCP/Studio와 SAC다. 방향 **IMPROVE**. 현재 aggregate·suite 결과와 남은 qualification은 [Current](docs/IMPLEMENTATION_STATUS.md)가 소유한다. 이 문서는 통합 분석과 코드 owner 경계를 기록한다.

| 단위 | 판정·이유 |
|---|---|
| application/adapters | 독립 제품 코어. 명령 registry, projection, SAC ingress, service command surface와 command-specific prepare/approve/promote/status owner. CLI/MCP/Studio 표면은 service를 쓰고 release operation은 각 owner에 남는다. NVC raw narrative ID는 원문을 보존하고 SceneGraph/projection ID에는 bounded canonical-or-hash identity를 사용한다. [application 모듈 분석](packages/application/ANALYSIS.md), [NVC adapter 분석](packages/adapters/ANALYSIS.md) |
| production | [독립 분석](packages/production/ANALYSIS.md): motion/clip/QC/editorial 후보와 ffprobe effect. 실제 생성·승인 비소유. |
| composer | Pure HTML composition only; semantic HTML parsing comes from contracts. The duplicate parser adapter and consumerless Composer barrel were removed. [분석](packages/composer/ANALYSIS.md). |
| core | SMC-local canonical values, raw/prefixed SHA-256 primitives, and closed schema-registry access; callers import owner modules directly. Callerless generic filesystem and implementation-bundle helpers were removed. [분석](packages/core/ANALYSIS.md). |
| contracts | Closed registries, stable producer-contract facades, Track B protocol-specific primitives including Track B raw-byte hashing, transport·semantic parser, artifact-specific validators, and one cross-artifact binding owner. Generic buffered byte hashing remains in core. [분석](packages/contracts/ANALYSIS.md). |
| source-integrity | [독립 분석](packages/source-integrity/ANALYSIS.md): public facade 아래 bounded inventory/manifest, deterministic ZIP create/inspect, isolated clean-install verifier |
| store + `engine/crates/smc-trackb-custody` | [store 분석](packages/store/ANALYSIS.md), [custody helper 모듈 분석](engine/crates/smc-trackb-custody/ANALYSIS.md): store barrel 아래 binary admission, stable file reads, purpose-bound session factories, Rust pointer custody owner를 분리한다. Hash primitive은 contracts owner를 소비하고, 구현 class는 barrel에서 숨긴다. 실제 Cargo member 경로는 Cargo.toml이 정본. |
| browser-renderer | [독립 분석](packages/browser-renderer/ANALYSIS.md): pinned Chromium CDP·measured evidence. |
| apps/mcp-server | [독립 분석](apps/mcp-server/ANALYSIS.md): direct CLI executable, strict launch configuration, Application command/tool composition, envelope validation. The unconsumed root barrel and names-only tool export were removed. |
| packages/cli | [독립 분석](packages/cli/ANALYSIS.md): executable argument adapter, registry-derived option grammar, exact Application command mapping, stateless result projection. |
| apps/studio-host, macos-launcher | Studio는 local HTTP/approval ingress. Server 조립, review-session lifecycle, project artifact access, macOS project-root admission은 [전용 분석](apps/studio-host/ANALYSIS.md), process/window lifecycle와 ready-project identity binding은 [launcher 분석](apps/macos-launcher/ANALYSIS.md)에서 나눈다. LauncherCore는 protocol parsing만 담당한다. SwiftPM launcher도 material entry다. |
| verification scripts | [scripts 분석](scripts/ANALYSIS.md): stable aggregate CLI facade 아래 gate policy, source identity, workspace/lock, process-group cleanup, bounded log capture, typed errors owners를 분리했다. |
| evaluation-report schema | UNWIRED 보존: AGENTS의 명시적 보존 요구. unused ≠ dead. |

## 2026-09-30 core dead-surface removal — source-only

SMC-local consumers use `canonical.js` and `schema-registry.js` directly. The unused core/adapters
barrels and core's uncalled generic filesystem and implementation-bundle helpers are removed. The
new core analysis records the remaining owners and the source-packaging boundary. The generated
manifest has 235 entries and owns its digest; generation is metadata only. No tests, build, lint,
runtime, or manifest verification was run.

## 2026-10-01 streaming hash owner clean break — source-only

`packages/core/src/canonical.js` now owns incremental SHA-256 over async byte chunks as well as
buffered hashes. `packages/production/src/io.js` keeps file admission, the 2 GiB bound, and
before/after identity checks, while delegating digest state to core; its private duplicate
`createHash` loop is gone. No path policy, serialized contract, or artifact identity changed.
At that source-only snapshot, tests/build/fmt/Clippy/runtime and source-manifest freshness were
**NOT_RUN**. The current docs-only test scope is recorded in [Current](docs/IMPLEMENTATION_STATUS.md).

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| project-narrative-bundle | public registry → projectNarrativeBundle → NVC adapter | Completed raw hash·package/map + bible/plan/approval·semantic occurrence | SMC projection files | graph + projection receipt | `command-registry.js`, `adapters/src/narrative-courier-release.js` |
| production-prepare | registry → prepareProduction | typed stage + current source/media/review | production 후보와 ffprobe 관찰 | request/clip/QC/timeline/OTIO, finalApproval=false | `packages/production/ANALYSIS.md` |
| compile-scene-html | public registry → compileHtml | SAC handoff·asset coverage/hash·width/height | application handoff facade composes contract and file owners; composer pure | HTML + provenance 또는 error | `trackb-commands.js`, `scene-asset-compiler-handoff*.js` |
| trackb-evidence-measure | public command → measurement runtime | locked browser/current HTML & semantic binding | `trackb-measure-runtime.js` orchestrates lifecycle; `trackb-measure-evidence.js` validates custody bytes and receipt binding; `trackb-measure-support.js` maps typed errors | measured evidence, not release approval | application analysis |
| prepare/promote/status | CLI/MCP/Studio → Application command service | candidate/artifact hashes·custody descriptors | runtime prepare/promote/status owners + Rust custody | candidate/current pointer/uncertainty | `trackb-release-runtime-{prepare,promote,status}.js`, service |
| trackb.release.approve·Studio 전용 | exact origin one-shot UI → studio dispatcher | 사람 intent/candidate binding/token | approval record | 승인 또는 거절 | `studio-dispatcher.js`, release runtime |
| macOS launcher | SwiftUI → host start/open/stop | locked runtime profile/project directory | launcher lifecycle | local Studio endpoint | `apps/macos-launcher` |

## 실제 흐름

main path: `NVC release-bundle + continuity + cinematic plan + human approval → verified raw files → exact shot/semantic projection → receipt → SAC prepare/approve/publish → SAC handoff → verified assets + graph → composeSceneHtml → output/provenance → measured evidence → release prepare → Studio human approve → custody promote → release status`.

중요 write의 caller는 `APPLICATION_COMMAND_REGISTRY → service/dispatcher → handler`다. CLI/MCP 일곱 명령과 Studio 전용 approval은 서로 다른 권한 surface다. `compileHtml`은 승인·release pointer를 수정하지 않는다.

추가 후보 경로: `projection → SAC visual pack/approval → motion request → 외부 결과 영수증/실제 clip → ffprobe/QC → editorial candidate → limited OTIO`. 실제 video provider와 final video release는 연결되어 있지 않다. 자세한 경계는 production ANALYSIS가 소유한다.

## 현재 state·contract·I/O owner

NVC owns narrative; SMC adapter owns derived projection; SAC owns approved asset handoff; composer owns deterministic HTML construction and delegates semantic-manifest interpretation to contracts; browser owns observations; Studio owns explicit human approval ingress; Rust custody owns atomic release pointer operation. Candidate hash·receipt version은 contract constant. browser executable/runtime profile은 configuration; timeout은 effect policy. UI state는 승인 authority가 아니다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| NVC approved cinematic projection | REACHABLE | SATISFIED | narrative boundary test coverage; literary review excluded. Current execution evidence: [Current](docs/IMPLEMENTATION_STATUS.md) | `tests/narrative-courier-release.test.js` |
| SAC ingress·HTML·provenance | REACHABLE | PARTIAL | SAC boundary/output test coverage; multi-file crash gap. Current execution evidence: [Current](docs/IMPLEMENTATION_STATUS.md) | `tests/scene-asset-compiler-asset-boundary.test.js`, `scene-html-output.test.js` |
| 폐쇄 schema·command authority | REACHABLE | SATISFIED | contract-authority test coverage; producer pin is a separate check. Current execution evidence: [Current](docs/IMPLEMENTATION_STATUS.md) | `tests/contract-authority.test.js`, scripts |
| measured evidence | REACHABLE | UNKNOWN | NOT_RUN: production Track B measurement on current HTML. The locked Chromium animatic test is a separate synthetic-fixture path. | browser 분석 |
| prepare/Studio approve/promote/status | REACHABLE | PARTIAL | Node regression coverage: [Current](docs/IMPLEMENTATION_STATUS.md); NOT_RUN: signed packaged-helper admission/lifecycle, Darwin Node-to-helper session, real Studio Human approval/release | store/custody·[Studio 분석](apps/studio-host/ANALYSIS.md) |
| motion/clip/QC/editorial 후보 | REACHABLE | PARTIAL | PASS: production2; 실제 ffprobe, fake review·clip이므로 품질 제외 | [production 분석](packages/production/ANALYSIS.md) |
| archival evaluation schema | UNWIRED | UNKNOWN | N/A: 명시적 보존 요구, runtime 조회는 거절 | AGENTS, closed registry |
| TrackA·직접 이미지 생성 | ABSENT_IN_SCOPE | UNKNOWN | N/A: 확정 비목표 | PRODUCT_CONTRACT |

## 의존성·findings

**M1 · HTML output effect 경계 — 수정, 범위 제한.** 문제 → lexical containment 뒤 `writeFile`이 symlink/원본 경로를 따라갔다. 근거 → `trackb-commands.js::compileHtml`; 원인 → 문자열 경계와 filesystem object를 혼동. 영향 → 외부 파일 또는 입력 artifact 덮어쓰기. 방향 → `scene-html-output.js`가 HTML/provenance 양쪽을 먼저 검사하고 ancestor/leaf symlink·special file·검증된 입력 alias를 거절한다. exclusive temp+fsync+rename을 적용했다. 협력 writer 환경의 실제 fs 회귀6개 PASS이며 적대적 directory-swap을 막는 descriptor sandbox라고 주장하지 않는다.

**M2 · output/provenance pair — 부분 개선.** 문제 → 두 파일을 하나의 원자 transaction으로 쓸 수 없다. 근거 → `publishSceneHtml`. 원인 → 독립 파일 교체. 영향 → 두 번째 rename/fsync 오류 시 partial publication. 방향 → 두 temp를 먼저 완성하고 HTML 다음 provenance를 publish하며 `scene-html-output-incomplete`에 `outputCommitted/provenanceCommitted`를 보고한다. 중단 시 pair recovery/immutable generation pointer는 아직 미구현이다.

**M5 · implicit shot authority — 수정.** 문제 → scene별 1 shot/4초와 composer 추가 delay가 승인되지 않은 timing/연출을 결정했다. 근거 → `narrative-courier-release.js`, `cinematic-contract.js`, `packages/composer/src/scene-html.js`와 신규 cinematic18 검사. 원인 → scene와 shot의 동일시 및 scene 단위 semantic owner. 영향 → multi-shot·대사 coverage·frame intent 손실. 방향 → 승인 CDP exact projection, occurrence별 shot allocation, frame 기반 preview, 모든 대사 출력. camera의 실제 영화적 재현·시각 품질은 여전히 별도 qualification이다.

**M6 · release source binding — 수정, 보관 범위 제한.** 문제 → authority의 원본 release/package/map hash가 receipt와 모두 대조되지 않았다. 근거 → release loader/contract validator 및 source capsule. 원인 → 개별 hash 검사와 lineage closure 혼동. 영향 → 다른 source authority의 잘못된 채택. 방향 → 세 hash 교차검증·capsule로 expected graph 재구성. native custody에 media 전체가 보관된다고 주장하지 않는다.

**M7 · source manifest 게시 ownership — 수정.** 문제 → temp descriptor를 닫은 뒤 inode 번호 재사용 때문에 다른 파일을 자기 temp로 오인할 수 있었다. 근거 → `scripts/source-manifest.mjs::publishSourceManifest`, fs 회귀10 PASS. 원인 → descriptor 수명과 pathname ownership 검사 분리. 영향 → 잘못된 cleanup/publication. 방향 → 검사/게시가 끝날 때까지 descriptor를 유지한다.

**M8 · HTML output containment — 수정.** 문제 → `scene-html-output.js`는 output containment를 `${root}${sep}` 문자열 prefix로 판정했다. `path.relative(root,target)`의 상대 경로를 검사하도록 바꿔 filesystem root workspace를 포함한 root decision을 정리했다. Targeted output-boundary tests cover the change; the complete current suite qualification is recorded in [Current](docs/IMPLEMENTATION_STATUS.md). Tests do not use `/` as a project root, so the OS-root case remains a separate runtime qualification.

## 실패·취소·재시도·복구와 남은 gap

custody는 rename 전 실패와 rename 이후 불확실성을 구분하며 후자는 reconcile-required이지 실패 전 상태라는 보장이 아니다. 승인 없는 promote와 임의 browser/provider fallback은 금지다. Signed packaged-helper admission/lifecycle, Darwin Node-to-helper session, product Track B measurement, and real Studio Human release remain separate qualification boundaries; their latest evidence is recorded in [Current](docs/IMPLEMENTATION_STATUS.md). Historical test and manifest results retain only their dated source snapshots.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.
