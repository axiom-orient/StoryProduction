# promptGen → SAC 자산 인계

geultu → NVC의 원문·의미·사람 검토를 유지한다. IPC는 프롬프트와 provider 실행, SAC는 PNG 검사와 자산 검토, SMC는 렌더 후보를 소유한다. LayoutSheet는 필요할 때 기존 `.layoutpack`으로만 연결하는 독립 도구다.

## 실행과 인증

`image.prompt.compile`은 외부 호출 없이 프롬프트만 작성한다. `image.generate`는 `task_mode=generate`, 참조 없음, `gpt-image-2` PNG를 요구한다. `image.edit`는 `task_mode=edit`, `index=1, role=base`인 참조 하나를 요구하고 snapshot의 실제 경로를 `--reference-image`로 전달한다. capability/payload 불일치는 실행 디렉터리 예약 전에 거절한다.

Codex 생성은 선택한 실행 파일과 로그인 홈을 사용한다. 같은 홈으로 `codex login status`를 확인한다. Story는 다음 옵션을 IPC에 전달하며 자격 증명 파일을 작업 폴더에 복사하지 않는다. 편집 API는 기존 `OPENAI_API_KEY` 인증을 사용한다. ChatGPT 로그인이나 `--codex-home`이 API 편집 인증을 대신하지 않는다. IPC는 다른 mode에 지정된 Codex 옵션을 무시하지 않고 오류로 반환한다.

```sh
integration/target/release/storyctl run-promptgen \
  --capability-root /absolute/my-assets \
  --request /absolute/my-assets/request.json --output-dir runs/shot-1 \
  --promptgen-bin /absolute/StoryProduction/ImagePromptCompiler/target/release/promptgen \
  --codex-binary /absolute/codex --codex-home /absolute/login-home
```

API 편집·복수 참조·cutout·PSD를 지원하지 않는 provider에 우회 전달하지 않는다. 참조·편집·cutout은 기존 Codex imagegen 또는 사용자가 제공한 자산 경로를 사용하며, 로컬 이미지 API client나 segmentation 엔진을 추가하지 않는다.

## 현재 NVC packet에 결속

기존 `text … animatic packet`으로 사람 검토를 거친 연출 packet을 export한다. 이 packet의 `contentHash`가 `sha256:HASH`라면 각 image.generate 요청의 source는 다음과 같다. 이는 NVC content-hash domain이며 raw-file hash로 바꾸지 않는다.

```json
{"kind":"nvc.animatic-packet","identity":"packet.HASH","sha256":"sha256:HASH"}
```

각 자산의 IPC 요청은 그 자산 한 개의 요구 크기를 사용한다. `cinematic_storyboard`는 한 이미지에 여러 패널을 담는 별도 시트 프로필이다. 이 프로필의 subject count와 placement는 각 패널 안에 적용하고, 검증 receipt의 expected/observed는 모든 패널의 등장 수 합계, instance center는 전체 이미지 좌표로 기록한다. Structured 이미지·스티커의 count 의미는 그대로다.

이미지 생성 결과를 같은 asset root에 보관하고 bindings.json을 작성한다. 모든 요구 assetId를 정확히 한 번씩 지정한다.

```json
{"assets":[{"assetId":"shot-1","result":"runs/shot-1/capability-result.json"}]}
```

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic asset-draft --asset-root /absolute/my-assets \
  --board storyboards/boards/board.json --board-review storyboards/reviews/board.json \
  --plan animatic/plan.json --review animatic/plan-review.json \
  --bindings bindings.json --output animatic/assets-draft.json
```

`asset-draft`는 현재 geultu source와 NVC export를 관찰하고, 성공한 candidateOnly Codex 결과의 source, asset coverage, 요구 크기, PNG raw hash·길이, thread/call provenance를 대조한다. 현재 NVC packet과 입력 bytes를 발행 직전에 다시 읽는다. 입력·결과 경로는 선택된 asset directory에 묶이며 symlink·상위 경로 탈출을 거절한다. 출력은 기존 `sac.animatic-assets/1` draft이며 contentHash나 approval을 만들지 않는다.

이 자동 인계는 provenance executor가 확인된 Codex 생성 결과에 한정한다. OpenAI API edit의 executor를 Codex로 바꾸어 표기하지 않는다. 기존 사용자 자산·cutout의 SAC draft 경로는 계속 사용할 수 있다.

다음은 기존 `animatic assets submit`, 실제 자산 검토 후 `assets review`, `animatic render` 순서다. SAC는 PNG 구조·alpha·coverage를 실제로 검사하고, SMC는 파일·프레임·codec을 검사한다. 파일 해시 일치와 모델의 fidelity 판정은 사람의 미술·연출·release 승인이 아니다. `finalApproval:false`인 렌더 후보를 최종 배포 승인으로 표시하지 않는다.

실패·취소·outcome_unknown은 별도 terminal 상태로 보존한다. 같은 execution output을 덮어쓰거나 unknown 결과를 자동 재호출하지 않는다.
