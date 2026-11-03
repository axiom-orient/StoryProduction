# 2026-10-01 현재 트리 실사용 재검증

## 판정

공개 텍스트 집필 흐름과 교차 owner-chain은 현재 실행 가능한 상태다. 새 격리 workspace에서 원문 intake부터 9개 파일 package까지 83회 공개 CLI 호출이 완료됐고, 별도 owner-chain은 geultu → NarrativeCourier → SMC → SAC → SMC를 통과했다. NVC 공식 gate와 SMC 10-gate aggregate도 통과했다.

전체 qualification은 **PARTIAL**이다. Geultu의 현재 Rust tests 96개는 통과했지만 공식 gate는 세 파일의 rustfmt 차이에서 멈췄다. 최신 root `verify-local.sh`는 SceneAssetCompiler skill manifest의 두 hash mismatch로 실패했다. SceneAssetCompiler의 별도 source-manifest verifier는 정의되지 않은 `sha256_file`을 호출해 실패했다. 실제 사용자 원고, 실제 writer/model 호출, 실제 사람의 review/adoption, 실제 provider·영상/이미지 품질은 실행하지 않았다.

## 검증 대상과 환경

저장소 root에는 Git metadata가 없어 commit SHA를 기록할 수 없다. SceneMotionCompiler의 현재 manifest는 source 243개, 2,575,738 bytes, tree hash `sha256:24b661db26d6de1460cd7186ef8ec0b45cd12c5556f0e992aaf878b0462b81c1`로 재생성 후 검증했다. 이는 SMC component tree identity이며 저장소 전체 revision을 뜻하지 않는다.

환경은 Python 3.12.14, Node 24.19.0, Cargo 1.97.1, rustc 1.97.1, jsonschema 4.26.0, `/opt/homebrew/bin/ffmpeg`였다. 격리된 Cargo/Python 환경을 사용했다. owner-chain은 Geultu의 component-local Cargo 설정을 적용하는 임시 Cargo shim으로 실행했다. 사용자 전역 설정은 바꾸지 않았다.

## 결과

| 경계 | 판정 | 명령과 관측 |
|---|---|---|
| 환경 | **PASS** | `python3 integration/scripts/storyctl.py doctor`; Python/Node/Cargo/rustc/jsonschema와 promptgen binary를 인식했다. |
| 원문→스토리→시놉시스→씬·대본→텍스트 콘티→문서 package | **PASS · synthetic QA** | 공개 `storyctl text` 흐름 83회. package `complete`, 선언 파일 9개, artifact hash 9/9 일치, package `source.md`와 intake 원문 bytes가 동일했다. [요약](</private/tmp/storyproduction-live-verify-20261001.eaqySt/text-authoring-current/replay-summary.json>) · [호출 transcript](</private/tmp/storyproduction-live-verify-20261001.eaqySt/text-authoring-current/logs/current-source-replay.jsonl>) |
| 교차 owner-chain | **PASS · synthetic QA** | `python3 integration/scripts/storyctl.py e2e`; 19 STS agent nodes, 10 simulated Human gates, 2 scenes, 5 shots, 10 asset requirements, 15,475-byte HTML. PNG/PSD는 1×1 fixture다. [결과](</private/tmp/storyproduction-live-verify-20261001.eaqySt/owner-chain-e2e-corrected-path.log>) |
| StoryProduction root integration | **FAIL · exit 1** | `bash integration/scripts/verify-local.sh`; Python integration tests 29/29와 앞선 contract/NVC/SMC 경계 검사를 통과한 뒤 SceneAssetCompiler `verify_skill.py`가 `scripts/scene_asset_compilerlib/asset_release_files.py`, `scripts/scene_asset_compilerlib/util.py`의 `MANIFEST_HASH` mismatch를 보고 중단했다. [로그](</private/tmp/storyproduction-live-verify-20261001.eaqySt/root-local-gate-final.log>) |
| SceneAssetCompiler source manifest | **FAIL** | SceneAssetCompiler 디렉터리에서 `python3 tools/source_manifest.py verify`를 실행하면 `verify_manifest()`가 정의되지 않은 `sha256_file`을 호출해 `NameError`를 낸다. root gate의 마지막 실패를 고친다는 뜻은 아니며, 별도로 관찰한 verifier 결함이다. [오류 로그](</private/tmp/storyproduction-live-verify-20261001.eaqySt/sac-source-manifest-verify.log>) |
| Geultu | **PARTIAL** | `cargo test --workspace --locked --offline`: **96/96 PASS**. `bash scripts/verify.sh`: **FAIL**; rustfmt가 `crates/geultu/src/store/history.rs`, `journal.rs`, `recovery.rs`에서 import/line-wrap 차이를 보고했다. fail-fast gate 뒤의 Clippy 등은 실행되지 않았다. [tests](</private/tmp/storyproduction-live-verify-20261001.eaqySt/geultu-tests-final.log>) · [공식 gate](</private/tmp/storyproduction-live-verify-20261001.eaqySt/geultu-official-gate.log>) |
| NarrativeCourier | **PASS** | `bash ./verify-all.sh --require-rust`: check/test/fmt/clippy 통과, Rust tests **73/73 PASS**. `python3 tools/source_manifest.py verify`: 512/512 entries, mismatch 0. [공식 gate](</private/tmp/storyproduction-live-verify-20261001.eaqySt/nvc-official-gate.log>) · [manifest](</private/tmp/storyproduction-live-verify-20261001.eaqySt/nvc-source-manifest-verify.log>) |
| SMC custody Rust | **PASS** | `cargo fmt --all -- --check` 통과. `cargo test --package smc-trackb-custody --offline`: unit 7 + protocol integration 25, 총 **32/32 PASS**. [tests](</private/tmp/storyproduction-live-verify-20261001.eaqySt/smc-custody-tests-final.log>) |
| SceneMotionCompiler | **PASS · bounded** | `bash scripts/verify.sh`: **10/10 PASS**, Node tests **218/218 PASS**, source identity 243/243 일치. `SOURCE_MANIFEST.json` tree hash는 위 identity다. Release smoke 출력은 `native: NOT_RUN_ENVIRONMENT`로 표시했다. [aggregate summary](</Users/ax/repository/StoryProduction/SceneMotionCompiler/.tmp/verify/summary.json>) · [Node tests](</private/tmp/storyproduction-live-verify-20261001.eaqySt/smc-node-tests-corrected-path.log>) · [custody rustfmt](</private/tmp/storyproduction-live-verify-20261001.eaqySt/smc-custody-rustfmt-final.log>) |

## 콘텐츠·운영 경계

텍스트 replay는 이 검증을 위해 준비한 synthetic QA 원문과 수동 QA 답안을 사용했다. 사용자의 실제 이야기가 제공되지 않아 문체·장편 확장·취향 적합성은 **NOT_RUN**이다. E2E의 Human gate는 시뮬레이션이며 실제 사람의 adoption이 아니다. package `complete`는 파일·hash·승인 기록의 기계적 결속이지 작품 품질 승인이 아니다.

실제 외부 writer/model, 실제 이미지 생성 provider와 pixel/PSD provenance, NVC renderer, SMC native helper·locked Chromium·실제 영상 release, SAC G09, UI에서의 사람 승인 lifecycle은 **NOT_RUN**이다. 이전 snapshot에서 수행했던 STEP 8D blocked negative replay는 [앞선 보고서](2026-10-01-live-use-validation-after-owner-cleanup.md)에 기록돼 있으며 이번 재검증에서는 다시 실행하지 않았다.
