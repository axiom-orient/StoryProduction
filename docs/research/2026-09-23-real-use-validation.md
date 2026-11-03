# 텍스트 제작 경로 실사용 검증

일자: 2026-09-23 · 마지막 runtime 실행 판정: **TECHNICAL FLOW PASS / HUMAN QUALITY NOT_RUN**. 이 문서는 여러 시점의 실행을 보존한다. 가장 최신 runtime evidence는 `2026-09-23 최신 module layout 실사용 검증` 섹션이며, 그 아래의 SAC scene-release source-only 기록은 이 evidence 이후의 변경을 구분한다. 실제 사용자 원고의 의미·문체 품질, 실제 사람 승인, 외부 image/video provider 결과는 검증하지 않았다.

저장소 스냅샷에 `.git` 메타데이터가 없어 commit/revision ID는 확인할 수 없다.

과거 실행에서 확인한 status 및 macOS temp-path test 문제는 최신 소스 재검증에서
수정·재검증했다. ImagePromptCompiler의 별도 historical release-evidence gate는 이 텍스트
제작 경로의 의존성이 아니며, 전체 제품 qualification과 실제 provider 결과는 여전히 미판정이다.

## 2026-09-23 현재 소스 재검증

이번 재검증은 모듈 분리 이후의 현재 소스를 대상으로 했다. revision ID는 여전히 알 수 없다. Python 3.12.14, `jsonschema==4.26.0`, Node v24.19.0, pinned Rust/Cargo 1.97.1을 확인했고 Rustup/Cargo 환경은 `/private/tmp/storyproduction-current-verify-20260923/`에 두었다. 최신 NVC CLI는 `cargo build --manifest-path NarrativeCourier/Cargo.toml --package narrative-courier --bin narrative-courier-cli --locked --offline`로 빌드했다. 최종 재빌드는 미사용 import 정리 뒤 경고 없이 끝났다.

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| doctor | `storyctl doctor`; Python/jsonschema/Node/Cargo/rustc 필수 런타임 탐지 | **PASS / RUNTIME** |
| canonical local gate | `TMPDIR=/private/tmp ./integration/scripts/verify-local.sh`; integration 29, NVC contract audit 60, delivery audit 112, STS audit 14 검사와 후속 SMC/SAC/manifest/source-snapshot 검사 통과 | **PASS / TEST** |
| 기본 macOS 임시 경로 | `test_safe_path_rejects_symlink_and_traversal`; `/var/...`와 `Path.resolve()`의 `/private/var/...` alias가 달라 실패 | **FAIL / TEST ENVIRONMENT** |
| NVC Rust module build | pinned 1.97.1, locked/offline CLI build. 계약 모듈 분리를 포함한 최신 소스 컴파일 | **PASS / BUILD** |
| `storyctl text` actual CLI flow | 새 격리 fixture에서 create → brief → story 후보/검토 → synopsis 후보/검토 → hash-bound handoff → 19 STS agent tasks/10 simulated gates → storyboard prepare/submit/review → package | **PASS / RUNTIME FIXTURE** |
| package integrity | 9개 파일 SHA-256과 원문 bytes를 manifest/source에 대조 | **PASS / RUNTIME** |
| storyboard failure boundary | 잘못된 board hash와 승인 없는 package가 각각 exit 2로 거절되고 새 receipt/package 출력 없음 | **PASS / RUNTIME NEGATIVE** |
| `text status` 완료 표시 | package 발행 뒤에도 `completion=requires_storyboard_review`, NVC narrative는 `completed` | **FAIL / RUNTIME** |
| 전체 `storyctl e2e` fresh build | 실행은 geultu 빌드 단계에서 멈춤. 오프라인 vendor source에서 `serde_json_canonicalizer`를 찾지 못해 교차 제품 E2E 본문은 실행되지 않음. 텍스트 흐름은 기존 geultu binary와 freshly built NVC CLI를 사용 | **NOT_RUN / BUILD INPUT** |
| 실제 사용자 작품/사람 승인 | 사용자 원문이 없고 writer/reviewer는 fixture 프로그램과 simulated actor | **NOT_RUN / HUMAN INPUT** |

실사용 경로 첫 실행에서는 분리한 `storyboard_impl/publication.py`가 `seal()`을 import하지 않아 제출이 실패했다. Integration은 이를 성공으로 바꾸지 않고 해당 attempt를 `outcome_unknown`으로 남겼다. 누락 import를 연결하고 이 attempt를 재사용하지 않은 새 workspace에서 전체 흐름을 다시 실행해 package 완료를 확인했다. source audit의 이전 8개 실패는 검사기가 `contracts/src/lib.rs`만 읽는 구조 불일치였다. 검사기를 owner 모듈 전체에서 DTO 정의가 정확히 하나인지, crate root가 공개 재수출하는지 확인하도록 갱신한 뒤 canonical local gate가 통과했다. 이 변경은 검사 조건을 제거하지 않았다.

검증 로그는 [`verify-local.log`](../../../../../../private/tmp/storyproduction-current-verify-20260923/verify-local.log), 성공 fixture의 명령 기록은 [`commands.jsonl`](../../../../../../private/tmp/storyproduction-real-use-ietz5p_u/commands.jsonl), 완성 package는 [`story-package.json`](../../../../../../private/tmp/storyproduction-real-use-ietz5p_u/project/package/story-package.json)에 남아 있다. 실행 작업공간과 임시 toolchain은 재검토를 위해 `/private/tmp` 아래에 보존했다.

## 실행 환경과 보존 범위

| 항목 | 관측 |
|---|---|
| 검증 Python / 의존성 | Python 3.12.14, `jsonschema==4.26.0` |
| Node | v24.19.0 |
| Rust | repository pin `1.97.1`, aarch64-apple-darwin |
| 바이너리 | 실행 시 geultu와 NarrativeCourier CLI를 빌드해 public CLI로 호출 |
| 임시 설치 | Python venv, Rust toolchain, Cargo registry/cache를 `/tmp` 아래에만 준비. 사용자 전역 설정은 바꾸지 않음 |
| 원문 | `/private/tmp`의 격리 합성 fixture. 사용자 원문은 제공되지 않음 |

NVC lockfile의 의존 crate는 임시 Cargo home에 받아두고 공식 E2E 자체는 `--offline`로 실행했다. E2E 빌드 산출물은 각 제품의 기존 `target` 경로에 남아 있다. 임시 Python/Rust/Cargo 환경과 합성 프로젝트 workspace는 검증 후 정리했다.

## 검증 결과

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| doctor | 격리 Python·Node·Rust 경로로 `storyctl doctor`; Python 3.12, jsonschema 4.26, Node v24.19, Cargo/rustc 1.97.1 모두 탐지 | **PASS / RUNTIME** |
| 공식 로컬 검증, 기본 macOS 임시 경로 | `./integration/scripts/verify-local.sh`; 29개 Python test 중 `test_safe_path_rejects_symlink_and_traversal` 1개 실패. 구현은 `/var/...` 임시 root를 `/private/var/...`로 canonicalize하지만 테스트는 lexical 경로를 비교 | **FAIL / TEST ENVIRONMENT** |
| 해당 경계 테스트 진단 | 같은 테스트를 `TMPDIR=/private/tmp`로 실행하면 통과. 이 경로에서는 `verify-local.sh` 전체도 `StoryProduction local deterministic verification: PASS`로 종료 | **PASS / TEST**, 단 기본 임시 경로 실패도 함께 유지 기록 |
| 공식 통합 E2E | `storyctl e2e`; 실제 geultu→NarrativeCourier→SMC→SceneAssetCompiler→SMC public CLI. 19개 STS 작업, 10개 simulated human gate, 두 씬, 5개 shot, 10개 asset requirement, 14,323-byte HTML 산출 | **PASS / RUNTIME FIXTURE** |
| 새 `storyctl text` 경로 | 합성 원문 `create → brief → story candidate/review → synopsis candidate/review → export → hash-bound handoff → NVC import → STS → storyboard prepare/submit/review → package` 완주 | **PASS / RUNTIME FIXTURE** |
| 문서 패키지 | `story-package.json`이 9개 파일을 기록하고, 각 파일의 SHA-256이 marker와 일치 | **PASS / RUNTIME** |
| 완료 상태 보고 | 승인 콘티 검토와 `status=complete` package 발행 뒤 `storyctl text status`가 `completion=requires_storyboard_review` 반환 | **FAIL / RUNTIME** |
| 실제 작품 품질 | 원문 없음. STS 단계는 기존 E2E의 합성 agent artifact와 `SIMULATED_HUMAN_FOR_TEMP_VALIDATION` 게이트를 사용 | **NOT_RUN / HUMAN INPUT** |

텍스트 workflow에 사용한 원문과 story/synopsis는 synthetic이다. STS artifact는 NVC 경로·게이트·release 동작을 확인하는 fixture라 앞서 작성한 story와 의미상 일치하는 작품 평가 자료가 아니다. 콘티·후보 검토도 임시 fixture 전용 simulated actor로만 수행했다. 따라서 이 결과는 process/wiring/hash/gate/package 경로의 실행 증거이며, 실제 작가 승인·서사 충실도·문체·콘텐츠 품질 증거가 아니다.

## 발견된 결함

