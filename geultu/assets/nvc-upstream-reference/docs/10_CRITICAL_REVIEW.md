# 10. Critical Review

## 1. NVC가 synopsis를 직접 발전시키는 구조

**폐기.** canonical pipeline에서는 NDS가 완성 서사를 책임진다.

## 2. 자연어 agent relay

**축소.** 역할명보다 IR + StageContract를 authority로 둔다.

## 3. 중간 IR

**유지·강화.** semantic drift, call-site ambiguity, retry ownership을 줄인다.

## 4. Director 다양성

**유지.** genre→style 고정이 아니라 meaning→affordance→feasibility.

## 5. 모든 장면 생성 영상

**폐기.** 2.5D/rig/3D/procedural/generative를 shot별로 route한다.

## 6. 캐릭터 consistency만 검사

**불충분.** causal/intention/emotional/consequence trajectory까지 hard semantic QA.

## 7. 마지막 단계에서만 QA

**폐기.** 각 pass 직후 verifier를 둔다.

## 8. universal score

**폐기.** semantic/continuity/technical hard gates와 aesthetic soft metric을 분리한다.

## 9. 전체 재생성

**폐기.** artifact dependency graph 기반 incremental rebuild.

## 남은 구조적 위험

1. LLM semantic parser가 자신있게 틀릴 수 있음.
2. VLM evaluator와 generator의 correlated blind spot.
3. preset library가 반복되어 corpus가 획일화될 수 있음.
4. backend capability registry가 stale해질 수 있음.
5. hard semantic evaluator가 불확실한데 자동 PASS할 위험.

대응 원칙은 `fail-open 금지`, `evidence`, `NEEDS_REVIEW`, `root-cause repair`다.
