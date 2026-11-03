# 2026-09-30 현재 작업 트리 실사용 검증

## 판정

검증 실행은 **DONE**, 제품 전체 qualification은 **PARTIAL**이다. 계약·컴포넌트 gate, 현재 바이너리를 통한 공개 owner-chain, 원문 intake에서 9개 파일 텍스트 package까지 이어지는 공개 CLI 흐름을 확인했다. package와 해시는 정상이나, 합성 작품의 의미 충실도는 **FAIL**이다. SceneAssetCompiler의 G09 provenance와 ImagePromptCompiler의 historical image evidence가 없어 해당 release qualification은 남아 있다.

실제 사용자 원고는 저장소에서 찾지 못해 기존 합성 QA 원고 `빛이 남은 역`을 사용했다. 집필·검토 답변과 승인 actor는 테스트 fixture가 제공했다. 따라서 이 실행은 제품 경로와 경계 동작의 실사용 증거이며, 실제 작가의 만족도·모델 집필 품질·사람의 승인/adoption을 입증하지 않는다.

## 대상·환경

저장소에 `.git` metadata가 없어 검증한 트리는 파일 지문으로 식별한다. build/cache 경로(`target`, `node_modules`, `.tmp`, `__pycache__`, `.pyc`, `.DS_Store`)와 symlink를 제외하고 정렬된 상대 경로, 경로 길이, 파일 길이, 바이트를 결합한 SHA-256은 **6,122 files / 77,341,641 bytes / `4558d3419f608259b690a1c263cf6ef7cd05de9ec5c71f2a1ba156561bff9764`**였다. 이는 이 보고서와 현황 문서를 쓰기 전의 현재 코드·문서 트리를 가리킨다. 실행 결과물과 transcript는 `/private/tmp/storyproduction-real-use-20260930-jtavf6sw/` 아래에 보존했다.

환경은 macOS 27.0 arm64, Node.js 24.19.0, Rust 1.97.1, Python 3.12.14, `jsonschema==4.26.0`, pnpm 11.19.0이었다. 호스트에 `npm`과 ImageMagick `magick`이 없어 SMC aggregate의 `npm` 호출은 격리된 shim에서 pnpm으로 전달했고, NarrativeCourier renderer gate는 실행하지 않았다. `jsonschema`는 임시 Python package path에 설치했다. 사용자 원고 workspace는 만들거나 수정하지 않았다.

## 결과

| 경계 | 판정 | 증거 |
|---|---|---|
| Root integration gate | **PASS** | 최종 `bash integration/scripts/verify-local.sh`: contract registry, Python integration 29/29, NVC audits, SMC↔SAC boundary tests, SAC manifests, IPC source-snapshot check 통과. [최종 로그](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/root-local-gate-final-tree.log>) |
| Geultu | **PASS** | `bash scripts/verify.sh`: `cargo fmt`, Clippy, 96 Rust tests, release build, 146 docs/canonical checks, CLI/MCP/Studio smoke 통과. 시작 시 한 줄 포맷 차이를 찾아 `cargo fmt --all`만 적용한 뒤 전체 gate를 재실행했다. [최종 로그](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/geultu-verify-rerun.log>) |
| NarrativeCourier | **PASS** | `bash verify-all.sh --require-rust`: 60 Rust tests, 112 source checks, 14 STS audit tests, fmt/Clippy 통과. `magick`이 없어 별도 renderer gate는 **NOT_RUN_ENVIRONMENT**다. [최종 로그](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/nvc-verify-final.log>) |
| SceneMotionCompiler | **PASS** | Node tests 227/227, 최종 문서 snapshot aggregate 10/10, source manifest 243/243 files, source-integrity, Track B, CLI, Studio contract, release gate 통과. aggregate tree hash는 `sha256:ee99727f8b00fbbcc07ac9095996f94301d1dcf64808db60543246b033a293f4`다. [Node suite](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/smc-node-tests-rerun.log>) · [최종 aggregate](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/smc-aggregate-final-all-docs.log>) |
| SceneAssetCompiler | **PARTIAL** | publication-boundary 14/14 tests와 root의 247/125/100 manifests 통과. 기존 작업 트리 release gate는 보존된 Python cache 때문에 `CLEAN_SCAN`에서 멈췄다. cache를 제외한 격리 사본에서는 `G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`으로 fail-closed했다. PSD/provenance는 만들지 않았다. [tests](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/sac-tests.log>) · [격리 release gate](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/sac-release-gate-isolated.log>) |
| ImagePromptCompiler | **PARTIAL** | 공식 gate의 273 Rust tests, fmt/Clippy, docs build와 release build까지 통과한 뒤 `evals/dogfood/historical/1.4.0/image/image-evidence.json` 부재에서 중단됐다. 이 gate 실행은 이후 CLI/Web smoke와 source verification receipt까지 도달하지 않았다. image evidence나 receipt를 만들지 않았다. [로그](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/ipc-verify.log>) |
| 공개 cinematic owner-chain | **PASS · fixture** | 최종 재실행에서 `geultu → NarrativeCourier → SMC → SceneAssetCompiler → SMC` 경로를 완료했다. STS 19 nodes와 10 simulated gates로 2 scenes, 5 shots, 10 asset requirements, 14,407-byte HTML을 만들었다. PNG/PSD는 test fixture다. [최종 로그](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/logs/owner-chain-e2e-final.log>) |
| `storyctl text` 원문→package | **PASS · mechanics / FAIL · fidelity** | 새 임시 workspace에서 의도된 흐름을 실행했다: 원문 저장→story/synopsis 후보·review→hash-bound synopsis handoff→STS→3-scene text storyboard→9-file package. source bytes와 모든 artifact hashes가 일치했다. [요약](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/text-authoring/authoring-summary.json>) · [transcript](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/text-authoring/logs/storyctl-authoring-transcript.jsonl>) · [package manifest](</private/tmp/storyproduction-real-use-20260930-jtavf6sw/text-authoring/authoring-workspace/package/story-package.json>) |

