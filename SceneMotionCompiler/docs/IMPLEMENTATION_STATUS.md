# SceneMotionCompiler — 현재 구현

## 2026-10-05 현재 검증 — 자동 prerequisites와 aggregate

`npm test`는 `scripts/run-test-suite.mjs`가 package test를 실제 실행한다. 매 invocation마다 Rust/Cargo
1.97.1을 확인하고 SMC `smc-animatic`, NarrativeCourier release CLI, SceneAssetCompiler `sac-animatic`을
각 owner의 `target/`에 `--release --locked --offline`으로 freshness 확인·빌드한다. 기존 실행 파일의
존재만으로 현재 owner source를 대체하지 않는다. 필요한 SceneAssetCompiler producer source, FFmpeg,
ffprobe, Chromium도 test 시작 전에 확인한다. 이 prerequisite가 없거나 준비·실행에 실패하면 suite는
nonzero로 끝나며 animatic/cinematic cases를 환경 실패로 skip 처리하지 않는다. 지원 host에서 IPv6 loopback을
사용할 수 없는 경우의 조건부 skip은 남아 있다.

NVC cinematic 검증은 NVC의 `tools/requirements-cinematic.txt`와 Python 3.10 이상을 사용해 SMC `.tmp/` 아래
test-only 격리 venv를 준비하고, `jsonschema==4.26.0` 및 `pip check`를 확인한다. Python은 bundled workspace
runtime 또는 PATH에서 선택하며 `SMC_TEST_PYTHON`으로 지정할 수도 있다. 격리 Python, 선택한 owner binaries,
Python 및 Node 경로를 child process에 전달한다. 이 준비는 global Python이나 production runtime dependency를
바꾸지 않는다.

준비에는 합산 15분, 각 Cargo build에는 최대 10분, venv 생성·pip 설치·검증에는 각각 60·180·60초 제한이
있다. Node suite도 최대 15분이며 aggregate의 `tests` gate는 준비와 suite를 합쳐 15분이다. 잠기지 않은 Rust
dependency를 network fallback으로 가져오지 않는다. package `test` 자체와 aggregate 모두 이 제한 안에서
실패를 반환한다.

Aggregate verifier의 `SMC_PACKAGE_MANAGER`는 기본 `npm`이고 `pnpm`을 명시적으로 지원한다. 다른 값은 gate 실행
전에 거절된다. pnpm 선택 시 각 gate는 실제 pnpm executable과 argv로 기록된다. `contracts:pin`은 aggregate
밖의 별도 검사다.

2026-10-05 KST current snapshot에서 `SMC_PACKAGE_MANAGER=pnpm pnpm run verify`는 **10/10 gates PASS**했고,
그 안에서 package test script는 **227/227 PASS, fail 0, skipped 0**이었다. `.tmp/verify/summary.json`의
10개 gate와 `SOURCE_MANIFEST.json`의 264-file identity가 일치했다. 이 검사는 production native runtime을
추가하거나 바꾸지 않았다. Suite에는 synthetic project/PNG와 simulated test-review를 사용한 locked
Chromium→FFmpeg animatic candidate 통합 case가 포함되며, 실제 사용자 원문·자산의 의미/품질이나 사람 승인을
검증하지 않는다.

## 2026-10-03 animatic producer conversion

2026-10-03 animatic producer를 owner-local Rust 실행 파일로 전환하고 실제 실행 경계를 검증했다.
Rust workspace 43 tests, native crate clippy/release build, 실제 `npm test` 226 tests는 **PASS**다.
현재 지원·수용 기준은 Darwin/arm64이며 다른 host의 동등 qualification을 주장하지 않는다.

정밀 재검토에서 root·parent·attempt·frame·media I/O를 열린 FD에 묶고, encoder/probe의
project pathname 입력을 bounded stdin/stdout stream으로 교체했다. 실제 24-frame WebM,
interpreter 없는 PATH, root rename 후 원본 candidate/receipt 유지, attempt ancestor symlink
교체 후 외부 쓰기 0개·unknown, 취소·강제중단·복구가 모두 PASS다. SAC의 기존 integral-float
producer hash를 JCS와 분리해 검증하고 숫자 크기 비교만 동등하게 처리했다. native 11 tests,
전체 workspace Clippy/fmt/release와 개발 venv를 사용한 전체 npm gate가 PASS다.

## 2026-10-04 historical environment-limited direct test run

