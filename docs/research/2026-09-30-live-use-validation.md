# 2026-09-30 현재 작업 트리 실사용 검증

## 판정

검증 실행은 완료했다. 공개 component 경계와 텍스트 제작 경로는 기계적으로 동작했지만 전체 qualification은 **PARTIAL**이다. NarrativeCourier의 공식 gate는 Rust 서식 검사에서 멈췄고, ImagePromptCompiler는 문서 링크 guard와 rustfmt에서 실패했다. SceneAssetCompiler는 출처가 없는 PSD fixture의 release를 차단했다. 가장 중요한 제품 품질 검사는 **FAIL**이다. hash-complete 패키지의 screenplay가 원문의 핵심 믿음과 미상 정보를 놓쳤다.

검증 대상은 `/Users/ax/repository/StoryProduction`의 격리 복사본이다. 실행 전 원본과 scratch source를 build/cache 파일을 제외해 비교했을 때 모두 6,133 files / 77,459,598 bytes / SHA-256 `3a441eb6d148382f0212951ab9f38df480bde3c920aac38cd764fcd4d0520725`로 일치했다. Rust/Cargo 1.97.1, Node 24.19.0, Python 3.12.14, jsonschema 4.26.0을 사용했다. root에 Git metadata가 없어 commit ID는 없다. 모든 결과 로그와 scratch package는 `/private/tmp/storyproduction-livecheck.QIEv56/` 아래에 보존했다. 검증이 끝난 뒤 이 보고서와 `docs/IMPLEMENTATION_STATUS.md`, `docs/TEXT_PRODUCTION.md`의 결과 안내만 갱신했으며 제품 코드는 수정하지 않았다.

## Component gate

| 경계 | 결과 | 관측 |
|---|---|---|
| StoryProduction root | **PASS** | `bash integration/scripts/verify-local.sh`; contract checks, Python integration tests, NVC/SMC/SAC 경계, source/skill/docs manifests, IPC source snapshot 검사를 통과했다. [로그](</private/tmp/storyproduction-livecheck.QIEv56/logs/root-integration.log) |
| Geultu | **PASS** | `bash geultu/scripts/verify.sh`; Rust 96 tests, fmt/Clippy, release build, canonical 146 checks, CLI/MCP/Studio local smoke 통과. [로그](</private/tmp/storyproduction-livecheck.QIEv56/logs/geultu.log) |
| NarrativeCourier | **PARTIAL** | `bash NarrativeCourier/verify-all.sh --require-rust`의 static/contract checks와 Rust tests 68개가 통과하고 별도 Clippy도 통과했다. 공식 gate는 `crates/delivery/src/render/verification.rs`의 import 정렬 차이에서 실패했다. `SOURCE_MANIFEST.sha256` 507/507 검증은 PASS. [gate](</private/tmp/storyproduction-livecheck.QIEv56/logs/nvc.log) · [Clippy](</private/tmp/storyproduction-livecheck.QIEv56/logs/nvc-clippy.log) · [manifest](</private/tmp/storyproduction-livecheck.QIEv56/logs/nvc-manifest.log) |
| SceneMotionCompiler | **PASS · 10/10 gates** | `bash SceneMotionCompiler/scripts/verify.sh`; 최종 전체 실행에서 218/218 Node tests, source manifest 239 files / `sha256:3259c51298af52357c4a5d02f4d362873570e316a79164f434c0b1ba19fc4044`, source integrity, Track B, CLI, Studio contract, release gate 통과. 검증 scratch의 `node_modules`는 source package가 symlink를 금지해 실제 폴더로 복사했다. 첫 실행의 시간 민감 테스트 2개는 단독 및 최종 전체 재실행에서 통과했다. [최종 aggregate](</private/tmp/storyproduction-livecheck.QIEv56/logs/smc-aggregate-final.log) · [manifest](</private/tmp/storyproduction-livecheck.QIEv56/logs/smc-manifest.log) |
| SceneAssetCompiler | **PARTIAL** | publication-boundary tests 14/14 PASS. `tools/verify_release.py`는 `G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`으로 종료했다. scratch에서 생긴 Python cache residue를 제거한 뒤 다시 실행해 재현했다. [tests](</private/tmp/storyproduction-livecheck.QIEv56/logs/sac-tests.log) · [release gate](</private/tmp/storyproduction-livecheck.QIEv56/logs/sac-release.log) |
| ImagePromptCompiler | **FAIL · official gate exit 101** | `cargo check`, release build, docs build, doc tests, 별도 Clippy와 CLI/Web smoke는 PASS다. Rust workspace tests는 component 내부 링크를 벗어나는 문서 링크 하나 때문에 실패했다. `verify-quality.sh`는 6개 분리 module에 걸친 import/공백 rustfmt 차이에서 실패했다. 이미지 evidence audit은 `evals/dogfood/historical/1.4.0/image/image-evidence.json` 부재로 실패했다. Catalog/representative QA/source snapshot audits는 통과했다. [official gate](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-verify.log) · [전체 test 실행](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-tests-no-fail-fast.log) · [quality](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-quality.log) · [Clippy](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-clippy.log) |

