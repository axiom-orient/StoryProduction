# 2026-09-24 현재 소스 실사용 검증

## 최종 후속 검증 — 현재 모듈 graph와 공개 text 제작 흐름

실행은 `/Users/ax/repository/StoryProduction`의 현재 소스를 `/private/tmp/storyproduction-user-real-use.n3eaAy/source`에 격리 복사해 수행했다. 원본에는 `.git`이 없어 revision은 없다. `target`, `node_modules`, `.tmp`, `.git`, Python cache를 제외한 6,107개 파일과 77,191,965 bytes가 checkout/snapshot 간 byte-hash로 같고 deterministic tree SHA-256은 `4950c970208eedc09f84e6daf0e950f247ee34b472795e575198e34d171ffdc4`다. 환경은 macOS arm64 27.0, Rust 1.97.1, Node 24.19.0, Python 3.12.14 / jsonschema 4.26.0이다.

이 fingerprint는 검증 결과를 이 문서와 `docs/IMPLEMENTATION_STATUS.md`에 기록하기 전의 tested snapshot을 식별한다. 이후 변경은 두 evidence 문서뿐이다.

| 검증 기준 | 결과 | 방법과 관측 |
|---|---|---|
| Root contract/integration gate | **PASS / TEST** | `bash integration/scripts/verify-local.sh`; owner schema pin, integration tests, SMC boundary checks, SAC manifests, IPC source-snapshot 검사 |
| geultu current source | **PASS / BUILD+TEST+LINT+RUNTIME** | `./scripts/verify.sh`; 146 tests, fmt/strict Clippy/release build, CLI/MCP smoke, 인증된 격리 Studio loopback의 project/scene 호출과 process 정리 |
| NarrativeCourier current source | **PASS / BUILD+TEST+LINT** | `./verify-all.sh --require-rust`; 60 repository contract checks, Rust workspace check/test/fmt/Clippy 통과 |
| NVC source identity | **PASS / STATIC** | 499 entries, manifest SHA-256 `139169d58d5755a1c3c150b14abe12c9704c4df590755f7d1869b6d996b65471`; 현재 트리와 mismatch 0 |
| NVC MCP stdio current binary | **PASS / RUNTIME FIXTURE** | initialize/list/call, 10개 도구, 프로젝트 상태/doctor, synopsis·screenplay import/export, 잘못된 delivery request 무부작용, `NEEDS_REVIEW`, 승인 도구 미노출, idle cancel 명시 오류 |
| SceneMotionCompiler aggregate | **PASS / TEST+RUNTIME FIXTURE** | aggregate scripts 10/10, Node 227/227, native custody 32/32, source integrity와 Track B/CLI/Studio/release smoke. 호스트에 npm이 없어 격리 scratch에서 기존 pnpm store의 잠긴 의존성을 사용하고 temporary `npm` shim을 통해 같은 package.json scripts를 pnpm으로 실행했다. native npm 설치/동작 자체를 인증하는 결과는 아니다. |
| SceneMotionCompiler source identity | **PASS / STATIC** | 240 files, source tree SHA-256 `8b5b61f0b06e3b16a8d97a790404fb08a6abbdb8c1930af40378ad7b00cd9b2b`, manifest mismatch 0 |
| SceneAssetCompiler publication | **PASS / TEST** | `python3 -m unittest discover -s tests -v`; 14/14, root-local skill/docs/release manifests 131/100/253 PASS |
| SceneAssetCompiler complete release verifier | **PARTIAL / INPUT MISSING** | `python3 tools/verify_release.py`는 G09에서 `tests/fixtures/legacy-psd/PROVENANCE.json` 누락으로 중단. 실제 PSD provenance postflight는 NOT_RUN |
| ImagePromptCompiler current graph | **PARTIAL / BUILD+TEST+LINT+RUNTIME FIXTURE** | `./scripts/verify.sh`는 check, 263 workspace tests, doc tests, fmt/Clippy, docs/release build를 통과한 뒤 없는 `evals/dogfood/historical/1.4.0/image/image-evidence.json`에서 중단. 공식 CLI와 local Web smoke는 별도 실행해 PASS. OpenAI/Codex provider 실제 요청은 NOT_RUN |
| `storyctl text` public authoring/package | **PASS / RUNTIME FIXTURE** | 현재 geultu release와 NVC CLI로 81회 public CLI 호출, 9개 hash-bound package 파일, `source.md` 입력 byte equality, 각 파일 SHA-256 확인. package marker는 `complete` |
| synthetic story fidelity | **FAIL / MANUAL REVIEW** | `빛이 남은 역` 원문은 지우가 지워진 근무 시간을 봉투 속 원본과 대조하고 기록을 정정한 뒤 혼자 떠나야 한다. 대본은 `방`의 두 대사와 옥상의 바람만 남겨 비교·정정·출발 행동을 빠뜨린다. storyboard는 옥상에도 기록실 구도/소리/전환을 반복한다. |
| 실제 사용자 원고와 실제 승인 | **NOT_RUN / HUMAN** | 사용자 원고나 작가가 직접 만든 후보·실제 편집/visual 승인은 제공되지 않았다. fixture writer/reviewer 및 STS simulated gates는 사람 승인 증거가 아니다. |
| 외부 writer/image/video와 18-stage runtime | **NOT_RUN / EXTERNAL·IMPLEMENTATION GAP** | 실제 외부 생성 provider는 호출하지 않았고, 문서의 18단계/144 세부 작업은 아직 현재 runtime에 연결되지 않았다. |

실사용 중 모듈 분할 회귀도 고쳤다. ImagePromptCompiler `artifact/discovery.rs`에서 `std::io` import가 빠졌고, publication의 미사용 import와 facade의 production 미사용 re-export가 compile gate를 막았다. `publish_file`는 기존 unit-test import 경로로만 재수출했다. geultu/NVC/IPC의 신규 owner 파일은 rustfmt에 맞췄고, NVC에서 분할 뒤 남은 unused import 세 개를 제거했다. NVC 499-entry source manifest를 다시 생성·검증했다. 이 변경은 owner, wire schema, review authority, serialized output을 바꾸지 않았다.

최신 증거 로그와 산출물:

- [root local gate](</private/tmp/storyproduction-user-real-use.n3eaAy/root-gate-current.log>) · [source fingerprint](</private/tmp/storyproduction-user-real-use.n3eaAy/current-source-fingerprint.json>)
- [geultu official gate](</private/tmp/storyproduction-user-real-use.n3eaAy/geultu-verify-current.log>) · [NVC official gate](</private/tmp/storyproduction-user-real-use.n3eaAy/nvc-verify-current.log>) · [NVC source manifest](</private/tmp/storyproduction-user-real-use.n3eaAy/nvc-manifest-verify-current.log>) · [NVC MCP stdio](</private/tmp/storyproduction-user-real-use.n3eaAy/nvc-mcp-stdio-final-current-source.log>)
- [SMC aggregate](</private/tmp/storyproduction-user-real-use.n3eaAy/smc-aggregate-verify.log>) · [SMC custody Rust tests](</private/tmp/storyproduction-user-real-use.n3eaAy/smc-custody-tests.log>) · [SMC manifest](</private/tmp/storyproduction-user-real-use.n3eaAy/smc-manifest-verify.log>)
- [SAC publication tests](</private/tmp/storyproduction-user-real-use.n3eaAy/sac-tests.log>) · [SAC release verifier and missing fixture](</private/tmp/storyproduction-user-real-use.n3eaAy/sac-release-verify.log>)
- [IPC official gate and missing evidence](</private/tmp/storyproduction-user-real-use.n3eaAy/ipc-verify-formatted.log>) · [IPC CLI smoke](</private/tmp/storyproduction-user-real-use.n3eaAy/ipc-cli-smoke-current.log>) · [IPC Web smoke](</private/tmp/storyproduction-user-real-use.n3eaAy/ipc-web-smoke-current.log>)
- [81-call authoring/package run](</private/tmp/storyproduction-user-real-use.n3eaAy/text-authoring-final-current-source.log>) · [package manifest](</private/tmp/storyproduction-user-real-use.n3eaAy/authoring-run-final/project/package/story-package.json>) · [screenplay](</private/tmp/storyproduction-user-real-use.n3eaAy/authoring-run-final/project/package/screenplay.md>) · [storyboard](</private/tmp/storyproduction-user-real-use.n3eaAy/authoring-run-final/project/package/storyboard.md>)

검증 실행은 완료됐지만, 전체 제품 qualification과 실제 작품의 창작 품질은 **PARTIAL**이다. package `complete`나 simulated `approved`를 사람의 내용 승인으로 해석하지 않는다.

## 후속 검증 — Application/MCP 모듈 분할 현재 소스

루트 작업본에 `.git`이 없어 commit ID 대신 격리한 source snapshot과 관련 경계 파일의 SHA-256을
사용했다. 실행은 `/private/tmp/storyproduction-live-20260924.7Eyk73/source` 복사본에서 했으며,
현재 geultu `application.rs`/`application/`와 NVC MCP `tools.rs`/`tools/`/source manifest를
shared workspace와 대조했다.

