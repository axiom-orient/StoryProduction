# StoryProduction — 움직이는 콘티 도입 명세

상태: **초기 실행 범위 구현, 후속 표현은 Target**. revision: 2026-10-02.
근거: [최신 조사와 방식 비교](research/2026-10-01-animated-storyboards.md).
아래는 2026-10-01의 도입 설계를 보존한 것이다. 현재 구현은 approved board →
explicit timing/layer/camera/pose/caption → 실제 PNG 검사·검토 → pinned CDP 합성 →
WebM candidate와 editable HTML export다. 실제 명령·파일·상태는
[운영 문서](ANIMATIC_PRODUCTION.md), 실행 근거와 미완료 항목은
[구현 기록](research/2026-10-02-animatic-implementation.md)이 정본이다.

## 1. 결과 정의

사용자와 에이전트가 현재 콘티의 그림, 컷의 호흡, 구도와 움직임을 함께 다듬고,
**편집 가능한 원본 + WebM 재생본**을 남긴다. 대사·행동·인물 관계의 의미는
현재 서사 owner에서 오며, 영상 renderer가 새로 확정하지 않는다.

주 표현은 움직이는 만화와 2.5D 종이극장이다. 정지·시선 이동·카메라·자세 전환·
말풍선·소리의 리듬을 조합한다. 영상 provider는 필요한 컷의 선택 능력이다.

기본 제출물:

```text
animatic/
  source-refs.json          원 board와 현재 서사 artifact에 대한 참조
  motion-plan.json          승인할 timing과 움직임의 실제 값
  asset-pack.json           실제 재료와 source/plan binding
  assets/                   승인된 그림·포즈·소리
  composition/              편집/재현할 원본과 renderer version
  previews/<revision>.webm  실제 재생 가능한 후보
  render-receipt.json       수행 범위·산출물·hash·실패/중단 상태
  review/                   별도 사람 판단 기록
```

경로와 파일명은 제안이다. 실제 구현에서 producer schema와 immutable publication을
먼저 확정한다. 위 파일이 있다는 설명만으로 완료 상태를 만들지 않는다.

## 2. owner와 경계

| 책임 | owner | 맡기지 않는 것 |
|---|---|---|
| 원문·스토리·시놉시스 | geultu | renderer가 원문 개작 |
| 씬의 의미·인과·대사·Completed release | NVC STS | 픽셀이나 영상 결과로 서사 정본 변경 |
| 콘티의 timing·연출 의도 | NVC의 별도 animatic plan producer | 새로운 scene semantic model |
| 그림·인물 reference·pose·alpha 재료와 승인 | SAC | 로컬 임의 segmentation/PSD 조립·직접 image API client |
| 프레임 합성·WebM encode·결과 관찰 | SMC renderer adapter | 자산 승인·원문 의미·사람의 release |
| 공개 호출 연결과 consumer schema pin | integration | domain state나 별도 승인 원장 |
| 문서·컷을 선택하고 고치는 제품 화면 | 별도 host | 이 저장소에 제품 화면/VisualScreenCompiler 추가 |

현재 text-storyboard v1, VisualReferencePack, SMC provider clip contract에 임의 필드를
추가하지 않는다. 필요한 새 계약은 각 producer가 소유하고 consumer copy를
`integration/contracts/registry.json`에 byte equality로 등록하는 방식으로 추가한다.

## 3. 제안 계약의 최소 의미

### NVC StoryboardAnimaticPlan v1 — 신규 producer 제안

- 원 board의 contentHash, current narrative/release source refs.
- 안정된 sceneId/panelId, 원 sourceRefs와 그 의미의 보존.
- fps, canvas width/height, 패널 startFrame/durationFrames, cutPoint.
- 공통 scene camera keyframes와 레이어별 제한된 motion tracks.
- 대사/자막의 실제 원문 참조와 display frame range.
- 소리 cue와 실제 audio asset 요구, 선택된 표현 mode.
- 창작적 가정과 결정이 필요한 항목.

비어 있는 duration이나 미결 공간을 renderer의 고정 4초/임의 layout으로 채우지
않는다. 에이전트가 실제 후보 값을 작성하고 적절한 owner가 검토한다. 의미가 같은
값을 React/GSAP/Pixi별로 각각 canonical하게 저장하지 않는다.