`Production.status()` (`../../integration/src/story_integration/text_workflow.py`, 당시 경로)는 NVC `agent-task`의 `completed` 값만 보고 `requires_storyboard_review`를 반환한다. 이미 승인된 storyboard receipt와 완료 package를 확인하지 않아 사용자에게 오래된 다음 작업을 표시한다. 코드 변경은 이 검증에서 수행하지 않았다. 이 수정은 NVC의 public storyboard entrypoint를 통한 최신 full-board review 확인을 거쳐야 한다.

기본 `verify-local.sh` 실패는 path boundary 테스트 (`../../integration/tests/test_story_integration.py`, 당시 경로)가 macOS `/var` alias를 보존한 `TemporaryDirectory` 결과와 `Path.resolve()`의 canonical 반환값을 직접 같다고 비교해 발생했다. 구현 경계 검사는 통과했지만 기본 환경에서 suite가 실패한다. 이 보고에서는 `TMPDIR` 우회 통과를 기본 명령의 PASS로 합치지 않았다.

## 다음 검증

1. `text status`가 current synopsis와 현재 source에 결속된 full approved board를 public NVC CLI에서 확인하도록 수정하고, 패키지 준비 상태를 반환하는지 재실행한다.
2. path 경계 테스트를 macOS canonical temp root와 비교하도록 수정하거나 테스트 temp 경로를 명시해 기본 `verify-local.sh`가 통과하는지 확인한다.
3. 사용자가 지정한 실제 원문으로 story·synopsis·STS·storyboard 후보를 직접 검토한다. synthetic fixture의 simulated approval은 이 확인을 대체하지 않는다.

## 2026-09-23 모듈 분리 후 재검증

이번 재실행도 저장소의 실제 사용자 원문이 없어 `/private/tmp`의 synthetic story와 격리 workspace를 사용했다. Python 3.12.14, `jsonschema==4.26.0`, Node v24.19.0, repository-pinned Rust/Cargo 1.97.1을 `/tmp` 아래에 준비했다. `storyctl doctor`는 PASS했다. 저장소에 `.git` 메타데이터가 없어 revision ID는 없다.

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| 기본 경로 deterministic 검증 | `./integration/scripts/verify-local.sh`; 29개 integration Python test에서 경로 test 하나만 실패. `/var/...`와 `/private/var/...`의 canonical alias 비교 | **FAIL / TEST ENVIRONMENT** |
| canonical 경로 deterministic 검증 | 같은 스크립트를 `TMPDIR=/private/tmp`로 실행. contract registry, integration/NVC tests, SMC Node checks, SceneAssetCompiler skill/source manifests 모두 통과 | **PASS / TEST** |
| geultu→NVC→SMC→SAC E2E | `python3 integration/tests/story_pipeline_e2e.py`; 공개 CLI로 19개 STS 작업, 10개 simulated gate, 2 scenes, 5 shots, 10 asset requirements, 14,323-byte HTML 생성 | **PASS / RUNTIME FIXTURE** |
| `storyctl text` writer 경로 | 격리 원문 → brief → 두 번의 `text run` fixture process → story/synopsis candidate review → hash-bound handoff/import → 19 STS tasks/10 simulated gates → storyboard prepare/submit/review → package | **PASS / RUNTIME FIXTURE** |
| recovery | 성공 응답을 `text recover`로 다시 저장. geultu가 같은 candidate를 `idempotent=true`로 확인했고 중복 revision은 만들지 않음 | **PASS / RUNTIME FIXTURE** |
| 최종 package integrity | `story-package.json`의 9개 파일 SHA-256을 원본 bytes와 대조 | **PASS / RUNTIME** |
| package 뒤 완료 상태 | full board review와 `status=complete` package 뒤 `text status`가 `completion=requires_storyboard_review`, NVC 상태는 `completed` 반환 | **FAIL / RUNTIME** |
| 실제 사용자 작품 품질 | 사용자 원고·실제 작가 review가 제공되지 않음. fixture writer와 simulated reviewer만 사용 | **NOT_RUN / HUMAN INPUT** |

재검증 중 새 모듈 분리 경계에서 `text_cli`의 `regular`/`Path`, `text_workflow`의 `sys`, PromptGen facade의 `shutil` 누락도 확인해 import를 복구했다. 그 뒤 canonical `verify-local.sh`와 text workflow를 다시 실행해 통과를 확인했다. 이 수정은 생성된 Rust build artifacts 이외의 user workspace를 변경하지 않았다.

최종 결과는 **PARTIAL**이다. 기술 흐름과 hash/gate/package 검증은 fixture 범위에서 통과하지만, `Production.status()`는 storyboard receipt와 package를 확인하지 않아 완료 후 오래된 작업 상태를 보여 준다. 이를 고치려면 NarrativeCourier의 공개 storyboard bundle 경계에서 현재 source-bound full approval을 다시 검증하는 동작이 필요하다. 실제 원문에서의 의미 보존·문체 적합성, 실제 writer/provider 품질 및 사람 검토는 fixture로 대체하지 않았다.

## ImagePromptCompiler Rust 모듈 분리 검증

격리 환경은 Python 3.12.14 / `jsonschema==4.26.0`, Node v24.19.0, pinned Rust/Cargo 1.97.1이었다. `CARGO_TARGET_DIR`과 Cargo cache는 `/private/tmp`에 두었다. Rust compile에서 path-module 위치, crate 내부 재export 범위, 나뉜 파일의 import 누락을 발견해 각 owner/facade의 내부 연결을 복구했다. `cargo fmt --all`로 공식 formatting gate도 맞췄다.

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| workspace compile | `cargo check --workspace --all-targets --locked --offline` | **PASS / BUILD** |
| 공식 tests | `verify.sh` 내 workspace/all-targets tests, doc-tests; CLI 21, Codex 41, core 144, MCP 9, OpenAI 12, Web 36 tests | **PASS / TEST** |
| quality/docs/build | `verify-quality.sh`의 fmt+clippy, rustdoc, release build | **PASS / BUILD** |
| CLI/Web 실사용 smoke | release CLI의 prompt-only example/catalog/schema 출력과 loopback Web health/catalog/interview flow. 모델 provider 호출 없이 실행 | **PASS / RUNTIME FIXTURE** |
| 독립 후속 audits | 문서 링크 71개, catalog PNG 6개·13,205,000 bytes, representative QA 6 cases/20 criteria, renewal/prompts, Rust structure, source snapshot, Python/shell/JS syntax | **PASS / STATIC+TEST** |
| 전체 공식 `scripts/verify.sh` | release build 뒤 `audit-image-evidence.py`가 `evals/dogfood/historical/1.4.0/image/image-evidence.json` 부재로 중단. checkout에 `ImagePromptCompiler/evals/` 자체가 없음 | **FAIL / MISSING INPUT** |
| verification receipt | 공식 gate의 마지막 `source_snapshot.py --write-verification`에 도달하지 않아 `VERIFICATION.json`을 쓰지 않음 | **NOT_RUN / GATE STOPPED** |
| 실제 이미지 생성·품질 | fake executable 테스트만 실행; 인증된 Codex/OpenAI provider 호출과 결과물 평가는 없음 | **NOT_RUN / EXTERNAL PROVIDER** |

누락된 historical evidence는 1.4.0 후보에 대한 보존 기록이며, 현재 run에서 새 이미지나 review를 만들어 대체하지 않았다. 따라서 compile/tests와 분리된 smoke는 통과했지만 subproject의 release receipt는 발급하지 않았고 전체 ImagePromptCompiler gate는 PARTIAL로 남긴다. 검증 workspace와 build output은 `/private/tmp`의 격리 경로에 두었고 Rust build target도 source tree 밖에 유지했다.

## 2026-09-23 STS·Delivery 모듈 분리 후 이전 실사용 재검증

최신 STS/Delivery/render 모듈 분리 소스를 pinned Rust/Cargo 1.97.1로 다시 빌드한 뒤, 새 격리 프로젝트에서 현재 public CLI를 실행했다. `.git` 메타데이터가 없어 revision ID는 확인할 수 없다. Python 3.12.14와 저장소 pin `jsonschema==4.26.0`은 `/private/tmp/storyproduction-current-verify-20260923/text-venv`에, Cargo/Rust 환경은 `/private/tmp/storyproduction-current-verify-20260923/`에 두었다.

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| 런타임 진단 | `storyctl doctor`; Python 3.12.14, jsonschema 4.26.0, Node v24.19.0, Cargo/rustc 1.97.1 필수 항목 탐지 | **PASS / RUNTIME** |
| 최신 NVC CLI build | `cargo build --manifest-path NarrativeCourier/Cargo.toml --package narrative-courier --bin narrative-courier-cli --locked --offline` | **PASS / BUILD** |
| STS·Delivery focused tests | `sts-workflow --lib` 1개, `narrative-courier-delivery --lib` 6개, `--test vertical` 1개 | **PASS / TEST, 8/8** |
| 실제 `storyctl text` 공개 흐름 | 새 합성 프로젝트에서 create → brief → story/synopsis 후보와 검토 → synopsis hash-bound handoff → STS 작업과 simulated gate → storyboard prepare/submit/current-hash review → package. 총 80개 CLI 명령 모두 exit 0 | **PASS / RUNTIME FIXTURE** |
| package 및 producer chain | complete package의 9개 파일, narrative package/release bundle/scene map의 상위 해시까지 총 12개 SHA-256 일치 | **PASS / RUNTIME** |
| NVC Delivery 공개 CLI | `deliver-context` → `deliver` → `deliver-list`; 같은 STS release에서 HTML과 JSON 생성·저장, 두 artifact hash 확인. HTML은 `NEEDS_REVIEW`, JSON은 `PASS` | **PASS / RUNTIME FIXTURE** |
| 완료 상태 표시 | 승인된 현재 storyboard와 `status=complete` package 뒤 `text status`가 계속 `completion=requires_storyboard_review` 반환. NVC narrative는 `completed` | **FAIL / RUNTIME** |
| 실제 원문·사람 승인·외부 미디어 renderer | 사용자 원고와 실제 review가 없고 writer/reviewer는 합성 fixture. PDF/raster/video renderer도 이번 실행에서 호출하지 않음 | **NOT_RUN / HUMAN·EXTERNAL** |

