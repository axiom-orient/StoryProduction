# 2026-09-30 현재 작업 트리 실사용 검증 재실행

> 실행 결과는 SMC store/Application/browser-renderer/source-integrity entrypoints, contracts aggregate,
> NVC adapter helper re-export 제거, production stage owner split의 후속 clean breaks 이전 기준이다. 해당 source-only 변경의
> current status와 NOT_RUN 범위는 [구현 현황](../IMPLEMENTATION_STATUS.md)과
> `SceneMotionCompiler/docs/IMPLEMENTATION_STATUS.md`에 기록했다.

## 판정

검증 수행은 **DONE**, 전체 제품 qualification은 **PARTIAL**이다. 최신 작업 트리에서 root와 owner별 gate, 두 개의 공개 제작 흐름을 다시 실행했다. 코드·계약·package 무결성은 검증됐지만, 합성 작품의 의미 충실도는 **FAIL**이다. SceneAssetCompiler의 G09 출처 파일과 ImagePromptCompiler의 historical image evidence가 없어 해당 qualification도 끝나지 않았다.

이 검증에서 Geultu의 제거된 route formatter 호출과 ImagePromptCompiler의 core API 가시성/unused import 문제를 컴파일 gate가 찾아냈다. 실제 consumer를 추적해 작은 수정 후 관련 gate를 통과시켰다. 이 내용은 수정 이력이지 작품 품질이 통과했다는 뜻은 아니다.

## 대상과 실행 환경

저장소는 `/Users/ax/repository/StoryProduction`이며 `.git` metadata가 없어 commit ID는 없다. Node.js 24.19.0, Rust 1.97.1, Python 3.12.14와 격리 Python 의존성 `jsonschema==4.26.0`을 사용했다. 호스트에 native `npm`이 없어 SMC package 명령은 로컬 wrapper를 통해 bundled pnpm으로 전달했다. 검증 로그와 새 authoring workspace는 `/private/tmp/storyproduction-active-verify` 아래에 있다.

실제 사용자 원고는 제공되지 않았다. 두 제작 흐름은 합성 fixture와 simulated review actor를 사용했다. 외부 writer/image provider, 실제 사람의 승인·채택, 생성 이미지 품질은 실행하지 않았다.

## 기준별 결과

