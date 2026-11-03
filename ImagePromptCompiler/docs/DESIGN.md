# Studio 시각·접근성 계약

이 문서는 browser presentation의 의무만 소유한다. 제품 정체성은 [IDENTITY_AND_EVOLUTION](IDENTITY_AND_EVOLUTION.md), 이미지 field 의미는 [SPEC](SPEC.md)와 [IMAGE_PROMPT_SPEC](IMAGE_PROMPT_SPEC.md)에 있다. 코드가 참조하는 아래 절 번호를 유지한다.

## 1. 한 방향 작업 흐름

이미지 brief→catalog/profile→typed decisions→prompt→선택 generation→result를 따른다. reverse import/reconstruction이나 다른 compiler tab을 묵시적으로 추가하지 않는다.

## 2. 의미 기반 시각 token

### 2.1 light/dark

light/dark를 동등하게 지원하고 component CSS는 semantic color tokens를 사용한다. 한 theme 전용 raw hex를 component에 심지 않는다. sidebar/workbench와 canvas의 책임을 분리한다.

## 3. 결과와 오류

prompt-only 결과를 보존한다. generation 성공 전 빈 이미지를 완성본처럼 표시하지 않는다. raw adapter stderr/내부 코드는 안전한 사용자 문구와 세부 진단을 구분한다.

## 4. catalog 탐색

### 4.3 전체 범위

모든 family·검색·선택 tray를 바로 사용할 수 있어야 한다. reference card는 실제 image aspect를 읽어 크기를 정하고 loading/error 상태를 표시한다.

### 4.4 preview

preview dialog는 focus를 가두고 닫힌 뒤 사용자의 위치를 잃지 않아야 한다. grid keyboard navigation과 selection focus를 보존한다.

## 5. typed 값의 표현

더 풍부한 UI control로 바꿔도 request가 허용한 aspect/quality/text-zone 값을 누락하지 않는다. 여섯 outcome과 category-bound profiles를 구분하고 category 변경 시 stale profile/answers를 정리한다. 사용자가 직접 답한 explicitAnswers와 normalizedAnswers는 합치지 않는다.

## 6. 상태 표현

### 6.4 알림

일상 상태 안내는 하나의 polite live region을 통해 전달한다. journey current-step, selected generation mode는 ARIA 상태와 함께 표현한다. visual color만으로 진행·선택을 알리지 않는다.

## 7. 키보드·보조 기술

radiogroup/roving tabindex/keyboard activation과 preview focus 보존을 지킨다. 기존 semantic control을 단순 div로 바꾸면서 접근성을 제거하지 않는다.

## 8. 사용자 결정과 내부 표현

내부 preset id·provenance·implementation terms는 사용자 답을 대신하지 않는다. UI는 core constraints를 재구현하지 않으며 알려진 source 선택과 사용자-facing label을 분리한다. 생성 provider의 부재는 명시한다.

실행 근거는 `crates/promptgen-web/src/asset_contract.rs`, 실제 browser 검증 범위는 [Current](IMPLEMENTATION_STATUS.md)에 있다. 이 규범의 존재가 runtime PASS는 아니다.
