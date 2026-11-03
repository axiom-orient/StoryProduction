# 2026-09-30 현재 소스 실사용 검증 재실행

> 이 보고서는 `741dba...` 격리 source snapshot의 기록이다. 이후 활성 작업 트리에서 수행한
> 최신 gate·runtime·품질 결과는 [현재 작업 트리 검증 보고서](2026-09-30-active-worktree-validation.md)를 본다.

## 판정

검증 수행은 **DONE**, 전체 제품 qualification은 **PARTIAL**이다. 최신 변경이 포함된 격리 소스에서 root gate, Geultu, NVC Rust gate, SMC aggregate, IPC Rust/CLI/Web 경로와 두 public E2E 흐름을 다시 실행했다. 기계 경로는 통과했지만 합성 작품의 의미 충실도는 **FAIL**이며, SAC G09와 IPC historical image evidence는 누락 상태다. `package_status=complete`와 simulated review는 작품 승인이나 사람의 채택을 뜻하지 않는다.

## 대상과 환경

저장소에 Git metadata가 없어 테스트한 소스를 지문으로 고정했다. 생성물과 cache(`target`, `node_modules`, `.tmp`, Python cache 등)를 제외한 source는 6,113개 파일, 77,257,662바이트, SHA-256 `741dba497bc6500198353f797ea53ca11fc0637720bb4d346af79bf2892684a8`이다. 격리 사본과 원본의 파일별 지문이 완전히 일치했다. 검증 로그와 지문은 [증거 묶음](/private/tmp/storyproduction-real-use-rerun-741dba49)에 보존했다.

환경은 Darwin 27.0 arm64, Node.js 24.19.0, Python 3.12.14 / jsonschema 4.26.0, Rust 1.97.1이다. SMC package gate는 host에 `npm`이 없어 `npm` 명령을 pnpm 11.19.0으로 전달하는 scratch shim을 사용했다. native npm 실행은 **NOT_RUN**이다. Geultu와 IPC에서 발견된 formatter 지적은 import/layout만 정리한 뒤 최종 지문을 캡처하고 gate를 재실행했다. 이 보고서와 status/README/사용 가이드/component analysis 색인은 지문 캡처 뒤 갱신했다.

SMC aggregate의 첫 scratch 실행은 검증 harness가 source root에 붙인 `node_modules` symlink를
source-integrity gate가 거부해 실패했다. 의존성을 scratch의 실제 `node_modules` 디렉터리로 복사한
후 동일 aggregate를 재실행해 10/10 PASS를 얻었다. 첫 실행은 제품 결함 증거로 집계하지 않았다.

## 실행 결과