2026-10-04 docs-only revision에서 `cargo build -p smc-animatic --release --locked --offline`는 **PASS**다.
`npm` executable이 없어 bundled Node로 package의 `node --test --test-concurrency=1 tests/*.test.js`
script body를 실행한 결과는 **219/226 PASS**였다. 6개 animatic tests는 `NarrativeCourier` release
CLI가 없어 시작 단계에서, 1개 cinematic test는 Python `jsonschema`가 없어 fixture 단계에서 실패했다.
SMC product source는 바꾸지 않았고 image provider 호출은 하지 않았다. 이 결과는 환경이 막은
7개 test를 qualification PASS로 바꾸지 않는다. 이것은 당시 test prerequisite runner가 없던 시점의
historical direct-run 결과다. 2026-10-05 source snapshot의 최신 결과는 위 섹션을 따른다.

## 현재 연결

| 범위 | 직접 읽은 근거·owner | 현재 의미 |
|---|---|---|
| 공개 command | [registry](../packages/application/src/command-registry.js)·[service](../packages/application/src/service.js) | 하나의 metadata/handler/CLI/MCP/Studio 등록. Human approve는 Studio 전용 |
| Projection·HTML | adapters·composer·application handoff/output owners | Completed NVC/approved cinematic → v3 graph/capsule → SAC handoff → moving HTML/provenance |
| Production 후보 | [production](../packages/production/src/index.js) | Motion request·clip observations·QC·editorial/OTIO 후보, generation/final release 없음 |
| Animatic | [native producer](../engine/crates/smc-animatic/src/animatic.rs)·[browser asset](../packages/production/src/animatic-runtime.js)·[Node adapter](../packages/production/src/animatic.js) | Approved packet/PNG → exclusive attempt → exact-frame native CDP → VP9 WebM/HTML/VTT candidate |
| Measurement | browser-renderer·application measure owners | Locked Chromium evidence와 renderer-only observation |
| Release custody | [store facade](../packages/store/src/index.js)·Rust custody | Descriptor/lock·durable bytes·exact candidate/approval·DeliverySet/pointer CAS |
| Transports·기존 UI | CLI/MCP/Studio/macOS launcher | Typed framing·loopback/origin/one-shot review·project 선택/host 수명, 별도 domain state 없음 |
| Source integrity | packages/source-integrity·scripts/source-manifest | Maintained source inventory·package/manifest·독립 gate. Metadata는 qualification 아님 |

Command forwarding·중복 public exports/hash·path candidate 경로를 정리하고 실제 owner를 직접 소비하게 했다.
Provider 실행·서사/자산 의미·Human approval·native custody는 각각의 경계에 남는다.
모듈/변경 상세는 [ANALYSIS](../ANALYSIS.md), 계약은 [SPEC](SPEC.md), 실행 구조는 [ARCHITECTURE](ARCHITECTURE.md)를 따른다.

## 완료 의미와 남은 한계

Animatic의 `candidate`는 `finalApproval:false`·`audio:none`이다. Exact same attempt 재호출은
저장 결과를 반환하며 cancel/recover는 renderer를 재실행하지 않는다. SMC는 전달된 입력 bytes를
다시 검사하고 live NVC source는 integration이 별도 관찰한다. 최종 작품 release adoption은 미구현이다.

기존 Track B의 prepare·Studio Human approve·promote/CAS는 서로 다른 effect다. Actual provider·
영상 final subject/retention·audio/NLE/OTIO roundtrip, signed native-helper/Studio lifecycle·최대 부하·
다른 host·물리 disk/OS fault와 모든 orphan 자동 정리는 별도 범위다. Source archive는 서명/공증된
독립 앱 배포본이 아니다. [PLAN](PLAN.md)에 후속 결정과 qualification을 둔다.

## 과거 실행 근거

2026-10-03 native animatic 검증은 실제 locked Chrome·font·FFmpeg/ffprobe와 합성 PNG/명시 TEST review를 사용했다.
24-frame candidate·same-attempt 재진입·motion 변경 거절·파일 hash 변조·cancel first-terminal·강제 중단·recover
4개 animatic cases가 통과했다. 같은 승인 fixture를 다른 attempt로 `PATH=/story-no-interpreters`에서 다시
render해 WebM candidate를 확인했으며 media 실행 파일은 절대 경로로 지정했다. 이 경로에는 Python/Node/Cargo/Rustc가 필요하지 않았다.

