# 06. ScriptForge Boundary

## 보존할 책임

- project lock과 single-writer 실행
- atomic state write
- source snapshot
- task/gate token binding
- doctor/preflight
- protected span과 patch range 검증

## 보완할 책임

ScriptForge project 생성 시 synopsis/style/canon/genre/source refs와 revision을 `ProjectState`에 동결한다. 파일 경로만 기억하거나 실행 중 최신 파일을 다시 읽어 의미가 바뀌면 안 된다.

## 경계

ScriptForge는 orchestration adapter다. artifact schema, validator pass, human approval semantics를 자체적으로 재정의하지 않는다. domain command를 만들고, port를 호출하고, 결정 결과를 표시한다.

## lifecycle

`Open Project → Acquire Lock → Preflight → Load State → Execute Admitted Command(s) → Atomic Commit → Release Lock`

crash 후에는 temp file을 현재 상태로 승격하지 않는다. last committed snapshot과 orphan artifact를 구분해 doctor가 보고한다.
