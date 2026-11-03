# 02. Architecture and Boundaries

## 구성

| 경계 | 소유 책임 | 금지 책임 |
|---|---|---|
| Domain contracts | artifact, stage, decision 불변조건 | 파일·provider I/O |
| Application orchestrator | 단계 순서, admission, transition | 검증 결과 조작 |
| State store | 현재 run/project 상태의 원자적 저장 | 과거 evidence 재해석 |
| Artifact store | immutable body와 revision | domain 승격 결정 |
| Validator adapters | body·binding·schema 검사 | 상태 변경 |
| Provider adapters | 후보 생성 | pass/commit 결정 |
| UI/CLI/MCP | 입력 수집·표시 | canonical state 소유 |
| NVC boundary | NVC release package 소비·visual output 생성 | TextMaster 수정 |

## 의존 방향

`UI/CLI/MCP → Application → Domain Contracts`

`Application → State/Artifact/Validator/Provider ports → adapters`

Domain은 adapter를 import하지 않는다. Provider output은 candidate일 뿐 artifact가 아니다.

## 제거할 중복 경계

- 별도 T33 envelope mapper: 제거. T33 schema payload가 곧 handoff이다.
- text 전용·NVC 전용 commit API: 제거. named output map 기반 공통 commit protocol 사용.
- receipt가 상태를 재구성하는 두 번째 store: 제거. state snapshot + append-only audit로 분리.
- schema enum의 Markdown 복제: 제거.