Native 7 tests에는 duplicate JSON·canonical hash·frame timestamp와 실제 malformed claim의 unknown 보존,
absent attempt의 not_started, 명시 owner-root resource 부재의 effect 전 거절이 포함된다. 기존 custody 32 tests도 통과했다.
`npm test`는 bundled Node와 임시 npm 11.6.2 CLI로 실제 실행했다. 별도 기존 cinematic Python test에는
임시 개발 venv의 jsonschema 4.26.0을 명시했다. 최초 system Python 실행의 223/224는 그 선택 dependency 부재로 실패했고,
이 환경에서 재실행한 최종 전체 224/224가 통과했다. 이는 native animatic 런타임 의존성을 추가한 것이 아니다.
SOURCE_MANIFEST는 마지막 문서 갱신 뒤 재생성하고 verify:manifest로 대조한다.
실제 사용자 원문/이미지 generation·Human approval·final video release·다른 host·물리 disk/OS fault는 검증하지 않았다.

[2026-10-02 animatic snapshot](../../docs/research/2026-10-02-animatic-implementation.md)은 package test script의
224/224 통과, 실제 Chrome/FFmpeg의 합성 입력 4 cases와 당시 source manifest를 기록한다.
원문 변경·hash/파일 변조·취소·강제 중단·recover의 기계 경계를 관찰했으며 실제 그림 생성·Human
승인·작품 품질·최종 release는 인증하지 않았다. 이전 227 Node/32 Rust/aggregate 기록은 각 source graph에 한정한다.
현재 문서 변경이나 새 host에 이 PASS를 자동 적용하지 않는다.

<details>
<summary>과거 source snapshot의 실행·변경 기록</summary>

## 2026-09-30 current-source local verification

The declared package test body, `node --test tests/*.test.js`, passed 227/227 tests. The current
242-file source tree passed `node scripts/verify-source-tree.mjs`; root integration boundary tests and
the public geultu→NVC→SMC→SAC→SMC owner-chain also passed. The aggregate 10-gate release verifier was
not run again in this validation. The current source manifest was regenerated and verified after
this status update. Native `npm` was unavailable on the host, so the package's declared Node test
command was executed directly. See the
[current-source validation report](../../docs/research/2026-09-30-real-use-validation-cancellation-hardening.md).

The QA story's semantic fidelity failed independent review; its mechanical package and hash checks
do not certify story quality. Image/video providers and human visual approval remain **NOT_RUN**.

## 2026-09-30 latest current-tree verification

전체 Node suite 227/227 및 현황 문서 갱신 후 공식 aggregate 10/10 PASS. Aggregate는 source manifest
243/243, source-integrity, Track B, CLI, Studio contract, release gates를 검증했다. 검증된 source tree
identity는 [root 실사용 보고서](../../docs/research/2026-09-30-real-use-validation-latest.md)를 본다.
이 현황 문서와 report를 갱신한 뒤 source manifest를 다시 생성·검증해 243/243 files를 확인했다.
현재 tree identity는 `SOURCE_MANIFEST.json`이 소유한다. 첫 direct suite의
child Python 환경과 첫 aggregate의 단일 Chromium cleanup test는 각각 환경 보정/격리 재현 후
재실행에서 해소됐으며 최종 gate는 PASS다. native `npm`은 없어 aggregate의 npm 명령은 임시 pnpm shim으로
전달했다. [실사용 결과](../../docs/research/2026-09-30-real-use-validation-latest.md).

## 2026-09-24 current-source revalidation — custody filesystem split

The current filesystem/admission/descriptors/publication split passes `cargo fmt --check`, custody
Clippy with `-D warnings`, and 32 Rust tests (7 unit + 25 protocol). The full Node regression suite
passes 227/227 on the same source graph, and the current 240-file `SOURCE_MANIFEST.json` verifies.
The extraction follow-up removed unused facade re-exports and test-only imports exposed by Clippy;
the filesystem behavior and public helper protocol remain unchanged. The workspace local gate passes
after producer-owned manifests were regenerated. The package aggregate still cannot run because
`npm` is absent; signed helper admission, locked Chromium, and Studio Human release remain NOT_RUN.
The current public owner-chain and text package runs are recorded in the
[root real-use report](../../docs/research/2026-09-24-real-use-validation.md).

## 2026-09-24 custody filesystem responsibility split — source-only

`filesystem.rs` remains the private facade for existing callers. Project/request admission, POSIX
descriptor and identity primitives, and owned temporary publication/cleanup now live in separate
private owners. Existing error strings, byte/path bounds, descriptor authority, fsync points, CAS
semantics, and test fault seams were moved intact. No tests/build/lint/runtime or manifest verifier
was run after this source move. The generator refreshed `SOURCE_MANIFEST.json` metadata only: 240
files. Exact manifest metadata is in the root current status report. The previous
Node/Rust PASS evidence applies to the preceding source graph.


## 2026-09-24 last verified source graph — before filesystem split

