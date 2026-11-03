> v4.1.0 편입 문서다. 통합 후보(geultu.integrated 1.0.0-candidate)의 소스 스캔
> 코퍼스에서 추출한 계약 상세 참조 자료다. 이 문서는 단계 설계와 상세 참고를 보존하며
> 현재 runtime surface로 간주하지 않는다. 제품 API는 [`geultu/docs/SPEC.md`](../../SPEC.md),
> 현재 source와 qualification은 [`Geultu Current`](../../IMPLEMENTATION_STATUS.md) 및
> [모듈 분석](../../../ANALYSIS.md)을 따른다. 18장은 2026-08-27 historical snapshot이다.

# 라이선스·권리·배포

## 구성별 경계

| 구성 | 상태 |
|---|---|
| Geultu-authored core/text/visual/CLI/docs/tools | MIT 의도; root LICENSE 적용 |
| ScriptForge v4 engine·prompt·workflow | CC BY-NC-SA 4.0 |
| synopsis-to-scenario 기반 지식 | CC BY-NC-SA 4.0, attribution/NC/SA 필요 |
| 14 language refinement archives | 각 archive 내부 Attribution License와 upstream notice 유지 |
| NVC v1 package | `[UNKNOWN]` 라이선스 파일 없음 |
| 사용자 corpus·작품 | 사용자가 권리 보유·허용 범위를 attestation해야 함 |

## 배포 판정

현재 통합 archive는 개발·평가 candidate다. ScriptForge의 NC 조건과 NVC 라이선스 미확인 때문에 상업 release로 표시할 수 없다.

상업 배포 가능한 형태는 다음 둘 중 하나다.

1. 권리를 별도 확보한 ScriptForge/NVC pack을 포함
2. MIT Geultu core만 배포하고 restricted CraftPack/NVC는 사용자가 로컬에서 설치

Prompt와 donor source를 삭제해서 라이선스 문제를 숨기는 것은 기능 보존 목표와 충돌하므로 금지한다.
