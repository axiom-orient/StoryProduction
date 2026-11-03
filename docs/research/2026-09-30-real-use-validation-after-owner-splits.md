# 2026-09-30 실사용 검증 — owner 분리 이후 현재 소스

## 판정

검증 실행은 완료했고 전체 제품 qualification은 **PARTIAL**이다. 공개 제작 경로와 SMC 전체 게이트는 통과했지만, ImagePromptCompiler의 Rust 빌드와 NarrativeCourier의 형식 검사가 실패했다. SceneAssetCompiler의 G09는 provenance가 없는 release fixture를 거부했다. 텍스트 제작 경로는 기계적으로 완료됐으나 최종 대본의 원문 충실도는 **FAIL**이다.

제품 코드나 계약은 이 검증에서 수정하지 않았다. root에는 Git metadata가 없어 commit ID 대신 component source identity로 실행 대상을 식별한다.

## 소스 identity와 환경

- NarrativeCourier: manifest **507/507**, SHA-256 `2167e00d925b4cc2e89a5109a340f8fc32bcacd87d37c43cfcc4ba22623e842`.
- SceneMotionCompiler: manifest **242/242**, tree hash `sha256:9a9497125bafc1b8afec1d13db2ca7eb5543f20ed89bcb43d0160cce4850b409`.
- ImagePromptCompiler: revision `promptgen-1.5.0-20260810.1`, **279 files / 17,382,009 bytes**, tree hash `ef5eb034794ef64933e7ec26eea9d9f3309c818d78fbeaf7edfb7f893a5693d2`.
- SceneAssetCompiler root gate가 release/skill/docs manifests **247/125/100**을 확인했다.
- 환경: macOS 27.0 / Darwin arm64, Python 3.12.14 + jsonschema 4.26.0(격리 venv), Node 24.19.0, Rust/Cargo 1.97.1, pnpm 11.19.0. `npm`과 `magick`은 설치되어 있지 않다. SMC aggregate는 임시 `npm → pnpm` shim을 통해 package scripts를 실행했다.

## 게이트 결과

| 경계 | 결과 | 근거 |
|---|---|---|
| Root integration | **PASS · exit 0** | `bash integration/scripts/verify-local.sh`; Python integration, NVC/SMC↔SAC 경계, SAC manifests, IPC source-snapshot 검사를 통과했다. [로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-root-integration.log>) |
| Geultu | **PASS · exit 0** | `bash geultu/scripts/verify.sh`; 96 Rust tests, 146 canonical checks, release build와 CLI/MCP/Studio smoke 통과. [로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-geultu-verify-final.log>) |
| NarrativeCourier | **PARTIAL · gate exit 1** | `bash NarrativeCourier/verify-all.sh --require-rust`에서 68 Rust tests와 앞선 source/contract/UI checks는 통과했다. 마지막 `cargo fmt --all -- --check`가 `delivery/src/engine/run.rs`, `sts/src/codex/workflow/lease.rs`, `sts/src/codex/workflow.rs`의 정렬 차이로 실패했다. 별도 `cargo clippy --workspace --all-targets --offline -- -D warnings`는 **PASS · exit 0**. producer manifest freshness도 **PASS · 507/507**. [gate 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-nvc-verify-final2.log>) · [Clippy 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-nvc-clippy.log>) · [manifest 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-nvc-manifest-verify.log>) |
| SceneMotionCompiler | **PASS · 10/10 gates** | aggregate source identity PASS, Node tests **227/227**, Track B/CLI/Studio/release gates 모두 통과. [aggregate 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-smc-aggregate-final.log>) |
| SceneAssetCompiler | **PARTIAL** | publication-boundary tests **14/14 PASS**. `python3 tools/verify_release.py`는 `G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`으로 종료해 provenance 없는 fixture의 release를 차단했다. [tests 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-sac-tests.log>) · [G09 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-sac-release-gate-final.log>) |
| ImagePromptCompiler | **FAIL · official gate exit 101** | 격리 사본의 `bash scripts/verify.sh`가 `cargo check`에서 중단됐다. [questions.rs](../../ImagePromptCompiler/crates/promptgen-core/src/interview/image/route_decisions/category_contracts/questions.rs)의 5행 `CategoryContractSpec`, 7행 `parse_subjects` 미사용 import 두 개가 `-D warnings`에 걸렸다. 별도 문서 링크(74), catalog, representative QA, prompt check, Rust structure, source-snapshot helper, syntax 검사들은 통과했다. 역사적 `evals/dogfood/historical/1.4.0/image/image-evidence.json`도 없어 image-evidence 감사가 별도로 실패한다. 전체 gate는 컴파일 실패 뒤의 tests/doc/build/smoke 및 source receipt 단계까지 도달하지 않았다. [official gate 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-ipc-verify.log>) · [독립 검사 로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-ipc-doc-links.log>) |