The test namespace, startup parser, bounded JSON-lines framing, and descriptor-safe project-root
admission splits now pass current-source verification. The full Node suite is 227/227; custody
formatting, Clippy (`-D warnings`), and 32 Rust tests (7 unit + 25 protocol) pass. The latest
source manifest verifies 237 files with tree hash
`sha256:8e30b0789a06a767155b52410644aa05377c6bccf6b5f4b3e3eb19d61833ee44`.
The full `npm run` aggregate gate is NOT_RUN because `npm` is unavailable; locked Chromium,
packaged helper release admission, and Studio Human release also remain NOT_RUN. The exact evidence
and corrected tree hash are in the [root real-use report](../../docs/research/2026-09-24-real-use-validation.md).

## 2026-09-24 earlier module graph — source package and Node suite verified (historical)

The current source-package modules keep the existing ESM exports behind
packages/source-integrity/src/source-package.js. The current source manifest verifies 222 files.
The full Node suite passes 227/227 on a source copy with the checked-in dependency lock, and
run-source-integrity.mjs created two byte-identical ZIPs and verified an isolated extraction against
the embedded manifest. The verification hashes are recorded in the root real-use report.

The root deterministic gate and current public owner-chain CLI E2E also pass. Clean npm install plus
the extracted package's aggregate verify, pinned Chromium measurement, packaged native custody helper,
and Studio human-approval release remain NOT_RUN. The deterministic ZIP/extraction smoke is not the
clean-install proof; source-integrity ownership is described in
[source integrity analysis](../packages/source-integrity/ANALYSIS.md).

## 2026-09-23 이전 runtime snapshot 검증 증거

The output boundary now computes containment with `path.relative`, so filesystem root `/`
is handled consistently with the current project-root validator. Parent, sibling, empty, and
absolute-relative results remain rejected. Existing symlink checks, protected artifact checks,
and two-file partial-commit reporting are retained. The source snapshot at that verification point
passed all 227 Node tests, 32 native custody Rust tests, and the 174-file source manifest check.
The complete aggregate release verifier, packaged Node-to-helper release cycle, locked Chromium,
and Studio approval flow were not run.

## Latest runtime/custody decomposition — revalidated 2026-09-23

`trackb-release-runtime.js` now delegates runtime support, registry, artifacts, custody, approval,
and delivery to internal modules while keeping its four command exports. The Node custody protocol
and response checks live in `trackb-custody-protocol.js` behind the existing
`trackb-custody.js` caller surface. The current layout passes the package `test` script through
`pnpm test` (227/227), the current 181-file source manifest verification, and the repository
`integration/scripts/verify-local.sh` gate. During validation, missing protocol constants and
Track B contract imports were found and restored; the complete suite passed after those fixes.
The app-to-packaged-native-helper lifecycle, locked Chromium observation, and aggregate SMC
release verifier remain unrun.

## NVC release adapter effect/projection separation — revalidated 2026-09-23

`narrative-courier-release.js` remains the existing public entrypoint and composes Completed release
ingress with durable projection publication/loading. Pure SceneGraph projection, semantic occurrence
checks, receipt binding, and rebuild-from-source-capsule proof now live in
`narrative-scene-projection.js`; project-root/JSON/bounded-read/atomic-write effects live in
`narrative-courier-release-files.js`; SMC-local error/hash primitives live in
`narrative-courier-release-support.js`. NVC narrative/plan authority, producer schemas, public
exports, receipt format, and custody ownership were not changed. The current layout passes all 227
Node tests, the public geultu→NVC→SMC→SAC→SMC CLI E2E, the root deterministic gate, and the final
185-file `SOURCE_MANIFEST` verification. These fixture checks do not qualify real cinematic approval,
locked Chromium, packaged custody helper publication, or external media generation.

## 검증·UNKNOWN

이전 runtime layout에서 `node --test tests/*.test.js`는 **227 PASS / 0 FAIL**, `cargo test -p smc-trackb-custody --locked`는 **32 PASS / 0 FAIL**, manifest는 174 files/mismatch 0이었다. 최신 runtime extraction 이후에는 이 명령들을 다시 실행하지 않았다. `scripts/verify.sh` aggregate/release gate도 실행하지 않았다.

MCP stdio tests는 이전 227개 Node suite에서 PASS했다. 최신 runtime extraction의 import/wiring, packaged native helper lifecycle, locked Chromium, Studio human approval/release는 NOT_RUN이다. 실제 영상·이미지·사람 검토 품질은 synthetic fixture로 검증하지 않았다. 전체 세부 결과와 미검증 경계는 [상위 Current](../../docs/IMPLEMENTATION_STATUS.md)에 집계한다.

</details>
