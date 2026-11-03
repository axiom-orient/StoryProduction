# ImagePromptCompiler — 정체성과 발전 방향

## 정체성

typed image 의도를 deterministic prompt로 컴파일하고 선택 provider로 실제 artifact를 만든다. 핵심 책임은 여섯 category outcome, category-bound profile/interview/catalog/LUT, prompt-only, Codex generation·fidelity/repair, one-base OpenAI edit, CLI/MCP/Studio다. 독립 caller가 공개 surface를 통해 사용하며 상위 StoryProduction은 조합 위치를 정의할 뿐 이 제품의 state owner를 대체하지 않는다. 다른 제품의 semantic state·승인·전역 scheduling은 비목표다.

## 변하면 안 되는 것

core compiler 의미·diagnostic-only failure·exact text/subject/profile 제약, optional provider 부재 시 fake success 금지, binary/crate promptgen, output/input alias 및 reference 계약, 후보와 Story 승인 분리. 유효한 optional capability, 사용자의 원문·기록과 intentional public contract는 정리 대상으로 취급하지 않는다. 검증되지 않은 결과는 완료로 선언하지 않는다.

## 변경 가능한 것

UI/provider execution·catalog/profile와 내부 구조는 기존 정상 contract와 허용 category 의미를 보존하며 개선할 수 있다. 미승인 비이미지 compiler/전역 story state 확장은 하지 않는다. 이 허용 범위는 추가 작업 권한이나 미승인 변경의 확정이 아니다.

## 발전 방향

프롬프트 의미→실제 artifact의 관찰 가능한 일치가 최우선이다. 코어 검증·fidelity·명시 reference transport를 지키면서 필요한 선택 effect만 확장한다. 최소 추상화, 하나의 authoritative owner, 명시적인 상태 전이와 격리된 I/O를 판단 기준으로 삼는다.
