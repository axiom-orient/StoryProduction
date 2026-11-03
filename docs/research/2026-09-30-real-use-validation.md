# 2026-09-30 현재 소스 실사용 검증

## 판정

검증 수행은 **DONE**이다. 실행한 기계 경로는 대체로 통과했지만 전체 제품 qualification은 **PARTIAL**이고, 합성 작품의 의미 충실도 검토는 **FAIL**이다. package.status=complete는 파일·해시·검토 상태가 맞다는 뜻이며 작품 품질 승인이나 실제 사람 승인이 아니다.

저장소 루트에 .git이 없어 revision ID는 없다. 검증 snapshot의 관리 대상 소스 지문은 6,107개 파일, 77,177,604바이트, SHA-256 160b21ebc093938296a9ba6be1224a24c641f19181009f3bd5bad036e8153788이다. 현재 소스를 격리 사본 /private/tmp/storyproduction-real-use-20260930.fN1pdV/source에서 실행했고, 검증 종료 시점에 원본 fingerprint가 입력 snapshot과 같음을 확인했다. 이 보고서와 루트 현황 문서는 그 대조가 끝난 뒤 기록했다. 빌드·테스트 산출물과 캐시는 fingerprint 대상에서 제외했다.

환경은 Darwin 27.0 arm64, Node 24.19.0, Python 3.12.14 / jsonschema 4.26.0, Rust 1.97.1, pnpm 11.19.0이다. 테스트 의존성은 격리된 임시 폴더에서만 준비했다. [환경 진단](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/environment.log>) · [소스 지문](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/source-fingerprint.json>) · [검증 후 비교](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/source-fingerprint-after.json>).

## 기계 검증

| 경계 | 결과 | 관측 |
|---|---|---|
| StoryProduction root gate | **PASS** | bash integration/scripts/verify-local.sh; producer contract pins, integration tests, SMC 경계, SAC manifests, IPC source-snapshot 검사 통과. storyctl doctor도 PASS. |
| geultu | **PARTIAL** | ./scripts/verify.sh는 cargo fmt --check에서 중단됐다. application/dispatch/review.rs와 review/candidates.rs에 formatter 차이가 있다. 별도 Clippy, 96 Rust tests, release build, CLI/MCP smoke, 인증된 격리 Studio loopback smoke는 모두 PASS. |
| NarrativeCourier | **PASS** | ./verify-all.sh --require-rust; 60 Rust tests와 계약·문서·UI·fmt·Clippy 검사 통과. tools/source_manifest.py verify도 503개 항목, manifest SHA-256 b30294716ede35be488eb27e86e8adc75c722894e2d46ba14e8279c45125fea5로 PASS. |
| SceneMotionCompiler | **PASS / 환경 한정** | 현재 aggregate verifier 10/10 gates, Node tests 227/227, source manifest, source integrity, Track B, CLI, Studio contract, release smoke PASS. 이 host에는 npm이 없어 임시 npm shim이 package script 호출을 pnpm 11.19.0으로 전달했다. 실제 npm 실행·설치는 qualification하지 않았다. |
| SceneAssetCompiler | **PARTIAL** | publication suite 14/14 PASS. 전체 release verifier는 G09에서 tests/fixtures/legacy-psd/PROVENANCE.json이 없어 중단됐다. skill/docs/release manifests 125/100/247 항목은 PASS. 실제 PSD provenance postflight는 미실행이다. |
| ImagePromptCompiler | **PARTIAL** | 공식 ./scripts/verify.sh는 rustfmt가 crates/promptgen-core/src/interview/image/builder.rs 형식을 거부해 멈췄다. 그 전 cargo check/test/doc 통과(273 tests); 별도 release build, CLI smoke, loopback Web smoke, 문서/카탈로그/대표 image QA/구조 검사 PASS. Clippy는 -D warnings에서 needless_return 1건과 불필요한 Option::as_deref 2건으로 FAIL. image-evidence 감사는 evals/dogfood/historical/1.4.0/image/image-evidence.json 누락으로 FAIL했다. |

