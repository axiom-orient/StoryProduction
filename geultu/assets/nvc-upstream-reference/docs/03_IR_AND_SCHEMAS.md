# 03. IR & Schemas

## IR chain

```text
NarrativePackage
  ↓
StoryIR + MeaningSpec
  ↓
DirectorIR
  ↓
AdaptationIR
  ↓
SceneBundleIR
  ↓
ShotBundleIR
  ↓
AssetManifest
  ↓
RenderIR
  ↓
TimelineIR
  ↓
EvaluationReport
```

## NarrativePackage

NVC의 유일한 canonical input. `canonical_narrative`, optional `planning_synopsis`, adaptation policy, target, constraints를 가진다.

## StoryIR

문장 요약이 아니라 entity/event/state 변화 representation이다. `explicit`로 선언한 정보는 source span이 반드시 있어야 한다.

## MeaningSpec

### must_keep_assertions
최종 결과에 반드시 남아야 하는 사실/의미.

### transition_obligations
장면 A와 B가 둘 다 있다는 것만으로는 충분하지 않다. A→B의 `causal / intentional / emotional / consequence / temporal / revelation` 연결이 복구 가능해야 한다.

## DirectorIR

“수묵화” 같은 style label 하나가 아니다.

- visual grammar
- motion grammar
- camera grammar
- typography grammar
- transition grammar
- sound grammar
- negative rules
- production constraints

## AdaptationIR

무엇을 image/text/sound가 담당할지, 무엇을 생략했는지 기록한다. 생략은 항상 이유와 안전성 판정을 가진다.

## SceneBundleIR

```text
state_in → state_delta → state_out
```

continuity를 free-form memory에 맡기지 않는다.

## ShotBundleIR

각 shot은 narrative function, Setup, Event, Lighting, Camera, Motion, hard constraints, assets, semantic obligations를 가진다.

## AssetManifest

shot 계획이 잠긴 뒤 실제 필요한 자산만 lazy materialize한다. identity-critical 자산은 `continuity_scope`, `allowed_mutations`, `forbidden_mutations`를 가진다.

## RenderIR

모델명이 아니라 capability requirement와 backend assignment를 저장한다. 모델 교체가 StoryIR/ShotIR schema를 바꾸면 안 된다.

## TimelineIR

시간은 floating second가 아니라 `fps + frame index`로 기록한다.
