> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# Native 텍스트 Craft Adapter

## 지원 장르

| adapter | 장르 | runtime | 상태 |
|---|---|---|---|
| geultu-native-synopsis | synopsis | rust | reachable |
| geultu-native-short-story | short_story | rust | reachable |
| geultu-native-novel | novel | rust | reachable |
| geultu-native-serial-novel | serial_novel | rust | reachable |

## 공통 7연산

```text
diagnose → structure → design → plan → draft → review → revise
```

| 연산 | 필수 direct upstream | 글투 적용 | 출력 목적 |
|---|---|---|---|
| diagnose | source | constraints only | 결함·가능성 진단 |
| structure | diagnosis | constraints only | 장르 구조 |
| design | diagnosis + structure | constraints only | 인물·세계·화자·형식 설계 |
| plan | structure + design | constraints only | unit/scene/chapter 계획 |
| draft | plan | surface voice | 승인 글투로 초고 작성 |
| review | draft | constraints only | 독립 구조·의미·문체 검토 |
| revise | draft + review | surface voice | 지적된 부분만 표적 수정 |

## 장르 완료 계약

- 시놉시스: 시작부터 결말까지 전체 인과·인물 변화·핵심 사건을 숨기지 않는다.
- 단편: 중심 변화 하나, 압축된 장면, 불필요한 subplot 없음, ending effect가 있다.
- 소설: premise·conflict·character arc·causal chapter/scene·setup/payoff·ending obligation을 충족한다.
- 연재소설: episode objective/change/hook/payoff와 장기 arc·continuity ledger를 충족한다.

각 연산은 전체 history가 아니라 선언된 direct frontier만 받는다. Adapter는 결과를 승인할 수 없다.