잘못된 fixture 환경변수 또는 과거 실행의 고정 storyboard hash를 사용한 별도 시도는 각각 `outcome_unknown`/`REVIEWED_CONTENT_CHANGED` 경계를 반환했다. 이 attempt와 workspace는 재사용하지 않았다. 성공 흐름은 새 workspace에서 fixture root와 현재 `contentHash`를 전달해 처음부터 실행했으며 실패 시도를 PASS로 합산하지 않았다.

재현 기록은 [80개 명령 로그](../../../../../../private/tmp/storyproduction-real-use-current-a0w_2jmn/commands.jsonl), [완료 package manifest](../../../../../../private/tmp/storyproduction-real-use-current-a0w_2jmn/project/package/story-package.json), [DeliveryRequest](../../../../../../private/tmp/storyproduction-real-use-current-a0w_2jmn/delivery-request.json), [persisted DeliverySet](../../../../../../private/tmp/storyproduction-real-use-current-a0w_2jmn/project/narrative/realuse-check/visual/requests/REALUSE-DELIVERY-001/delivery-set.json)에 남아 있다. package와 request workspace 및 toolchain은 `/private/tmp`에 두었다.

그 실행 시점의 결과는 **PARTIAL**이었다. STS release 이후 문서 package와 기존 deterministic HTML/JSON Delivery까지 공개 CLI에서 생성하고 해시를 확인했지만, `Production.status()`가 승인 receipt/package를 확인하지 않는 완료 상태 결함이 남아 있었다. 아래 최신 소스 재검증에서 이 상태 결함과 기본 임시 경로 검사를 고쳤다. 사람의 작품 승인, 서사·문체 품질, 외부 PDF/image/video renderer는 synthetic runtime 증거로 대체하지 않는다.

## 2026-09-23 최신 소스 재검증 — 현재 판정

이번 재검증은 위 과거 결과에서 드러난 모듈 연결·완료 상태·macOS temp 비교 문제를
수정한 뒤 수행했다. 저장소에는 `.git` 메타데이터가 없어 commit ID는 확인할 수 없다.
Python 3.12.14/jsonschema 4.26.0, Node v24.19.0, Rust/Cargo 1.97.1을 사용했다.
Cargo cache와 geultu/NVC build target은 `/private/tmp/storyproduction-current-verify-20260923/`
아래에 두어 repository `target`을 재사용하거나 덮어쓰지 않았다.

이번에 고친 항목은 다음과 같다.

- STS scene-map 분리 중 남은 `valid_scn_id` 중복 정의와 NarrativePackage atomic-target helper import를 바로잡았다.
- CLI 파일 모듈의 Rust 경로를 명시해 새 `commands`, `delivery`, `sts` 하위 모듈을 현재 bin target에서 찾도록 했다. 분리된 CLI/runtime 파일의 unused import도 제거했다.
- `verify_contract.py`가 façade의 wildcard import 문자열 대신 STS owner module 전체의 명시적 Runtime primitive 사용을 확인하도록 고쳤다.
- `text status`는 해시가 일치하는 board/review 경로를 후보로만 고른 뒤 NVC 공개 `storyboard bundle` entrypoint가 현재 synopsis source와 full approval을 검증한 경우에만 `complete`를 반환한다. 승인·scene state를 integration에 복제하지 않는다.
- path 경계 테스트가 macOS의 `/tmp` alias와 canonical `/private/tmp`를 비교하지 않도록 fixture root를 canonicalize했다. 경계 구현은 변경하지 않았다.

| 기준 | 현재 소스 실행 결과 | 판정 |
|---|---|---|
| 환경 doctor | Python/jsonschema, Node, Cargo/rustc 필수 항목 확인 | **PASS / RUNTIME** |
| geultu CLI build 및 library tests | isolated pinned target build; **15/15** tests | **PASS / BUILD+TEST** |
| NarrativeCourier CLI build | locked, pinned current source; warnings 없이 build 완료 | **PASS / BUILD** |
| NVC focused Rust tests | STS lib **1**, Delivery lib **6**, Delivery vertical **1** | **PASS / TEST, 8/8** |
| StoryProduction local gate | 기본 macOS temp 환경의 `./integration/scripts/verify-local.sh`; integration **29**, NVC contract audit **60**, delivery audit **112**, STS audit **14**, SMC Node suites, SAC manifests, IPC source snapshot 통과 | **PASS / TEST** |
| 최신 소스 text flow | 새 격리 workspace에서 intake → brief → story/synopsis candidate/review → hash-bound handoff → 19 STS tasks/10 simulated gates → storyboard submit/review → package. 전체 **80개 flow CLI 호출** exit 0 | **PASS / RUNTIME FIXTURE** |
| package 및 원문 | 9개 package artifact의 SHA-256 전부 일치, package `source.md` bytes가 intake와 일치 | **PASS / RUNTIME** |
| 완료 상태 수정 | 81번째 post-fix `text status`에서 current source true, exact approved board hash 반환, `completion=complete` | **PASS / RUNTIME** |
| geultu→NVC→SMC→SceneAssetCompiler→SMC | 현재 격리 build 바이너리와 public CLI. 19 STS tasks, 10 simulated gates, 2 scenes, 5 shots, 10 asset requirements, 14,323-byte HTML | **PASS / RUNTIME FIXTURE** |
| 실제 사용자 원고·사람 승인·실제 생성 provider | 사용자 원고가 없어 고정 synthetic writer와 simulated reviewer 사용; 이미지/video 생성 및 사용자 승인 없음 | **NOT_RUN / HUMAN·EXTERNAL** |

최초 `storyctl e2e` 실행은 격리 Cargo cache에 `serde_json_canonicalizer`가 없어 offline build 단계에서 멈췄다. 잠긴 의존성을 임시 cache에 받고 두 현재 소스를 격리 target에 빌드한 뒤, 같은 E2E 본문이 그 바이너리를 사용하도록 호출해 public CLI 경계 전체를 실행했다. 따라서 저장소의 오래된 `target` 바이너리는 현재 source runtime 증거로 사용하지 않았다.

텍스트 실행 기록은 [80개 flow 호출과 post-fix status](../../../../../../private/tmp/storyproduction-authoring-current-hc3ltose/commands.jsonl), [완료 package manifest](../../../../../../private/tmp/storyproduction-authoring-current-hc3ltose/project/package/story-package.json)에 남아 있다. 최신 상태 응답도 같은 log의 마지막 row에 저장했다.

현재 결론은 **합성 fixture 기술 흐름 PASS / 작품 품질·사람 승인 NOT_RUN**이다. `text status`의 오래된 완료 표시와 기본 임시 경로 검사는 해결됐다. 실제 원고의 의미·문체 평가, 실제 writer/provider 품질, SMC native custody와 전체 ImagePromptCompiler release qualification은 이 검증에 포함되지 않았다.

## 검증 이후 구조 개선 범위

위 실행 증거 이후 `narrative-courier-cli` command modules를 Rust 표준 `src/bin/commands/`
tree로 옮겼고, integration의 storyboard adapter/status discovery를 `text_storyboard.py`로
분리했으며, SAC 단일 `asset_release.py`를 contracts/files/projection/release owners로,
SMC custody root를 session/lock-release/process-identity/artifact/filesystem/protocol modules로,
application Track B binding/candidate logic을 별도 modules로 나눴다. SAC와 SMC source manifests는
generator로만 갱신했다. 이 후속 구조 변경에는
당시에는 build/test/lint/verify가 실행되지 않았다. 아래는 사용자의 후속 요청에 따라 현재
module layout에서 새로 수행한 검증 결과다.

## 2026-09-23 후속 module split 이후 현재 소스 검증

