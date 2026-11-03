# 17. 채택 기준선 (Historical Adoption Baseline)

> **STATUS: HISTORICAL BASELINE — 2026-08-23 / canonical package v4.0.0.** 이 문서는
> 해당 기준 시점의 하이브리드 채택·기각 결정을 보존한다. 현재 구현, schema 수,
> 테스트 수의 정본이 아니다. 현재 사실은 실제 source·tests·contracts, [Geultu Current](../../IMPLEMENTATION_STATUS.md),
> [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)의 근거를 구분해 판정한다. 18장은 2026-08-27 snapshot이다.

이 문서는 당시 **무엇을 어디서 채택했고 무엇을 무엇으로 기각했는지**를 보존하는
결정 기록이다. 현재 문서가 채택/기각을 언급할 때는 이 기록을 역사적 출처로만
참조하고, 현재 상태를 이 표에서 추정하지 않는다.

## 1. 우선순위 규칙 (충돌 시 위에서 아래로)

1. **동작·필드·허용 값:** 저장소 루트 기준 `docs/canonical/contracts/*.schema.json`과
   실제 source·tests가 현재 동작의 정본이다.
2. **현재 상태:** [Geultu Current](../../IMPLEMENTATION_STATUS.md)와 [저장소 루트의 `ANALYSIS.md`](../../../ANALYSIS.md)의 source 및 날짜별 측정이 current code와 qualification을 각각 설명한다.
3. **역사적 채택 결정:** 이 문서는 2026-08-23 기준의 선택·기각 근거만 소유한다.
4. **보조 자료:** `catalogs/`, `fixtures/`, 나머지 documentation은 각 책임을 보조한다.
   현재 판정은 역사 문서의 서술만으로 승격하지 않는다.

## 2. 채택 결정표

| 원천 | 항목 | 결정 | 근거 |
|---|---|---|---|
| mece-doc v3.1.0 | documentation 00–16 | **ADOPTED (historical baseline)** | MECE 자체 검증 79/0 통과 기록 |
| mece-doc v3.1.0 | baseline contract schemas + registry | **ADOPTED (historical baseline)** | clean-break TARGET 설계, F-001~F-009 결함 반영 기록 |
| mece-doc v3.1.0 | catalogs(authority·failure-codes), fixtures | **ADOPTED (historical baseline)** | 당시 계약과 동일 리비전 기록 |
| mece-doc v3.1.0 | evidence/(구세대 감사 4.8MB) | **NOT ADOPTED** — 원본 패키지에 불변 보존 | 감사 이력이지 정본이 아님. 계보 참조는 README.md |
| integrated-candidate v1.0.0 | craft-packs/scriptforge-v4 | **ADOPTED as DATA ASSET** → `assets/craft-packs-scriptforge-v4` | ScriptForge v4 프롬프트와 워크플로 매니페스트. 업스트림 zip SHA-256 `c4ce79818873205e3fcc13c3b7679f6e4d9198f12d07d8cac5f74ff9c88349f0`와 일치 확인 |
| integrated-candidate v1.0.0 | refinement-packs/language-v1 | **ADOPTED as DATA ASSET** → `assets/refinement-packs-language-v1` | 언어 윤문팩 14종. 단, 원본 내부 RELEASE_MANIFEST.sha256는 stale(29개 중 15개 미포함) — 본 저장소에서는 디렉터리 단위 SHA256SUMS를 재생성해 대체했다(19장) |
| integrated-candidate v1.0.0 | assets/nvc-v1 | **ADOPTED as REFERENCE** → `assets/nvc-upstream-reference` | NVC S00–S12 업스트림 참조 구현. 업스트림 zip SHA-256 `c4b7b3319636dc3c1b91d2c4a835837a305cc7895d577438fedf568bdc2c560d` 일치 확인 |
| integrated-candidate v1.0.0 | crates/ Rust 코드 전체(약 7.6k LOC) | **REJECTED** | 컴파일 불가 실측: geultu-core E0283 3건(`impl Into<String>` 인자 이중 `.into()`), 툴체인 1.75 고정 vs 잠긴 의존성 edition2024 충돌. 컴파일된 역사 없음(evidence/toolchain exit=127) |
| integrated-candidate v1.0.0 | historical v2 contract files + schemas/artifacts·runtime | **SUPERSEDED** | mece TARGET 스키마가 동일 도메인을 결함 수정 형태로 대체. 원본은 mece v3.1.0 패키지 evidence 내 인벤토리로만 계보 추적 |
| session-handoff v1.0.0 | LOCKED_DECISIONS 8항목 | **ADOPTED** as 제품 정체성 제약 | Geultu 코어 권위·ScriptForge 축약 금지·NVC DAG·GeultuStoryPackage 단일 경계 등 |
| repo v5.0.0 워크스페이스 | crates/geultu·geultu-cli, adapters | **RETAINED implementation base** | 현재 구현 기반으로 유지. 당시 TARGET 계약 준수는 미완성으로 기록되었으며, 현재 범위는 Geultu Current와 source/tests로 확인한다 |

## 3. 금지 규칙

- 폐기된 v1.0.0 crates 코드를 "참고해서" 복원하는 것을 금지한다. 필요 로직은 TARGET 계약과
  현재 엔진 위에서 재작성한다.
- 데이터 자산 파일을 현장에서 직접 수정하는 것을 금지한다. 파생이 필요하면 새 버전
  디렉터리(예: `assets/craft-packs-scriptforge-v5`)를 만들고 19장 레지스트리에 등록한다.
- 본 문서의 결정표를 암묵적으로 수정하는 것을 금지한다. 변경은 이 파일의 git diff로 남긴다
  (커밋은 사용자 지시 시에만).
