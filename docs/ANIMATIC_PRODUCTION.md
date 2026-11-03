# 움직이는 콘티 운영

2026-10-02 제품 계약, 2026-10-03 native 실행 계층 전환. 승인된 텍스트 콘티에서 연출 계획과 실제 PNG 재료를 연결해
**2.5D 합성 원본 + VP9 WebM 후보**를 만든다. 서사·그림·렌더의 owner는 각각
NVC·SAC·SMC다. 기존 원문 → 스토리 → 시놉시스 → 씬 → 텍스트 콘티 제작은
미디어 도구 없이 그대로 실행된다.

## 실행 흐름과 authority

```text
현재 STS release + 승인 text board
  → NVC source → agent/user direction draft → NVC submit(candidate)
  → 사람의 exact-hash direction review → NVC export(packet)
  → SAC 실제 PNG 검사 → asset candidate → 사람의 exact-hash asset review
  → integration 현재 원문/packet 재확인
  → SMC attempt claim → pinned Chromium seekFrame(n) → PNG sequence
  → pinned FFmpeg VP9 → ffprobe 실제 frame/codec/canvas/fps 검사
  → 입력 재확인 → immutable candidate receipt
  → integration 현재 NVC packet 재확인 → 사용자에게 후보 표시
```

렌더 결과에는 `finalApproval: false`가 고정돼 있다. 렌더 완료는 Human
contract/visual/IR/release 승인이나 기존 Track B DeliverySet promotion이 아니다.
현재 이 새 lane에는 최종 release adoption 명령을 추가하지 않았다.

## 준비

native storyctl, NVC CLI, `sac-animatic`, `smc-animatic`, 시스템
Chromium, libvpx-vp9를 포함한 FFmpeg와 ffprobe가 필요하다. Python·Node는 이 공개 lane의 실행 전제가 아니다. 추가 브라우저를
다운로드하거나 Puppeteer/Playwright/Remotion 의존을 설치하지 않는다.

- Chromium/font: 기존 SMC renderer lock의 path·raw hash·version·OS·architecture.
- `SMC_FONT_PATH`를 지정하면 그 파일을 사용한다. 잘못된 명시 설정을 다른 폰트로
  대체하지 않는다. 공유할 합성 원본에는 사용자가 선택한 폰트의 사용 조건도 따른다.
- Encoder/probe: `SMC_FFMPEG_BIN` / `SMC_FFPROBE_BIN`, 기본은 PATH의 실행 파일.
  실제 경로·raw hash·version을 attempt에 기록한다.
- 데이터 설정: 연출 계획의 fps·canvas·각 panel의 duration/keyframes.
- 재생 설정: `--motion full|reduced`. reduced에서는 연속 이동을 첫 transform으로
  고정하고 컷·포즈·대사 정보는 보존한다.
- 제한 정책: NVC의 10분 direction limit, SMC `ANIMATIC_POLICY`의 실행 시간,
  이미지·프레임 수·저장량, 인코딩 품질. owner가 달라 서로의 상태를 소유하지 않는다.

## 사용 순서

아래 절대 경로는 실제 사용자 workspace와 자산 작업 공간으로 바꾼다.
`storyctl text` 공통 옵션은 `animatic` 앞에 둔다. 각 component의 public CLI도
직접 사용할 수 있지만, 현재 글투/시놉시스와 NVC source의 교차 확인은
integration 경로에서 수행한다.

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic direction source --board storyboards/board.json \
  --board-review storyboards/review.json
```

이 응답의 `source`, `board.contentHash`, `boardReview.contentHash`, 원문 occurrence를
사용해 agent와 사용자가 draft를 작성한다. 원고나 이미지를 임의로 테스트 데이터로
채우지 않는다. [연출 작성 지침](../NarrativeCourier/tools/animatic-craft.md)과
NVC `contracts/animatic-plan-v1.schema.json`이 실제 입력 계약이다.

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic direction submit --board storyboards/board.json \
  --board-review storyboards/review.json --draft direction-draft.json \
  --output animatic/plan-r1.json
```

draft는 NVC project root 아래의 파일이다. 새 revision은 새 output 경로를 사용한다.
원본 board v1을 변경하거나 현재 scene/duration을 추측하지 않는다. 계획은 해당
board의 panel 순서와 coverage를 그대로 따른다. `captions`에는 해당 panel의 원문
semantic ID만 넣으며, 실제 문자열은 NVC가 현재 scene-map에서 export한다.

계획을 읽고 사용자가 선택한 뒤 `direction review`를 호출한다. review에는
`--plan`, `--board`, `--board-review`, `--expected-hash`, `--reviewer`,
`--decision approved|rejected`, `--output`이 필요하다. reviewer 문자열은 인증이나
사용자 동의를 만들어 주지 않는다. openQuestions가 남아 있으면 approved를 거절한다.

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic packet --board storyboards/board.json --board-review storyboards/review.json \
  --plan animatic/plan-r1.json --review animatic/plan-r1-review.json \
  --asset-root /absolute/my-assets
