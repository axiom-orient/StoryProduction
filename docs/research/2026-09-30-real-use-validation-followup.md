# 2026-09-30 최신 소스 실사용 검증 후속

## 판정

검증 수행은 **DONE**, 전체 제품 qualification은 **PARTIAL**이다. 검사 대상 소스 지문은 6,107개 파일, 77,190,136바이트, SHA-256 `58dface0f50a6399a700be522000cb77684541ba8261f8e8ec033a28a6821a5f`다. 저장소 루트에 `.git`이 없어 commit ID 대신 이 지문으로 대상을 식별했다. 테스트는 `/private/tmp/storyproduction-real-use-20260930.fN1pdV/source` 격리 복사본에서 실행했고, 복사 전후와 원본/복사본 지문이 일치했다. 빌드 산출물과 캐시는 지문에서 제외했다. 이 보고서와 구현 현황 문서는 검증 뒤 추가했다.

환경은 Darwin 27.0 arm64, Node 24.19.0, Python 3.12.14 / jsonschema 4.26.0, Rust 1.97.1, pnpm 11.19.0이다. `npm`은 설치되어 있지 않아 격리된 shim이 SMC의 `npm run`을 pnpm으로 전달했다. 실제 npm 실행 환경은 qualification하지 않았다.

## 현재 스냅샷 결과

| 경계 | 결과 | 실행 증거 |
|---|---|---|
| StoryProduction root | **PASS (exit 0)** | `bash integration/scripts/verify-local.sh`; 계약, Python integration, SMC↔SAC 경계, SAC skill/release manifests, IPC source-snapshot gate 통과. [최종 root gate 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/root-local-gate-final.log>) |
| geultu | **PASS (exit 0)** | `./scripts/verify.sh`; 96 Rust tests, fmt, Clippy, release build, CLI/MCP/Studio smoke 통과. [로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/geultu-verify.log>) |
| NarrativeCourier | **PASS (exit 0)** | `./verify-all.sh --require-rust`; 60 Rust tests와 계약·문서·UI·fmt·Clippy 통과. [로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/nvc-verify.log>) |
| SceneMotionCompiler | **PASS (exit 0)** | `npm run verify:aggregate`; 10/10 gates 및 227 Node tests, manifest·source integrity·Track B·CLI·Studio contract·release smoke 통과. [aggregate 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/smc-aggregate.log>) · [tests 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/source/SceneMotionCompiler/.tmp/verify/tests.log>) |
| SceneAssetCompiler | **PARTIAL** | publication-boundary 14/14 tests와 root의 125/100/247 manifest 검사 통과(exit 0). `python3 tools/verify_release.py`는 exit 1, `FAIL G09_SOURCE_EVIDENCE_MISSING:tests/fixtures/legacy-psd/PROVENANCE.json`으로 실제 provenance가 없는 PSD release를 거부했다. traceback 없이 typed failure로 끝났지만 release evidence 자체는 **FAIL**이다. [tests 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/sac-tests.log>) · [G09 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/sac-release-gate.log>) |
| ImagePromptCompiler | **PARTIAL** | 수정 후 Rust check/test/doc/quality/release build 통과. 공식 `./scripts/verify.sh`는 exit 1, `evals/dogfood/historical/1.4.0/image/image-evidence.json` 누락으로 image-evidence 감사에서 중단됐다. 별도 CLI와 loopback Web smoke, 문서 링크, catalog asset, representative image QA, catalog check, source-snapshot tests는 통과(exit 0). source verification receipt는 생성하지 않았다. [공식 gate 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/ipc-verify.log>) · [CLI smoke](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/ipc-cli-smoke.log>) · [Web smoke](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/ipc-web-smoke.log>) |

검증 중 이전 정리에서 놓친 두 실행 결함도 고쳤다. SAC CLI가 제거된 `validate_path_psd`를 여전히 import해 공개 CLI 초기화와 SMC asset-boundary tests를 깨뜨리고 있었다. 의존하던 미문서화 generic `psd-validate` command를 함께 제거하고 profile-bound validator 경로를 유지하자 root gate와 경계 테스트가 통과했다. ImagePromptCompiler는 중복 `profile` 이름을 정리한 뒤 남은 JSON 출력에서 미정의 변수를 참조하고 있었다. 직렬화가 원래 입력인 `raw_profile`을 사용하도록 고친 뒤 Rust gate와 CLI/Web smoke가 통과했다.

## 실사용 fixture 흐름

Geultu의 새 project는 snapshot과 journal envelope에 schema v2를 기록했다. legacy probe에서는 workspace metadata, project snapshot, journal을 모두 v1로 낮추고 v1 형태의 누락 필드도 반영했다. 공개 `storyctl text status`는 exit 2, `unsupported workspace schema 1`로 거부했고, 세 상태 파일의 SHA-256은 실행 전후 같았다. 기존 사용자 데이터에 자동 migration이나 rewrite가 없음을 확인했다. [probe 결과](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/geultu-v1-reject.json>).

새 합성 workspace에서 공개 CLI를 82회 호출해 원문 intake → story/synopsis 후보·검토 → hash-bound synopsis handoff → NVC STS → text storyboard → 9개 파일 package를 끝까지 수행했다. 의도적인 잘못된 read-only export probe는 exit 2 / `invalid_input`이었고 project revision은 바뀌지 않았다. 정상 작업 흐름은 완료됐으며, 18개 STS task, 10개 simulated gate, 3개 storyboard scene, 15개 semantic occurrence를 처리했다. manifest의 파일별 hash가 모두 맞고 `source.md`는 intake 원문과 byte-equal이었다. [실행 요약 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/text-authoring-current.log>) · [CLI transcript](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/text-authoring/logs/storyctl-authoring-transcript.jsonl>) · [package manifest](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/text-authoring/authoring-workspace/package/story-package.json>).

별도 공개 `python3 integration/tests/story_pipeline_e2e.py` owner-chain E2E도 exit 0, geultu → NVC → SMC → SAC → SMC까지 **PASS**했다. 19개 agent node, 10개 simulated gate, 2 scenes, 5 shots, 10 asset requirements, 14,323바이트 HTML을 만들었다. [owner-chain 로그](</private/tmp/storyproduction-real-use-20260930.fN1pdV/logs/followup/owner-chain-e2e.log>).

이 두 실행의 사람 결정은 QA fixture다. 콘텐츠 충실도는 **FAIL**이다. story는 원문에 없는 “어머니는 그 기록을 보여줄 때마다”를 추가했고, 9:10 출발과 9:40 **화재 신고 시각**만으로 “민준은 불이 나기 전에 역을 떠났다”고 단정했다. 원문은 발화 시각을 정하지 않는다. 따라서 기계적 `package_status=complete`, hash 일치, simulated review는 작품 품질 또는 사람 승인을 입증하지 않는다.

## 미실행 범위와 한계

실제 사용자 원고, 실제 사람의 창작/visual 승인, 외부 writer·image·video provider 호출, 실제 생성 이미지의 시각 품질, PSD provenance/postflight는 확인하지 않았다. locked Chromium·packaged native helper·NVC desktop UI의 실사용도 이 실행 범위에 없다. 18-stage/144-work-item catalog는 runtime 기능이 아니라 target specification으로 남아 있다.
