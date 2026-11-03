> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# 윤문·퇴고·수정·피벗

## 윤문 경계

지원 locale은 ar, de, en, es, fr, hi, it, ja, ko, pt-BR, pt-PT, ru, zh-Hans, zh-Hant다. 각 pack은 archive SHA-256과 release inventory로 검증한다.

허용 연산:

- `diagnose`: 언어·register·번역투·리듬·어색함 진단
- `plan_patch`: exact source digest에 결합된 PolishPatch 생성
- `choose`: 주어진 후보 중 선택
- `replace`: 지정 범위만 교체

금지:

- 번역
- 사실·사건·주장 추가
- 장르·화자·시점 변경
- 보호 문자열 변경
- source digest가 다른 patch 적용
- 근거 없는 전체 재작성

## 작가 퇴고

```text
T19 구조 검토
→ T20 의미·canon 검토
→ T21 style drift 검토
→ T22–T25 윤문
→ T26 작가 전체 검토
→ T27 수정 지시
→ T28 표적 수정
→ T29 재검토
→ T30 accept/revise/pivot/reject
→ T31 장르 완료
→ T32 TextMaster 승인
```

## 반복 제한

- 동일 digest 반복은 진전 없음으로 기록
- stale patch·directive 거절
- 이전 revision은 `prior:` exact ref로 보존
- revision 횟수·pivot 횟수는 계약의 budget을 초과할 수 없음
- pivot은 현재 run terminal이며 새 run으로 이어짐
