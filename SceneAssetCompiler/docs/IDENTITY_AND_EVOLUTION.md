# SceneAssetCompiler — 정체성과 발전 방향

## 정체성

외부 픽셀 생성과 로컬 계약·검사·승인·배포를 분리하여 재현 가능한 scene assets를 만든다. 핵심 책임은 request/authority admission, task/attempt/ref snapshot, imagegen 및 제한된 typed provider, mechanical/visual/human review, repair, source-bound visual reference/keyframe 승인과 파생 memory index, layered scene/PSD 검증, AssetPack와 SMC handoff다. 독립 caller가 공개 surface를 통해 사용하며 상위 StoryProduction은 조합 위치를 정의할 뿐 이 제품의 state owner를 대체하지 않는다. 다른 제품의 semantic state·승인·전역 scheduling은 비목표다.

## 변하면 안 되는 것

선택 animatic에서는 승인 NVC packet의 요구와 실제 PNG pack/review를 소유한다. NVC의 연출
정본을 수입하거나 SMC 합성·final release를 대신 수행하지 않는다. Submit과 실제 사람 review,
외부 executor provenance의 선언·인증을 구별한다.

승인 대상 source/pixel hash, 외부 참조/권한, local fake pixel success 금지, scene-release subject binding, dispatch outcome_unknown와 automatic retry 금지, schema/skill 입력과 PSD 검증 증거. 유효한 optional capability, 사용자의 원문·기록과 intentional public contract는 정리 대상으로 취급하지 않는다. 검증되지 않은 결과는 완료로 선언하지 않는다.

## 변경 가능한 것

프롬프트/레이아웃/선택 provider와 내부 모듈은 역할·입출력·승인 의미를 유지하면 개선할 수 있다. reference/edit/PSD를 reference-free PNG provider에 몰래 라우팅하지 않는다. 이 허용 범위는 추가 작업 권한이나 미승인 변경의 확정이 아니다.

## 발전 방향

task 준비·외부 효과·증거·승인·배포의 경계를 정확히 한다. provider 수 확대보다 승인 대상 바인딩과 실패한 단계의 안전한 복구가 우선이다. 최소 추상화, 하나의 authoritative owner, 명시적인 상태 전이와 격리된 I/O를 판단 기준으로 삼는다.