현황 문서 반영 뒤 NVC producer source manifest(504 entries), SMC source manifest(243 entries), SAC skill/docs/release manifests(125/100/247 entries)를 regenerate/verify했다. NVC full gate, SMC aggregate와 root integration gate도 최신 현황 문서 및 metadata 상태에서 다시 PASS했다.

텍스트 제작 흐름은 setup과 STS/storyboard/package를 합쳐 82개의 의도된 공개 `storyctl` 호출을 사용했다. 이 중 한 호출은 잘못된 read-only export에 대한 의도된 negative probe로 exit 2를 반환했고, revision 변화가 없음을 확인했다. fixture setup에서 누락된 입력 파일을 보충한 뒤 정상 경로를 완료했다. 현재 Geultu/NVC debug binary로 `storyctl text status`를 다시 읽었을 때도 source/story/synopsis가 current, STS와 storyboard review가 완료/승인 상태, completion이 complete였다. 이 상태들은 테스트 workspace 안의 결과이며 작가의 사람 승인을 뜻하지 않는다.

## 독립 콘텐츠 대조

합성 원문에서 어머니는 한 줄의 근무 기록을 근거로 민준을 의심한다고만 한다. 제출된 story는 “어머니는 그 기록을 보여줄 때마다”라고 써서 반복 대면을 새로 만들었다. 원문이 말하는 9시 40분은 **화재 신고 시각**인데 story는 “민준은 불이 나기 전에 역을 떠났다”고 서술해 신고 시각을 발화 시각처럼 바꿨다. 병원 접수증도 아버지의 9시 22분 응급실 도착만 기록하는데, story는 그가 “그때 병원에 있었다”고 단정한다.

screenplay는 어머니가 민준을 범인으로 믿는다는 핵심 갈등을 dialogue, action, text 중 어느 것으로도 복원하지 않는다. 그런데 제출된 STEP 8D 검증은 `SRC-003`과 `REL-001`을 PRESERVED, `STS_STATUS: READY`라고 적었다. independently recovered Screenplay IR에는 근무표를 읽는 `OBS-001`은 있지만 어머니의 믿음을 뒷받침할 `OBS`/`OREL`이 없다. 현재 `step_08_verify` task packet은 독립 Screenplay IR ID와 장면 근거를 요구했으나, fixture가 직접 제출한 답은 그 요구를 따르지 않았다.

이 차이는 두 가지를 보여 준다. 9개 파일 완성과 hash 일치는 작품 충실도를 증명하지 않는다. 또 task prompt의 규칙이 후보 내용 자체의 기계적 보증은 아니다. 이번 실행은 고정된 QA 답안을 CLI에 제출했으며 LLM/provider를 호출하지 않았으므로, 실제 모델이 강화된 prompt를 따르는지는 **NOT_RUN**이다. 사람 review와 final approval도 simulated fixture였다.

## 남은 범위

실제 사용자 원고에 대한 세밀한 fidelity·문체·장편 확장 평가는 원고가 없어 **NOT_RUN**이다. 외부 writer/image/video provider, 실제 human review/adoption, 생성 픽셀 품질, NVC ImageMagick renderer와 실제 browser/client session은 검증하지 않았다. SAC G09 provenance와 IPC historical image-evidence가 남아 전체 release qualification은 **PARTIAL**이다. `package_status=complete`는 package 구조와 byte/hash publication 완료만 뜻한다.

최종 root gate는 formatter 정리 이후 실행했다. Geultu source 변화는 formatter가 지적한 한 줄의 whitespace/layout뿐이며, 그 뒤 Geultu gate, SMC aggregate, owner-chain과 `storyctl text status`를 다시 통과했다. 검증 중 생긴 build/cache output은 기존 `target/`·`.tmp/verify` 경로에만 남겼다.