검증 대상은 `/Users/ax/repository/StoryProduction`의 현재 파일 트리다. 이 폴더에는 `.git`
metadata가 없어 commit/revision ID는 없다. Python 3.12.14과 `jsonschema==4.26.0`, Node
v24.19.0, Darwin/arm64 Rust/Cargo 1.97.1을 사용했다. Cargo target/cache와 Python 환경은
`/private/tmp/storyproduction-current-verify-20260923/` 아래에 두었다.

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| 기본 macOS 임시 경로 local gate | `./integration/scripts/verify-local.sh`; integration Python 29개, NVC source/contract audits, SMC boundary suites, SAC manifests, IPC source snapshot 포함 | **PASS / TEST** |
| NVC module-split CLI build | `cargo build --manifest-path Cargo.toml --package narrative-courier --bin narrative-courier-cli --locked --offline`, 격리 `CARGO_TARGET_DIR` | **PASS / BUILD** |
| SMC application/contract/runtime | `node --test tests/*.test.js`; application binding/candidate/runtime, negative cases, custody/Studio/measurement suites 포함 | **PASS / TEST, 227/227** |
| SMC source manifest | `node scripts/source-manifest.mjs --out SOURCE_MANIFEST.json`, 이어서 `node scripts/verify-source-tree.mjs`; 174 files, mismatches 0 | **PASS / MANIFEST** |
| SMC native custody crate | `cargo test --manifest-path Cargo.toml --package smc-trackb-custody --locked`; 7 unit + 25 protocol tests | **PASS / TEST, 32/32** |
| SAC scene-release boundaries | `python3 -m unittest SceneAssetCompiler.tests.test_publication_boundaries`; 14 filesystem/approval boundary tests | **PASS / TEST, 14/14** |
| owner-boundary public CLI E2E | `python3 integration/tests/story_pipeline_e2e.py`; 현재 geultu/NVC CLI build 후 19 STS task, 10 simulated gates, 2 scenes, 5 shots, 10 asset requirements, 14,323-byte HTML | **PASS / RUNTIME FIXTURE** |
| `storyctl text` authoring/package | 새 격리 workspace에서 81 public CLI calls: 원문·brief·story/synopsis 후보와 review·hash-bound handoff·STS·storyboard·package·최종 status | **PASS / RUNTIME FIXTURE** |
| package integrity/completion | 9개 산출물 hash 모두 marker와 일치, `source.md` bytes가 입력과 동일, 승인된 full board와 current source에 `completion=complete` | **PASS / RUNTIME** |
| 사용자 원고와 실제 창작 판단 | 실제 원고가 없어서 fixture writer와 simulated reviewer만 사용 | **NOT_RUN / HUMAN INPUT** |
| 실제 외부 writer/image/video provider, locked Chromium·Studio helper e2e, SMC aggregate release gate | provider 호출·실제 사람 승인·전체 release qualification은 실행하지 않음 | **NOT_RUN / EXTERNAL OR FULL GATE** |

첫 local-gate 실행에서 `trackb-release-candidate.js`가 artifact validator를 소유하지 않는
`trackb-contract.js`에서 가져오려는 잘못된 import를 발견했다. validator의 실제 owner는 기존
`trackb-artifact-schemas.js`여서 import를 바로잡고, SMC 전체 Node suite와 local gate를 다시
실행해 통과시켰다. 첫 SMC 전체 테스트 시도는 격리 환경에 package dependencies가 없어 중단됐고,
package-lock의 고정 dependency를 `/private/tmp`에만 설치한 뒤 재실행했다. Node suite는
`node --test`로 실행했으며 테스트용 `node_modules` symlink는 종료 시 제거했다.

첫 text-flow writer attempt는 fixture harness가 `STORYPRODUCTION_ROOT`를 설정하지 않아
시스템에 `outcome_unknown`으로 기록됐다. 해당 attempt/workspace는 재사용하지 않고, 새 workspace와
repo-root fixture 설정으로 81개 호출을 처음부터 실행했다. 성공 기록은 [명령 로그](</private/tmp/storyproduction-authoring-current-f9ptu2ta/commands.jsonl>),
[완료 package marker](</private/tmp/storyproduction-authoring-current-f9ptu2ta/project/package/story-package.json>),
[owner-boundary E2E 로그](</private/tmp/storyproduction-public-e2e-current.log>),
[최종 local-gate 로그](</private/tmp/storyproduction-verify-local-current-final.log>),
[SMC Node test 로그](</private/tmp/storyproduction-smc-tests-current-full.log>)에 남아 있다.

현재 판정은 **모듈 분리 후 합성 fixture 기술 흐름 PASS / 실제 작품 품질·사람 승인·외부 생성 NOT_RUN**이다.
NVC CLI build와 native custody crate tests는 통과했지만 NVC 전체 Rust tests/fmt/clippy,
SMC Node↔packaged native helper의 실제 application release cycle, locked Chromium,
full SMC aggregate/release gate는 이번 실행에 포함되지 않았다. 따라서 결과를 전체 제품 release
qualification이나 작품 품질의 승인으로 해석하지 않는다.

## 후속 SMC runtime/custody와 SAC Scene Provider protocol 분리 — source-only

이 검증 이후 1,470줄 `trackb-release-runtime.js`의 내부 책임을 나눴다. 네 개 command
entrypoint(`prepare`, `approve`, `promote`, `status`)는 facade에 두고, 공통 오류/값 helper,
schema registry load, custody-returned artifact bytes/parser, session/protocol validation,
Studio approval receipt binding, DeliverySet/pointer construction·current-base reconciliation을
각 internal module에 배치했다. 972줄 Node custody bridge의 fixed helper receipt/response
invariants도 `trackb-custody-protocol.js`로 분리하고 기존 `trackb-custody.js` entrypoint를
유지했다. SAC의 typed Scene Provider request/response/PNG wire invariants는
`scene_provider_protocol.py`로 분리하고 process/attempt effect owner는 그대로 `scene_provider.py`에
뒀다. Narrative/visual/human approval/durable custody/provider owner를 새로 합치지 않았다.

현재 지시를 따라 이 가장 최근 refactor에서는 build/test/lint/verify를 실행하지 않았다.
앞선 **227 Node tests PASS**와 **174-file SMC manifest PASS**는 이전 runtime layout의 증거이며
현재 runtime/custody/provider protocol module graph의 동작 증거가 아니다. SMC source manifest는
181 files / `sha256:cfef8571f5c3f1d61ba5726085c1c83d445bbe3bc74cff42ab61e3a110cd62b4`를 다시
기록했고 SAC docs/release/skill manifests도 각각 98/225/107 entries로 generator에서 갱신했다.
이는 생성 메타데이터일 뿐 component manifest verifiers, `verify:manifest`, `npm test`는
미실행으로 남긴다.

## 2026-09-23 후속 module split 실사용 재검증 — current source

이전 source-only 상태를 사용자 요청에 따라 다시 검증했다. checkout에 `.git`이 없어
commit/revision ID는 확인할 수 없다. 격리 Python 3.12.14/jsonschema 4.26.0, Node 24.19.0,
Rust/Cargo 1.97.1 환경을 사용했다. SMC npm package의 `test` script는 격리 package-lock
dependency 설치와 `pnpm test`로 실행했다. Rust 바이너리 재빌드는 하지 않았다. cross-product
E2E에서는 저장소의 geultu/NVC binary를 사용했고, 그보다 새로운 Rust source는 없음을 확인했다.

| 기준 | 현재 관측 | 판정 |
|---|---|---|
| StoryProduction deterministic local gate | `./integration/scripts/verify-local.sh`; 29 integration test, NVC source/contract audits, SMC suites, SAC skill/release manifests, IPC source snapshot을 포함해 종료 0 | **PASS / TEST** |
| SMC 전체 Node suite | `pnpm test` → `node --test tests/*.test.js`; 현재 runtime/custody module graph에서 227/227 | **PASS / TEST** |
| SMC source manifest | generator와 `verify-source-tree.mjs`; 181 files, `sha256:135cb3a6fb63776f75280ac26dfa25b4d5f8b77c8bb2c0bcf84582e32c2c23be`, mismatch 0 | **PASS / MANIFEST** |
| SAC 경계 suite·CLI import | `python -m unittest discover -s tests -v` 14/14 및 `scene_asset_compiler.py --help`; provider facade와 internal modules import | **PASS / TEST+RUNTIME** |
| SAC manifests | `tools/source_manifest.py verify`와 `verify_skill.py`; docs/release/skill 각각 98/227/109 files | **PASS / MANIFEST** |
| owner-boundary production E2E | 새 temporary run에서 geultu CLI → NVC CLI → SMC public CLI → SAC scene release CLI → SMC HTML CLI. NVC 19 agent nodes, 10 **simulated** human gates, 2 scenes, 5 shots, 10 asset requirements, HTML 14,323 bytes | **PASS / RUNTIME FIXTURE** |
| `storyctl text` 제작·패키지 | 새 workspace의 81 public CLI calls: source intake → brief → story/synopsis candidates/reviews → hash-bound handoff → 19 NVC nodes → text storyboard review → package/status. package 9개 파일 hash와 intake source bytes 일치, `completion=complete` | **PASS / RUNTIME FIXTURE** |
| Scene Provider/PerfectPixel 실행 | pinned `scene-provider-v2`·`perfectpixel` binary가 checkout에 없고 필요한 provider 환경변수가 모두 unset. generation/inspection 프로세스는 호출하지 않음 | **NOT_RUN / EXTERNAL** |
| 실제 작품 품질·사람 승인 | 사용자 원고가 없어 synthetic story와 fixture writer를 사용했다. storyboard reviewer와 NVC gates도 simulated actor이며 실제 저자/사람 승인이 아님 | **NOT_RUN / HUMAN** |

검증 과정의 module split 결함도 수정했다. SMC custody protocol 분리에서 native helper와 맞춰야 할
project-root bounds 및 safe ID/hash/target-triple 규칙이 빠져 있었고, facade가 사용하는
`hashTrackBAuthority`·`validateTrackBCommandResult` import도 누락돼 있었다. 이전 실패 결과는
PASS에 합치지 않았다. owner fixture 처음 재실행에서 이전 board hash가 고정된 review가
`REVIEWED_CONTENT_CHANGED`로 거부됐고 outer attempt는 `outcome_unknown`으로 기록됐다. 그
workspace를 재사용하지 않고 새 workspace에서 방금 작성된 board의 exact `contentHash`에
review fixture를 결속해 81개 호출을 처음부터 다시 통과시켰다.

