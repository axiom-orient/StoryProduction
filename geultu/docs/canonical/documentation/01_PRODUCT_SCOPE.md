# 01. Product and Scope

## 한 문장 정체성

Geultu는 사람이 승인한 창작 계약을 프로그램이 통제하는 제작 단계로 실행하고, 독립 검증 증거를 만족한 결과만 다음 단계로 승격하는 로컬 제작 시스템이다.

## 사용자와 문제

- **구현이 전제하는 사용자:** 하나의 작품을 장기간 관리하는 작성자·감독자.
- **호출 주체:** CLI/UI adapter 또는 승인된 agent/provider.
- **문제:** 생성 결과가 style·canon·source·revision과 분리되어 재현성, 승인 근거, 시각화 handoff가 깨지는 문제.
- **검증된 실제 사용자:** [UNKNOWN]. 저장소만으로 시장 수요는 증명하지 않는다.

## 입력 → 처리 → 출력

`Approved Project Inputs → Text Stages → TextMaster → T33 GeultuStoryPackage → optional NVC Stages → Committed Artifacts + Receipts`

## 핵심 범위

- 프로젝트 입력 동결과 revision binding
- T00–T33 텍스트 제작·검증
- ScriptForge의 결정적 실행 상태와 보호된 수정
- TextMaster commit
- T33 GeultuStoryPackage export
- 선택적 NVC 시각 제작
- 독립 validator receipt와 domain decision

## 비목표

- 모델 자체의 학습·파인튜닝
- 자동 배포·클라우드 동기화
- 인간의 의미 승인 대체
- 임의 provider가 상태·승격을 결정하는 구조
- 과거 envelope와의 runtime 호환 계층
