# 2026-09-30 current-source real-use validation

## 판정

이 검증 실행은 **DONE**이며 대상 제품 qualification은 **PARTIAL**이다. 격리 사본에서 root integration gate, SMC Node suite와 aggregate, 두 public runtime flow를 실행했다. 기계적 경계는 통과했지만 합성 작품의 충실도는 **FAIL**이다. 이 기록은 아래 검증 지문에 해당하며, 마지막 production contract/I/O ownership clean break의 검증 증거가 아니다.

## 대상과 환경

Git metadata가 없어 실행 전 소스 fingerprint로 대상을 식별했다. 생성물인 `target`, `.tmp`, `node_modules`, Python cache를 제외한 6,123개 파일 / 77,328,238바이트, 경로와 파일 SHA-256을 결합한 fingerprint는 `1b822a07cdce186a13f8597c482ac38d3e7b612dab01e250ecb6c9c23882a45b`이다. 격리 사본 생성 직후 fingerprint가 일치했고, 원본은 검증 뒤에도 같은 fingerprint를 유지했다. 실행 사본은 `/private/tmp/storyproduction-current-real-use-aphvm6ys/source`이다.

환경은 Darwin arm64, Node 24.19.0, Python 3.12.14 / `jsonschema==4.26.0`, Rust 1.97.1, pnpm 11.19.0, ffmpeg/ffprobe 9.0.1이다. native `npm`이 없어 SMC aggregate의 `npm` 호출만 임시 npm-to-pnpm shim으로 전달했다. SMC manifest는 격리 사본에서 생성·검증했다. 원본 파일은 변경하지 않았다.

## 기준별 결과

| 기준 | 결과 | 증거 |
|---|---|---|
| StoryProduction local gate | **PASS · exit 0** | [로그](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/root-local-gate.log). 29개 root integration tests와 14개 SAC tests를 포함한다. |
| SMC 전체 Node suite | **PASS · 227/227** | [로그](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/smc-node-tests.log). |
| SMC aggregate | **PASS · 10/10 gates** | [로그](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/smc-aggregate.log), [summary](/private/tmp/storyproduction-current-real-use-aphvm6ys/source/SceneMotionCompiler/.tmp/verify/summary.json). 격리 SMC tree: 244 files, `sha256:ceb9b5272594d3dfea2e38e8492943a9de97e0200e007912ddbefe7920ad1309`. |
| public owner-chain | **PASS · RUNTIME FIXTURE** | geultu → NarrativeCourier → SMC → SAC → SMC. 19 agent nodes, 10 simulated gates, 2 scenes, 5 shots, 10 asset requirements, 14,407-byte HTML. PNG/PSD는 1×1 fixture다. [로그](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/owner-chain-e2e.log). |
| `storyctl text` 원문→package | **PASS · RUNTIME FIXTURE** | 82회 CLI 호출, 18 STS agent tasks, 10 simulated gates, 3 storyboard scenes, 15 semantic occurrences, 9개 package files. `source.md` bytes와 package hashes가 일치했다. [setup summary](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/text-authoring-setup-summary.json), [authoring summary](/private/tmp/storyproduction-current-real-use-aphvm6ys/authoring-summary.json), [transcript](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/storyctl-authoring-transcript.jsonl), [package manifest](/private/tmp/storyproduction-current-real-use-aphvm6ys/authoring-workspace/package/story-package.json). 잘못된 export는 exit 2 / `invalid_input`; NameError와 project revision 변화는 없었다. |
| 합성 작품 의미 충실도 | **FAIL · REVIEW** | 스토리는 반복 대면(“그 기록을 보여줄 때마다”)을 새로 만들고, 9:10 출발과 9:40 신고 시각만으로 “불이 나기 전에” 떠났다고 단정한다. 원문은 발화 시각을 주지 않는다. 대본은 어머니의 핵심 믿음/갈등을 전달하지 않는다. [원문](/private/tmp/storyproduction-current-real-use-aphvm6ys/authoring-workspace/package/source.md), [story](/private/tmp/storyproduction-current-real-use-aphvm6ys/authoring-workspace/package/story.md), [screenplay](/private/tmp/storyproduction-current-real-use-aphvm6ys/authoring-workspace/package/screenplay.md). `complete`, `READY`, hash, simulated approval은 작품 품질이나 사람의 채택을 증명하지 않는다. |
| 검증 대상 원본의 SMC manifest | **FAIL · exit 1** | [로그](/private/tmp/storyproduction-current-real-use-aphvm6ys/logs/source-manifest-original.log). 그 시점 원본 manifest는 239 files / 2,587,237 bytes를 선언했으나 소스는 244 files / 2,595,643 bytes였다. 격리 사본에서 갱신한 manifest만 검증 PASS했다. 이후 metadata 생성 및 verification 상태는 위 구현 현황을 따른다. |

## 범위 제한

실제 사용자 원고가 없어서 fixture만 사용했다. Writer/provider 호출, 실제 사람의 승인·adoption, 생성 image/video 품질, locked Chromium, packaged native helper와 Studio release는 확인하지 않았다. 모든 review actor는 시뮬레이션이다.

검증 후 SMC production contract guards/sealing을 I/O에서 contract owner로 옮겼고 buffered raw-byte hash는 core의 `sha256Prefixed`를 사용한다. I/O는 대용량 media의 bounded streaming digest를 유지한다. Application의 forwarding-only release runtime 파일과 derived `TRACK_B_HANDLERS` map도 삭제하고 callers/tests는 canonical operation/registry owner를 사용한다. 이 후속 변경들은 **NOT_RUN**이며 위 PASS 결과를 현재 source로 확장하지 않는다.

Integration의 `text_package.py`도 output root 중복 path check를 제거하고 각 `immutable` leaf write의 path admission만 유지하도록 정리했다. 이 refactor의 CLI/package effect는 **NOT_RUN**이다.

그 뒤 production contract/I/O ownership, application facade deletion, handler-map clean break를 모두
포함해 `SOURCE_MANIFEST.json` metadata를 243 files / 2,601,260 bytes /
`sha256:900bd62dfc57d493b185d4b71452f268293ded23bfd4eeaf5c49eb6e13c376a2`로 다시 생성했다. 이 생성
결과는 freshness 검증이 아니다. `verify-source-tree.mjs`는 이 current source에 대해 실행하지 않았다.

또한 STS STEP 8D의 alignment row는 독립 Screenplay IR `OBS/OREL`과 해당 장면 evidence를 인용해야 하며,
source-only memory는 대본 출현 근거가 될 수 없도록 prompt/principle을 강화했다. 이 NVC prompt change도
**NOT_RUN**이다. NVC source manifest metadata는 504 entries / `sha256:a87ce44b601dfe84e52dc7b8f24b7db6e23b8b8a69ec1300d3e31ae02d9c6055`로 재생성했지만 verification은 실행하지 않았다.