## 공개 사용자 흐름

- `integration/tests/story_pipeline_e2e.py`가 새 임시 workspace에서 `geultu → NarrativeCourier → SMC → SceneAssetCompiler → SMC`를 완료했다. **19 agent nodes**, **10 simulated Human gates**, 10 asset requirements, 5 shots, 15,442-byte HTML 결과를 확인했다. 입력 PNG/PSD는 1×1 합성 fixture다. **PASS · exit 0**. [로그](</private/tmp/storyproduction-realuse-20260930.lpaWN8/logs-owner-chain-e2e.log>)
- 새 `storyctl text` workspace가 intake → story/synopsis 후보와 hash-bound review → synopsis handoff → STS → 텍스트 storyboard review → package까지 완료했다. **19 STS agent nodes**, 10 simulated approvals, 2 scenes, 4 semantic occurrences를 거쳤다. source.md는 입력 원문 569 bytes와 byte-equal이고 manifest-listed 9 artifacts의 hash가 맞으며 package status는 `complete`였다. **기계적 흐름 PASS · exit 0**. [요약](</private/tmp/storyproduction-realuse-20260930.lpaWN8/text-authoring/summary.json>) · [대본](</private/tmp/storyproduction-realuse-20260930.lpaWN8/text-authoring/package/screenplay.md>) · [CLI transcript](</private/tmp/storyproduction-realuse-20260930.lpaWN8/text-authoring/storyctl-text-transcript.jsonl>)
- 독립 충실도 대조는 **FAIL**이다. 원문과 story/synopsis에는 어머니가 민준을 화재 원인으로 믿는다는 점, 기록 변경자와 화재 원인이 밝혀지지 않았다는 점이 있다. 최종 screenplay는 세 사실을 모두 담지 않았다. Source IR에는 세 개의 SRC assertion만 작성됐고 위 정보가 빠졌으며, STEP 8D 검증 표도 그 세 ID만 PRESERVED로 확인했다. 따라서 구조적으로 유효한 `READY`와 hash-complete package가 원문 전체의 의미 보존을 보장하지 못했다.
- 현 소스 STS Codex 실행도 local fake executable로 시험했다. 실행 중 취소는 명시적 cancel로 돌아오고 step_00을 pending 상태로 남겼으며 artifact 없이 두 marker를 정리했다. 별도 성공 실행은 `run_one_codex → submit_with_lock`을 거쳐 state와 artifact hash를 일치시켰다. 두 경로 모두 **PASS**이며 외부 계정/API는 사용하지 않았다. [취소 요약](</private/tmp/storyproduction-realuse-20260930.lpaWN8/codex-cancel-current/summary.json>) · [성공 요약](</private/tmp/storyproduction-realuse-20260930.lpaWN8/codex-cancel-current/success-summary.json>)

실제 사용자 원고, 실제 사람의 창작 승인/adoption, 외부 writer/image/video provider, 생성 이미지의 픽셀 품질은 **NOT_RUN**이다. `magick`이 없어 NarrativeCourier ImageMagick renderer도 **NOT_RUN**이다. Candidate/package의 기계적 완료는 작품 승인을 뜻하지 않는다.
