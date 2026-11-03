# SceneAssetCompiler — 구현 분석

## 현재 분석·개선 범위

`packaging_preservation.py` owns PSD packaging metrics, comparison-receipt paths and exact
attempt-output binding. `review_contracts.py` owns visual review only; mechanical review imports
packaging validation directly. Provider execution imports only component preflight; the CLI imports
protocol probe from its preflight owner. Unused CLI/workflow imports and release-tool digest
implementation duplication are removed. Error tokens and producer schemas stay unchanged.
Tests/build/lint/runtime for this source cleanup are NOT_RUN. Final manifest verification after the
docs-only update is recorded in [Current](docs/IMPLEMENTATION_STATUS.md); it does not qualify these
code changes.

파일 발행 파트는 소스 정리를 마쳤다. JSON/text publication은 `util.atomic_bytes`의 같은
byte writer를 소비하고 exclusive claim의 partial-file 보존은 별도 정책으로 유지한다.
Asset-release 경로 해석의 반복 split도 제거했다. Tests/runtime for this source cleanup remain NOT_RUN.
상세는 [generation workflow 분석](docs/analysis/generation-workflow/ANALYSIS.md)을 따른다.

## Verdict·책임 경계

별도 Python package manifest는 없는 CLI/agent skill 제품으로, authority→task/attempt→검사→승인→자산 배포를 담당한다. source-release·skill·docs manifests와 runtime component lock은 명시된 integrity inputs다. caller는 Codex skill 사용자, integration, SMC다. 방향 **IMPROVE**, 픽셀 생성과 승인 authority의 분리는 **MAINTAIN**. 2026-10-01 실행 snapshot에서 public SMC→SAC release CLI→SMC HTML owner-chain은 통과했지만 root `verify-local.sh`는 skill manifest의 `asset_release_files.py`·`util.py` hash mismatch로 실패했다. 별도 source-manifest verifier도 정의되지 않은 `sha256_file` 참조로 종료했다. 이를 현재 streaming digest owner 호출로 고치고 manifests를 metadata로 재생성했으며 이 후속 source graph의 freshness 검증은 **NOT_RUN**이다. PSD/PNG 입력은 1×1 fixture였고 실제 imagegen·typed provider·PSD 시각 품질·Human visual approval은 **NOT_RUN**이다. 실행 증거는 [current-tree report](../docs/research/2026-10-01-live-use-revalidation-current-tree.md)에 있다.

