# 2026-10-01 현재 소스 실사용 follow-up

## 판정

현재 소스로 빌드한 공개 owner-chain과 새 격리 workspace의 원문→스토리→시놉시스→NVC handoff 경로는 통과했다. 완성 package 재생성과 stale board 거부도 현재 integration 코드에서 확인했다. 전체 qualification은 여전히 **PARTIAL**이며, 실제 사용자 원고·실제 작성 모델·사람의 adoption은 검증하지 않았다.

StoryProduction root에 Git metadata가 없어 aggregate revision/commit SHA는 기록할 수 없다. 명령은 기존 Python 3.12.14 + `jsonschema 4.26.0` venv, Rust 1.97.1, Node 24.19.0으로 실행했다. Rust와 Node는 설치된 절대경로를 PATH에 넣었다. 사용자 전역 패키지 설치는 하지 않았다.

## 현재 소스 실행 결과

| 흐름 | 판정 | 관측 |
|---|---|---|
| 새 텍스트 workspace intake→스토리/시놉시스 후보 제출·QA 검토→시놉시스 export/handoff→NVC import | **PASS · synthetic QA** | 공개 `storyctl text` 16회가 모두 exit 0. Geultu source export bytes와 intake bytes가 동일했다. source/story/synopsis 모두 `current`; 두 후보의 QA review 결정은 `accepted`; NVC import `initialized`; 첫 STS node는 `step_00` `agent_task`. [요약](</private/tmp/storyproduction-live-verify-20261001.eaqySt/fresh-text-workflow/summary.json>) · [호출 출력](</private/tmp/storyproduction-live-verify-20261001.eaqySt/fresh-text-workflow/logs/>) |
| 교차 owner-chain | **PASS · synthetic QA** | `python3 integration/tests/story_pipeline_e2e.py`, exit 0. 현재 Geultu/NVC 소스로 debug CLI를 빌드하고 실행했다. Geultu→NarrativeCourier→SMC→SceneAssetCompiler→SMC, 19 NVC agent nodes, 10 simulated human gates, 2 scenes, 5 shots, 10 asset requirements, HTML 15,475 bytes. [E2E 결과](</private/tmp/storyproduction-live-verify-20261001.eaqySt/verification-evidence/owner-chain-followup.json>) |
| 현재 text status/package 공개 CLI | **PASS · synthetic QA** | `status`와 `package` 모두 exit 0. 앞선 full text replay에서 완성된 격리 QA workspace를 현재 Geultu/NVC 바이너리와 integration 코드로 다시 읽고 `package-current`에 export 및 동일 경로 재사용을 확인했다. status `complete`, 산출물 9개, manifest hash 9/9 일치, package source와 intake 원문 bytes 동일. [재실행·hash 결과](</private/tmp/storyproduction-live-verify-20261001.eaqySt/verification-evidence/package-current-check.json>) · [package manifest](</private/tmp/storyproduction-live-verify-20261001.eaqySt/text-authoring-current/authoring-workspace/package-current/story-package.json>) |
| stale board 거부 | **PASS · negative case** | QA workspace 복사본에서 board `contentHash`를 바꿔 package를 요청했다. CLI exit 2, structured stderr `not_applied`, 오류 `CONTENT_HASH:text-storyboard-v1`; package manifest는 생성되지 않았다. [호출 출력](</private/tmp/storyproduction-live-verify-20261001.eaqySt/verification-evidence/stale-board-rejection.json>) |

E2E 결과 JSON:

```json
{
  "status": "PASS",
  "agent_nodes": 19,
  "human_gates": 10,
  "scene_ids": ["SCN-001", "SCN-002"],
  "downstream": {
    "sceneAssetCompiler": {"assetCount": 10, "requirementCount": 10},
    "smcCompile": {"shotCount": 5, "outputBytes": 15475}
  }
}
```

E2E 명령의 exit code는 0이었다. text status/package와 stale board의 정확한 인자 및 응답은 위 evidence JSON에 저장했다. 새 text replay 입력은 `fresh-text-workflow/inputs/`에, 각 공개 호출의 stdout/stderr는 `fresh-text-workflow/logs/`에 있다.

```sh
PATH=/Users/ax/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin:/Users/ax/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PATH \
PYTHONDONTWRITEBYTECODE=1 \
/private/tmp/storyproduction-realuse-20260930.lpaWN8/venv/bin/python3 integration/tests/story_pipeline_e2e.py
```

## 미검증 범위

- 실제 사용자의 원고와 작가 취향에 대한 적합성: **NOT_RUN**. QA fixture만 사용했다.
- 실제 모델/provider 호출, 실제 사람의 review/adoption, 실제 이미지·PSD pixel provenance: **NOT_RUN**. review actor와 gate는 격리 시나리오용이다.
- 현재 소스 전체의 새 83-call text replay: **NOT_RUN**. 이번 새 workspace는 STS `step_00` 준비까지 검증했고, 완성 package CLI는 앞선 replay 산출물을 이용했다.
- root `integration/scripts/verify-local.sh`, component 전체 공식 gate, ImagePromptCompiler/OpenAI 호출, manifest freshness, SMC native/browser/release qualification: **NOT_RUN**.

package `complete`와 기계적 hash 일치는 구조·출처 결속 결과이며 작품 품질이나 실제 사람의 승인을 뜻하지 않는다.