주요 로그는 [root gate](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/root-local-gate.log>), [geultu tests](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/geultu-tests.log>), [geultu runtime smokes](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/geultu-studio-smoke.log>), [NVC gate](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/nvc-verify.log>), [SMC aggregate](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/smc-aggregate-rerun.log>), [SAC tests](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/sac-tests.log>), [SAC release verifier](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/sac-release-gate-rerun.log>), [IPC official gate](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/ipc-verify.log>), [IPC Clippy](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/ipc-clippy.log>), [IPC release build](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/ipc-release-build.log>), [IPC CLI smoke](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/ipc-cli-smoke.log>), [IPC Web smoke](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/ipc-web-smoke.log>).

## 공개 제작 흐름

새 격리 workspace에서 현재 공개 CLI로 82회 호출했다. 합성 한국어 fixture를 geultu 원문 저장·브리프·스토리/시놉시스 후보와 검토에 넣고, 시놉시스의 정확한 revision/hash로 handoff한 뒤 NVC Source IR, STS, 대본, 텍스트 콘티, package까지 진행했다. 18개 STS 작성 task, 10개 STS gate, 3개 storyboard scene, 15개 semantic occurrence를 처리했다. 후보 검토자와 모든 human gate는 **시험용 모의 결정**이다.

9개 package 파일의 hash가 manifest와 일치하고, package source.md는 intake 원문과 byte equality였다. 상태는 complete였다. 잘못된 read-only export 입력은 invalid_input으로 종료했고 geultu NameError 없이 프로젝트 revision도 유지했다. [실행 요약](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-summary.json>) · [CLI transcript](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/storyctl-authoring-transcript.jsonl>) · [package manifest](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/story-package.json>).

별도 owner-chain E2E도 현재 공개 진입점으로 geultu → NVC → SMC → SAC → SMC까지 PASS했다. 19개 agent node, 10개 simulated gate, 2 scenes, 5 shots, 10 asset requirements, 14,323바이트 HTML을 만들었다. 픽셀과 PSD 입력은 1×1 test fixture였고 실제 image provider 결과나 시각 품질을 뜻하지 않는다. [owner-chain 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/owner-chain-e2e.log>).

## 작품 충실도 — FAIL

합성 원문과 결과를 사람이 읽고 비교했다. 스토리는 원문에 없는 “어머니가 그 기록을 보여줄 때마다 말했다”는 반복 대면을 추가했고, 9:40 화재 **신고 시각**으로부터 “민준은 불이 나기 전에 떠났다”고 추론했다. 원문은 화재의 실제 발화 시각을 정하지 않는다. 대본은 어머니의 책임 belief와 이를 둘러싼 갈등을 전달하지 않아, 원문의 핵심 동기와 재평가가 cold read에서 복구되지 않는다. NVC의 마지막 verification 산출물은 READY라고 쓰지만 이 독립 검토와 맞지 않았다.

결과물: [합성 원문](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/source.md>) · [스토리](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/story.md>) · [시놉시스](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/synopsis.md>) · [대본](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/screenplay.md>) · [씬 설계](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/scene-blueprint.md>) · [텍스트 콘티](</private/tmp/storyproduction-real-use-20260930.fN1pdV/authoring-workspace/package/storyboard.md>).

실제 사용자 원고, 실제 사람의 창작/visual 승인, 외부 writer/image/video provider, PSD 품질, packaged native helper, locked Chromium/Studio release는 **NOT_RUN**이다. 18-stage/144-work-item 문서는 현재 runtime 구현이 아닌 target specification이다.

검증을 이어가며 IPC의 source_snapshot.py --write-verification을 공식 gate가 formatter에서 멈춘 뒤 별도 실행해 임시 PASS 형식 receipt를 만들었다. 그 파일은 전체 gate 통과 증거가 아니므로 격리 사본에서 제거했고 qualification에 포함하지 않았다. root gate의 test-source-snapshot.py 결과만 PASS로 인정한다.