| 단위 | 판정·이유 |
|---|---|
| `.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py` + `scene_asset_compilerlib` | 독립 실행 제품. CLI `main` owns argument/command composition; `_print_json` is a private output projection helper. Cargo/package manifest가 없다고 제외하지 않는다. |
| workspace/execution/review/repair/publish | [독립 workflow 분석](docs/analysis/generation-workflow/ANALYSIS.md): task·attempt·승인/발행 상태; `mechanical_review.py` 판정, `visual_review_template.py` Human 입력 scaffold, `output_manifest.py` result writer, `png_inspection.py` SAC-local parser. |
| contracts | [독립 task/review 계약 분석](docs/analysis/task-contracts/ANALYSIS.md): stable façade와 run/authority/plan 및 visual-review/packaging evidence owners. |
| asset_release | [독립 scene-release 분석](docs/analysis/scene-release/ANALYSIS.md): lifecycle facade, builder, read-only subject validation, pure projection contracts와 exact-copy import owners; subjectHash 승인과 handoff 발행. |
| scene/psd/reconstruct/profile | [독립 PSD·scene 분석](docs/analysis/psd-postflight/ANALYSIS.md): `scene_plan.py` contract/budget, `scene_units.py` GenerationUnit/capsule, `scene_files.py` scene path/review checks, `util.py` raw file hashing, `scene_locks.py` lock/seal; `psd.py` bounded parser, `psd_validation.py` structural policy, `psd_profile.py` profile resolution. No forwarding `scene.py` or pixel producer. |
| historical PSD profile evidence | [`psd-profile-regression`](experiments/psd-profile-regression/FIELD_TEST.md): v0.7 field-test record and strict-validator snapshots. The original PSD bytes and provenance are absent; the SHA ledger records reported historical identities only and does not satisfy G09. These files are evidence, not runtime inputs or passing fixtures. |
| render_contract | [독립 contract 분석](docs/analysis/render-contract/ANALYSIS.md): direct semantic validator, attempt-bound evidence validator, and deterministic prompt projection owners; no `render_contract.py` forwarding façade. |
| scene_provider | [독립 provider 분석](docs/analysis/scene-provider/ANALYSIS.md): public facade가 provider dispatch 및 durable pre-wait process evidence를 조합하고, identity/preflight/attempt/protocol/teardown/PerfectPixel owners는 SAC 내부 모듈로 나눈다. Probe와 PerfectPixel은 shared bounded runner를 사용하고 paid provider dispatch는 dispatch evidence 기록 때문에 facade에서 spawn한다. |
| imagegen·PerfectPixel·provider binaries | 외부 effect 경계. tools lock과 evidence가 필요하며 source에 실행 binary 없음. |
| skill MD·template·schema copies | 실행 입력/배포물. 일반 설명 중복과 달리 원본·복사 경로 보존. |

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| init/compile-plan/status/ready | CLI → workspace | request·authority hash·dependency graph | staged run + task files | initialized/compiled/status 또는 error | `.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/workspace.py` |
| begin/manifest/record-execution | CLI/skill → attempt functions | deps·refs·edit target·provider route | attempt directory·execution record | attempt manifest/returned artifact | `workspace.py`, `execution.py` |
| execute/probe-scene-provider·조건부 | CLI → typed provider | lock version/hash·no refs·PNG·explicit timeout | subprocess group/dispatch ledger | typed response/outcome unknown | `.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_provider.py` |
| review/visual/repair/approve | CLI → review/repair/publish | current pixel hash·mech/visual/wireframe gates | immutable selection + repair instructions | candidate approval 또는 blocked | 해당 lib modules |
| human-approve/publish | CLI → publish | review policy·selected assets | human-approval + AssetPack files | portable pack/manifest | `.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/publish.py` |
| scene-release prepare/approve/validate/publish | CLI → release coordinator | SMC exact receipt, asset rights/hash, subjectHash | prepared pack/audit/binding; bound human approval | scene-motion-asset-handoff/v2 | `asset_release.py`, `asset_release_builder.py`, `asset_release_validation.py`, `asset_release_contracts.py`, `asset_release_files.py`, projection contract/import modules |
| scene-plan/locks·PSD·reconstruction | CLI → scene/psd/reconstruct | layered plan·profile·actual file/evidence | run-local locks/evidence | instructions/validation (not fabricated pixels) | 해당 lib modules |
| release verifier | maintainer → stable `tools/verify_release.py` CLI facade | schema/test/fixture/manifest checks or a standalone bundle archive | check-only | gate report or standalone bundle result | `tools/verify_release.py`, `tools/standalone_bundle_verifier.py`, `tools/release_verifier_support.py` |
| visual-reference prepare/approve/inspect/index | public CLI → visual_reference | current v3 projection/capsules + image bytes/hash/refs + expected pack hash | SAC visual pack/approval; derived index | sealed pack/approval 또는 drift 거절 | `scene_asset_compilerlib/visual_reference.py` |

## 실제 흐름

일반 경로: `RunRequest → init staging/rename → compile-plan → ready → begin refs snapshot → external imagegen/provider → record execution → mechanical review → visual review → approve selection → human-approve(policy) → publish asset-pack`.

Story 경로: `SMC projection receipt/raw files → SAC source binding → scene assets/tasks → scene-release prepare(pack,audit,binding) → human approve(subjectHash) → validate current bytes/rights → publish immutable handoff → SMC ingress`. `asset_release_contracts.py`는 request·field·path·handoff grammar와 canonical hash 규칙, `asset_release_files.py`는 root/path containment·JSON/byte/signature I/O와 verified file refs, `asset_release_projection_contracts.py`는 pure SMC receipt/graph/cinematic/semantic validation을 소유한다. `asset_release_projection_import.py`는 검증한 source bytes를 읽고 SAC run에 exact-copy한다. `workspace.py`와 `asset_release.py`는 projection contract/import owners를 직접 호출한다; 제거된 `narrative.py`와 `asset_release_projection.py` re-export facade는 다시 추가하지 않는다. `asset_release_builder.py`는 검증한 release input에서 pack/audit/binding 후보를 조립하고 run request project ID read도 files owner의 safe-path resolver를 거친다. `asset_release_validation.py`는 read-only prepared-subject/current-source 재검증을 소유하며 `asset_release.py`는 approval과 publication 순서를 조합한다. 모든 모듈은 SAC 안에 머물고 component authority를 합치지 않는다. 두 approve는 의미가 다르다. 기존 public CLI/schema, error tokens, validation 순서는 보존한다.

