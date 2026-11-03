# SMC Browser renderer — 구현 분석

## Verdict·책임 경계

pinned Chromium CDP 관찰·측정 effect. caller는 measure runtime → chromium/CDP/render/measured-evidence. 방향 **MAINTAIN**. reachable wiring 확신도 높음, 실환경 검증 확신도 낮음(NOT_RUN).

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | application → `measureTrackBHtml` → Chromium/CDP/render owners | HTML/provenance/profile/viewport + locked browser; measurements/artifacts | 아래 owner 구분 | 정상 result 또는 명시적 오류 | `src/index.js`, `src/trackb-measured-evidence.js` |

`src/index.js` exposes only the measurement operation consumed by Application. Chromium launch,
endpoint/CDP/navigation-guard helpers, measurement error class/constants, and navigation-contract
reexports stay with their defining modules. White-box tests import those modules directly.

## 실제 흐름

locked binary launch → CDP session → exact HTML load → frame/semantic measurements → evidence capture → process/session cleanup → application validation

## 현재 state·contract·I/O owner

renderer owns observation bytes and browser lifecycle, composer owns intended HTML, release workflow owns evidence adoption, human UI owns approval. profile/dimensions are Configuration; waits/timeouts Policy.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 공개 surface | REACHABLE | PARTIAL | NOT_RUN: Rust/실제 provider/제품 runtime; 정적 호출만 확인 | `src/chromium.js, cdp-client.js, render.js, trackb-measured-evidence.js` |
| 독립 renderer package 배포 | ABSENT_IN_SCOPE | N/A | SMC root package is private, no separate renderer package manifest exists, and the only production consumer is SMC Application | root `package.json`, `src/index.js`, application import |

## 의존성·findings

**MB1.** 문제 → browser rendering 결과가 없는 상태에서 source assertion을 visual evidence로 해석할 수 없다. executable pin/실환경 browser가 필수다. 근거 → `src/chromium.js, cdp-client.js, render.js, trackb-measured-evidence.js`. 원인 → 측정 mechanism과 실제 관찰의 차이. 영향 → caller가 작업 완료/승인/복구 가능성을 잘못 해석할 수 있다. 방향 → 원래 CDP route/locked profile을 유지; real browser로 cancellation/startup/close/failure와 intended semantic binding을 확인한다.

## 실패·취소·재시도·복구와 남은 gap

platform absence NOT_RUN; browser state/retry와 release state는 분리. substitute browser/harness를 넣어 gate를 통과시키지 않는다.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)