| 기준 | 판정 | 방법·관측 |
|---|---|---|
| 루트 cross-component gate | **PASS / TEST** | `bash integration/scripts/verify-local.sh`; schema pin, integration·SMC·SAC·IPC 경계 검사 |
| geultu 현재 Application 분할 | **PASS / TEST+RUNTIME** | `./scripts/verify.sh`; 146 tests, fmt/clippy, release build, CLI/MCP smoke, 격리 Studio loopback·인증·project/scene 호출·정리 |
| NarrativeCourier 현재 MCP 분할 | **PASS / TEST** | `./verify-all.sh --require-rust`; 60 Rust tests, check/fmt/clippy. MCP protocol tests 3/3 |
| NVC source manifest | **PASS / STATIC** | 474 entries, SHA-256 `1295b1fc6feaafcfe7683aa15092d381ccddfb2c1fc71b51da210a3b3a5e3c51` |
| MCP stdio 실제 프로세스 | **PASS / RUNTIME FIXTURE** | initialize/tools/list 및 10개 도구 전부 호출. status/doctor/context, synopsis·screenplay import/export, HTML run/list, invalid request 무부작용, idle cancel 명시 오류 확인. 승인 도구 미노출 |
| public owner-chain E2E | **PASS / RUNTIME FIXTURE** | geultu→NVC 19 STS 작업·10 simulated gates→SMC→SAC→SMC; 2 scenes, 5 shots, 10 asset requirements, HTML 14,323 bytes |
| `storyctl text` authoring/package | **PASS / RUNTIME FIXTURE** | 81 public CLI 호출, 9 artifact, package `complete`, hashes와 intake source bytes 일치 |
| fixture 창작 충실도 | **FAIL / MANUAL FIXTURE REVIEW** | 원문은 기록 정정의 증거 확인·기록 보존·혼자 떠남을 요구하지만 대본은 그 중심 행동을 누락한다. 옥상 storyboard도 기록실 구도/소리를 반복한다. |
| 실제 사용자 원고·실제 사람의 창작/visual 승인·외부 생성 provider | **NOT_RUN / HUMAN·EXTERNAL** | 현재 실행은 `빛이 남은 역` 합성 원고, fixture writer/reviewer와 simulated gates만 사용했다. |

현재 source module split의 구체 파일 해시는 [source comparison](</private/tmp/storyproduction-live-20260924.7Eyk73/current-module-source-hashes.json>)에,
실행 로그는 [root gate](</private/tmp/storyproduction-live-20260924.7Eyk73/root-local-gate-current-modules.log>),
[geultu gate](</private/tmp/storyproduction-live-20260924.7Eyk73/geultu-verify-current-modules.log>),
[NVC gate](</private/tmp/storyproduction-live-20260924.7Eyk73/nvc-verify-current-modules.log>),
[manifest verify](</private/tmp/storyproduction-live-20260924.7Eyk73/nvc-manifest-current-modules.log>),
[MCP stdio](</private/tmp/storyproduction-live-20260924.7Eyk73/nvc-mcp-stdio-current-modules.log>),
[owner-chain E2E](</private/tmp/storyproduction-live-20260924.7Eyk73/owner-chain-e2e-current-modules.log>),
[81-call package flow](</private/tmp/storyproduction-live-20260924.7Eyk73/text-authoring-current-modules.log>)에 있다.

이번 실행은 변경된 Rust/MCP 경계와 합성 story-to-package flow를 검증했으며 실제 사용자 이야기의
품질을 인증하지 않는다. 18단계/144 세부 작업은 현재 runtime에 연결되지 않았다. 전체 제품
qualification은 ImagePromptCompiler의 historical evidence fixture 부재와 locked browser/native
및 사람 승인 범위의 미실행 때문에 계속 **PARTIAL**이다.

## 판정

검증 실행은 **DONE**, 전체 제품 qualification은 **PARTIAL**이다. User workspace 원문 대신 `빛이 남은 역` 합성 fixture와 격리 writer/reviewer를 사용했다. 루트 작업본에는 `.git`이 없어 commit ID를 연결할 수 없다. 검증용 복사본은 `/private/tmp/storyproduction-live-20260924.7Eyk73/source`에 두었고, 마지막 대조에서 리팩터링·수정한 24개 소스/manifest 파일의 SHA-256이 shared workspace와 복사본 사이 byte equality였다.

환경은 macOS arm64 27.0, Rust 1.97.1, Node 24.19.0, Python 3.12.14, `jsonschema==4.26.0`, headless Google Chrome 153.0.8010.53이다. Rust와 Python dependency 설치 및 Cargo target, browser profile, 프로젝트 작업공간은 모두 격리 검증 디렉터리 또는 임시 폴더에 두었다.

| 경계 | 판정 | 실행과 관측 |
|---|---|---|
| StoryProduction local deterministic gate | **PASS** | `bash integration/scripts/verify-local.sh`; schema/producer pin, integration Python 14 tests, SMC boundary suites, SAC manifests, IPC source snapshot |
| NarrativeCourier | **PASS** | `./verify-all.sh --require-rust`; 60 Rust tests, check/fmt/clippy. Source manifest 470 entries, SHA-256 `bed09732bcfd6685ff2605fc95510f739afad4c239e88e1ffbc52278c20307c9` |
| NarrativeCourier MCP screenplay adapter | **PASS** | 추가 `cargo test -p narrative-courier-mcp` 3/3 및 실제 stdio initialize/tools/list/tools/call; 10개 도구, `import_screenplay`와 `export_screenplay` 파일 I/O, 승인 도구 미노출 확인 |
| geultu | **PASS** | `./scripts/verify.sh`; 146 tests, fmt/clippy, release build, CLI/MCP smoke, 실제 격리 Studio loopback·인증·project/scene 호출·정리 smoke |
| SceneMotionCompiler | **PASS** | `node --test --test-concurrency=1 tests/*.test.js`; 227/227. Source manifest 210 files, tree SHA-256 `ddcc9fff33cb9c19106fd2a5b0e04cfade6ce5cc37cf4f080bd002085d9048f7`. Headless Chrome endpoint cleanup test 4/4 및 별도 실행에서 child 종료·임시 profile 삭제·socket 정리를 확인 |
| SceneAssetCompiler | **PASS** | publication suite 14/14. Release/docs/skill manifests 각각 250/100/130 entries, mismatch 0 |
| ImagePromptCompiler Rust and non-fixture gates | **PASS** | full gate에서 cargo check/test/doc/clippy/release build, workspace Rust tests와 doc-test suite, docs/catalog/representative QA/Rust structure audits, CLI/web smoke와 source snapshot 검사 통과 |
| ImagePromptCompiler full official gate | **PARTIAL** | `./scripts/verify.sh`의 `audit-image-evidence.py`가 원본에도 없는 `evals/dogfood/historical/1.4.0/image/image-evidence.json`에서 중단했다. 외부 evaluation evidence를 만들거나 복원하지 않았다 |
| Public owner-chain E2E | **PASS / FIXTURE** | geultu→NVC 19 STS 작업·10 simulated human gates→SMC→SAC→SMC; 2 scenes, 5 shots, 10 requirements, HTML 14,323 bytes |
| `storyctl text` authoring/package | **PASS / FIXTURE** | 81개 public CLI 호출, 9개 package artifact, `status=complete`, 각 SHA-256 일치, package `source.md`와 intake 원문 bytes 일치 |

실사용 흐름 중 리팩터링 결함을 찾아 최소 수정했다. SMC release preparation은 current delivery resolver를 schema-registry 모듈에서 잘못 가져오고 promote runtime은 custody의 bounded-write validator를 import하지 않았다. geultu authoring mutation의 private helper 타입과 Studio 추출 뒤 남은 imports/test path를 clippy/compiler 경계에 맞췄다. ImagePromptCompiler의 render profile에 옮겨진 `BTreeSet` import를 복원하고 profile-text 테스트 전용 re-export를 `cfg(test)`로 제한했다. NVC MCP screenplay export는 `Vec<u8>`를 generic JSON wrapper에 넘겨 응답을 숫자 배열로 인코딩했으므로, adapter가 structured JSON value를 반환하도록 고쳤다. 해당 변경 후 영향받은 component gate와 MCP process flow를 다시 확인했다.

### 콘텐츠 품질과 남은 범위

81-call authoring run은 저장·채택·패키징 경로를 확인했지만, fixture writer의 최종 대본은 이야기의 기록실 진실 확인 장면과 맞지 않았고 텍스트 콘티는 옥상 장면에도 기록실 구도를 반복했다. fixture reviewer가 승인해 package가 `complete`가 됐지만, 이는 구조·hash·review state 증거이지 서사 품질 승인으로 볼 수 없다. 실제 원문·작가가 작성한 응답·사람의 편집 검토는 **NOT_RUN**이다.