| 경계 | 결과 | 실행 증거 |
|---|---|---|
| StoryProduction root | **PASS · TEST/RUNTIME** | `bash integration/scripts/verify-local.sh`, exit 0. Registry pin, 29개 Python integration test, NVC/SMC/SAC 경계, SAC 247/125/100-entry manifests, IPC source-snapshot regression test를 통과했다. [최종 로그](/private/tmp/storyproduction-active-verify/logs/root-local-gate-current-final.log) |
| Geultu | **PASS · TEST/RUNTIME** | `cd geultu && bash scripts/verify.sh`, exit 0. 96 Rust tests, fmt/Clippy, release build, CLI/MCP/Studio smoke가 통과했다. 컴파일에서 `RouteId::as_str`의 두 실제 consumer가 드러나 formatter를 crate-private으로 복구한 뒤 전체 gate를 재실행했다. [로그](/private/tmp/storyproduction-active-verify/logs/geultu-verify-rerun.log) |
| NarrativeCourier | **PASS · TEST/RUNTIME** | `cd NarrativeCourier && bash verify-all.sh --require-rust`, exit 0. Rust test, contract/STS/delivery audit, UI graph, fmt/Clippy가 통과했다. 문서 link 정리 뒤 `SOURCE_MANIFEST.sha256` 504 entries / SHA-256 `461a662cfef11135bbabcfe97d4b11ebdb3758286770cf1b91b7c20fe5d43136`도 generate/verify PASS했다. ImageMagick 기반 renderer smoke는 host에 `magick`이 없어 **NOT_RUN_ENVIRONMENT**다. [gate 로그](/private/tmp/storyproduction-active-verify/logs/nvc-verify.log) · [manifest 생성·검증](/private/tmp/storyproduction-active-verify/logs/nvc-manifest-verify.log) |
| SceneMotionCompiler | **PASS · TEST/RUNTIME** | `npm test` 227/227, `npm run verify` aggregate 10/10, source manifest PASS. `SOURCE_MANIFEST.json`은 240 entries, tree hash `sha256:f1578ffc1aeb7a926bdffc1ffcde70c2a195a469397fe85f4af22901fa62209f`다. 첫 test 호출은 Python child에 격리 `jsonschema` 경로가 빠져 1개가 환경 실패했으며, 의존성 경로를 전달한 재실행은 227/227 PASS했다. [suite](/private/tmp/storyproduction-active-verify/logs/smc-tests-rerun.log) · [aggregate](/private/tmp/storyproduction-active-verify/logs/smc-aggregate.log) |
| SceneAssetCompiler | **PARTIAL** | `python3 -m unittest discover -s tests -v`: 14/14 PASS. Root gate의 release/skill/docs manifests 247/125/100 entries도 PASS했다. cache 없는 격리 사본에서 `tools/verify_release.py`는 `G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`으로 fail-closed 했다. 누락된 출처 evidence는 만들지 않았다. [tests](/private/tmp/storyproduction-active-verify/logs/sac-tests.log) · [release gate](/private/tmp/storyproduction-active-verify/logs/sac-release-gate.log) |
| ImagePromptCompiler | **PARTIAL** | 공식 `bash ImagePromptCompiler/scripts/verify.sh`에서 Rust check/test/doc/fmt/Clippy, docs build와 release build가 통과한 뒤 `evals/dogfood/historical/1.4.0/image/image-evidence.json` 누락으로 멈췄다. 문서 링크·catalog assets/prompts·대표 image QA·Rust structure·`test-source-snapshot.py` regression test 및 release CLI/Web smoke를 별도로 실행해 모두 PASS했다. 전체 source identity receipt 검증은 실행하지 않았고 receipt도 생성하지 않았다. `CatalogFamily::parse`는 MCP가 사용하는 core-owned parser로 공개 유지했다. [최종 공식 gate 로그](/private/tmp/storyproduction-active-verify/logs/ipc-verify-final2.log) · [문서 links](/private/tmp/storyproduction-active-verify/logs/ipc-audit-doc-links-latest.log) · [source-snapshot regression](/private/tmp/storyproduction-active-verify/logs/ipc-source-snapshot-regression-latest.log) · [CLI](/private/tmp/storyproduction-active-verify/logs/ipc-cli-smoke.log) · [Web](/private/tmp/storyproduction-active-verify/logs/ipc-web-smoke.log) |
| 공개 owner-chain | **PASS · RUNTIME FIXTURE** | `python3 integration/tests/story_pipeline_e2e.py`: `geultu → NarrativeCourier → SMC → SceneAssetCompiler → SMC`를 완료했다. 19개 STS 작성 node와 simulated human gate 10개를 거쳐 2 scenes, 5 shots, 10 asset requirements와 14,407-byte HTML을 만들었다. PNG/PSD는 1×1 test fixture다. [로그](/private/tmp/storyproduction-active-verify/logs/owner-chain-e2e.log) |
| `storyctl text` 원문→package | **PASS · RUNTIME FIXTURE** | 새 격리 workspace에서 82개 공개 CLI 호출로 intake→story/synopsis candidate/review→hash-bound handoff→STS→3-scene storyboard→9-file package를 완료했다. `source.md`는 입력 원문과 byte-equal이고 파일 hash가 모두 일치했다. 의도한 invalid export는 exit 2와 `invalid_input` owner error를 반환했고 project revision은 9에서 변하지 않았다. 응답의 최상위 상태는 보수적인 `outcome_unknown`이므로 deterministic input rejection의 표시 개선 여지는 남는다. [요약](/private/tmp/storyproduction-active-verify/text-authoring-rerun/authoring-summary.json) · [setup transcript](/private/tmp/storyproduction-active-verify/text-authoring-rerun/logs/setup-transcript.jsonl) · [STS/package transcript](/private/tmp/storyproduction-active-verify/text-authoring-rerun/logs/storyctl-authoring-transcript.jsonl) · [package manifest](/private/tmp/storyproduction-active-verify/text-authoring-rerun/authoring-workspace/package/story-package.json) |
| 작품 의미 충실도 | **FAIL · INDEPENDENT REVIEW** | 합성 원문은 어머니가 민준을 의심한다고만 한다. Story는 원문에 없는 반복 대면(“그 기록을 보여줄 때마다”)을 추가하고, 신고 시각 9:40만으로 민준이 “불이 나기 전에” 떠났으며 아버지가 당시 병원에 있었다고 단정한다. Screenplay에는 어머니의 핵심 BELIEF가 나타나지 않아 갈등이 복원되지 않는다. STS 자체 review가 이 drift를 놓쳤다. [원문](/private/tmp/storyproduction-active-verify/text-authoring-rerun/authoring-inputs/original.md) · [story](/private/tmp/storyproduction-active-verify/text-authoring-rerun/authoring-workspace/package/story.md) · [screenplay](/private/tmp/storyproduction-active-verify/text-authoring-rerun/authoring-workspace/package/screenplay.md) |

## 수정과 적용 범위

Geultu `visual_run`이 실제로 사용하는 `RouteId::as_str`를 contracts owner에 crate-private으로 복구했다. ImagePromptCompiler에서는 소비되지 않는 `CatalogSelection` import를 제거했고, MCP family filter가 호출하는 `CatalogFamily::parse`를 canonical identifier parser로 공개 유지했다. rustfmt를 적용한 뒤 Geultu, ImagePromptCompiler와 root gate를 재실행했다. Schema, serialized fields, package format, owner boundaries는 바꾸지 않았다.

`package_status=complete`는 package files와 hash binding이 완료됐다는 뜻이며 작품 승인이나 작가 채택은 아니다. 이번 합성 fixture의 18 STS tasks는 18-stage/144-work-item target specification이 runtime에 등록됐다는 증거가 아니다. 실제 사용자 원고, 사람 편집 승인, 외부 writer/image provider, 실제 브라우저 client session, 픽셀/PSD provenance qualification은 **NOT_RUN**이다.
