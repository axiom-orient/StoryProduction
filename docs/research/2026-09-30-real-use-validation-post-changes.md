# 2026-09-30 practical verification snapshot before import cleanups

## 판정

이 검증 실행은 **DONE**, 대상 제품 qualification은 **PARTIAL**이다. 변경된 NarrativeCourier gate,
root integration gate, 공개 component owner-chain, 그리고 현재 `storyctl text`의 원문 intake부터
reviewed storyboard와 package publication까지를 실행했다. 기계적 계약·hash·공개 경계는 통과했다.
실제 사용자의 원고, 외부 writer, 실제 사람의 창작 승인은 사용하지 않았다.

저장소에 Git metadata가 없어 commit revision은 없다. 실행 명령과 stdout은
`/private/tmp/storyproduction-validation-20260930-csPIRM/`에 보존했다. 테스트 원고와 승인 actor는
모두 격리 QA fixture다.

## 환경과 결과

`storyctl doctor`는 Python 3.12.14, `jsonschema` 4.26.0, Node 24.19.0, Cargo 1.97.1, Rust 1.97.1을
확인하고 `PASS`했다.

| 경계 | 판정 | 증거 |
|---|---|---|
| NarrativeCourier gate | **PASS** | `bash NarrativeCourier/verify-all.sh --require-rust`: 68 Rust tests, 60 contract checks, 112 Delivery/source checks, 14 STS audit tests, `cargo check`, fmt, Clippy. [로그](</private/tmp/storyproduction-validation-20260930-csPIRM/logs/nvc-verify-final2.log>) |
| NVC producer source manifest | **PASS** | Docs 반영 뒤 manifest를 생성하고 검증했다. 505/505 entries, 누락·추가·hash drift 없음. |
| Root integration gate | **PASS** | `bash integration/scripts/verify-local.sh`, exit 0. 29 Python integration tests, NVC contract/audit checks, SMC↔SAC boundary tests, SAC manifests, IPC source snapshot이 통과했다. [로그](</private/tmp/storyproduction-validation-20260930-csPIRM/logs/root-local-gate-final.log>) |
| 공개 owner-chain | **PASS · RUNTIME FIXTURE** | `python3 integration/scripts/storyctl.py e2e`, exit 0. `geultu → NVC → SMC → SAC → SMC`; 19 STS agent nodes, 10 simulated Human gates, 2 scenes, 5 shots, 10 asset requirements, 15,442-byte HTML을 만들었다. 이미지/PSD는 합성 1×1 fixture다. [로그](</private/tmp/storyproduction-validation-20260930-csPIRM/logs/storyctl-e2e-pass.log>) |
| 현재 `storyctl text` authoring/package path | **PASS · RUNTIME FIXTURE** | 새 workspace에서 공개 CLI로 source intake → brief → story/synopsis 후보와 hash-bound review → synopsis handoff → STS Completed(19 agent nodes, 10 simulated Human gates) → 2-scene/6-occurrence storyboard review → package까지 완료했다. package 10개 파일의 hash와 원문 569 bytes가 일치한다. [요약](</private/tmp/storyproduction-validation-20260930-csPIRM/text-authoring-current/summary-full.json>) · [초기 authoring transcript](</private/tmp/storyproduction-validation-20260930-csPIRM/text-authoring-current/storyctl-authoring.jsonl>) · [package/status recheck](</private/tmp/storyproduction-validation-20260930-csPIRM/text-authoring-current/package-recheck.jsonl>) · [package manifest](</private/tmp/storyproduction-validation-20260930-csPIRM/text-authoring-current/workspace/package/story-package.json>) |
| 이전 QA workspace 접근 | **EXPECTED FAIL-CLOSED** | 구 workflow bundle에서 만든 임시 workspace의 `storyctl text status`는 exit 2와 `outcome_unknown`을 반환했다. 현재 실행은 `domain bundle changed after project creation; mixed-domain execution is blocked`로 종료했다. 이는 옛 QA fixture의 재개/마이그레이션이 아니며 성공 상태로 처리하지 않았다. [응답](</private/tmp/storyproduction-validation-20260930-csPIRM/logs/text-status-stale.json>) |

검증 과정에서 Runtime cancel 경로의 중괄호와 명시 import 누락이 Rust build에 잡혔고, 다음 gate에서는 stale `step_08_reverse` fixture와 rustfmt/Clippy 위반이 드러났다. 코드를 고치고 fixture에 실제 `SCN-*` evidence와 alignment row를 넣은 뒤 NVC 전체 gate를 다시 통과했다. 검증 기준을 낮추지 않았다.

첫 package 실행의 QA runner는 상대경로 `package`를 넘겨 저장소 작업 디렉터리 아래에 산출물을 만들었다. 산출물의 `source.md`가 임시 원문과 byte-equal임을 확인한 후 전부 격리 workspace로 옮겼고, `storyctl text package`를 절대 scratch 경로로 다시 실행해 hash와 최종 status를 재확인했다. 저장소 루트에는 `package/`가 남아 있지 않다.

## 미확인 범위

실제 사용자 원고는 저장소에서 찾지 못해 새 QA 원고만 사용했다. 후보 review와 Human gate는 자동 시험 actor가 제출했으며 실제 사람 승인/adoption을 뜻하지 않는다. 외부 writer/model 응답, 실제 모델의 STEP 8D 준수, 이미지·영상 provider, 픽셀 품질, NVC renderer는 **NOT_RUN**이다. 이번 QA synopsis·대본의 정합성을 기계적 validator만으로 의미 품질 인증하지 않았다. 이전 82-call flow는 최신 source graph보다 앞선 snapshot이며, 그 합성 작품의 독립 fidelity 검토는 **FAIL**이었다. hash와 `package_status=complete`는 의미 충실도 승인이 아니다. 이전 실패의 상세는 [기존 current-source report](2026-09-30-real-use-validation-current.md)에 있다.