request/files owners, preparation builder, read-only validation owner로 분리한 최신 module layout을 현재 소스에서 다시 확인했다. 첫 SMC asset-boundary 실행은 CLI가 가져오는 `approve_scene_asset_release`의 facade export 누락을 잡았다. 이를 validation owner에서 재수출한 뒤 승인 시 `_string` 의존성이 빠진 것이 드러나, 파일을 쓰는 approval effect를 `asset_release.py` lifecycle facade로 되돌리고 `asset_release_validation.py`에는 read-only subject 검사만 남겼다. 수정 뒤 SAC publication tests 14/14, SMC scene-asset boundary tests 8/8, 전체 root local gate, public owner-chain E2E가 PASS했다. full release verifier는 G09 fixture 문제로 별도 FAIL 기록이 있고 duplicate-key adversarial coverage와 source-mutation race injection은 NOT_RUN이다.

`tools/verify_release.py`는 repository G01–G11을 순서대로 조합하는 CLI facade로 유지한다. 독립 `--standalone-bundle` 경로는 archive membership/hash, extracted package, installation probe와 extracted integration smoke를 `tools/standalone_bundle_verifier.py`가 소유한다. `GateError`, streaming file digest, and in-memory byte digest는 repository verifier, standalone verifier, standalone bundle builder가 공유하는 `tools/release_verifier_support.py`에 둔다. Builder와 verifier의 이전 중복 `hashlib.sha256` implementations는 제거했고, digest values는 동일하다. CLI arguments, G01–G11 gate order, error tokens, and standalone output remain unchanged; tests/build/lint/runtime have not been run after this source-only cleanup.

PSD 경로는 typed layered plan와 lock·placeholder·asset seal을 거친 외부 픽셀/PSD artifact의 검증이다. 로컬 Python으로 생성 성공을 위조하지 않는다.

visual reference 경로: `현재 SMC v3 capsule/graph→실제 reference/keyframe bytes+명시 draft→pack→사람 expected-pack-hash 승인→재생성 가능한 index`. `visual_reference.py`가 pack/review/index의 owner이며 새로운 image producer나 vector DB는 없다. `inspect`와 `memory_index`는 하나의 parsed-and-validated pack snapshot을 공유해 승인 확인 뒤 pack 파일을 다시 열어 다른 revision을 인덱싱하지 않는다.

## 현재 state·contract·I/O owner

request/run UUID·compiled marker·task·attempt·selection·approval은 다른 단계의 State다. task status는 파일들에서 계산한 projection. source_kind/schema/contract names는 Constant, locked binary/path는 Configuration, retry/timeout/external permission·human_final_review는 Policy, provider routing은 input에 명시된 capability다. scene-release는 정수 한정 canonical JSON hash를 쓰며 general RFC8785와 무조건 같다고 주장하지 않는다. raw hash와 `sha256:` canonical identity는 분리된다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| task/attempt·일반 AssetPack 승인/발행 | REACHABLE | PARTIAL | PASS: focused fs 회귀14; host crash/다중 writer 제외 | `tests/test_publication_boundaries.py` |
| scene-release | REACHABLE | SATISFIED | PASS: current public owner-chain E2E와 14 publication tests; 실제 pixels/human 제외 | release owner modules, SMC boundary test |
| visual reference/keyframe pack·approval·memory index | REACHABLE | PARTIAL | PASS: actual Python CLI2; synthetic PNG는 실품질 검증 아님 | `visual_reference.py`, SMC `visual-reference-boundary.test.js` |
| typed provider v2 | REACHABLE | UNKNOWN | NOT_RUN: locked binary·PerfectPixel·network | provider 상세 분석 |
| imagegen·layer/PSD | REACHABLE | UNKNOWN | NOT_RUN: 실제 tool·passing PSD/render-back 부재 | PSD 상세 분석 |
| 전체 release gate | REACHABLE | PARTIAL | FAIL: 원본 real-PSD fixture/provenance 누락; 기존 gate 유지 | `tools/verify_release.py` G09 |

## 의존성·findings

**A1 · 일반 승인 subject — 수정.** 문제 → APPROVED 파일 존재만으로 선택 자산 승인을 재사용했다. 근거 → `publish.py::human_approve/publish`; 원인 → receipt와 selected-content identity 미결합. 영향 → 승인 후 바뀐 자산의 발행. 방향 → request/run/compiled/task/selection/실제 artifact bytes 전체 hash와 public approval receipt hash를 내부 `human-approval-subject.json`에 결합한다. publish 직전에 재검증하고 불변 blob을 해시로 확인한다. public approval DTO v1과 no-human-review 선택 정책은 유지한다. 기존 unbound approval은 묵시 변환하지 않고 재검토·재승인이 필요하다.

