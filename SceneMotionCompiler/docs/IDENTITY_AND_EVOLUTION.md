# SceneMotionCompiler — 정체성과 발전 방향

## 정체성

사람 승인된 cinematic intent와 자산을 실행 가능한 scene/motion/editorial 산출물로 투영하는 독립 제작 mechanism이다. caller는 NVC/SAC integration, operator CLI/MCP, 사람 검토 Studio다. narrative/shot 의미를 새로 발명하지 않고 검증된 입력·실제 관찰·승인을 구별해 결과를 제공한다. NLE·이미지 모델 runtime·제품 UI 복원·전역 agent scheduler는 비목표다.

## 변하면 안 되는 것

선택 animatic은 승인된 NVC 연출·SAC PNG에서 exact-frame 합성 후보를 만드는 경로다.
HTML/VTT/WebM 결과와 renderer provenance를 보존하지만 기존 Studio Human release나
최종 작품 approval은 수행하지 않는다. Asset generation·audio·NLE·새 제품 화면을 소유하지 않는다.

narrative/plan/continuity/asset의 원본과 hash binding, shot identity/order/frame/camera intent, semantic occurrence 보존, source revision과 stale 거부를 유지한다. provider 성공·QC와 사람 승인은 다르다. final approval은 Studio Human boundary와 정확한 candidate subject/CAS에 결합한다. release 전·후 오류와 outcome unknown을 구별한다. 현재 유효한 HTML 및 native custody public contract를 새 영상 후보 기능의 이름으로 없애지 않는다.

## 변경 가능한 것

projection 내부 표현, preview UI, 선택 provider, editorial interchange, transport와 패키지 구조는 위 불변조건을 지키며 교체할 수 있다. 형식 전환은 producer/consumer를 함께 다루고 미지원 효과를 근사 성공으로 숨기지 않는다. 이 허용 범위 자체가 추가 작업 권한은 아니다.

## 발전 방향

연출 계획보다 실행기가 더 많은 의미를 소유하지 않게 한다. shot/reference/clip/timeline의 최소 실패 단위를 따라 수정하고 실제 media·동작·의미 증거를 분리한다. 코어는 검증·projection·승인 경계이며 외부 생성기와 interchange는 선택 capability다. 기능 수보다 정확한 owner, 설명 가능한 실패, 보존 가능한 승인 산출물을 우선한다.
