# 2026-09-30 현재 소스 실사용 검증 — owner 경로 clean break 후

## 판정

검증 수행은 **DONE**이다. 현재 소스의 root·Geultu·NarrativeCourier·SceneMotionCompiler gate와 두 공개 제작 흐름은 기계적으로 통과했다. 텍스트 패키지는 원문 bytes와 파일 hash가 맞고 `complete`로 발행됐다. 하지만 합성 작품의 독립 의미 대조는 **FAIL**이며, STEP 8D가 근거 정렬 없는 `READY`를 반환해도 최종 release가 통과하는 제품 결함이 확인됐다. 제품 전체 qualification은 **PARTIAL**이다.

## 대상과 환경

저장소에 `.git` metadata가 없어 실행 트리를 파일 지문으로 식별했다. gate 실행 직전·직후 지문은 동일했다: **6,121 files / 77,388,439 bytes / `4ea99d7c3ceae114efdfb2df04b793ad070599bf8f9d684905dcbb638e912337`**. 지문은 정렬된 상대 경로와 파일 bytes의 SHA-256이며 `target`, `node_modules`, `.tmp`, `__pycache__`, `.DS_Store`, `.pyc` 및 symlink는 제외한다. 이 보고서와 root README/구현 현황의 결과 요약은 fingerprint 대조 뒤 추가했다. 제품 코드와 설정은 대조 대상에서 실행 전후 그대로였다.

Darwin arm64, Node.js 24.19.0, Python 3.12.14 / `jsonschema==4.26.0`, Rust 1.97.1, pnpm 11.19.0을 사용했다. 시스템 기본 셸 PATH에는 Node/Rust가 없어 Codex 번들 런타임 경로를 명시했다. Python 의존성은 기존 격리 설치 경로를 사용했다. 제품 소스와 설정은 수정하지 않았고 build/test 산출물은 component `target` 및 `SceneMotionCompiler/.tmp/verify`에만 생겼다.

## 실행 결과

| 경계 | 판정 | 관측 |
|---|---|---|
| Root integration | **PASS · exit 0** | `bash integration/scripts/verify-local.sh`; contract pin, integration test, NVC audit, SMC↔SAC boundary, SAC manifests, IPC source-snapshot 검사를 완료했다. [로그](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-root-local-gate-final.log>) |
| Geultu | **PASS · exit 0** | `bash geultu/scripts/verify.sh`; format, Clippy, Rust tests, release build, CLI/MCP/Studio smoke를 통과했다. [로그](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-geultu-verify.log>) |
| NarrativeCourier | **PASS · exit 0** | `bash NarrativeCourier/verify-all.sh --require-rust`; 계약·STS audit, UI/source checks, Rust check/test/fmt/Clippy를 통과했다. Source manifest verify도 **504/504 entries PASS**, manifest SHA-256 `833b7f1881c537a21099ff582fa8a4067feb54fd48ba7858dd5a7c6b2e5b36bf`다. [gate 로그](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-nvc-verify.log>) · [manifest 로그](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-nvc-source-manifest.log>) |
| SceneMotionCompiler | **PASS · 10/10 gates** | Node suite **227/227 PASS**, source manifest와 source-integrity, Track B, CLI, Studio contract, release gates 통과. 첫 병렬 aggregate 호출은 Python child의 `jsonschema` 경로 누락과 브라우저 종료 테스트의 일회성 실패로 멈췄다. 의존성 경로를 고친 전체 재실행 및 aggregate는 통과했고 브라우저 테스트 실패는 재현되지 않았다. [Node suite](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-smc-node-tests-env-corrected.log>) · [aggregate](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-smc-aggregate-env-corrected.log>) |
| 공개 owner-chain | **PASS · RUNTIME FIXTURE** | 최신 CLI 빌드로 `geultu → NarrativeCourier → SMC → SceneAssetCompiler → SMC`를 실행했다. 19 STS 작성 node, 10 simulated gate, 2 scenes, 5 shots, 10 asset requirements, 14,407-byte HTML을 만들었다. 입력 PNG/PSD는 1×1 fixture다. [로그](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs-owner-chain-e2e.log>) |
| `storyctl text` 원문→package | **PASS · RUNTIME FIXTURE** | 새 임시 workspace에서 82회 공개 CLI 호출로 intake → story/synopsis 후보·모의 검토 → hash-bound handoff → NVC STS → 3-scene text storyboard → 9-file package를 완료했다. 15 semantic occurrences와 10개 모의 STS gate를 처리했다. package manifest hash가 전부 일치했고 `source.md` bytes는 intake 원문과 같았다. 잘못된 export는 exit 2 / `invalid_input`으로 거절됐으며 project revision은 유지됐다. [setup 요약](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs/text-authoring-setup-summary.json>) · [작업 요약](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/authoring-summary.json>) · [전체 transcript](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/logs/storyctl-authoring-transcript.jsonl>) · [package manifest](</private/tmp/storyproduction-live-current-20260930.ZPbiOt/authoring-workspace/package/story-package.json>) |

## 창작 충실도 — FAIL

합성 원문에서 `9:40`은 화재 **신고 시각**이다. 생성 스토리는 “민준은 불이 나기 전에 역을 떠났다”고 단정하고, 아버지가 “그때 병원에 있었다”고 확대했다. 원문은 발화 시각과 응급실 체류 시각을 말하지 않는다. 또한 “어머니는 그 기록을 보여줄 때마다”라는 반복 대면을 만들어 원문에 없는 장면을 추가했다.

Screenplay는 어머니가 민준이 불을 냈다고 믿는 핵심 갈등을 대사나 화면에 표현하지 않는다. 따라서 원문 없이 대본만 읽는 관객은 그 믿음과 지우의 대응을 복원할 수 없다. 텍스트 스토리보드와 package hash는 구조적으로 유효하지만 이 결손을 고치지 않는다.

현재 STEP 8D prompt는 모든 material SRC/REL에 대해 복원 Screenplay IR ID와 SCN 근거를 담은 alignment row를 요구한다. 그런데 실행된 `step_08_verification.md`에는 그 표나 `OBS-*`/`OREL-*` 인용이 없고, 근거 없는 “No hard failure remains”와 `STS_STATUS: READY`만 있다. 공개 workflow가 이 응답을 받아 최종 release를 진행했고 package가 `complete`가 됐다. 이 fixture는 고정 작성·검토 응답을 사용했으므로 실제 LLM의 오류율을 검증한 것은 아니지만, 현재 runtime이 명백히 미준수인 `READY` 출력을 차단하지 않는다는 관측은 재현했다.

## 범위와 미확인

원고 `빛이 남은 역`은 합성 QA fixture다. 스토리/시놉시스 작성물, NVC task 응답, 작가·리뷰·Human gate 결정은 모두 fixture 또는 모의 actor이며 실제 모델, 실제 사용자 원고, 실제 사람의 승인/adoption을 사용하지 않았다. 18단계/144 세부 작업 카탈로그는 여전히 target specification이며 runtime에 등록되지 않았다. 외부 writer/image/video provider, 실제 생성 픽셀 품질, NVC renderer, SAC G09 provenance release, IPC historical image evidence release qualification은 **NOT_RUN**이다.

따라서 현재 증거는 **기술적 공개 경로 PASS / 합성 작품 fidelity FAIL / 전체 제품 qualification PARTIAL**이다. 다음 수정 우선순위는 STEP 8D의 alignment를 실제 recovered IR·scene evidence에 결속하고, 필수 row가 빠졌거나 protected obligation이 대본에서 복원되지 않으면 `READY`와 final release를 차단하는 것이다.
