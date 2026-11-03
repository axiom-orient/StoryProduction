# 19. 데이터 자산 레지스트리 (Data Asset Registry)

이 문서는 저장소 `assets/`에 이식된 데이터 자산의 유일한 권위 있는 목록이다.
각 자산 디렉터리는 자체 `SHA256SUMS`를 가지며, `tools/verify.py`가 매 실행마다
실제 파일과 대조한다. 무결성이 깨지면 패키지 검증이 실패한다 — 그것이 의도다.

## 1. 자산 목록

| 자산 | 경로(저장소 루트 기준) | 지위 | 원천 |
|---|---|---|---|
| ScriptForge v4 craft-pack | `assets/craft-packs-scriptforge-v4/` | ACTIVE_INPUT(예정) — IMPL 단위에서 어댑터 입력으로 사용 | integrated-candidate v1.0.0 craft-packs/scriptforge-v4 |
| Story baseline v1 craft-pack | `assets/craft-packs-story-baseline-v1/` | OPTIONAL_INPUT — `generation.prepare`의 craft 참조 자료; 기본 authoring 및 채택 권한에는 필요하지 않음 | StoryProduction 작성 원문 및 pack-local provenance archive |
| 언어 윤문팩 language-v1 | `assets/refinement-packs-language-v1/` | ACTIVE_INPUT(예정) — refinement 어댑터 입력 | integrated-candidate v1.0.0 refinement-packs/language-v1 |
| NVC 업스트림 참조 | `assets/nvc-upstream-reference/` | REFERENCE_ONLY — 실행 코드로 임포트 금지, S00–S12 대조 참고 | integrated-candidate v1.0.0 assets/nvc-v1 |

- **파일 수와 집계 해시의 기계 정본은 `evidence/asset-aggregates.json`이다.** 이 문서는 값을 복제하지 않는다.
- 집계 해시 알고리즘의 유일한 정의는 `tools/verify.py`의 `dir_aggregate()`이다
  (디렉터리 내 파일을 경로 정렬 순으로 `sha256  상대경로` 라인화 → 라인 전체를 다시 sha256).
  셸 재현이 필요하면 verify.py를 사용한다 — 별도 셸 변형 정의는 만들지 않는다.
- 각 자산의 파일 단위 체크섬은 `<자산>/SHA256SUMS`(파일별 상대경로). 검증:
  `cd <자산> && shasum -a 256 -c SHA256SUMS --quiet` → 종료코드 0.
- 참고: language-v1 원본 내부 RELEASE_MANIFEST.sha256는 stale(29개 중 15개 미포함)이라
  폐기했으며, 디렉터리 단위 SHA256SUMS 재생성으로 대체했다.

## 2. 출처(provenance) 해시

| 원천 아카이브 | SHA-256 | 확인 위치 |
|---|---|---|
| `story-baseline-v1.zip` | `05483ac6dda22937ec65ed901dc4807d007016a56d4f8e1ed86dcb721b18fe0f` | 동일 Story baseline pack의 작성 프롬프트·workflow·MIT LICENSE로부터 고정 시각과 경로 순서를 사용해 생성; 실제 바이트 재생성 일치 |
| scriptforge-source-v4-final.zip | `c4ce79818873205e3fcc13c3b7679f6e4d9198f12d07d8cac5f74ff9c88349f0` | integrated-candidate COMPLETION_REPORT.md 주장값과 일치 실측 |
| Narrative Visualization Compiler v1.0 zip | `c4b7b3319636dc3c1b91d2c4a835837a305cc7895d577438fedf568bdc2c560d` | 동일 |

craft-pack 내부 프롬프트 15종의 개별 해시는 원본
`UPSTREAM_PACKAGE_MANIFEST.sha256`(출처 기록용)와 일치함을 감사에서 확인했다.

## 3. 라이선스 표기

| 자산 | 라이선스 | 비고 |
|---|---|---|
| craft-packs-story-baseline-v1 | MIT | StoryProduction 작성 원문 workflow 및 프롬프트; 기본 원고 authoring이나 별도 채택 결정의 권한이 아님 |
| craft-packs-scriptforge-v4 | CC-BY-NC-SA-4.0(craft-pack.json 명시) | 비상업 조건 — 배포 계획 시 법무 확인 필요 |
| refinement-packs-language-v1 | 원본 catalog.json policy 따름(translation_forbidden 등) | 내부 사용 전제 |
| nvc-upstream-reference | 미확인(NVC 원본 라이선스 게이트는 구세대에서도 OPEN 과제였음) | REFERENCE_ONLY 유지 |

## 4. 수정 정책

- 현장 직접 수정 금지(17장 금지 규칙 승계).
- 파생 필요 시 새 버전 디렉터리 생성 + 본 문서에 행 추가 + SHA256SUMS 재생성.
- REFERENCE_ONLY 자산을 빌드/런타임이 링크하는 것을 금지한다.
