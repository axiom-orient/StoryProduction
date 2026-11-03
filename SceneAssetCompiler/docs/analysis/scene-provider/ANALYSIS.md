# SAC typed Scene Provider — 구현 분석

## Verdict·책임 경계

제한된 reference-free PNG 생성의 process/network effect owner다. caller는 execute-scene-provider/probe-scene-provider다. 방향 **MAINTAIN**; 실환경 qualification은 필요하다. 상위 제품과 별도 제품으로 포장하지 않으며, 이 문서는 독립 material state/effect의 상세를 소유한다. wiring 확신도는 높고 실환경 결과는 아래 범위로 제한한다.

## Module ownership

`scene_provider_identity.py` owns executable path rules, strict component-lock loading, and exact provider/PerfectPixel hash identity. `scene_provider_preflight.py` owns sealed protocol/schema probes and public preflight functions. `scene_provider_protocol.py` owns the fixed v2/PerfectPixel values, duplicate-key bounded JSON parser, deterministic request-byte encoding, request/response/error envelopes, and PNG payload agreement. `scene_provider_process.py` owns required provider environment checks, process-group identity, teardown, and the shared bounded child runner used by probes and PerfectPixel inspection. `scene_provider_inspection.py` owns PerfectPixel inspection and returned-byte/hash/dimension/alpha agreement. Both protocol validation and inspection use SAC-local `png_inspection.py` directly; neither imports the review workflow. `scene_provider_attempt.py` owns admission, immutable prior-attempt checks, receipt finalization, and postflight error evidence. `scene_provider.py` is the image dispatch orchestrator and owns the selected provider's direct spawn/communication sequence because it persists `PROCESS_EVIDENCE` with the live PID/group before waiting. It uses the shared teardown primitives, but does not route dispatch through `_run_bounded_process`. Only the canonical Scene Provider executable, receipt tool identity, evidence names, Python functions, and CLI commands are accepted; noncanonical attempt evidence fails closed.

The previous protocol/process/inspection split had a successful facade import, CLI help, 14 publication-boundary tests, repository local gate, and manifest snapshot. The additional identity/preflight/attempt split above is source-only and has not been tested, built, linted, or manifest-verified under the current request. Those earlier checks do not execute the typed provider or PerfectPixel. Their pinned binaries are absent from this checkout and the required provider environment variables are unset, so actual inspection/generation remains NOT_RUN.

## 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| probe/preflight | CLI → preflight_components/probe_scene_provider_protocol | exact binary hash·misegen.provider/2·component lock·PerfectPixel schema | side-effect-free protocol probes | sealed identities 또는 reject | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_provider.py |
| attempt admission | execute_scene_provider → _attempt_contract/_request | PNG·no reference/edit target·capsule/hash·permission | one immutable dispatch record | typed request 또는 pre-dispatch failure | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_provider.py |
| external dispatch | execute_scene_provider → Popen/communicate | exact request bytes·deadline·pre-wait process evidence | process-group teardown + immutable attempt evidence; provider owns network | response/unknown/no implicit retry | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_provider.py |
| inspect/finalize | _inspect_with_perfectpixel → _finish_attempt_safe | PNG bytes/hash/dimensions/alpha + tool receipt | attempt receipt/result/mechanical QA | ready-for-visual-review or UNKNOWN | ../../../.agents/skills/scene-asset-compiler/scripts/scene_asset_compilerlib/scene_provider.py |

## 실제 흐름

`attempt → reference-free admission → sealed probes → immutable dispatch record → bounded provider process → strict response → PNG inspect → bound receipt → mechanical review`. Dispatch persists its PID/group/deadline before `communicate`, then uses the process module's identity-checked teardown helpers on timeout or surviving descendants. Protocol and PerfectPixel helper processes use `_run_bounded_process`. Reference/edit/cutout/PSD는 이 lane으로 fallback하지 않는다.

## 현재 state·contract·I/O owner

protocol/version은 Constant; executable/hash/lock은 Configuration; timeout/retry prohibition은 Policy; dispatch/teardown/receipt는 attempt State. provider가 실제 paid/network effect를 소유하며 local process termination은 network rollback이 아니다.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| provider v2 process routing | REACHABLE | UNKNOWN | NOT_RUN: 외부 locked provider·PerfectPixel 없음 | scene_provider.py |
| reference/edit/PSD fallback | ABSENT_IN_SCOPE | UNKNOWN | N/A: 의도적으로 다른 effect | _attempt_contract |

## 의존성·findings

**AP1.** 문제 → source 안의 protocol 구현을 실제 packaged-binary 성공으로 오인할 위험. 근거 → binary/hash lock preflight 및 누락된 original provider fixtures. 원인 → source와 외부 실행 증거의 혼합. 영향 → fake qualification. 방향 → 현재 code/optional capability는 보존하고 실제 negative probe, timeout/no-survivor, PNG/alpha/inspect binding 검증을 PLAN에 남긴다.

## 실패·취소·재시도·복구와 남은 gap

UNKNOWN 뒤 자동 재시도·v1 downgrade 금지. process group custody에도 PID reuse, escaped process/session, host crash에는 한계가 있다. fixture executable은 provider 인증·품질·비용 증거가 아니다.

## gap·검증·근거

이번 일반 workflow14 및 integration29는 이 provider의 원래 live/negative suite를 대신하지 않는다. read-only source 추적만 수행했고 외부 process는 NOT_RUN.

[상위 ANALYSIS](../../../ANALYSIS.md) · [제품 Current](../../IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../PLAN.md). source 근거 경로는 이 분석 단위 기준이다. PASS는 표시한 검사에만 적용하며 PUBLIC_LIBRARY/REACHABLE은 실행 성공을 뜻하지 않는다.

The CLI imports `probe_scene_provider_protocol` from `scene_provider_preflight` directly.
`scene_provider` exports its execution function without forwarding probe/preflight names. Internal
component preflight still precedes provider dispatch; policy, evidence and uncertainty are unchanged.
Unused inspection hashing imports are removed. This source-only change is NOT_RUN.