**A2 · attempt 준비 실패 — 수정.** 문제 → reference/repair validation 전 directory를 생성했다. 근거 → `workspace.py::begin_attempt`. 원인 → preflight와 효과 순서 역전. 영향 → 잘못된 GENERATING·retry 차단. 방향 → 입력·prompt/capsule을 먼저 준비하고 exclusive attempt allocation 후 오류에는 자신이 만든 directory만 정리한다. 실제 failure injection PASS; process crash 정리까지 보장하지 않는다.

**A3 · plan 부분 commit — 부분 개선.** 문제 → task 순차 생성 중 오류가 기존 run을 막았다. 근거 → `workspace.py::compile_plan`. 원인 → 전체 preflight/staging 없이 개별 쓰기. 영향 → 재실행 불가·불완전 task. 방향 → 전체 task를 private staging에서 완성 후 rename하며 ordinary exception 때 자신이 발행한 task만 정리한다. 기존/깨진 symlink와 task-root symlink를 거절한다. compiled marker까지의 host crash atomicity·다중 writer isolation은 미구현이다.

**A4 · release evidence 부재 — 일부 개선, FAIL 유지.** 문제 → 입력에 원래 tests 및 `tests/fixtures/legacy-psd/PROVENANCE.json`이 없다. 근거 → original archive와 G06/G09. 원인 → package 입력 증거 누락. 영향 → 과거105 테스트·실PSD PASS를 재현할 수 없다. 방향 → focused14 회귀와 zero-test 거부를 추가했지만 기존 golden PSD를 창작하지 않았다. G09와 full release 자격은 충족하지 못한다.

**A5 · 규칙 충돌 — 해결.** 문제 → docs 전용 `AGENTS.md` 사본이 component-root의 제한된 typed-provider PNG 경로를 금지했다. 변경 → 중복 사본을 제거해 `SceneAssetCompiler/AGENTS.md`만 픽셀 원천 정책을 소유하게 했다. 정책은 reference-free PNG text-to-image만 provider에 허용하고 reference·edit·cutout·path·PSD는 imagegen에 둔다. Runtime/provider capability나 승인 경계는 바뀌지 않았다.

**A6 · JSON duplicate-key parsing — 공통화, adversarial case 미실행.** 문제 → `scene_provider.py`의 component-lock/response parser만 duplicate key를 거절했지만 workspace RunRequest·일반 state·publish approval·SMC capsule path에서는 기본 parser가 마지막 key를 사용했다. 변경 → `scene_asset_compilerlib.util`이 strict object-pairs loader를 소유하고 내부 JSON file/bytes boundary가 공유한다. provider lock의 기존 error prefix와 정상 DTO는 유지한다. 모듈 분리 뒤 publication suite 14개와 public E2E는 통과했지만 duplicate-key 입력을 직접 주는 검사는 이번 실행에 포함하지 않아 해당 거부 동작은 **NOT_RUN**이다.

**A7 · SMC projection snapshot의 재복사 — 정상 경로 실행 확인, race injection 미실행.** 문제 → scene-release가 receipt/graph bytes를 읽고 검증한 뒤 source path를 다시 열면 내용이 바뀔 수 있었다. 변경 → 이미 검증한 in-memory bytes와 raw hash를 publish하며 destination에는 같은 bytes만 재사용하고 충돌 bytes는 거절한다. 신규 destination은 temp file을 fsync한 뒤 exclusive hard link로 게시한다. 모듈 분리 이후 public scene-release E2E에서 현재 projection snapshot을 통해 approval/handoff/HTML까지 통과했다. 검증 직후 source를 교체하는 adversarial race는 이번 실행에 포함하지 않았다.

## 실패·취소·재시도·복구와 남은 gap

typed provider에는 dispatch/attempt custody와 불확실한 실행을 자동 재시도하지 않는 제약이 있다. 일반 workflow의 다중 process writer exclusion과 multi-file crash atomicity는 완전하지 않다. scene-release hash 재검사는 drift를 거절하지만 lock의 대체물이 아니다. 실제 이미지/PSD 검증과 synthetic fixture 계약 검증을 구별한다. 과거 fixture를 창작해서 golden evidence로 추가하지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.
