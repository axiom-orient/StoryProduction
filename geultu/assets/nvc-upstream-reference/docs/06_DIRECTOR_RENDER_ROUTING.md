# 06. Director & Render Routing

## Genre lookup 금지

`SF → 우주선`, `사극 → 수묵화` 같은 1:1 룩업은 쓰지 않는다.

```text
theme / narrative function
        ↓
medium affordance candidates
        ↓
production feasibility
        ↓
pairwise semantic ranking
        ↓
DirectorIR lock
```

corpus novelty는 tie-breaker일 뿐 semantic fit보다 우선하지 않는다.

## 기본 motion modes

- `LAYERED_2_5D`: 배경 고정, 부분 모션, 패럴랙스
- `RIGGED_2D`: 종이 인형/마리오네트/그림자극
- `STOP_MOTION`: 프레임 기반 인형극 미학
- `GEN_I2V`: anchor/reference 기반 유기 motion
- `GEN_T2V`: identity/continuity 부담이 낮은 자유 shot
- `SCENE_3D`: 정확한 geometry/camera/miniature stage
- `HYBRID`: deterministic core + generative organic motion

## 가장 중요한 routing rule

**material hard invariant가 uncontrolled stochastic generator 하나에 의존하면 설계 실패다.**

예: “최초 그림자 이전 인간에게 그림자가 없어야 한다”는 prompt 희망사항으로 두지 않고 character layer + deterministic compositor 또는 검증 가능한 geometry로 구현한다.

## Director presets

`directors/*.json`에는 예시로 living parchment, marionette, 16mm film, analog sci-fi를 넣었다. preset은 style prompt가 아니라 6개 grammar와 supported motion modes를 포함한다.