현재 증거 로그와 산출물은 [SMC 227 tests](</private/tmp/storyproduction-smc-verify.xfng9j/smc-test-final.log>),
[최종 local gate](</private/tmp/storyproduction-smc-verify.xfng9j/verify-local-final-after-docs.log>),
[owner-boundary E2E](</private/tmp/storyproduction-smc-verify.xfng9j/story-pipeline-e2e.log>),
[81개 text CLI 호출](</private/tmp/storyproduction-text-verify.oQPtcy/commands.jsonl>),
[완료 package marker](</private/tmp/storyproduction-text-verify.oQPtcy/project/package/story-package.json>)에
남아 있다. 이전 hash를 사용한 실패 fixture 기록은
[`/private/tmp/storyproduction-text-verify.QxLLAM/commands.jsonl`](</private/tmp/storyproduction-text-verify.QxLLAM/commands.jsonl>)에
별도 보존했고 successful run으로 계산하지 않았다.

최종 판정은 **현재 deterministic gates 및 synthetic text/owner CLI flow PASS / 실제 원고·창작 품질·사람 승인·Scene Provider/PerfectPixel NOT_RUN**이다. SMC aggregate release verifier, packaged Node-to-native custody helper lifecycle, locked Chromium runtime observation, actual external image/video provider 결과는 이 실행 범위에 없다.

## 후속 SAC Scene Provider responsibility split — source-only

위 검증 이후 `scene_provider.py`에 남은 책임을 정적 구조 분석으로 분리했다. 실행 파일과
component-lock/hash 검증은 `scene_provider_identity.py`, provider/PerfectPixel handshake는
`scene_provider_preflight.py`, generation-unit admission과 attempt receipt/postflight는
`scene_provider_attempt.py`가 맡는다. 기존 `scene_provider.py`는 typed image dispatch의 public
facade로 유지한다. `scene_provider_protocol.py`, `scene_provider_process.py`,
`scene_provider_inspection.py`와 SAC component owner 경계는 유지했다.

사용자의 현재 지시에 따라 이 추가 분리 뒤에는 테스트·build·lint·CLI execution·manifest
verifier를 실행하지 않았다. 따라서 직전 14개 SAC publication tests, `verify-local.sh`, owner
E2E, manifest verification은 이전 provider module graph의 증거이며 이 layout을 검증하지
않는다. component lock, producer/consumer contract, public import path는 변경하지 않았고
candidate/approval/outcome_unknown 의미도 그대로 유지하도록 내부 책임만 이동했다. 이
source manifest generator를 실행해 docs/release/skill manifests를 98/230/112 entries로
갱신했다. 이는 생성 메타데이터이며 verifier는 실행하지 않았다.

## 후속 SAC RenderContract split — source-only

정적 구조 분석 뒤 594줄 `render_contract.py`의 pure prompt projection과 attempt-bound visual
layout evidence를 semantic contract validator에서 분리했다. `render_contract.py`는 기존
`__all__`/import 경로를 유지하는 façade가 됐고, producer schema 및 `contracts.py`, `scene.py`,
`prompts.py` caller 경계는 바꾸지 않았다. 새 `render_contract_validation.py`,
`render_layout_evidence.py`, `render_contract_prompt.py`가 각각 계약 의미, hash-bound evidence,
안정된 prompt 줄을 소유한다.

요청에 따라 이 최신 RenderContract layout에서는 테스트·build·lint·CLI runtime·manifest
verifier를 실행하지 않았다. 앞선 227 SMC tests·SAC 14 boundary tests·`verify-local.sh`·owner
E2E 결과는 이 RenderContract source split 이전의 증거다. 이후 generator를 실행해 docs/release/
skill manifests를 99/234/115 entries로 갱신했으며, generation-only metadata는 이 layout의
동작 증거가 아니다.

## 후속 SAC task/review contract split — source-only

정적 caller 추적 뒤 `contracts.py`의 서로 다른 validator 묶음을 `request_contracts.py`와
`review_contracts.py`로 나눴다. `contract_primitives.py`가 오류/unknown-field/hash 기본기와
RenderContract 오류 변환을 소유하고 기존 `contracts.py`는 같은 public caller import를
제공하는 façade로 남는다. workspace, prompts, review, repair, publish, CLI 호출부와 producer
schema·review/approval authority는 이동하지 않았다.

현재 지시에 따라 이 layout은 실행하거나 테스트/build/lint/verify하지 않았다. 이전 local gate
및 SAC boundary tests가 이 source snapshot의 동작을 증명하지 않는다. manifest generator를
실행해 docs/release/skill metadata를 100/238/118 entries로 갱신했지만 verifier 결과로
해석하지 않는다.

## 후속 SAC review/PNG inspection split — source-only

`review.py`의 output manifest writer와 PNG decoder를 각각 `output_manifest.py`와
`png_inspection.py`로 이동하고, 기계 판정과 Human-review template을 `mechanical_review.py`와
`visual_review_template.py`로 나눴다. CLI의 기존 `review.manifest_outputs`, `review.mechanical_review`,
`review.visual_template` 경로와 PNG 함수들은 façade re-export로 유지했다. Scene Provider
protocol/inspection은 PNG parser를 직접 사용하므로 provider byte/protocol 검증은 visual-review
workflow에 의존하지 않는다. 기존 attempt, selection, human approval, provider, PSD contract
owners는 변경하지 않았다.

사용자의 지시에 따라 이 최신 source split도 실행 검증하지 않았다. 직전 14개 boundary tests,
owner E2E와 manifest evidence는 이전 review/PNG module layout에 대한 결과다.
Source-manifest generator만 실행해 docs/release/skill metadata를 100/242/122 entries로
갱신했다. manifest verifier는 실행하지 않았다.

## 후속 SAC SceneProductionPlan/lock lifecycle split — source-only

정적 책임 분석 뒤 359줄 `scene.py`를 기존 CLI import path를 유지하는 façade로 바꿨다.
`scene_plan.py`는 plan 검증/budget/layout-slot 의미를, `scene_units.py`는 scene staging과
GenerationUnit/capsule 생성을, `scene_files.py`는 scene-local path/hash/review artifact access를,
`scene_locks.py`는 prerequisite order와 immutable lock/seal 전이를 소유한다. PSD validation은
기존 read-only postflight owner에 남겼고 픽셀/PSD writer는 추가하지 않았다.

요청에 따라 이 latest module graph는 실행하거나 테스트/build/lint/verify하지 않았다. 앞선
scene lock, GenerationUnit, PSD 관련 source inspection과 PASS/FAIL 기록은 이 refactor 전의
layout에 적용된다. generator를 실행해 docs/release/skill inventories를 100/246/126 entries로
갱신했으며 verifier는 실행하지 않았다. Generated metadata is not behavioral evidence.

## 2026-09-23 latest module layout end-to-end verification — current source

사용자의 후속 요청으로 SAC SceneProductionPlan/lock split, NVC Delivery asset split과 최신
SMC module graph를 현재 소스에서 다시 검사했다. `.git`이 없는 작업 트리라 commit ID는 없다.
실행 host는 Darwin/arm64, Python 3.12.14 + jsonschema 4.26.0, Node v24.19.0, Rust/Cargo 1.97.1이다.
Rust toolchain·Cargo cache와 주요 로그는 `/private/tmp/storyproduction-current-verify-20260923/`에 뒀다.