### SAC AnimaticAssetPack v1 — 신규 producer 제안

- board/plan hash, stable asset/entity identity.
- 실제 path, raw byte hash, MIME, dimensions, alpha 여부.
- layer role, pivot, safe crop/overscan, reference/pose/clean-plate 관계.
- 승인 subject와 실제 그림·소리의 범위.

기존 visual pack이 요구하는 cinematic/sceneGraph binding을 가짜 값으로 채우지
않는다. 기존 경로를 실제로 통과할 수 있는 자산은 그 계약을 재사용하고, 그렇지
않은 standalone animatic 자산에는 명시적 새 producer contract가 필요하다.

### SMC AnimaticRenderRequest/Result v1 — 신규 producer 제안

- 현재 plan/asset pack refs와 선택한 revision·씬 범위.
- renderer/library/browser/encoder fingerprint, fps와 canvas.
- 요청한 output container/codec/alpha 옵션, 시간·출력량 한도.
- 완료된 실제 파일의 path/hash/frames와 원본 composition refs.
- failed/cancelled/outcome_unknown와 남은 부분 결과.

영상 생성 provider의 성공 receipt로 frame composition 렌더링을 가장하지 않는다.
필요한 공통 실행 envelope만 integration에 두며 범용 shared-image-core를 만들지 않는다.

## 4. 움직임의 데이터 모델

renderer별 코드 이전에 명시할 값:

| 요소 | 예시 | 기준 |
|---|---|---|
| 공통 camera | crop center, scale, pan | 같은 camera가 배경·인물·소품에 작용 |
| 레이어 movement | position, opacity, rotation, scale | 정규화 좌표와 pivot의 의미 명시 |
| 제한 pose-swap | pose asset 참조와 frame | 승인된 인물 reference 유지 |
| 말풍선·자막 | text ref, bbox, visibility range | 한국어 원문 문자열과 줄바꿈 보존 |
| cut/hold | 정지 구간, 컷 시작과 끝 | 카메라 이동·대사와 겹치는 규칙 명시 |
| 효과 | 승인된 효과 종류·seed·amplitude | 매 frame의 임의 난수로 결과 변경 금지 |
| 소리 | audio asset, start/end, gain | 실제 자산과 동기화, 새 대사 생성 금지 |

frame f의 모습은 plan·assets·f에서 계산할 수 있어야 한다. 실시간 animation
timer나 “이전 frame에 몇 픽셀 더하기”를 유일한 실행 규칙으로 사용하지 않는다.
pose나 seed가 같다고 미학적 품질이 보장되는 것은 아니다.

## 5. 사용자와 에이전트의 수정 흐름

1. 현재 작업물에서 씬/패널을 선택한다. 대상은 화면 이동과 무관하게 고정한다.
2. “문이 열리기 전에 조금 더 기다려”, “배경은 유지하고 인물의 시선만 바꿔”처럼
   선택 범위의 실제 motion/asset 후보를 만든다.
3. before/after의 컷 길이·camera·pose·자막·소리 변화와 실제 재생 후보를 제공한다.
4. 사용자가 직접 고친 최종 후보를 같은 source 기준에서 채택한다.
5. 영향을 받는 컷과 인접 전환만 다시 렌더링한다. 기존 출력은 개정별로 남긴다.
6. WebM과 편집 원본을 같은 revision에 결속해 export한다.

브라우저의 영상 currentTime이 편집 원장의 현재 revision을 소유하지 않는다.
호스트는 신뢰된 selection·stop·review 상태를 유지하고, 생성 내용이 버튼의
권한이나 “승인됨” 상태를 만들지 못하게 한다.

## 6. 구현 순서

| 단계 | 실제 결과 | 필요한 의존 |
|---|---|---|
| 첫 연결 | 그림이 있는 패널·hold/cut·common camera·불투명 WebM 후보 | 새 source-bound plan, 실제 자산, exact seek(frame), WebM export |
| 주 표현 | 2.5D layer·pose-swap·원 대사 text·sound cues | SAC 레이어/pose 재료, 공통 frame clock |
| 수정 경험 | 선택 컷·타이밍/시선 수정·부분 재렌더링·원본 저장 | host 선택과 owner CAS, dependency-aware cache |
| 선택 고급 표현 | rig·shader·3D 또는 짧은 I2V 후보 | 요구된 capability만 갖춘 adapter·별도 사용 범위 |

