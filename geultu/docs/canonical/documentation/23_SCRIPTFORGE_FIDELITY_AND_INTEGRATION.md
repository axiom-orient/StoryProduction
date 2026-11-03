> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# ScriptForge 기능 무손실 통합

## 보존 대상

- source archive SHA-256: `c4ce79818873205e3fcc13c3b7679f6e4d9198f12d07d8cac5f74ff9c88349f0`
- workflow manifest SHA-256: `36766d7eeb5a131d8af8cd47cdb723ae0db4c77ca225c3072d274a5d737181a9`
- node: 26
- agent node: 18
- human gate: 8
- entry: `step_00`
- terminal: `step_08_summary`
- license: `CC-BY-NC-SA-4.0`

## 왜 재구현하지 않는가

ScriptForge의 가치가 prompt 문장 몇 개가 아니라 역할 분리, 단계 의존성, 사람 checkpoint, 막별 분할 집필, revision topology에 있기 때문이다. Geultu는 원본 Rust engine과 prompt bytes를 보존하고 바깥에서 style·canon·artifact·approval을 결합한다.

## Prompt layer 순서

```text
L0 Geultu 실행·출력 계약
L1 ScriptForge exact role system prompt
L2 ScriptForge exact node craft prompt
L3 GenreContract + CompiledCanon
L4 ApprovedStyle overlay
L5 선언된 direct dependency
L6 output schema + evidence gate
```

L1·L2는 source manifest hash와 일치해야 하며 in-place 수정이 금지된다. 변경은 새 CraftPack version으로만 가능하다.

## 완료 인계

다음을 모두 만족해야 `CraftReviewedDraft`로 T19에 복귀한다.

1. `ProjectStatus::Completed`
2. ScriptForge doctor `ok`
3. final screenplay 존재·비어 있지 않음
4. Synopsis TextMaster·ApprovedStyle·CompiledCanon·GenreContract exact ref
5. prompt pack digest 일치
6. final output digest와 revision 기록

## 라이선스

이 pack과 원본 engine/prompt는 CC BY-NC-SA 4.0이다. Geultu MIT core와 분리 배포하며, 상업 배포에는 별도 권리 확보가 필요하다.