| 기준 | 실행 및 관측 | 판정 |
|---|---|---|
| StoryProduction deterministic local gate | `./integration/scripts/verify-local.sh`; integration 29 tests, NVC contract/source guard, SMC boundary suites, SAC skill/release manifests, IPC snapshot 포함 | **PASS / TEST** |
| NVC current workspace Rust gate | `NarrativeCourier/verify-all.sh --require-rust`; workspace check, 56 Rust tests, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings` | **PASS / BUILD+TEST+LINT** |
| NVC Delivery focused behavior | `cargo test --package narrative-courier-delivery --locked --offline`; 6 unit + 1 vertical materialize/reuse/review test | **PASS / TEST, 7/7** |
| NVC static delivery source guard | `tools/verify_delivery_source.py`; Delivery asset child modules까지 분석 대상으로 연결 후 112 checks / 0 errors | **PASS / STATIC** |
| SMC 전체 Node suite | `node --test tests/*.test.js`; current source layout | **PASS / TEST, 227/227** |
| SAC publication boundaries | `python3 -m unittest discover -s tests -v` | **PASS / TEST, 14/14** |
| SAC scene plan/init public CLI | canonical SceneProductionPlan example으로 `scene-plan-check` 및 `scene-init`; 격리 경로에서 composition 4, target 2, layer 6, PSD 1로 13 units 생성 | **PASS / RUNTIME FIXTURE** |
| owner-boundary public CLI E2E | current geultu/NVC build 후 `geultu → NVC → SMC → SAC scene-release → SMC HTML`; 19 STS tasks, 10 simulated gates, 2 scenes, 5 shots, 10 requirements, HTML 14,323 bytes | **PASS / RUNTIME FIXTURE** |
| `storyctl text` full authoring/package | 새 workspace에서 원문→brief→story/synopsis candidate·review→handoff→STS→board→current-hash review→package, 81 public CLI calls | **PASS / RUNTIME FIXTURE** |
| story package integrity | `story-package.json`의 9개 output hash 일치, `source.md` bytes가 intake source와 완전히 동일, completion `complete` | **PASS / RUNTIME** |
| NVC public `deliver` CLI | Completed package로 fixture PNG asset을 `AssetIndex`/content-addressed blob/HTML에 결속. 동일 request 재실행 hash·DeliverySet 유지 | **PASS / RUNTIME FIXTURE** |
| delivery review authority | 생성된 DeliverySet은 `NEEDS_REVIEW`; 사람 review command는 호출하지 않음 | **PASS / BOUNDARY** |
| renderer smoke | `verify-all.sh --require-rust --renderers`가 `magick unavailable`에서 종료. 이 host에서 ImageMagick executable 부재 | **NOT_RUN / ENVIRONMENT** |
| 실제 사용자 작품·사람의 창작/visual 승인 | 실제 원문·작가 검토자가 없음. writer/reviewer/gates는 named synthetic actors | **NOT_RUN / HUMAN INPUT** |
| external image/video/Scene Provider/PerfectPixel·real PSD | 외부 호출 없음. E2E asset bytes와 Delivery PNG는 test fixture | **NOT_RUN / EXTERNAL** |
| app surface/native/browser qualification | Tauri UI `import_asset`, packaged native helper, locked Chromium/Studio release cycle 실행하지 않음 | **NOT_RUN / PRODUCT RUNTIME** |

`storyctl text` 첫 replay의 storyboard review fixture는 이전 run의 `contentHash`를 사용해 call 78에서
`REVIEWED_CONTENT_CHANGED`로 거절됐고 wrapper는 attempt를 `outcome_unknown`으로 기록했다. 이 실행은
성공에 포함하지 않았고 해당 workspace를 재사용하지 않았다. 새 workspace에서 freshly produced board의
현재 hash로 review request를 구성해 81개 호출을 처음부터 통과시켰다. 실패 로그는
[`first replay`](</private/tmp/storyproduction-text-e2e-current-dgurdsu9/commands.jsonl>), 성공 로그는
[`current-hash replay`](</private/tmp/storyproduction-text-e2e-current-bound-h5e7g1ws/commands.jsonl>)와
[완료 package marker](</private/tmp/storyproduction-text-e2e-current-bound-h5e7g1ws/project/package/story-package.json>)에 있다.

현재 source 검증에서도 실제 결함을 고쳤다. NVC asset submodule의 parent visibility와
`NarrativePackage` import를 보완했고 test-only helper를 `cfg(test)`로 제한했다. MCP의
`JsonSchema` derive에 lockfile 버전 `schemars 1.2.2`를 direct dependency로 선언했으며,
Tauri crate가 private `commands.rs`를 binary로 자동 수집하지 않도록 `autobins = false`로
설정했다. Static delivery guard는 동일 불변조건을 유지하면서 `assets/imported_image.rs`와
`assets/materialize.rs`도 읽게 갱신했다. STS formatting/doc-comment/clippy gate 오류도 고쳤다.

첫 SMC 전체-suite 시도 중 동시에 실행되던 subprocess-heavy gate에서 timing-bound test 2개가
실패했다. 한 번의 isolated 재시도는 child process에 bundled Python/Node `PATH`를 넘기지 않아
`jsonschema`와 `node`를 찾지 못했다. 이 두 run은 PASS로 계산하지 않았다. 올바른 Python/Node path를
명시하고 단독 실행한 전체 suite는 227/227 PASS다.

주요 재현 증거: [NVC full gate](</private/tmp/storyproduction-current-verify-20260923/nvc-verify-all-PASS.log>),
[root local gate after docs/manifests](</private/tmp/storyproduction-current-verify-20260923/storyproduction-local-gate-after-docs.log>),
[SMC 227 tests](</private/tmp/storyproduction-current-verify-20260923/smc-node-tests-current-source.log>),
[owner-chain E2E](</private/tmp/storyproduction-current-verify-20260923/owner-e2e-current-source.log>),
[scene-init output](</private/tmp/storyproduction-current-verify-20260923/sac-scene-init>),
[NVC delivery set](</private/tmp/nvc-geultu-smc-e2e-u3idjl50/nvc-projects/nvc-book/visual/requests/REALUSE-DELIVERY-001/delivery-set.json>).

최종 manifest 확인: NVC 456 entries / `sha256:2cd3d39ea88f47f37ee50c54e59a5e55b4e8669061eaf43f7cffea6c3934bfe4`; SAC docs/release/skill 100/246/126 entries, 모두 zero mismatch; SMC 181 files / `sha256:135cb3a6fb63776f75280ac26dfa25b4d5f8b77c8bb2c0bcf84582e32c2c23be`.

현재 판정은 **기술적 제작/전달 fixture PASS / 실제 사용자 서사 품질·사람 승인·외부 media generation NOT_RUN**이다.
`NEEDS_REVIEW` candidate를 human-approved 상태로 승격하지 않았다. 전체 제품 release qualification도 미판정이다.

## 후속 SMC NVC release adapter 분리 — source-only

이후 `packages/adapters/src/narrative-courier-release.js`의 책임을 분리했다. 기존 module path는
Completed NVC release ingress와 durable projection publication/loading의 composition entry로 남기고,
pure SceneGraph/semantic/receipt revalidation을 `narrative-scene-projection.js`로, root/path/bounded
JSON/atomic file effects를 `narrative-courier-release-files.js`로, SMC-local error/hash primitives를
`narrative-courier-release-support.js`로 옮겼다. Existing public exports·NVC producer schema·Narrative
authority·projection receipt/handoff contract는 바꾸지 않았다.

사용자의 현재 지시에 따라 이 마지막 source split 뒤 tests/build/lint/CLI/runtime/manifest verifier는
실행하지 않았다. 앞서 기록된 227 SMC Node tests, 181-file SMC manifest, owner E2E는 이전 adapter
module graph에만 적용된다. Source manifest generator만 current source/docs tree의 185 files /
`sha256:3a9a54a693f0009cfa4415a79c64bf4727930b1ab33eda2fbe79ba0dee179648`로 갱신했으며,
그 manifest verifier는 **NOT_RUN**으로 남긴다.

## 후속 NVC NarrativeSceneMap contract split — source-only

`crates/contracts/src/scene_map/validation.rs`의 map-level facade는 기존 public validator, schema
version/hash identity 계산과 오류 순서를 조합한다. Per-scene shape/order constraints는
`validation/scenes.rs`로, source locator·semantic occurrence bijection·entry/manifest hash chain은
`validation/semantic_manifest.rs`로 옮겼다. DTO, serde wire shape, NVC contracts ownership, crate-root
`validate_narrative_scene_map` entrypoint는 유지했다.

이 변화 뒤에는 tests/build/fmt/clippy/runtime/manifest verifier를 실행하지 않았다. 바로 앞 NVC
56 Rust tests와 전체 local gate는 이 validator split 이전 snapshot의 결과이며 이 latest source
graph의 동작 증거가 아니다. NVC source-manifest generator만 current 458-file tree로 갱신했으며
manifest file hash `9ef98318a4fd440cc20266bd5e49694430ef73546c3aeb5cf33efa5119736021`이다. Verifier는
사용자의 현재 no-verify 범위 아래 계속 **NOT_RUN**이다.

## 후속 STS Codex adapter 분리 — source-only

350줄 `crates/sts/src/codex.rs`를 facade와 `codex/config.rs`, `codex/runtime_status.rs`,
`codex/runner.rs`, `codex/workflow.rs`로 나눴다. Policy/path resolution, runtime capability probe,
private workspace/process effect, STS task lock/submit/cancel orchestration을 분리했다. `Engine`
public method names, `selected_generation_policy` crate path, timeout/cancel ownership와 STS state
authority는 유지하도록 구성했다.

사용자의 현재 no-verify 요청에 따라 이 split 뒤 build/test/fmt/clippy/CLI/runtime/manifest verifier를
실행하지 않았다. 앞선 NVC workspace 56 Rust tests와 owner CLI E2E는 이 Codex module graph 이전의
source evidence다. Source manifest generator만 current 462-file tree /
`sha256:7080d797e1082823f94a01ab6eca70f925c6927eb185b21af38530809adf230f`로 갱신했으며 verifier는 **NOT_RUN**이다.

## 2026-09-23 최신 module layout 실사용 검증 — current source

사용자의 최신 요청에 따라 위 세 module split을 포함한 현재 소스를 다시 검증했다. checkout에는
`.git`이 없어 commit ID는 없다. macOS arm64에서 Python 3.12.14 + `jsonschema==4.26.0`, Node
v24.19.0, Rust/Cargo 1.97.1을 사용했다. 저장소를 `/private/tmp/storyproduction-current-live.NpuZZ4/source`
에 target/node_modules 제외 복제하고 build/test 산출물도 임시 경로에 격리했다.

| 기준 | 명령·관측 | 판정 |
|---|---|---|
| StoryProduction local gate | `./integration/scripts/verify-local.sh`; integration, NVC source/contract, SMC boundary, SAC manifest/skill, IPC snapshot 검사 | **PASS** |
| producer source manifests | NVC 462 entries / `sha256:c2c1d3b04df8c522339cc15468ba23c0046349570c2e93d75cbbb49ac215dd66`; SMC 185 files / `sha256:fde7ec8ad9e5544d647d9cab1f7b1372084db91b88009caf50f5fe877ca0180b`; mismatch 0 | **PASS / MANIFEST** |
| NVC full gate | `./verify-all.sh --require-rust`; workspace check, **56 Rust tests**, `cargo fmt --check`, clippy `-D warnings` | **PASS / BUILD+TEST+LINT** |
| SMC full Node suite | `node --test tests/*.test.js`; 올바른 Python venv를 child `PATH`에 포함 | **PASS / 227/227** |
| SAC publication boundaries | `python3 -m unittest discover -s tests -v` | **PASS / 14/14** |
| owner-chain public CLI E2E | `python3 integration/tests/story_pipeline_e2e.py`; geultu→NVC→SMC→SAC scene release→SMC HTML | **PASS / RUNTIME FIXTURE** |
| text authoring flow | 현재 geultu/NVC 바이너리로 create→brief→story/synopsis 후보·review→hash-bound handoff→STS→storyboard→package; **81/81 CLI calls** | **PASS / RUNTIME FIXTURE** |
| package integrity/status | 9개 artifact SHA-256, `source.md` 원문 bytes 대조, 마지막 `status`의 `completion=complete` | **PASS / RUNTIME** |
| 실제 사용자 원고·창작 승인 | 입력은 synthetic story `빛이 남은 역`; writer와 reviewer는 임시 fixture | **NOT_RUN / HUMAN INPUT** |
| 실제 Codex/image/video provider·사람 승인 | 외부 writer·media provider를 호출하지 않았고 사람은 승인하지 않음 | **NOT_RUN / HUMAN·EXTERNAL** |
| SMC aggregate/native/browser qualification | `scripts/verify.sh`, packaged helper lifecycle, locked Chromium/Studio approval은 실행 범위 밖 | **NOT_RUN / PRODUCT RUNTIME** |

Owner-chain E2E는 19 STS 작업과 10 simulated gate 뒤 2 scenes, 5 shots, 10 asset requirements를
전달해 14,323-byte HTML을 만들었다. 이는 source authority·hash·public entrypoint·artifact handoff의
기술 경로를 확인한 것이며 문학적 품질이나 실제 사람 승인 증거가 아니다. text package는 입력 원문과
byte-identical `source.md`를 보존했고 아홉 artifact 모두 package marker의 hash와 일치했다.

검증 도중 발견한 결함도 숨기지 않고 수정했다. Codex `runtime_status.rs`의 `Command` 누락과 unused
imports가 처음 Rust compile에서 드러났고, 뒤이어 `workflow.rs` import order 및 semantic-manifest의
needless borrow가 fmt/clippy에서 드러났다. NVC static guards는 모듈 이동 뒤 facade만 읽고 있어
Codex timeout/bounded-process 조건을 하위 module 전체에서 확인하도록 고쳤다. 각 변경 뒤 NVC 전체
gate와 root gate를 다시 통과했다.

첫 SMC 전체 suite 시도는 subprocess가 `jsonschema` 없는 Python을 골라 226 tests 뒤 한 테스트가
환경 오류로 실패했다. 격리 Python venv 경로를 child `PATH`에 추가한 새 실행은 227/227 통과했다.
text flow의 첫 workspace는 writer fixture에 `STORYPRODUCTION_ROOT`를 넘기지 않아 `outcome_unknown`이
됐고 재사용하지 않았다. 두 번째 새 workspace는 77번째 호출까지 통과했지만 이전 storyboard hash를
쓴 review를 거절했다. 해당 workspace도 재사용하지 않았으며 마지막 새 workspace에서는 writer 응답의
현재 `contentHash`를 review input에 넣어 81개 호출을 모두 통과시켰다.

재현 로그와 산출물은 다음 위치에 보존했다: [최종 root gate](</private/tmp/storyproduction-current-live.NpuZZ4/verify-local-latest.log>),
[NVC full gate](</private/tmp/storyproduction-current-live.NpuZZ4/nvc-verify-all-final3.log>),
[SMC 227 tests](</private/tmp/storyproduction-current-live.NpuZZ4/smc-tests-with-python.log>),
[SAC 14 tests](</private/tmp/storyproduction-current-live.NpuZZ4/sac-tests.log>),
[owner-chain E2E](</private/tmp/storyproduction-current-live.NpuZZ4/owner-e2e-final.log>),
[81-call CLI log](</private/tmp/storyproduction-current-live.NpuZZ4/authoring-final/current-commands.jsonl>),
[NVC source manifest verify](</private/tmp/storyproduction-current-live.NpuZZ4/nvc-manifest-verify.json>),
[SMC source manifest verify](</private/tmp/storyproduction-current-live.NpuZZ4/smc-manifest-verify.json>),
[completed package marker](</private/tmp/storyproduction-current-live.NpuZZ4/authoring-final/project/package/story-package.json>).

최종 판정은 **synthetic fixture 기술 흐름 PASS / 실제 작품 품질·사람 승인·외부 provider 및 전체 SMC
release qualification NOT_RUN**이다. 앞선 `source-only` 단락은 당시 상태를 보존한 이력이고, 이 마지막
단락이 최신 module layout의 실행 증거를 소유한다.

## 후속 SAC scene-release responsibility split — source-only

실사용 검증 뒤 `asset_release.py`에서 pure request validation을 `asset_release_contracts.py`로,
raw-byte/hash/size/signature file reference validation을 `asset_release_files.py`로 이동했다.
`_relative_path` syntax owner도 contracts로 옮겼다. `_build_release_objects`는 새
`asset_release_builder.py`에서 pack/audit/binding 후보를 조립하고, facade는 생성 결과의
publication lifecycle을 조합한다. Builder의 run request project ID read도 safe files owner에
두었다. `_load_prepared`와 `validate_scene_asset_release`는 새로운 read-only owner
`asset_release_validation.py`에 배치하고 기존 facade import path는 유지했다. CLI, producer schema,
error tokens, validation order,

현재 사용자의 지시대로 이 split 뒤 build/test/lint/CLI/runtime/manifest verifier는 실행하지 않는다.
SAC source-manifest generator만 metadata로 실행해 docs/release/skill inventories를 100/248/128
entries로 갱신했다. 이는 실행 증거가 아니며 manifest verifier는 **NOT_RUN**이다. 이동 전 SAC 14
publication tests와 public owner-chain E2E는 이전 source graph에만 적용된다.

이후 `visual_reference.memory_index()`의 입력 readset도 좁혔다. `_inspect_pack_snapshot()`이
pack을 한 번 읽고 검증한 object를 `inspect()`와 `memory_index()`가 공유하도록 해, approval/hash
검증 뒤 파일을 다시 열어 다른 revision으로 derived index를 만드는 경로를 제거했다. CLI와 index
fields는 유지했다. 이 변경 뒤에도 tests/build/lint/CLI/runtime/manifest verifier는 실행하지 않았다.


## 후속 Integration text-file ingress refactor — source-only

`text_files.regular`는 path component symlink만 확인하면서 regular-file 보장처럼 읽혀
`non_symlink_path`로 이름을 바꾸고 내부 callers를 갱신했다. `text_file`은 leaf `lstat`,
no-follow/nonblocking descriptor open, `fstat` type/identity 대조 뒤에만 16 MiB bounded UTF-8 read를
한다. Public CLI/schema와 기존 regular-file 입력 결과는 유지했다. 이 변경 뒤 tests/build/lint/CLI/
runtime/verifier를 실행하지 않았다.

## 2026-09-23 latest current-source 실사용 재검증 — SAC release·text ingress split 이후

이 재검증은 SAC scene-release responsibility split, visual-reference snapshot read, Integration
text-file ingress refactor를 포함한 현재 소스 복사본에서 진행했다. 확인된 SAC approval owner 수정은
작업 트리에 반영하고 그 뒤 격리 복사본에 동기화했다. 빌드·테스트 산출물과 합성 workspace는
`/private/tmp/storyproduction-live-verify.mDGxGI/` 아래에 뒀다. 체크아웃에 `.git`이 없어
commit/revision ID는 확인할 수 없다. macOS arm64, Python 3.12.14 +
`jsonschema==4.26.0`, Node v24.19.0, pinned Rust/Cargo 1.97.1을 사용했다.

| 기준 | 실행·관측 | 판정 |
|---|---|---|
| geultu 현재 Rust source build | `cargo build --manifest-path geultu/Cargo.toml -p geultu-cli --bin geultu --offline --target-dir <isolated-target>` | **PASS / BUILD**; unused `require_scene`와 두 `project` 변수 경고 3개는 남음 |
| NarrativeCourier 현재 Rust source build | `cargo build --manifest-path NarrativeCourier/Cargo.toml -p narrative-courier --bin narrative-courier-cli --locked --offline --target-dir <isolated-target>` | **PASS / BUILD** |
| StoryProduction local gate | `./integration/scripts/verify-local.sh`; Integration Python 29 tests, SAC 14 publication tests, SMC 경계 27 tests, contract/source/manifest checks | **PASS / TEST+MANIFEST** |
| SAC source manifests after documentation refresh | `python3 tools/source_manifest.py generate` followed by `python3 tools/source_manifest.py verify`; docs/release/skill inventories 100/248/128 | **PASS / MANIFEST** |
| SAC publication owner 경계 | `python3 -m unittest discover -s tests -v` | **PASS / TEST, 14/14** |
| SMC→SAC boundary | `node --test tests/scene-asset-compiler-asset-boundary.test.js` | **PASS / TEST, 8/8** |
| owner-chain public CLI E2E | 현재 geultu/NVC builds와 public SMC/SAC CLIs로 `integration/tests/story_pipeline_e2e.py` 실행 | **PASS / RUNTIME FIXTURE**; STS 19 nodes, simulated gates 10, 2 scenes, 5 shots, 10 asset requirements, HTML 14,323 bytes |
| `storyctl text` full authoring | 새 synthetic workspace에서 create→brief→story/synopsis candidate review→handoff→STS→storyboard review→package | **PASS / RUNTIME FIXTURE, 80 CLI calls** |
| package/status integrity | 9개 package files의 SHA-256, intake와 `source.md` byte equality, 마지막 `completion=complete` | **PASS / RUNTIME** |
| text-file ingress boundaries | current `text_file()`에서 regular UTF-8, symlink, FIFO, 16 MiB 초과 입력을 subprocess로 확인; 각 프로세스 제한 3초 | **PASS / RUNTIME, 4/4**; FIFO는 약 32ms 안에 거부 |
| 실제 사용자 원고·사람 승인 | 입력은 synthetic `빛이 남은 역`; writer와 reviewer는 임시 fixture | **NOT_RUN / HUMAN INPUT** |
| 외부 writer·image/video provider 및 SAC image/PSD 검사 | 실제 provider·imagegen·PerfectPixel·PSD를 호출하지 않음 | **NOT_RUN / EXTERNAL** |
| 전체 SMC release/native/browser qualification | SMC aggregate `scripts/verify.sh`, packaged helper, locked Chromium/Studio 실행 안 함 | **NOT_RUN / PRODUCT RUNTIME** |

검증 중 SAC module split 회귀를 발견했다. 첫 실행은 CLI가 요구하는 `approve_scene_asset_release`가
`asset_release.py` facade에서 사라져 import error로 실패했다. 이를 새 validation owner에서 재수출하자
approval 동작이 `_string`을 찾지 못하는 `NameError`가 드러났다. approval은 파일을 쓰는 lifecycle
effect이므로 `approve_scene_asset_release`를 `asset_release.py`로 되돌리고,
`asset_release_validation.py`에는 `_load_prepared`와 read-only `validate_scene_asset_release`만
남겼다. 수정 후 SAC 14 tests, SMC boundary 8 tests, 전체 root local gate와 owner-chain E2E가 통과했다.
첫 실패 로그는 [facade import](</private/tmp/storyproduction-live-verify.mDGxGI/verify-local.log>),
두 번째 실패는 [approval dependency](</private/tmp/storyproduction-live-verify.mDGxGI/verify-local-after-fix.log>)에 있다.

재현 가능한 최종 증거: [root local gate](</private/tmp/storyproduction-live-verify.mDGxGI/verify-local-final.log>),
[geultu build](</private/tmp/storyproduction-live-verify.mDGxGI/geultu-build.log>),
[NVC build](</private/tmp/storyproduction-live-verify.mDGxGI/nvc-build.log>),
[SAC 14 tests](</private/tmp/storyproduction-live-verify.mDGxGI/sac-tests-after-fix.log>),
[SMC/SAC 8 boundary tests](</private/tmp/storyproduction-live-verify.mDGxGI/smc-sac-boundary-after-fix.log>),
[owner-chain E2E](</private/tmp/storyproduction-live-verify.mDGxGI/owner-chain-e2e.log>),
[80-call text run](</private/tmp/storyproduction-live-verify.mDGxGI/story-text-live-final.log>),
[text ingress edge check](</private/tmp/storyproduction-live-verify.mDGxGI/text-ingress-edge-check.json>),
[80 CLI command records](</private/tmp/storyproduction-real-use-o0n_uyp6/commands.jsonl>),
[package integrity result](</private/tmp/storyproduction-live-verify.mDGxGI/story-package-integrity.json>),
[completed package marker](</private/tmp/storyproduction-real-use-o0n_uyp6/project/package/story-package.json>).

최종 판정은 **현재 source의 기술 흐름·경계 fixture PASS / 실제 사용자 작품 품질·사람 승인·외부 provider와
전체 SMC qualification NOT_RUN**이다. Build/test/runtime 검증 수행이 완료된 것이며 전체 제품 release
승인이나 콘텐츠 품질 판정은 아니다.

## 2026-09-23 latest current-source verification — SMC helper cleanup and NVC app-config owner

이 run은 이전 module refactor와 추가 app-config owner를 포함한 현재 소스를 대상으로 했다. StoryProduction
작업본에 `.git`이 없어 revision ID는 식별하지 못했다. 검증 복사본은
`/private/tmp/storyproduction-real-use-current.I067aP/source`에 두고 Cargo target을 분리했다.
macOS arm64에서 Rust/Cargo 1.97.1, Node 24.19.0, Python 3.12.14 / jsonschema 4.26.0을 사용했다.

| 기준 | 명령·관측 | 판정 |
|---|---|---|
| StoryProduction local gate | `./integration/scripts/verify-local.sh`; Integration Python 29 tests, NVC source/contract audits, SMC boundary suites, SAC skill/release manifests, IPC snapshot | **PASS** |
| NVC full workspace | `NarrativeCourier/verify-all.sh --require-rust`; workspace check, **60 Rust tests**, fmt, clippy `-D warnings` | **PASS / BUILD+TEST+LINT** |
| NVC manifest | `tools/source_manifest.py verify`; 468 entries, mismatch 0, manifest SHA-256 `413a4260e8c777b589e3746fdd4a2714085c3e90b530f5efd1d2c53c26b7d2c2` | **PASS / MANIFEST** |
| geultu official gate | `geultu/scripts/verify.sh`; **146 tests**, clippy, release build, CLI/MCP/Studio smoke | **PASS / BUILD+TEST+RUNTIME FIXTURE** |
| SMC full Node suite | `node --test --test-concurrency=1 tests/*.test.js`; **227/227** | **PASS / TEST** |
| SMC manifest | `node scripts/verify-source-tree.mjs`; 190 files, mismatch 0, tree SHA-256 `abece46c40bf671eaf2707b684e094be5b218e9a70edca4344ccfe13c38912e3` | **PASS / MANIFEST** |
| SAC publication boundaries | `python3 -m unittest discover -s tests -v`; **14/14** | **PASS / TEST** |
| Owner-chain CLI E2E | Current geultu/NVC CLI → SMC projection → SAC scene release → SMC HTML; 19 STS tasks, 10 simulated gates, 2 scenes, 5 shots, 10 asset requirements, 14,323-byte HTML | **PASS / RUNTIME FIXTURE** |
| Text authoring package | Fresh synthetic workspace, **81 public CLI calls** from source intake through story/synopsis review, STS release, storyboard review and package | **PASS / RUNTIME FIXTURE** |
| Package integrity | 9 files have matching manifest SHA-256; package `source.md` is byte-identical to intake; terminal status and package marker both `complete` | **PASS / RUNTIME** |
| Text ingress boundaries | Current `text_file()` with canonical `/private/tmp`: regular UTF-8, symlink, FIFO, 16 MiB + 1 byte | **PASS / RUNTIME, 4/4**; FIFO rejected in 35 ms |
| NVC MCP process | stdio initialize `2025-11-25`, tools/list and tools/call; 10 tools; explicit projects root showed `realuse-check` despite conflicting `STS_PROJECTS_ROOT` | **PASS / RUNTIME**; approval/decision tools absent |
| Real user content, human approval, external writer/image/video providers | The workspace fixture was synthetic; approval actors and media were test fixtures | **NOT_RUN / HUMAN·EXTERNAL** |
| SMC aggregate/native/locked browser release qualification | Full `scripts/verify.sh`, packaged helper lifecycle, Studio approval and release custody were not run | **NOT_RUN / PRODUCT RUNTIME** |

The refactor exposed defects before the final pass. SMC's extracted artifact module both imported and locally redefined
canonical helper functions, which caused a JavaScript parse error; the stale local definitions were removed. geultu's
official gate then exposed format drift and unused import/schema variables, and the NVC root consolidation exposed one
Clippy documentation-spacing error. Those narrow issues were corrected, followed by passing full component gates.
The first SMC full-suite run used a stripped snapshot without `node_modules` or executable media-tool PATH entries and
had a concurrent Chrome-cleanup failure. After installing the exact npm-lock versions into the isolated copy, adding
the host ffmpeg/ffprobe/Chrome paths and running files serially, the isolated cleanup test and all 227 tests passed.

Reproducible evidence: [final root gate](/private/tmp/storyproduction-real-use-current.I067aP/root-verify-after-docs-final.log),
[NVC gate](/private/tmp/storyproduction-real-use-current.I067aP/nvc-verify-after-docs-final.log),
[geultu gate](/private/tmp/storyproduction-real-use-current.I067aP/geultu-verify-final.log),
[SMC 227 tests](/private/tmp/storyproduction-real-use-current.I067aP/smc-tests-final.log),
[SAC 14 tests](/private/tmp/storyproduction-real-use-current.I067aP/sac-tests-final.log),
[owner-chain E2E](/private/tmp/storyproduction-real-use-current.I067aP/owner-chain-e2e-after-config.log),
[81-call command log](/private/tmp/storyproduction-real-use-current.I067aP/authoring-run-after-config/current-commands.jsonl),
[package integrity result](/private/tmp/storyproduction-real-use-current.I067aP/authoring-run-after-config/integrity-summary.json),
[MCP stdio/root-precedence result](/private/tmp/storyproduction-real-use-current.I067aP/nvc-mcp-stdio-after-config.json),
[text ingress boundary result](/private/tmp/storyproduction-real-use-current.I067aP/text-ingress-edge-check-final.json).