여기에는 시연용 fake pixels·synthetic approval을 제품 완료의 증거로 포함하지 않는다.
current SMC의 `probeClip`은 mov 전용이므로 WebM을 검사·수입하는 지원을 별도로
명시하거나, animatic render 결과에 독립된 observation 경계를 만든다.

## 7. renderer 선택

**제품 도입의 우선 기준:** 기존 SMC locked Chromium/CDP를 유지하면서 plan을
정확한 frame으로 합성하고 encode하는 실행 경로.

Pixi/Canvas 또는 제한된 SVG/DOM frame composition을 후보로 두고, 필요한
library를 renderer adapter 내부에 한정한다. Remotion은 agent 작성 생태계와
프레임 표현이 강한 선택지이지만, 현재 browser lock·network policy·라이선스·
telemetry와 실제 호환되는지를 후속 구현의 결정 항목으로 남긴다.

새 Puppeteer/Playwright harness, 암묵 browser download, 외부 telemetry, provider
전환을 숨기지 않는다. Remotion browser export의 CSS subset 때문에 사용할 수
없는 표현을 빈 화면이나 다른 효과로 바꾸지 않는다.

브라우저-only export는 명시적인 선택 capability다. 재생 지원과 encode 지원,
alpha 지원을 따로 판정하며 지원하지 않는 경우 기존 작업물을 보존한다.

## 8. 도입 설계의 요구와 완료 판단 기준

| ID | 요구 | 후속 작업에서 확인할 결과 |
|---|---|---|
| AN-01 | 원 씬 의미와 대사 보존 | 모든 panel/sourceRef의 현재 원본 연결, 자막의 실제 bytes |
| AN-02 | 재미있는 시간·시선 연출 | 선택한 작품에서 hold/행동/반응/컷의 목적이 재생에 드러남 |
| AN-03 | 실제 이미지 재료 | placeholder를 실제 완성 자산처럼 표시하지 않음 |
| AN-04 | frame-addressable composition | 특정 frame을 바로 선택해 같은 plan의 상태를 읽음 |
| AN-05 | 사용자 편집 보존 | 선택 밖의 독립된 수정과 현재 원본 revision 유지 |
| AN-06 | WebM 산출물 | 요청한 범위의 실제 재생 파일, 정확한 source와 raw hash |
| AN-07 | 편집 원본 export | 영상과 함께 timing/layers/assets/renderer identity 보존 |
| AN-08 | 실패·중단·미확정 구별 | 부분 파일과 복구 경로, outcome_unknown 자동 재실행 금지 |
| AN-09 | 승인 의미 보존 | render 완료와 사람의 narrative/visual/release 판단 구별 |
| AN-10 | 접근 가능한 재생 | pause/seek/mute/caption/줄인 움직임에서 장면 정보 유지 |
| AN-11 | 현재 컴포넌트 경계 | owner별 public entrypoint와 immutable 계약, schema pin |
| AN-12 | 외부 효과의 명시 | 실제 image/video provider·upload·telemetry·비용 경계 |

AN-02는 해시나 구조 검사만으로 판정할 수 없는 미학적 기준이다. 사용자의 작품을
직접 보고 판단해야 한다. 현재 실행 판정은 연결된 구현 기록을 따른다.

## 9. 다음 구현의 첫 대상

실제 원고의 **장면 하나**, 해당 장면에서 의미가 이어지는 몇 개 패널,
재사용 인물 reference와 배경, 명시적으로 선택한 미술 방향을 입력으로 한다.
정지·깊이·시선·대사·소리의 리듬이 연결되는 짧은 composition과 WebM을 만든다.

이때 결과는 그 장면에 한정된 candidate이며, 저장소 전체의 영상 제작이나 원고
품질 완료를 뜻하지 않는다. source/hash와 각 owner의 review/adoption은 유지한다.