| 경계 | 결과 | 실행 근거 |
|---|---|---|
| StoryProduction root | **PASS · RUNTIME** | `bash integration/scripts/verify-local.sh`, exit 0. 계약 pin, Python integration tests, SMC↔SAC 경계, SAC manifests, IPC source-snapshot test를 통과했다. [로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/root-local-gate-post-format.log) |
| Geultu | **PASS · TEST/RUNTIME** | `./scripts/verify.sh`, exit 0. Rust 96 tests, fmt/Clippy, release build, CLI/MCP/Studio local smoke를 통과했다. [로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/geultu-verify.log) |
| NarrativeCourier | **PASS · TEST/RUNTIME** | `./verify-all.sh --require-rust`, exit 0. Rust 60 tests, fmt/Clippy, 112 source checks, 14 audit tests와 UI graph checks를 통과했다. 별도 `--renderers` 경로는 `magick` 부재로 **NOT_RUN_ENVIRONMENT**다. [Rust gate 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/nvc-verify-rust.log) · [renderer 시도 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/nvc-verify.log) |
| SceneMotionCompiler | **PASS · TEST/RUNTIME** | Node suite 227/227. Aggregate gate 10/10 PASS: doctor, contracts, syntax, tests, source manifest/integrity, Track B, CLI, Studio contract, release. npm 명령은 pnpm shim으로 전달했다. [전체 suite 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/smc-node-tests.log) · [aggregate 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/smc-aggregate-retry.log) |
| SceneAssetCompiler | **PARTIAL** | publication-boundary tests 14/14 PASS. `python3 tools/verify_release.py`는 `FAIL G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`으로 fail-closed 했다. 출처 없는 PSD evidence는 만들지 않았다. [tests 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/sac-tests.log) · [release gate 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/sac-release-gate.log) |
| ImagePromptCompiler | **PARTIAL** | workspace Rust 273 tests, fmt/Clippy, docs build와 release build 통과. release CLI prompt-only smoke, loopback Web smoke, 문서·catalog·대표 QA·source-structure audits도 통과했다. 공식 `./scripts/verify.sh`는 historical `evals/dogfood/historical/1.4.0/image/image-evidence.json` 부재로 exit 1이다. verification receipt는 생성하지 않았다. [공식 gate 로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/ipc-verify-post-format.log) · [CLI smoke](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/ipc-cli-smoke.log) · [Web smoke](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/ipc-web-smoke.log) |
| Public owner-chain | **PASS · RUNTIME** | 실제 CLI 경로 `geultu → NarrativeCourier → SMC → SAC → SMC` 완료. 19 agent nodes, 10 simulated gates, 2 scenes, 5 shots, 10 asset requirements, 14,407-byte HTML을 만들었다. PSD/PNG는 1×1 test fixture다. [로그](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/owner-chain-e2e.log) |
| Text authoring/package | **PASS · RUNTIME** | 새 synthetic workspace에서 `storyctl text` 82회 호출로 원문 intake → story/synopsis candidate와 review → hash-bound handoff → STS 18 task/10 simulated gates → 3-scene storyboard → 9-file package를 끝냈다. package hashes가 모두 맞고 `source.md`는 intake 원문과 byte-equal이다. 의도적으로 잘못된 export는 exit 2 / `invalid_input`이었고 revision은 변하지 않았다. [setup 요약](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/text-authoring-setup-summary.json) · [CLI transcript](/private/tmp/storyproduction-real-use-rerun-741dba49/logs/storyctl-authoring-transcript.jsonl) · [package manifest](/private/tmp/storyproduction-real-use-rerun-741dba49/authoring-package/story-package.json) |
| 작품 의미 충실도 | **FAIL · REVIEW** | 아래 원문 대조에서 반복 대면과 발화 시각을 새로 단정한 drift가 재현됐다. 대본은 어머니의 핵심 BELIEF를 전달하지 않는다. [원문](/private/tmp/storyproduction-real-use-rerun-741dba49/authoring-package/source.md) · [story](/private/tmp/storyproduction-real-use-rerun-741dba49/authoring-package/story.md) · [screenplay](/private/tmp/storyproduction-real-use-rerun-741dba49/authoring-package/screenplay.md) |

## 작품 대조와 제품 한계

원문은 어머니가 민준이 불을 냈다고 **믿는다**고만 말한다. story는 “그 기록을 보여줄 때마다”라고 반복된 대면을 추가한다. 원문이 주는 9:40은 **화재 신고 시각**인데 story는 “민준은 불이 나기 전에 역을 떠났다”고 단정해 신고 시각을 발화 시각으로 바꾼다. screenplay는 이 핵심 비난/BELIEF를 대사나 다른 명시적 정보로 전달하지 않아, 원문 없이 핵심 갈등을 복원하기 어렵다. STS의 마지막 자체 검토가 “No hard failure”라고 결론 내렸지만, 독립 대조에서 이 모순들이 확인됐다.

입력과 리뷰 답변은 합성 QA fixture이고 모든 승인 actor/gate는 시뮬레이션했다. 따라서 package의 `complete`, storyboard의 `검토 후 채택`, hash 일치는 생산 흐름과 파일 무결성만 증명한다. 실제 사람의 창작 검토·승인·adoption은 수행하지 않았다. 사용자의 실제 원고도 제공되지 않아 이 결과는 사용자 작품의 품질 판정이 아니다.

18-stage/144-work-item 설계는 여전히 target specification이고 runtime에 연결되지 않았다. 이 실행의 18 STS tasks는 그 설계가 구현됐다는 증거가 아니다. 실제 원고 장편 품질, external writer/image/video providers, 생성 픽셀 품질, NVC ImageMagick renderer, NVC desktop UI와 locked browser/native helper, PSD provenance/postflight는 **NOT_RUN**이다. IPC official gate의 missing historical image evidence와 SAC G09는 source evidence gap이지 성공으로 간주하지 않는다.
