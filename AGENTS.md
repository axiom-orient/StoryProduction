# StoryProduction repository rules

1. Story domain owner를 합치지 않는다. geultu/NarrativeCourier/SMC/SAC의 authoritative state는 각 component가 소유한다.
2. cross-component integration은 `integration/`의 immutable contract와 public entrypoint를 통해서만 추가한다.
3. 범용 `shared-image-core`를 만들지 않는다. provider별 실행은 adapter이고 Story integration envelope만 공유한다.
4. producer schema는 producer가 소유한다. consumer copy는 `integration/contracts/registry.json`에서 byte equality로 pin한다.
5. candidate 결과는 승인 결과가 아니다. source/hash가 current authority와 일치하는지 검증한 뒤 별도 review/adoption을 수행한다.
6. 실패·취소·`outcome_unknown`을 성공으로 변환하지 않는다.
7. `Jangmyeondo` source를 import하거나 runtime dependency로 추가하지 않는다. 필요한 의미는 Story-owned 구현으로 명시적으로 재작성한다.
8. `VisualScreenCompiler` 및 제품 화면 제작 코드는 이 저장소에 추가하지 않는다.
