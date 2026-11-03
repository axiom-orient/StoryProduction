# Product Specification

## 제품

NarrativeCourier는 **시놉시스→검증된 시나리오(STS)**와 **검증된 Narrative→다중 시각/매체 결과(DirectorIR)**를 하나의 로컬 우선 파이프라인으로 연결한다.

## 핵심 사용자

- synopsis를 screenplay와 시각 개발물로 발전시키는 제작자.
- 하나의 승인된 screenplay에서 HTML/storyboard/pitch/video 등 여러 결과를 만드는 소규모 팀.
- source fidelity, 변경 근거, 재현성이 중요한 검토 사용자.

## 핵심 Job

1. synopsis를 의미 손실 없이 screenplay로 확장한다.
2. screenplay 승인 후 같은 Narrative로 여러 delivery를 만든다.
3. character/location/prop asset을 delivery마다 다시 만들지 않는다.
4. 결과가 어떤 `SRC/REL/GAP/CON/SCN`을 표현했는지 추적한다.
5. 결과 하나를 바꾸면 영향받는 최소 단위만 재실행한다.

## Non-goals

- 범용 prose editor
- 감독/작가 imitation mode
- genre→style preset engine
- renderer marketplace
- cloud render farm
- 모든 영상/3D 엔진 내장
- output A를 semantic source로 읽어 output B를 생성하는 chained conversion

## 제품 성공 조건

- Narrative 의미 변경은 STS만 수행.
- DirectorIR/asset generation이 새로운 false fact/actor/relationship/chronology를 만들지 않음.
- 동일 asset identity가 delivery 간 유지.
- `FULL` projection의 protected/CORE drift는 render 전에 BLOCK.
- backend가 바뀌어도 5 canonical contract는 유지.
- backend 미지원/모호 선택은 silent fallback 대신 명시 실패.
