# Geultu Canonical Documentation Package v4.1.0

이 패키지는 Geultu의 versioned design/reference 문서와 계약 자료를 묶는다. 여기의 target·기준선·참고 자료는 현재 제품 동작이나 실행 qualification을 대신하지 않는다. 제품 SPEC은 [`../SPEC.md`](../SPEC.md), 현재 source snapshot과 날짜별 gate 결과는 [`../IMPLEMENTATION_STATUS.md`](../IMPLEMENTATION_STATUS.md), 모듈·caller 분석은 [`../../ANALYSIS.md`](../../ANALYSIS.md)가 소유한다.

## 계보

| 버전 | 원본 | 본 패키지에서의 지위 |
|---|---|---|
| 3.1.0 | `mece-doc v3.1.0` 외부 불변 원본(집계 해시 `d6e2cf709fc57acf644a449c5aa2667482a91beada6160d4f5fd3262fcc38960`) | 00–16 문서·기준선 계약·카탈로그·픽스처의 역사적 편입 원본 |
| 4.1.0 | 이 패키지 | 구현 저장소에 결합된 design/reference package. 17장은 역사적 채택 기준선, 18장은 2026-08-27 source snapshot이며 현재 code status가 아니다. |

현재 schema inventory와 파일 경로는 `contracts/schema-registry.json`이 소유하며, `tools/verify.py`가 레지스트리·파일 존재·버전 일치를 검사한다. 17장의 역사적 채택 표는 현재 registry나 schema 내용을 대체하지 않는다.

## 역할 분담

- `documentation/` — 책임·경계·절차·설계 target과 source-date가 명시된 기록.
- `contracts/` — JSON Schema Draft 2020-12. 필드·enum·정규식·required의 정본.
- `catalogs/` — 권한 소유자와 failure-code catalog의 정본.
- `fixtures/` — 계약 수용/거부 규칙의 실행 가능한 사례.
- `evidence/` — 패키지 검증·자산 출처의 날짜 또는 생성 도구가 있는 기록.
- `manifests/` — 패키지 무결성 SHA-256 목록. 생성 report와 manifest 자신은 해시 범위에서 제외한다.
- `tools/verify.py` — 독립 검증기. 실행 결과를 `evidence/verification-report.json`에 기록한다.

## 읽기 순서

1. [제품 Current](../IMPLEMENTATION_STATUS.md) — 실제 source snapshot, 날짜별 실행 결과, 미검증 범위
2. [모듈·caller 분석](../../ANALYSIS.md) — reachable implementation과 소유 경계
3. `documentation/00_INDEX.md` — 이 패키지의 설계·참고 문서 지도
4. `documentation/17_ADOPTION_BASELINE.md` — 2026-08-23 역사적 채택 기준선
5. `documentation/18_CURRENT_STATE.md` — 2026-08-27 역사적 source snapshot
6. `documentation/19_DATA_ASSET_REGISTRY.md`와 `documentation/20_VERIFICATION_GATES.md` — 패키지 자산 inventory와 검증 절차
7. `documentation/01–16`의 설계 target, 이어서 21–29의 상세 참고 자료

## 성숙도 선언

이 패키지의 지위는 `SPEC_READY_ADOPTED`다. 제품 코드의 구현·qualification 상태는 [제품 Current](../IMPLEMENTATION_STATUS.md)와 [모듈 분석](../../ANALYSIS.md)을 따른다. 외부 provider 품질과 전체 cross-repository pipeline은 완료로 표시하지 않는다.