문서에 정의된 18단계/144개 세부 작업 카탈로그는 아직 runtime에 등록되지 않았다. 이번 authoring 검증은 현재 `storyctl text` 경로와 NVC 19-task fixture를 실행했으며, 18단계 전체 실행을 증명하지 않는다. Scene Provider v2/PerfectPixel, 실제 PSD·pixel review, 외부 image/video/writer provider, NVC Tauri UI, SMC aggregate release/native-helper/locked-renderer qualification도 **NOT_RUN**이다. 따라서 이 실행은 deterministic 로컬 경계와 fixture 흐름의 통과를 의미하며, 실제 작품·사람 승인·미디어 품질의 전체 qualification은 남는다.

## 증거 로그

- [최종 root local gate](/private/tmp/storyproduction-live-20260924.7Eyk73/root-local-gate-final-final.log)
- [NVC workspace gate](/private/tmp/storyproduction-live-20260924.7Eyk73/nvc-verify-current.log) · [MCP live import/export](/private/tmp/storyproduction-live-20260924.7Eyk73/nvc-mcp-live-final.log) · [NVC source manifest](/private/tmp/storyproduction-live-20260924.7Eyk73/nvc-manifest-verify.log)
- [geultu official gate](/private/tmp/storyproduction-live-20260924.7Eyk73/geultu-verify-after-studio-fix.log)
- [SMC 227-test suite](/private/tmp/storyproduction-live-20260924.7Eyk73/smc-full-tests-corrected-env.log) · [SMC source manifest](/private/tmp/storyproduction-live-20260924.7Eyk73/smc-manifest-verify-final.log) · [browser cleanup runtime](/private/tmp/storyproduction-live-20260924.7Eyk73/smc-browser-cleanup-runtime.log)
- [SceneAssetCompiler 14 tests](/private/tmp/storyproduction-live-20260924.7Eyk73/sac-tests-current.log) · [release/docs/skill manifests](/private/tmp/storyproduction-live-20260924.7Eyk73/sac-manifest-current.log)
- [ImagePromptCompiler official gate attempt](/private/tmp/storyproduction-live-20260924.7Eyk73/ipc-verify-after-fix.log) · [CLI smoke](/private/tmp/storyproduction-live-20260924.7Eyk73/ipc-cli-smoke.log) · [web smoke](/private/tmp/storyproduction-live-20260924.7Eyk73/ipc-web-smoke.log) · [other source audits](</private/tmp/storyproduction-live-20260924.7Eyk73/ipc-doc-links.log>)
- [owner-chain E2E](/private/tmp/storyproduction-live-20260924.7Eyk73/owner-chain-e2e-current.log) · [81-call text authoring and package hashes](/private/tmp/storyproduction-live-20260924.7Eyk73/text-authoring-current-final.log)
- [current vs isolated source SHA-256 comparison (24 files)](/private/tmp/storyproduction-live-20260924.7Eyk73/source-copies-sha-match.json)

## 2026-09-24 latest current module graph — real-use retest

Root StoryProduction has no .git metadata, so this run identifies the tested source through the SMC and NVC
generated manifests. Host: macOS arm64, Node 24.19.0, Python 3.12.14 with jsonschema 4.26.0, and Rust
1.97.1. Cargo builds used the existing isolated cache at /private/tmp/storyproduction-live-20260924.7Eyk73/cargo-home.

