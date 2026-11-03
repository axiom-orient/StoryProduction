# 2026-09-30 활성 작업 트리 실사용 검증

> 이 기록은 현재 작업 트리의 이전 실행이다. 후속 helper/API clean-break와 현재 재실행 결과는
> [최신 작업 트리 재실행 보고서](2026-09-30-active-worktree-rerun.md)를 따른다.

> 이 실행 결과는 아래 gate/runtime 실행 시점의 작업 트리에 해당한다. 이후 Geultu contract-helper와
> Integration digest/file-helper clean break, storyboard leaf-check deduplication, ImagePromptCompiler의 Codex/OpenAI helper visibility와 core LUT/catalog-selection implementation surface 정리는 [구현 현황](../IMPLEMENTATION_STATUS.md)에 기록했으며 검증하지 않았다.

검증 수행은 **DONE**, 전체 제품 qualification은 **PARTIAL**이다. 현재 작업 트리의 공식 컴포넌트 gate와 두 공개 제작 흐름을 실행했다. 기계적 생성·hash binding은 통과했지만, 합성 작품의 독립 의미 대조는 **FAIL**이다. SceneAssetCompiler의 G09 출처 증거와 ImagePromptCompiler의 historical image evidence가 없어 해당 release qualification도 완료되지 않았다.

## 대상과 환경

저장소 루트에 Git metadata가 없어 commit ID는 없다. 이 기록은 2026-09-30 현재 작업 트리에서 실행한 명령과 로그를 식별자로 삼는다. Rust 1.97.1, Node.js 24.19.0, Python 3.12.14를 사용했다. `integration/requirements.txt`의 `jsonschema==4.26.0`은 격리된 `/private/tmp/storyproduction-current-real-use/python-deps`에만 설치했다. 테스트 로그와 새 텍스트 workspace는 `/private/tmp/storyproduction-current-real-use` 아래에 있다. 실제 사용자 원고, 외부 작성·이미지 provider, 사람의 승인·채택은 사용하지 않았다.

## 확인 결과

| 경계 | 판정 | 현재 실행 증거 |
|---|---|---|
| StoryProduction root | **PASS · TEST/RUNTIME** | `bash integration/scripts/verify-local.sh`, exit 0. 계약 pin, integration Python tests, NVC→SMC→SAC 경계, SAC source/skill/docs manifest, IPC source-snapshot 검사가 문서 갱신 뒤에도 통과했다. [로그](/private/tmp/storyproduction-current-real-use/logs/root-local-gate-after-docs.log) |
| Geultu | **PASS · TEST/RUNTIME** | `./scripts/verify.sh`, exit 0. Rust 96 tests, fmt/Clippy, release build, CLI/MCP/Studio local smoke가 통과했다. [로그](/private/tmp/storyproduction-current-real-use/logs/geultu-verify.log) |
| NarrativeCourier | **PASS · TEST/RUNTIME** | `./verify-all.sh --require-rust`, exit 0. Rust 60 tests와 fmt/Clippy가 통과했다. Renderer 경로는 이 명령에 포함되지 않아 **NOT_RUN**이다. [로그](/private/tmp/storyproduction-current-real-use/logs/nvc-verify.log) |
| SceneMotionCompiler | **PASS · TEST/RUNTIME** | `node --test tests/*.test.js` 227/227 PASS. `./scripts/verify.sh` aggregate 10/10 PASS. `SOURCE_MANIFEST.json` 240 entries / 2,580,997 bytes / tree hash `sha256:f1578ffc1aeb7a926bdffc1ffcde70c2a195a469397fe85f4af22901fa62209f`; `verify-source-tree.mjs` PASS. Host에 native `npm`이 없어 격리 `npm` shim이 pnpm 11.19.0에 전달했다. [suite](/private/tmp/storyproduction-current-real-use/logs/smc-node-tests.log) · [aggregate](/private/tmp/storyproduction-current-real-use/logs/smc-aggregate.log) |
| SceneAssetCompiler | **PARTIAL** | root gate의 247/125/100-entry manifests와 공개 E2E의 init→release 경로는 PASS. source tree에서 `tools/verify_release.py`를 직접 실행하면 기존 Python cache residue로 `CLEAN_SCAN:JUNK`에서 멈췄다. cache를 제외한 격리 사본에서는 `FAIL G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`이 재현됐다. 출처 증거를 만들거나 기존 cache를 지우지 않았다. [격리 release gate](/private/tmp/storyproduction-current-real-use/sac-release-gate.log) |
| ImagePromptCompiler | **PARTIAL** | `./scripts/verify.sh`의 Rust 273 tests, check/fmt/Clippy, docs build, release build는 통과한 뒤 `audit-image-evidence.py`가 `evals/dogfood/historical/1.4.0/image/image-evidence.json` 누락으로 중단했다. 뒤이어 CLI prompt-only smoke, loopback Web smoke, source-snapshot 검사는 각각 PASS했다. verification receipt는 만들지 않았다. [gate 로그](/private/tmp/storyproduction-current-real-use/logs/ipc-verify.log) · [CLI](/private/tmp/storyproduction-current-real-use/logs/ipc-cli-smoke.log) · [Web](/private/tmp/storyproduction-current-real-use/logs/ipc-web-smoke.log) · [snapshot](/private/tmp/storyproduction-current-real-use/logs/ipc-source-snapshot.log) |
| 공개 owner-chain | **PASS · RUNTIME FIXTURE** | `python3 integration/tests/story_pipeline_e2e.py`: `geultu → NarrativeCourier → SMC → SceneAssetCompiler → SMC` 성공. 19 agent nodes, 10 simulated human gates, 2 scenes, 5 shots, 10 asset requirements와 14,407-byte HTML을 만들었다. 입력 PNG/PSD는 1×1 테스트 fixture다. [로그](/private/tmp/storyproduction-current-real-use/logs/owner-chain-e2e-final.log) |
| 원문→문서 package | **PASS · RUNTIME FIXTURE** | 새 격리 workspace에서 `storyctl text` 82회 호출. 원문 intake, story/synopsis 후보 검토, hash-bound handoff, STS 18 agent tasks/10 simulated gates, 3-scene storyboard, 9-file package까지 완료했다. `source.md` bytes가 intake 원문과 같고 manifest의 파일 hash가 전부 맞았다. 잘못된 export는 exit 2였고 직전·직후 authoring state가 같았다. [setup 증거](/private/tmp/storyproduction-current-real-use/text-authoring/logs/text-authoring-setup-current-summary.json) · [전체 transcript](/private/tmp/storyproduction-current-real-use/text-authoring/logs/storyctl-authoring-transcript.jsonl) · [package manifest](/private/tmp/storyproduction-current-real-use/text-authoring/authoring-workspace/package/story-package.json) |
| 작품 의미 충실도 | **FAIL · REVIEW** | 아래 원문 대조에서 반복 대면과 화재 발생 시점에 관한 근거 없는 단정이 확인됐다. screenplay는 어머니의 핵심 BELIEF를 명시하지 않는다. STS 자체 검토가 이 drift를 놓쳤다. [원문](/private/tmp/storyproduction-current-real-use/text-authoring/authoring-workspace/package/source.md) · [story](/private/tmp/storyproduction-current-real-use/text-authoring/authoring-workspace/package/story.md) · [screenplay](/private/tmp/storyproduction-current-real-use/text-authoring/authoring-workspace/package/screenplay.md) |