```

응답의 packet 경로는 SAC/SMC root에 복사한 불변 producer export다. 자산 draft는
`sac.animatic-assets/1`, `packetHash`, `assets`를 포함한다. 각 asset은 요구된
assetId·width·height·alpha, 실제 PNG의 상대 path·raw sha256, provenance
`{executor, source}`를 갖는다. `contentHash`는 submit 때 owner가 추가한다.

실제 재료는 사용자 파일, Codex imagegen 또는 허용된 Scene Provider v2에서
준비한다. references/편집/cutout은 imagegen 또는 사용자가 제공한 cutout을 쓴다.
Scene Provider v2는 reference-free opaque PNG만 받는다. 이 코드가 이미지 API,
matting, PSD 조립을 대신 실행하지 않는다. provenance는 선언이며 외부 실행의
cryptographic attestation이 아니다.

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic assets --asset-root /absolute/my-assets submit \
  --packet animatic/packets/PACKET_HASH.json --draft assets-draft.json \
  --output animatic/assets-r1.json
```

SAC는 실제 PNG 구조·CRC·dimensions·alpha·raw hash·coverage를 검사한다.
단순히 RGBA 형식인 것만으로 cutout이 아니다. 전체 투명 PNG도 거절한다.
자산을 사람이 검토한 후 같은 `assets` 경로의 `review`에 pack, packet,
expected-hash, reviewer, decision, output을 지정한다. approval 전 렌더는 거절된다.

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic render --board storyboards/board.json --board-review storyboards/review.json \
  --plan animatic/plan-r1.json --review animatic/plan-r1-review.json \
  --asset-root /absolute/my-assets --assets animatic/assets-r1.json \
  --asset-review animatic/assets-r1-review.json --attempt take-01 --motion full
```

## 결과와 수정

`/absolute/my-assets/animatic/renders/take-01/`에 저장한다.

| 파일 | 의미 |
|---|---|
| preview.webm | opaque VP9 후보. 실제 frame count/canvas/fps 검사 후 publish |
| composition.html | PNG·font·원문 자막을 포함한 자급식 합성 원본 |
| captions.vtt | 원문 문자열과 panel timing을 가진 별도 자막 |
| source.json | NVC packet, SAC pack, SAC review의 정확한 snapshot |
| request.json | 입력 hash, 실행 worker identity, browser/font/encoder/probe 및 정책 |
| result.json | 위 출력의 raw hash를 가진 terminal receipt |
| frames/ | 캡처한 프레임. 오류·중단 조사 자료 |

HTML은 제품 화면이 아니라 media composition이다. 호스트에서
`await animaticReady`, `animatic.seekFrame(n)`, `animatic.play()`,
`animatic.pause()`로 조작한다. 자동 재생하지 않는다. native `<video controls>`로
WebM을 재생할 수 있다. 자막은 별도 VTT와 합성 영상에 보존되며 canvas의 텍스트
접근성 이름도 갱신한다. 자막이 화면에 들어가지 않으면 잘라 내지 않고 실패한다.

카메라/레이어/포즈/자막을 수정하려면 원래 JSON을 새 draft로 작성해 새 revision을
submit하고 정확한 새 hash를 검토한다. 영향을 받지 않은 panel은 그대로 유지한다.
packet이 바뀌면 SAC pack/review를 새 packet에 결속하고 새 attempt를 쓴다.
재료는 stable assetId로 요구하고 pack은 packetHash로 결속하므로 계획과 자산
사이에 순환 hash 의존이 없다. 현재 부분 프레임 cache를 도입하지 않았다.

## 취소·실패·복구

```sh
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic cancel --asset-root /absolute/my-assets --attempt take-01
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic status --asset-root /absolute/my-assets --attempt take-01
integration/target/release/storyctl text --workspace /absolute/my-story \
  animatic recover --asset-root /absolute/my-assets --attempt take-01
```

첫 terminal receipt가 승리한다. 취소와 candidate commit은 같은 atomic
no-overwrite publication을 사용한다. 취소가 먼저면 late worker가 성공으로
바꿀 수 없다. worker는 캡처 경계와 인코딩 중 polling으로 취소를 관측하며
FFmpeg를 중단하고 Chromium을 정리한다. SIGINT/SIGTERM도 처리한다.

동일 attempt의 같은 입력 재호출은 기존 상태를 반환하고 재실행하지 않는다.
다른 입력으로 같은 ID를 쓰면 거절한다. 실패한 작업을 고쳤다면 새 attempt ID로
명시적으로 실행한다. `outcome_unknown`은 자동 retry하지 않는다. 강제 kill 후에는
부분 파일과 해당 실행의 orphan process를 확인·정리한 후 새 attempt를 선택한다.
recover는 기존 결과를 검증하거나 unknown을 고정하며 렌더를 다시 시작하지 않는다.
PID 재사용을 running으로 오인하지 않도록 프로세스 시작 identity도 비교한다.

renderer의 current input과 integration의 live NVC source를 별도로 확인한다.
원문이 바뀌면 기존 파일은 보존하되 `source_stale`을 반환한다. status는 과거
candidate 기록 조회이며 현재 작품의 채택 가능성을 보증하지 않는다.

Story의 렌더 응답은 `story.animatic-observation/1`이다. 원래 SMC receipt는
`render` 안에 그대로 보존한다. `sourceStatus`는 current/changed/unobserved이며
조회 실패를 원문 변경으로 단정하지 않는다. 확인하지 못하면 outcome_unknown이다.
integration 정보로 producer의 contentHash 본문을 확장하거나 다시 seal하지 않는다.

실제 소리 합성은 이 lane에 없다. `audio: none`을 명시하고 원래 soundNotes는
source packet에 보존한다. 나머지 제약·실행 근거는
[개발 기록과 UNKNOWN](research/2026-10-02-animatic-implementation.md)에 있다.