| 기준 | 판정·방법 |
|---|---|
| Root local deterministic gate | PASS / TEST — bash integration/scripts/verify-local.sh; source producer pins, integration, SMC boundaries, SAC manifests, and IPC source-snapshot check. Final log: [root gate](/private/tmp/storyproduction-smc-validation.2W5mTI/root-local-gate-final.log). |
| NarrativeCourier current DeliveryEngine graph | PASS / BUILD+TEST+LINT — ./verify-all.sh --require-rust, 60 Rust tests, cargo check/test/fmt/clippy; static Delivery source guard 112/112. SOURCE_MANIFEST.sha256 has 477 entries; manifest file digest sha256:c35a8deb453120df033ec86a1af5834477bba8c37f80e1e334f65ce2cc898a22. Log: [NVC gate](/private/tmp/storyproduction-smc-validation.2W5mTI/nvc-verify-current-source-after-docs.log). |
| SceneMotionCompiler current JS graph | PASS / TEST — node --test --test-concurrency=1 tests/*.test.js, 227/227. The current source tree has 222 files and tree hash sha256:ceabb16ddf52e84689e6d317ab8fd1dbe1d17a8aa5015712ee16b9e8542638cc. [Suite log](/private/tmp/storyproduction-smc-final.4hFFyv/node-tests.log), [manifest verification](/private/tmp/storyproduction-smc-final.4hFFyv/manifest.log). |
| SMC source package | PASS / RUNTIME FIXTURE — two byte-identical ZIPs; 222-file embedded manifest verified after isolated extraction. Package hash sha256:15dfe5fe7fcdeeaf92b3c2c3cb980f3e17761bb3a94cd8a5978c14ac10673f9f. Clean npm install plus aggregate verify is NOT_RUN. [Smoke summary](/private/tmp/storyproduction-smc-final.4hFFyv/smc/.tmp/source-integrity-final/summary.json). |
| Public owner-chain | PASS / RUNTIME FIXTURE — geultu CLI → NVC CLI, 19 STS tasks and 10 simulated gates → SMC projection → SAC release → SMC HTML. Output: 2 scenes, 5 shots, 10 requirements, 14,323-byte HTML. [E2E output](/private/tmp/storyproduction-smc-validation.2W5mTI/owner-chain-current.log). |
| storyctl text authoring/package | PASS / RUNTIME FIXTURE — 81 public CLI calls; 9 hash-bound files; source.md is byte-identical to intake; all package hashes match. Workspace: /private/tmp/storyproduction-smc-validation.2W5mTI/authoring-run-current-root/project. [CLI output and hash summary](/private/tmp/storyproduction-smc-validation.2W5mTI/text-authoring-current-root.log). |
| ImagePromptCompiler OpenAI adapter | PASS / TEST — 263 workspace tests, check, doc tests, fmt/clippy quality gate, documentation build and release build. Separate CLI prompt-only and loopback web smokes also PASS. Official ./scripts/verify.sh remains PARTIAL because audit-image-evidence.py requires absent evals/dogfood/historical/1.4.0/image/image-evidence.json; later catalog audits and source-snapshot write/verify were not reached. [Official gate log](/private/tmp/storyproduction-smc-validation.2W5mTI/ipc-verify-current-source-after-format.log), [CLI smoke](/private/tmp/storyproduction-smc-validation.2W5mTI/ipc-cli-smoke-current.log), [web smoke](/private/tmp/storyproduction-smc-validation.2W5mTI/ipc-web-smoke-current.log). |
| NVC renderer smoke and SMC clean package install | NOT_RUN / ENVIRONMENT — NVC host has no ImageMagick magick executable; the SMC clean package flow requires npm, which is unavailable in the host PATH. |

### Content fidelity result

The package marker says complete and the storyboard review file says approved, but the reviewer is explicitly
SIMULATED_HUMAN_FOR_VALIDATION. The original fixture requires Jiwoo to compare the erased roster time with
the original evidence, correct the record, leave the correction, and depart alone. The generated screenplay
instead contains two short lines in a room and only wind on the rooftop; its SceneMap consequently loses the
evidence comparison, record correction, and departure. Both storyboard panels reuse the record-room framing,
including the rooftop scene. That is a manual content-fidelity FAIL, separate from the mechanical PASS.
See the [fixture source](/private/tmp/storyproduction-smc-validation.2W5mTI/authoring-run-current-root/project/package/source.md),
[screenplay](/private/tmp/storyproduction-smc-validation.2W5mTI/authoring-run-current-root/project/package/screenplay.md),
and [storyboard](/private/tmp/storyproduction-smc-validation.2W5mTI/authoring-run-current-root/project/package/storyboard.md).

No user-provided story was available. Real human creative/visual review, actual OpenAI/Codex/image/video
provider calls, locked-Chromium measurement, packaged native custody-helper release, and Studio human
approval are NOT_RUN. The documented 18-stage/144-task catalog is not connected to the current runtime.

The validation run is DONE; whole-product and story-quality qualification remains PARTIAL.

## 2026-09-24 후속 실사용 재검증 — SMC aggregate verifier 분할 이후

현재 shared checkout에서 확인했다. root에 `.git` 메타데이터가 없어 commit ID는 없다. 환경은 macOS arm64,
Python 3.12.14 / jsonschema 4.26.0, Node 24.19.0, Rust 1.97.1이다. 처음에는 기본 Python 실행 경로에
`jsonschema`가 없어 doctor가 PARTIAL이었고, 기존 격리 Python 환경을 사용한 뒤 doctor와 root gate가
통과했다. owner-chain의 첫 Cargo offline build는 cache에 `serde`가 없어 중단됐다. 잠긴 Cargo 의존성을
`--locked` 온라인 build로 채운 뒤 동일한 public E2E를 다시 실행했다.

| 기준 | 판정 | 실행·관측 |
|---|---|---|
| 런타임 진단 | **PASS / RUNTIME** | `storyctl doctor`; Python 3.12.14, jsonschema 4.26.0, Node 24.19.0, Cargo/rustc 1.97.1 |
| root deterministic gate | **PASS / TEST** | `bash integration/scripts/verify-local.sh`; root integration, owner-contract pins, SMC selected boundaries, SAC manifests, IPC source-snapshot check |
| 현재 geultu/NVC 바이너리 | **PASS / BUILD** | `cargo build --locked`로 두 CLI를 현재 checkout에서 빌드한 뒤 public 흐름에서 사용 |
| SMC aggregate verifier split | **PASS / TEST** | `node --test tests/aggregate-verifier.test.js`; **14/14**. timeout/cancel process-group cleanup, lock, source identity, summary durability 경계 포함 |
| `storyctl text` 제작·패키지 | **PASS / RUNTIME FIXTURE** | 새 격리 project에서 81 public CLI calls; 9개 hash-bound artifact, `source.md`가 intake bytes와 일치, 전체 package hash 일치, status `complete` |
| public owner-chain | **PASS / RUNTIME FIXTURE** | geultu CLI → NVC 19 STS tasks / 10 simulated gates → SMC projection → SAC scene release → SMC HTML. 2 scenes, 5 shots, 10 requirements, 14,323-byte HTML |
| 현재 SMC source manifest | **FAIL / STATIC** | 선언 222 files / `sha256:ceabb16ddf52e84689e6d317ab8fd1dbe1d17a8aa5015712ee16b9e8542638cc`; 관측 230 files / `sha256:85d8c1e826422dad9d37f36c95cbafe4cd436e9a11ca02c5950865f099eb9056`. 새 7개 aggregate-owner 모듈과 `scripts/ANALYSIS.md`가 manifest에 없고 root facade 및 기존 분석 문서도 변경됨 |
| 전체 SMC aggregate release gate | **NOT_RUN / ENVIRONMENT** | host에 `npm` 실행 파일이 없고 SMC package scripts/aggregate gates가 `npm run`을 사용한다. 별도 manifest 검사는 실행해 위 FAIL을 확인했다. |
| fixture 창작 충실도 | **FAIL / MANUAL REVIEW** | 원문은 지우의 증거 대조·기록 정정·정정 기록 남김·혼자 떠남을 요구한다. 대본은 기록실 대사 두 줄과 옥상의 바람만 남기며 해당 선택/행동을 누락한다. 두 storyboard panel은 같은 기록실 구도·소리·전환을 옥상 씬에도 재사용한다. |
| 실제 사용자 원고·사람 승인·외부 writer/media provider | **NOT_RUN / HUMAN·EXTERNAL** | 검증은 합성 `빛이 남은 역`과 fixture writer/reviewer, simulated approval gates만 사용했다. |

명령 로그와 proof file은 [doctor](</private/tmp/storyproduction-real-use-20260924.M5jZGj/doctor-with-venv.json>),
[root gate](</private/tmp/storyproduction-real-use-20260924.M5jZGj/root-verify-local-with-venv.log>),
[geultu build](</private/tmp/storyproduction-real-use-20260924.M5jZGj/geultu-build-online.log>),
[NVC build](</private/tmp/storyproduction-real-use-20260924.M5jZGj/nvc-build-online.log>),
[SMC focused tests](</private/tmp/storyproduction-real-use-20260924.M5jZGj/smc-aggregate-tests.log>),
[SMC manifest inspection](</private/tmp/storyproduction-real-use-20260924.M5jZGj/smc-source-manifest.log>),
[owner-chain E2E](</private/tmp/storyproduction-real-use-20260924.M5jZGj/owner-chain-e2e.log>),
[text authoring run](</private/tmp/storyproduction-real-use-20260924.M5jZGj/authoring-run-after-build.log>),
[package integrity summary](</private/tmp/storyproduction-real-use-20260924.M5jZGj/authoring-run-after-build/integrity-summary.json>)에 남아 있다.
실제 fixture 비교 대상은 [원문](</private/tmp/storyproduction-real-use-20260924.M5jZGj/authoring-run-after-build/project/package/source.md>),
[대본](</private/tmp/storyproduction-real-use-20260924.M5jZGj/authoring-run-after-build/project/package/screenplay.md>),
[콘티](</private/tmp/storyproduction-real-use-20260924.M5jZGj/authoring-run-after-build/project/package/storyboard.md>)다.

기계 경로는 통과했지만 `complete` package는 fidelity 승인과 다르다. 검증 수행은 DONE이며, SMC release gate와
실제 작품 품질 qualification은 **PARTIAL**이다. 앞선 227/227 전체 suite와 222-file source archive 결과는
SMC aggregate verifier 분할 전 source graph에 대한 역사적 결과다.

## 2026-09-24 current-source real-use retest — before SMC Session split

현재 `/Users/ax/repository/StoryProduction` 소스에서 다시 실행했다. 작업본에는 `.git` metadata가 없어
commit ID를 연결할 수 없다. 환경은 macOS arm64, Node 24.19.0, Python 3.12.14 / jsonschema 4.26.0,
Rust 1.97.1이다. SMC 전체 Node suite와 custody Rust tests는 dependency lock 기준으로 격리한
`/private/tmp/storyproduction-real-use-current.yRAC5w` 사본에서 실행했고, source mirror의 비교에서
제품 파일 차이는 없었다. 원문·writer·reviewer·human gate는 전부 합성 fixture였다.

| 경계 | 판정 | 현재 실행 결과 |
|---|---|---|
| Root deterministic gate | **PASS / TEST** | `bash integration/scripts/verify-local.sh`; cross-owner pins, boundary suites, SAC manifests, IPC source-snapshot test |
| geultu CLI/current graph | **PASS / TEST+RUNTIME** | `./scripts/verify.sh`; 146 tests, fmt/clippy/release build, CLI and MCP smoke, isolated authenticated Studio HTTP flow |
| NarrativeCourier current graph | **PASS / BUILD+TEST+LINT** | `./verify-all.sh --require-rust`; 60 tests, check/fmt/clippy; `SOURCE_MANIFEST.sha256` 480 entries verified |
| SceneMotionCompiler current graph | **PASS / TEST** | `node --test --test-concurrency=1 tests/*.test.js`; 227/227 |
| SMC Track B native custody | **PASS / TEST** | `cargo test --manifest-path Cargo.toml -p smc-trackb-custody --locked --offline`; 7 unit + 25 protocol tests |
| SMC source manifest | **PASS / STATIC** | Declared generator refreshed `SOURCE_MANIFEST.json`; 230 files, tree hash `sha256:b8219da0b0c08f21b89b244593f0addb800f84e39c3d9d901f4dad18817306bf`; verifier returned no mismatches |
| SceneAssetCompiler | **PASS / TEST+STATIC** | Publication suite 14/14; skill/docs/release manifests 130/100/252 entries all verified |
| ImagePromptCompiler Rust | **PASS / BUILD+TEST+LINT** | Official gate reached its historical evidence audit after workspace check, 273 tests, doc tests, fmt/clippy, docs and release build; all preceding stages passed |
| ImagePromptCompiler official full gate | **PARTIAL / FIXTURE ABSENT** | `scripts/verify.sh` stops at missing `evals/dogfood/historical/1.4.0/image/image-evidence.json`; no evidence was fabricated or restored |
| IPC CLI / Web / MCP | **PASS / RUNTIME** | Release CLI prompt-only examples, loopback guided Web interview, and current stdio MCP initialize/list/interview flow (4 tools, `ready` prompt) |
| IPC follow-on audits | **PASS / STATIC** | Source-snapshot test, doc links, catalog assets/prompts, representative image QA, and Rust structure checks |
| StoryProduction owner-chain | **PASS / RUNTIME FIXTURE** | Current public CLI chain: geultu → NVC (19 work items / 10 simulated gates) → SMC → SAC → SMC; 2 scenes, 5 shots, 10 asset requirements, 14,323-byte HTML |
| `storyctl text` package flow | **PASS / RUNTIME FIXTURE** | 81 public CLI calls; 9 hash-bound files; intake `source.md` byte-equal and every package hash matched |
| Synthetic story fidelity | **FAIL / MANUAL REVIEW** | Screenplay omitted comparing the erased roster time against the original evidence, correcting the record, and leaving alone. The rooftop storyboard repeated record-room framing, sound, and transition. |
| SMC aggregate release gate | **NOT_RUN / ENVIRONMENT** | `npm` is absent. The direct locked Node suite and source-manifest verifier passed, but this does not qualify `npm run` aggregate/install behavior. |
| Real user story, human creative/visual approval, external writer/image/video provider | **NOT_RUN / HUMAN·EXTERNAL** | No user manuscript or external provider execution was used. The 18-stage/144-task catalog remains design-only, not runtime behavior. |
| Locked Chromium, packaged native-helper lifecycle, Studio Human release | **NOT_RUN** | These production qualification boundaries are outside the executed local CLI/HTTP/test-fixture flows. |

During this run, validation exposed and fixed two module-move defects: ImagePromptCompiler route helpers were
re-exported beyond their visibility scope, and the moved interview tests lacked their `JsonValue` import.
The core now passes 144/144 tests, and the complete workspace gate passes up to the missing historical
evidence fixture. The geultu CLI split also had a rustfmt ordering issue; after formatting, its full official
gate passed. SMC's stale 222-entry source manifest was regenerated to match the 230-file tree and then passed
verification. No story/scene ownership, producer schema, approval outcome, or external provider behavior was
changed.

Evidence logs are under `/private/tmp/storyproduction-real-use-current.yRAC5w`: [root gate](</private/tmp/storyproduction-real-use-current.yRAC5w/root-local-final.log>),
[geultu gate](</private/tmp/storyproduction-real-use-current.yRAC5w/geultu-verify-final.log>),
[NVC gate](</private/tmp/storyproduction-real-use-current.yRAC5w/nvc-verify.log>),
[SMC Node suite](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-node-suite-final.log>),
[SMC custody tests](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-rust.log>),
[SMC manifest](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-source-manifest-final.log>),
[SAC publication tests](</private/tmp/storyproduction-real-use-current.yRAC5w/sac-publication.log>),
[IPC official gate](</private/tmp/storyproduction-real-use-current.yRAC5w/ipc-verify-final.log>),
[IPC MCP stdio](</private/tmp/storyproduction-real-use-current.yRAC5w/ipc-mcp-stdio.log>),
[owner-chain E2E](</private/tmp/storyproduction-real-use-current.yRAC5w/owner-chain-e2e.log>),
[81-call authoring run](</private/tmp/storyproduction-real-use-current.yRAC5w/authoring-81-call.log>).
The produced synthetic package is under `/private/tmp/storyproduction-real-use-current.yRAC5w/authoring-run-latest/project/package`.

Verification execution is **DONE**. The whole product and story-quality qualification remains **PARTIAL**.

## 2026-09-24 SMC custody Session owner split — post-retest verification

After the cross-product real-use run, the native custody `Session` implementation was separated behind
the existing `session.rs` dispatcher. The `Session` type remains in `main.rs`; operation names, fixed read
order, purpose gates, lock/custody authority, pointer CAS, hash/error behavior, and public Node protocol
were retained. Project ingress, narrative lifecycle, measurement receipt/read, pointer publication, and
descriptor/storage helpers now have separate private module owners. Three 1 MiB limits were written as
`1024 * 1024`, `str` path byte checks use the equivalent `str::len()`, the Darwin `st_ino` identity keeps
its existing `u64` value, and the two cfg-selected publisher branches now return their final expression.
These clarity/lint changes do not change protocol values or bounds.

| Criterion | Result | Evidence |
|---|---|---|
| Native custody formatting | **PASS / STATIC** | `cargo fmt --manifest-path Cargo.toml --all -- --check` |
| Native custody Clippy | **PASS / LINT** | `cargo clippy --manifest-path Cargo.toml -p smc-trackb-custody --all-targets --locked --offline --target-dir <isolated-target> -- -D warnings` |
| Native custody tests | **PASS / TEST** | 7 in-binary unit tests + 25 protocol integration tests on the current source |
| SMC full Node suite | **PASS / TEST** | 227/227 on the current source mirror with the locked JS dependency set |
| SMC source manifest | **PASS / STATIC** | 235 files; `sha256:65e44cbceaf0f68723fb0efca7b87f2ebb08d4001b086a1e05dfda5e2dbc6206`; no verifier mismatches |
| SMC aggregate install/release gate | **NOT_RUN / ENVIRONMENT** | Host has no `npm` executable; exact `npm run` aggregate behavior remains unqualified |
| Signed Node-to-helper admission, locked Chromium, Studio Human approval | **NOT_RUN** | Darwin policy fails closed for an unsigned local helper; these production boundaries are not replaced by Rust protocol tests |

The public story owner-chain and 81-call text package described above were run immediately before this
custody-only source split. This follow-up verifies the changed native Rust source, the current Node graph,
and its source receipt; it did not rerun the end-to-end story fixture or modify any producer schema or
approval authority. Execution of this follow-up is **DONE**; SMC release qualification remains **PARTIAL**.

Logs: [native fmt](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-fmt-final.log>),
[native Clippy](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-clippy-after-session-split.log>),
[native tests](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-rust-after-session-split.log>),
[Node suite](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-node-suite-final-after-session.log>),
[source-manifest generation and verification](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-source-manifest-final-after-session.log>).

## 2026-09-24 SMC custody Session split — latest source follow-up

`smc-trackb-custody/src/session.rs` is now only the stable request dispatcher and private module
facade. `Session` remains owned by `main.rs`. Purpose-specific files own project ingress, session
lifecycle, measurement reads/receipt publication, pointer CAS publication, and shared descriptor
storage. Operation names, DTOs, response ordering, purpose guards, descriptor authority, CAS and
uncertain-commit behavior were preserved. The source split did not change producer schemas or approval
authority.

| Criterion | Result | Evidence |
|---|---|---|
| Native formatting | **PASS / STATIC** | `cargo fmt --manifest-path Cargo.toml --all -- --check` |
| Native Clippy | **PASS / LINT** | `cargo clippy --manifest-path Cargo.toml -p smc-trackb-custody --all-targets --locked --offline --target-dir <isolated-target> -- -D warnings` |
| Native custody behavior | **PASS / TEST** | Current source: 7 in-binary tests + 25 protocol tests |
| SMC Node regressions | **PASS / TEST** | Current source mirror: `node --test --test-concurrency=1 tests/*.test.js`, 227/227 |
| Cross-component deterministic gate | **PASS / TEST** | Re-run after this split: `bash integration/scripts/verify-local.sh` |
| Source manifest | **PASS / STATIC** | 235 files; `sha256:65e44cbceaf0f68723fb0efca7b87f2ebb08d4001b086a1e05dfda5e2dbc6206`; verifier reports no mismatches |
| Full SMC aggregate gate | **NOT_RUN / ENVIRONMENT** | `npm` is not installed on this host, so package `npm run` orchestration is unqualified |
| Darwin signed helper/runtime, locked Chromium, Studio Human release | **NOT_RUN** | These release-policy and measurement boundaries are not inferred from crate tests |

The 19-task owner-chain and 81-call document-package execution above happened immediately before this
native-only split. The split was then checked through the full Node suite, native Clippy/tests, source
manifest, and root deterministic gate; the story package/owner-chain were not rerun for this internal
custody reorganization. The verification state remains: execution **DONE**, product release and creative
quality qualification **PARTIAL**.

Post-split logs: [root gate](</private/tmp/storyproduction-real-use-current.yRAC5w/root-local-post-session.log>),
[native formatting](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-fmt-final.log>),
[native Clippy](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-clippy-after-session-split.log>),
[native tests](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-custody-rust-after-session-split.log>),
[Node suite](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-node-suite-final-after-session.log>),
[source manifest](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-source-manifest-final-after-session.log>).

## 2026-09-24 SMC test namespace extraction — source-only continuation

The seven inline tests were moved intact from `main.rs` to `src/tests.rs`, registered with
`#[cfg(test)] mod tests;`. The `Session` type, its private state, and protocol ownership remain in the
binary root. No assertion or product operation was changed. Following the active analysis/refactor-only
instruction, tests/build/lint/runtime and manifest verification were **NOT_RUN** after this test-file
move. The declared source-manifest generator refreshed metadata only: 236 files, total bytes 2,555,599,
tree hash `sha256:69d63b442136f618426969fbf88ecb1cfa9876f1336b87a542f4983dc7c28a10`. The result is not
verified; the prior 7/25 Rust, 227 Node, Clippy, root gate, and 235-file manifest results apply to the
source graph immediately before this final layout change.

Generation log: [SMC source-manifest metadata](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-source-manifest-tests-split-generate.log>).

## 2026-09-24 SMC startup, framing, and filesystem owners — source-only continuation

`main.rs` no longer owns CLI option parsing, bounded JSON-lines input framing, or project-root
descriptor admission. Those implementations moved to `startup.rs`, `protocol.rs`, and
`filesystem.rs` respectively. The binary root still owns `Session` and shared protocol/state types;
`run()` remains the order-preserving composition point. This followed the test-namespace extraction,
so both moves are newer than the last passing native/Node gate evidence above.

Per the active analysis/refactor-only instruction, this final source graph has **NOT_RUN** test, build,
lint, runtime, and manifest verification. The declared SMC generator refreshed `SOURCE_MANIFEST.json`
metadata only: 237 files, 2,557,242 total bytes, tree hash
`sha256:8e30b0789a06a767155b52410644aa05377c6bccf6b5f4b3e3eb19d61833ee44`. No verifier was run.

The owner-chain and 81-call authoring E2E remain evidence for the source graph before both of these
layout-only changes. The latest code changes preserve public CLI/helper entrypoints by construction,
but their behavior remains unverified as requested. The SMC aggregate npm release gate, signed
native-helper admission, locked Chromium, and Studio Human release remain NOT_RUN.

Generation log: [SMC source-manifest metadata after startup split](</private/tmp/storyproduction-real-use-current.yRAC5w/smc-source-manifest-startup-split-generate.log>).


## 2026-09-24 current-source real-use verification — after latest module splits

This follow-up verifies the current source layout after the source-only refactors recorded above. The checkout has no `.git` metadata, so the target is the isolated copy-on-write snapshot `/private/tmp/storyproduction-real-use-check-20260924/source`; the workspace source was not used for builds or runtime writes. Environment: macOS arm64 27.0, Rust 1.97.1, Node 24.19.0, Python 3.12.14 with `jsonschema` 4.26.0. The first root-gate attempt selected the system Python 3.9 and stopped before tests because it lacked `jsonschema`; the exact gate was rerun with the existing validation venv and passed. No dependency was installed.

| Criterion | Result | Command or observation |
|---|---|---|
| Root cross-component deterministic gate | **PASS / TEST** | `bash integration/scripts/verify-local.sh`; schema and integration checks, SMC boundaries, SAC manifests, and IPC source-snapshot test passed |
| geultu current CLI/Application graph | **PASS / TEST+RUNTIME** | `./scripts/verify.sh`; 146 Rust tests, format/Clippy/release build, CLI/MCP smoke, authenticated isolated Studio loopback and cleanup |
| NarrativeCourier current graph | **PASS / TEST** | `./verify-all.sh --require-rust`; 60 Rust tests, check/fmt/Clippy. `python3 tools/source_manifest.py verify`: 480 entries, manifest SHA-256 `493cd16ac40418ad09d5a0dd741dbdfcf83af514f88b2bdfc95701d5652e1f0e` |
| NarrativeCourier renderer qualification | **NOT_RUN / ENVIRONMENT** | Both `./verify-all.sh --renderers` and `./verify-all.sh --require-rust --renderers` exit at `FAIL renderer smoke: magick unavailable`; the renderer check cannot proceed. The separate required Rust gate above passes. |
| SceneMotionCompiler current JS and custody graphs | **PASS / TEST+LINT** | `node --test --test-concurrency=1 tests/*.test.js`: 227/227; custody `cargo fmt --check`, Clippy with `-D warnings`, and Rust tests: 7 unit + 25 protocol tests; `node scripts/verify-source-tree.mjs`: 237 files, tree hash `sha256:8e30b0789a06a767155b52410644aa05377c6bccf6b5f4b3e3eb19d61833ee44` |
| SceneMotionCompiler aggregate npm release gate | **NOT_RUN / ENVIRONMENT** | This host has no `npm` executable. The declared Node test command and native gate were run directly; `npm run` orchestration and package installation remain unqualified. |
| SceneAssetCompiler current publication boundaries | **PASS / TEST+STATIC** | 14/14 `tests/test_publication_boundaries.py`; root gate verifies 130 skill, 100 docs, and 252 release manifest entries |
| SceneAssetCompiler full release gate | **PARTIAL / G09 NOT_RUN** | `python3 tools/verify_release.py` exits when G09 attempts to read absent `tests/fixtures/legacy-psd/PROVENANCE.json`. The required PSD evidence could not be checked; no provenance was fabricated. |
| ImagePromptCompiler current workspace | **PARTIAL** | Official `./scripts/verify.sh` completed workspace checks/tests/docs/quality and release build, then `audit-image-evidence.py` stopped because `evals/dogfood/historical/1.4.0/image/image-evidence.json` is absent. `VERIFICATION.json` is also absent, so `source_snapshot.py --verify-verification` cannot pass; `test-source-snapshot.py`, release CLI smoke, and loopback Web smoke pass. No receipt or historical evidence was created. |
| Public owner-chain | **PASS / RUNTIME FIXTURE** | Rebuilt current-source geultu and NVC CLIs; 19 STS work items, 10 simulated approval gates, 2 scenes, 5 shots, 10 asset requirements, and 14,323-byte SMC HTML |
| `storyctl text` | **PASS / RUNTIME FIXTURE** | 81 current-source CLI calls; nine package artifacts, each hash matches its manifest, and package `source.md` is byte-identical to intake |
| Story fidelity | **FAIL / DIRECT ARTIFACT REVIEW** | The synthetic package is structurally complete, but its screenplay and storyboard omit or distort central source beats, detailed below |
| Real user manuscript, real human approval, external writer/image/video provider | **NOT_RUN / INPUT+HUMAN+EXTERNAL** | No user manuscript was present in the task; fixture writers/reviewers and all approval gates were simulated |

### Direct artifact review

The input `빛이 남은 역` requires Jiu to compare the erased shift time with the original envelope, establish that Minjun was elsewhere during the fire, correct the false record, leave the correction, and depart alone before the first train. The story and synopsis retain these beats. The generated screenplay instead says `INT. 방 - 밤`, gives Jiu “문을 닫아” and “이제 알겠어,” and reduces the rooftop to “바람이 분다.” It does not dramatize the evidence comparison, correction, or departure. The storyboard repeats the archive-room composition and paper/first-train sound on the rooftop and gives both scenes the same intent. The exact outputs are [screenplay](</private/tmp/storyproduction-real-use-check-20260924/authoring-run-current/project/package/screenplay.md>), [storyboard](</private/tmp/storyproduction-real-use-check-20260924/authoring-run-current/project/package/storyboard.md>), and [source](</private/tmp/storyproduction-real-use-check-20260924/authoring-run-current/project/package/source.md>).

Both run summaries mark the package complete because hashes, expected files, and simulated review state are complete. Those signals do not verify semantic fidelity or human approval. The same fidelity mismatch was visible in the separate owner-chain fixture. This is a product qualification failure, not a tooling failure.

### Evidence files

- [Root deterministic gate](</private/tmp/storyproduction-real-use-check-20260924/root-gate-current.log>)
- [geultu official gate](</private/tmp/storyproduction-real-use-check-20260924/geultu-gate-current.log>) and [current debug CLI build](</private/tmp/storyproduction-real-use-check-20260924/geultu-debug-cli-build.log>)
- [NarrativeCourier Rust gate](</private/tmp/storyproduction-real-use-check-20260924/nvc-gate-current.log>), [source manifest](</private/tmp/storyproduction-real-use-check-20260924/source/NarrativeCourier/SOURCE_MANIFEST.sha256>), and [renderer gate](</private/tmp/storyproduction-real-use-check-20260924/nvc-renderers-current.log>)
- [SMC Node suite and manifest](</private/tmp/storyproduction-real-use-check-20260924/smc-node-current-final.log>) and [custody Rust/Clippy](</private/tmp/storyproduction-real-use-check-20260924/smc-rust-current-retry.log>)
- [SAC publication suite](</private/tmp/storyproduction-real-use-check-20260924/sac-tests-current.log>) and [G09 release failure](</private/tmp/storyproduction-real-use-check-20260924/sac-release-current.log>)
- [ImagePromptCompiler official gate](</private/tmp/storyproduction-real-use-check-20260924/ipc-gate-current.log>), [CLI smoke](</private/tmp/storyproduction-real-use-check-20260924/ipc-cli-smoke-current.log>), [Web smoke](</private/tmp/storyproduction-real-use-check-20260924/ipc-web-smoke-current.log>), and [source receipt check](</private/tmp/storyproduction-real-use-check-20260924/ipc-source-receipt-current.log>)
- [Owner-chain run](</private/tmp/storyproduction-real-use-check-20260924/owner-chain-current.log>) and [81-call package run](</private/tmp/storyproduction-real-use-check-20260924/authoring-current.log>)

No product source, schema, or approval authority was changed by this validation. Verification execution is **DONE**; whole-product qualification is **PARTIAL**, synthetic story fidelity is **FAIL**, and real user work / genuine human approval / external generation remain **NOT_RUN**. The documented 18-stage/144-task catalog is still not wired to runtime. Locked Chromium, packaged helper admission, and Studio Human release were not run.


## 2026-09-24 SMC custody filesystem responsibility split — source-only

`smc-trackb-custody/src/filesystem.rs` remains the private facade and retains the names imported by `main.rs`, session modules, and unit tests. Implementation moved into three owners:

- `filesystem/admission.rs` owns project-root admission, safe relative and immutable path grammar, IDs, and strict request-field checks.
- `filesystem/descriptors.rs` owns descriptor-relative open/mkdir/link/rename/unlink/fsync operations and stable file/directory identity.
- `filesystem/publication.rs` owns temporary-file identity checks, cleanup and publish sequencing, plus the existing `cfg(test)` fault seams.

The extraction preserves the existing facade names, bounds, error text, fd-relative authority, no-follow behavior, fsync order, uncertainty/cleanup semantics, and test fault values by moving the original function bodies intact. It does not change custody state ownership, JSON-lines protocol, producer schema, cross-component contracts, or Human approval authority.

Tests, build, lint, runtime, and source-manifest verification were not run per the current user instruction. The declared source-manifest generator refreshed metadata only: 240 files, 2,561,673 bytes, tree hash `sha256:e5d89760957e9b29276b3cf8f96c2dd8cfeb22bcfbad78b67bc0061d84fd3c71`. The earlier 227 Node/32 Rust/Clippy/237-file manifest PASS evidence applies to the source graph before this split.


## 2026-09-24 ImagePromptCompiler structured render owners — source-only continuation

`render.rs` remains the internal entrypoint for render-profile dispatch, shared formatting helpers, and the structured prompt's stable concatenation order. The former inline scene, subject, and composition projections moved to `render/scene_sections.rs`; camera, lighting, color/LUT, and material projections moved to `capture_sections.rs`; required/excluded elements, safety clauses, and output parameters moved to `constraints.rs`. Exact text remains between composition and capture. Existing `editorial_flat`, `exact_text`, and `profiles` modules keep their contracts and behavior. This is a structural move of existing function bodies; no request schema, public CLI/Web/MCP surface, provider effect, or output ordering was intentionally changed.

Per the current instruction, tests, build, lint, runtime, prompt-output comparison, source-identity verification, and receipt generation were **NOT_RUN** after this move. The previous ImagePromptCompiler workspace and CLI/Web smoke evidence applies to the source graph before this split. The checked-out historical image-evidence fixture and `VERIFICATION.json` remain absent.


## 2026-09-24 ImagePromptCompiler subject inference owners — source-only continuation

`interview/image/inference/subjects.rs` remains the internal compatibility facade used by the existing `inference.rs` re-exports and image builder. The subject phrase parser, repeated-panel count/identity and relationship projection, finish/material extraction, shared lexical terms/text replacement, and count candidate logic are now separately owned under `subjects/`. Existing `ParsedSubject` fields and inference function names remain at their prior facade path. This change does not move image/profile authority, interview persistence, provider effects, schemas, or approval state.

No tests, build, lint, runtime, prompt-output comparison, source snapshot test, or receipt generation was run per instruction. The previous ImagePromptCompiler check/test/CLI/Web results are evidence for the graph before the render and subject inference splits, not for this current source graph.


## 2026-09-24 ImagePromptCompiler field inference owners — source-only continuation

`inference/fields.rs` remains the stable private facade for existing route callers. Scene/composition
extraction, surface/wardrobe/lighting recognition, and typography/palette extraction moved to
`fields/spatial.rs`, `fields/visual_attributes.rs`, and `fields/graphics.rs`. Public serialized values,
search precedence, diagnostics, and caller sequence were not intentionally changed. Tests, build,
lint, runtime, prompt-output comparison, source snapshot checks, and receipt generation were not run
per the current user instruction. The preceding workspace PASS results do not qualify the structured
render, subject inference, or field inference source-only graphs.


## 2026-09-24 NVC STS, SceneMap, and fixture-validator owners — source-only continuation

`engine/manifest.rs` stays the `Engine` entrypoint for `load_manifest`, `validate_manifest`, and the
engine-scoped `domain_identity` method. Validation is now separated into manifest ordering/dataflow,
Agent-node contract, and Human-gate contract owners. The ordered `available_paths` set is the single
source of truth for prior-output dependencies and duplicate Agent output paths; only successful Agent
outputs enter it. Domain identity remains an independent sorted workflow/prompt/principle file set
and hashes the same raw bytes. Error text, serde DTOs, public method paths, STS state ownership, and
Human decisions are not intentionally changed.

The STS scene-map `screenplay.rs` facade now keeps `parse_scenes`, `ParsedScene` state composition,
and its existing `valid_scn_id` path for the sibling blueprint parser. Canonical SCN heading/setting/
time normalization lives in `screenplay/heading.rs`; inline and indented dialogue, cue, and
parenthetical recognition live in `screenplay/dialogue.rs`. The parser still projects syntax and does
not infer story meaning. Scene order, scene DTO assembly, and source occurrence generation stay in
their previous STS owners.

The tools-side SceneMap v2 fixture checker keeps `scene_map_v2_validator.py` and all names imported by
the CLI facade. Bounded values/contracts, integer-only JCS identity, field/schema-shape checks, and
whole-map occurrence/hash orchestration now have separate source files. These helpers remain fixture
validation only and do not become an STS or cross-component authority.

Delivery's planner now delegates supported-request capability checks and `FULL`/`SELECTION`/`EXCERPT`
scene selection to `planner/plan/request_policy.rs`; `build_plan` remains the existing composition and
public entrypoint. The static source guard reads the new owner file so its existing assertions still
follow the policy implementation. This refactor and source guard update remain untested per instruction.

The boundary follows Rust's standard module-file mapping and parent/descendant privacy rules while
keeping the existing Engine methods at their old owner path: [Rust module file organization](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html),
[Rust Reference: visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html).
Per the current user instruction, tests, build, fmt/Clippy, CLI/runtime, and manifest verification are
**NOT_RUN** for this slice. The NVC producer manifest is regenerated as metadata only after source and
owner documentation updates; generation is not verification.


## 2026-09-24 final current-source revalidation — filesystem/render/inference splits

The checkout has no `.git` metadata. Validation ran against the copy-on-write snapshot
`/private/tmp/storyproduction-real-use-post-splits-20260924/source`. After syncing the latest source,
the workspace and snapshot contained 6,082 matching non-generated files with no missing, extra, or
hash-mismatched paths. Host: macOS arm64 27.0, Rust 1.97.1, Node 24.19.0, Python 3.12.14 with
`jsonschema==4.26.0`. No dependencies were installed. Test/build output stayed in the isolated copy
and `/private/tmp` workspaces. The source inventory comparison is
[recorded here](</private/tmp/storyproduction-real-use-post-splits-20260924/source-snapshot-compare.json>).

| Boundary | Result | Current evidence |
|---|---|---|
| Root cross-component deterministic gate | **PASS / TEST** | `bash integration/scripts/verify-local.sh`; final log: [root gate](</private/tmp/storyproduction-real-use-post-splits-20260924/root-gate-final-exact-source.log>) |
| geultu | **PASS / TEST+RUNTIME** | `./scripts/verify.sh`: 146 Rust tests, fmt/Clippy/release, CLI/MCP smoke, authenticated isolated Studio loopback and cleanup. [log](</private/tmp/storyproduction-real-use-post-splits-20260924/geultu-gate-post-splits.log>) |
| NarrativeCourier | **PASS / TEST+RUNTIME** | `./verify-all.sh --require-rust`: 60 Rust tests, check/fmt/Clippy; 480-entry source manifest SHA-256 `1259c5d535992e8971a26fe32a4869b556318003afdf99873fd1465c54db37dd`. Live stdio MCP: 10 tools; verified synopsis and screenplay import/export; delivery remained `NEEDS_REVIEW`; invalid input had no side effects; idle cancel errored explicitly; no approval tools. [Rust gate](</private/tmp/storyproduction-real-use-post-splits-20260924/nvc-gate-post-splits.log>) · [MCP process](</private/tmp/storyproduction-real-use-post-splits-20260924/nvc-mcp-stdio-post-splits-final.log>) |
| SceneMotionCompiler JS and custody | **PASS / TEST+LINT** | Node suite 227/227; custody `cargo fmt --check`, Clippy `-D warnings`, and 32 Rust tests (7 unit + 25 protocol). Current 240-file source tree hash `sha256:8b5b61f0b06e3b16a8d97a790404fb08a6abbdb8c1930af40378ad7b00cd9b2b`. [Node suite](</private/tmp/storyproduction-real-use-post-splits-20260924/smc-node-final-current.log>) · [Rust test](</private/tmp/storyproduction-real-use-post-splits-20260924/smc-cargo-test-final.log>) · [manifest verification](</private/tmp/storyproduction-real-use-post-splits-20260924/smc-manifest-verify-final.log>) |
| SceneMotionCompiler aggregate/package gate | **NOT_RUN / ENVIRONMENT** | `npm` is absent; direct Node, Rust, and manifest checks do not qualify the `npm run` aggregate or clean package install. Signed helper admission, locked Chromium, and Studio Human release remain NOT_RUN. |
| SceneAssetCompiler | **PASS / PARTIAL** | Publication-boundary tests 14/14; skill/docs/release manifests verify at 130/100/252 entries. The current `verify_release.py` stops at **FAIL / CLEAN_SCAN** because `tests/__pycache__` and `.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/__pycache__` already exist in the shared workspace; they were left untouched under its cleanup ownership rule. A prior cache-clean isolated gate on the same code graph reached G09 and found missing `tests/fixtures/legacy-psd/PROVENANCE.json`. No provenance was fabricated. [publication tests](</private/tmp/storyproduction-real-use-post-splits-20260924/sac-publication-current.log>) · [current release scan](</private/tmp/storyproduction-real-use-post-splits-20260924/sac-release-current.log>) |
| ImagePromptCompiler | **PARTIAL / TEST+RUNTIME** | Current graph passes 263 workspace tests, doc tests, fmt/Clippy, documentation build, and release build. Official `scripts/verify.sh` stops after those checks at absent `evals/dogfood/historical/1.4.0/image/image-evidence.json`; no evidence or `VERIFICATION.json` receipt was created. CLI and loopback Web smokes, source-snapshot tests, 73 doc links, six catalog assets, six representative QA cases, Rust structure, and catalog checks pass. Seven representative prompt-only JSON outputs are byte-identical to the preceding verified binary. [official gate](</private/tmp/storyproduction-real-use-post-splits-20260924/ipc-gate-post-format.log>) · [CLI smoke](</private/tmp/storyproduction-real-use-post-splits-20260924/ipc-cli-smoke.log>) · [Web smoke](</private/tmp/storyproduction-real-use-post-splits-20260924/ipc-web-smoke.log>) · [output parity](</private/tmp/storyproduction-real-use-post-splits-20260924/ipc-prompt-parity.log>) |
| Public owner-chain | **PASS / RUNTIME FIXTURE** | Current geultu CLI → NVC CLI → SMC CLI → SAC CLI → SMC CLI; 19 STS tasks, 10 simulated Human gates, 2 scenes, 5 shots, 10 asset requirements, 14,323-byte HTML. [run](</private/tmp/storyproduction-real-use-post-splits-20260924/owner-chain-final-exact-source.log>) |
| `storyctl text` | **PASS / RUNTIME FIXTURE** | 80 public CLI calls; nine packaged artifacts; every manifest hash matched and `source.md` was byte-identical to intake. [run summary](</private/tmp/storyproduction-real-use-post-splits-20260924/text-authoring-final-exact-source.log>) · [package manifest](</private/tmp/storyproduction-real-use-hcwd_48p/project/package/story-package.json>) |
| Synthetic story fidelity | **FAIL / DIRECT ARTIFACT REVIEW** | The screenplay omits the shift-sheet/original-envelope comparison, record correction, and solo departure. The rooftop storyboard repeats record-room framing and sound. The package status `complete` reflects hashes/files and simulated review, not story quality. [screenplay](</private/tmp/storyproduction-real-use-hcwd_48p/project/package/screenplay.md>) · [storyboard](</private/tmp/storyproduction-real-use-hcwd_48p/project/package/storyboard.md>) · [source](</private/tmp/storyproduction-real-use-hcwd_48p/project/package/source.md>) |
| Real user story, genuine Human review, external providers | **NOT_RUN / INPUT+HUMAN+EXTERNAL** | No user manuscript was supplied. Fixture writer/reviewer and all approval gates were simulated. |

### Corrections made after the gates exposed issues

The custody extraction had unused facade exports/imports and relied on `main.rs` test imports; these
were removed or moved into `tests.rs`, then formatted and rechecked. IPC compilation exposed nested
module paths that did not resolve from explicit `#[path]` parents and a missing moved `quote` helper;
the facades now name the nested owners explicitly and the child imports the existing helper. We also
removed obsolete private subject re-exports/helper and an IPC component document link that escaped its
repository root. The current-source IPC suite, docs, release build, CLI/Web smoke, and prompt parity
all passed after these changes. Producer-owned SMC, NVC, and SAC manifests were regenerated and
verified; hashes and generation records are stored at `/private/tmp/storyproduction-real-use-post-splits-20260924/`.

The fixture confirms artifact integrity and ownership boundaries, but it also reproduces the open
creative gap: screenplay/storyboard candidates can be structurally complete while losing essential
source actions. The documented 18-stage/144-task catalog is still not connected to runtime. Actual
user authorship, real Human approval, external writing/image/video execution, signed helper lifecycle,
locked Chromium, and Studio release remain unqualified. Overall verification execution is **DONE**;
whole-product qualification is **PARTIAL** and fixture fidelity is **FAIL**.


## 2026-09-24 NVC post-split build findings and source-only repair

A later current-source attempt ran the root deterministic gate successfully, then stopped in the NVC
Rust check. `screenplay/heading.rs::valid_scn_id` was visible only to its parent while the screenplay
facade re-exported it to the sibling blueprint module. The Delivery planner also retained an unused
`Scene` import. The same attempt found rustfmt differences and a stale producer source manifest: 493
source files were present, while 490 were listed; the changed `state.rs` hash differed and the three
new `state/{config,inputs,source_snapshot}.rs` owners were absent.

The source now widens that helper only to `crate::scene_map`, removes the unused planner import, and
incorporates the reported rustfmt layout. STS state ownership is documented as facade, config/path,
source snapshot, and durable-input verification modules. Agent task packet materialization and
token-bound submission/history were also separated below the existing workflow facade. The NVC
producer manifest was regenerated for the current source graph (metadata generation only; verification
was not run after the later task split and documentation edits). No build, test, lint, CLI, or runtime
command was run after these source changes. Current-source public authoring and owner-chain
E2E therefore remain **NOT_RUN**; the previous pre-split fixture results do not qualify this graph.

The earlier attempt and its captured output are summarized in the
[isolated diagnostic report](</private/tmp/storyproduction-current-live-verify-20260924-v2/report.md>);
the raw [NVC build log](</private/tmp/storyproduction-current-live-verify-20260924-v2/nvc-gate.log>),
[manifest comparison](</private/tmp/storyproduction-current-live-verify-20260924-v2/nvc-manifest-verify.log>),
and [format report](</private/tmp/storyproduction-current-live-verify-20260924-v2/nvc-fmt.log>) remain available.


## 2026-09-24 NVC semantic-manifest entry owner — source-only continuation

The SceneMap v2 contract validator now delegates entry-local semantic ID/kind grammar, hash-string
shape, and source-locator bounds to validation/semantic_manifest/entries.rs. The existing
semantic_manifest.rs remains responsible for cross-entry identity, scene occurrence binding,
bijection/contiguity, and manifest-hash ordering. DTOs, schemas, public validator path, and authority
remain in the same owner. This source change and the corresponding source-manifest generation were
not followed by build, tests, lint, CLI, runtime, or manifest verification.

## 2026-09-24 SAC PSD reader and policy owners — source-only continuation

SceneAssetCompiler retains its existing psd module path as the compatibility facade and bounded
binary reader. PSD path/generated-output acceptance functions now live in psd_validation.py and
resolve parse_psd at call time. Existing CLI and mechanical-review imports remain on psd.py. The
producer manifests were generated for 131 skill, 100 docs, and 253 release files (generation only).
No tests, build, lint, CLI, runtime, or manifest verification followed this edit.


## 2026-09-24 NVC reviewed-synopsis ingress owner — source-only continuation

The stable Engine public method for geultu handoff import now lives under
project/creation/reviewed_synopsis.rs, separated from generic NewProject staging/persistence. It
retains handoff identity, raw-byte synopsis hash, title and actor binding, path-root resolution, and
the existing exact-byte persistence helper. No schema, approval owner, or public method name was
intentionally changed. The NVC source manifest was regenerated as metadata; tests, build, lint, CLI,
runtime, and manifest verification remain NOT_RUN.


## 2026-09-24 DeliveryRequest contract validation owner — source-only continuation

The DeliveryRequest DTOs stay in request.rs, and the existing validator is re-exported at its original
request-module and crate-root paths. Cross-field validation now lives in request/validation.rs with
its existing checks and error order. This change does not move the producer DTOs or capability-policy
owner. NVC source-manifest metadata was regenerated; build, tests, lint, CLI, runtime, and verification
were not run after the edit.


## 2026-09-24 DirectorPlan validator owner — source-only continuation

DirectorPlan DTOs remain in director.rs and validate_director_plan remains re-exported at its existing
module and crate-root paths. DeliveryRequest-bound hashes, asset provenance, narrative references,
unit order, and timed-frame invariants moved to director/validation.rs without an intentional change
to validation order or error text. NVC source manifest metadata was regenerated; tests, build, lint,
CLI, runtime, and manifest verification were not run after this edit.


## 2026-09-24 geultu GenerationRegistry owners — source-only continuation

GenerationRegistry remains the compatibility facade and shared registration owner. Writing-adapter
selection, plan identity checks, and prompt bounds moved to generation/registry/writing.rs; external
provider selection, invocation, and draft bounds moved to generation/registry/provider.rs. Public
Application routes and method names remain in place. The previous 146-test/release/CLI/MCP/Studio
results predate this source edit; no checks ran afterward.


## 2026-09-24 geultu HumanDecision contract owner — source-only continuation

ValidationReceipt remains owned by contracts/receipts.rs, while the separate HumanDecision schema,
actor/scope validation, and RFC 3339 checks now live in contracts/human_decision.rs. The old
contracts::receipts::{HumanDecision, DecisionKind} re-export path remains. Lowercase-hex identity
validation is shared from contracts/validate.rs rather than being owned by receipt serialization.
The existing gate evidence predates this source move; no tests, build, lint, or runtime checks followed.


## 2026-09-24 IPC Codex artifact owners — source-only continuation

The Codex artifact facade retains its sibling import paths. Thread/call-bound PNG discovery and
identity now have a private discovery owner; path admission, bounded copy, PNG validation, and atomic
publication have a private publication owner. No provider or receipt contract changed intentionally.
The earlier source verification receipt was not refreshed, and no tests/build/lint/CLI/runtime/source-
snapshot work followed this edit.