## 재현된 결함과 수정

공개 owner-chain 첫 실행은 SMC에서 `trackb-custody-binary.js`가 protocol module에서 제거된 `sha256Prefixed`를 import해 멈췄다. raw-byte SHA-256 owner가 `contracts`로 이동한 뒤 caller import를 함께 옮기지 않은 것이 원인이었다. binary admission은 `contracts`의 `hashTrackBRawBytes`를 직접 사용하도록 고쳤다. 출력은 동일한 `sha256:<hex>` 형식이다. custody 회귀 테스트 16/16, 전체 Node suite 227/227, manifest verification, 10/10 aggregate와 최종 public owner-chain이 모두 통과했다.

공식 gate에서 발견한 잔여 정리도 반영했다. Geultu test imports와 NVC DTO exports를 rustfmt 순서로 맞췄고, NVC에서 사용하지 않는 `Path` import를 제거했다. IPC 문서의 하위 저장소 밖 상대 링크를 평문 경로로 바꾸고 Codex adapter import를 rustfmt 순서로 정리했다. 수정 뒤 Geultu/NVC 전체 gate와 root gate를 재실행해 통과했다.

## 콘텐츠 대조와 한계

합성 원문은 어머니가 근무표 때문에 민준이 불을 냈다고 **믿는다**고 말한다. Story는 “그 기록을 보여줄 때마다”라고 반복 대면을 만들었다. 원문에 적힌 9:40은 **화재 신고 시각**인데 story는 “민준은 불이 나기 전에 역을 떠났다”고 단정했다. 9:10 출발과 9:40 신고는 발화 시점을 확정하지 않는다. Screenplay에는 근무표와 세 시각은 있지만 어머니의 비난/믿음이 나오지 않아 핵심 가족 갈등이 원문 없이 복원되지 않는다. STS 산출물은 이를 “No protected source or ending drift”라고 판정했다.

Story와 시놉시스 검토, STS gates, storyboard review는 전부 테스트 actor로 시뮬레이션했다. 따라서 `package_status=complete`는 package 구조·hash·source byte 연결의 완료만 뜻한다. 작품 승인, 의미 충실도, 작가 채택을 의미하지 않는다. 실제 사용자 원고가 제공되지 않아 이 결과는 사용자의 작품 품질 판정이 아니다.

18단계/144 세부 작업 설계는 여전히 target specification이며 runtime에 등록되지 않았다. 이번 STS의 18개 task는 그 설계가 구현됐다는 증거가 아니다. NVC image renderer, SMC native custody helper·실제 Studio human approval, IPC 실제 browser/client session, 외부 writer/image/video provider와 생성 이미지 품질은 **NOT_RUN**이다. 이 범위의 완료 qualification은 **PARTIAL**로 남는다.
