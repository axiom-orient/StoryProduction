# 04. DirectorIR

DirectorIR은 Narrative를 다시 해석하는 두 번째 story compiler가 아니다. **이미 검증된 의미를 output-local presentation으로 투영하는 planner**다.

## Input

- NarrativePackage
- DeliveryRequest
- compatible MediaBackend descriptors

## Output

- DirectorPlan 하나

StoryIR, MeaningSpec, SceneBundleIR 같은 두 번째 global Narrative IR을 만들지 않는다.

## Semantic-bearing surface

독립 semantic audit 대상은 최소 다음이다.

- `ProjectionUnit.content`
- `AssetRequest.brief`
- `AssetRequest.source`가 가리키는 USER/DERIVED material이 narrative claim을 전달하는 경우 그 observable content

`direction`, camera/style/layout 자체는 Narrative fact를 만들지 않는 한 presentation instruction이다.

## Coverage

- `FULL`: protected/CORE 의미의 무단 OMIT/contradiction hard fail.
- `SUMMARY/SELECTION/EXCERPT/MOOD`: omission은 가능하지만 false fact/actor/relationship/chronology/world-rule/knowledge/ending을 만들 수 없다.

## Output-local structure

section/panel/shot/timed unit은 Delivery 내부 구조다. 실제 두 번째 global consumer가 생기기 전까지 별도 global IR로 승격하지 않는다.