ImagePromptCompiler 문서 검사는 `docs/IMPLEMENTATION_STATUS.md`의 `../../docs/research/2026-09-30-real-use-validation-after-owner-splits.md`와 `ANALYSIS.md`의 `../docs/research/...` 링크가 component repository 밖으로 나간다고 지적했다. 출처 자료가 없는 이미지 evidence나 SceneAssetCompiler provenance를 만들지 않았다. ImagePromptCompiler에서 `cargo test --workspace --all-targets --locked --offline --no-fail-fast`를 별도로 실행했으며 실패 target은 이 문서 경계 테스트 하나였다. `cargo test --doc`은 통과했다. `cargo doc`와 `cargo build --release`도 통과했다. [문서 링크 audit](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-doc-links.log) · [evidence audit](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-image-evidence.log) · [CLI smoke](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-cli-smoke.log) · [Web smoke](</private/tmp/storyproduction-livecheck.QIEv56/logs/ipc-web-smoke.log>)

## 공개 사용자 흐름

`integration/tests/story_pipeline_e2e.py`가 현재 소스의 public CLI를 통해 `geultu → NarrativeCourier → SMC → SceneAssetCompiler → SMC`를 완료했다. 19개 STS agent node, 10개 simulated Human gate, 2 scenes, 10 asset requirements, 5 shots, 15,442-byte HTML을 만들었다. 입력 이미지와 PSD는 1×1 synthetic fixture다. **PASS · runtime fixture**. [로그](</private/tmp/storyproduction-livecheck.QIEv56/logs/owner-chain-e2e.log>)

새 `storyctl text` workspace에서는 78회 공개 호출로 원문 intake → story/synopsis 후보 제출·hash-bound review → synopsis handoff → 19개 token-bound STS 작업·10개 simulated gate → storyboard review → package를 실행했다. 569-byte `source.md`는 intake 원문과 byte-equal이고, manifest의 9개 파일 hash가 일치했으며, `status=complete`를 반환했다. **기계적 경로 PASS**. [요약](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/summary.json>) · [transcript](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/storyctl-text-transcript.jsonl>) · [package manifest](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/package/story-package.json>)

## 원문 충실도

독립 대조는 **FAIL**이다. source/story/synopsis는 어머니가 민준을 화재 원인으로 믿는다는 사실, 기록을 바꾼 사람과 화재 원인이 미상이라는 사실을 보존한다. 최종 screenplay는 9:10 퇴근 기록과 9:40 신고 시각 및 정정·출발 결말은 담았지만 어머니의 믿음과 두 명시적 unknown은 전달하지 않는다. 그 결과 원문 없이 대본만 읽으면 핵심 가족 갈등과 미해결 범위를 복원할 수 없다. Source IR fixture 자체에서 세 사실이 빠져 STEP 8D의 구조적 coverage도 이 누락을 발견하지 못했다. 완료 상태와 byte/hash 일치는 콘텐츠 충실도나 작가 승인 증거가 아니다. [source](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/package/source.md>) · [story](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/package/story.md>) · [synopsis](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/package/synopsis.md>) · [screenplay](</private/tmp/storyproduction-livecheck.QIEv56/text-authoring-run/package/screenplay.md>)

실제 사용자 원고는 제공되지 않아 synthetic QA fixture를 썼다. Writer/reviewer/Human gate는 모두 fixture actor이며 사람의 실제 승인·adoption은 수행하지 않았다. 외부 writer/model, image/video provider, 생성 픽셀 품질은 **NOT_RUN**이다. `magick`이 없어 NarrativeCourier renderer도 **NOT_RUN**이다.

## 후속 작업 순서

1. IPC/NVC의 현재 rustfmt 차이와 IPC component 밖 문서 링크를 정리한 뒤 공식 gate를 다시 실행한다.
2. SAC G09와 IPC historical image evidence는 실제 출처 자료가 확보될 때만 검증한다. 임의의 provenance/evidence로 대체하지 않는다.
3. Source IR 구성에서 원문의 핵심 믿음과 명시적 unknown을 누락 없이 추적하고, screenplay가 원문 사실을 실제로 재현하는지 독립적으로 확인하는 조건을 보강한다. 이 semantic fidelity 실패를 해소하기 전에는 기계적 `complete`를 콘텐츠 승인으로 사용하지 않는다.
