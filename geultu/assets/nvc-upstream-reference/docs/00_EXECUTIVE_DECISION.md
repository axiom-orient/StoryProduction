# 00. Executive Decision

## 판정

이전의 “이야기/시놉시스/시나리오를 별도 프로그램으로 만들 필요는 없다”는 표현은 폐기한다.

**더 좋은 구조는 Narrative Development System(NDS)과 Narrative Visualization Compiler(NVC)를 분리하는 것이다.**

```text
Idea / Source Text
      ↓
Narrative Development System
      ├─ Synopsis                 [planning artifact]
      ├─ Story development       [causality, character, setup/payoff]
      └─ Canonical Narrative     [Story OR Screenplay]
      ↓ NarrativePackage
Narrative Visualization Compiler
      ↓
Verified Visual Master
```

## 왜 이 경계가 더 좋은가

1. **목적 함수가 다르다.** NDS는 “이야기가 좋은가?”를 최적화하고, NVC는 “확정된 이야기를 의미 손실 없이 어떻게 보여줄 것인가?”를 최적화한다.
2. **시놉시스는 최종 authority가 아니다.** 압축된 계획물에는 micro-causality, 상태 변화, payoff object, blocking이 생략될 수 있다.
3. **story와 screenplay는 직렬 필수 단계가 아니다.** NDS가 작품 목적에 따라 최종 canonical representation 하나를 선택한다.
4. NVC가 synopsis를 다시 확장하면 서사 개발과 시각화가 섞이고 semantic drift가 생긴다.
5. NVC가 단일 input port를 가지므로 schema/gate/test를 고정할 수 있다.

## 호환성

raw prose/synopsis/screenplay를 직접 넣고 싶다면 별도 **Compatibility Adapter**가 `NarrativePackage`로 변환한다. Core NVC는 입력 종류별 분기 코드를 갖지 않는다.
