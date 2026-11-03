# 01. Boundary & Authority Contract

## NDS 책임

`무엇을 이야기할 것인가`를 확정한다.

- 소재/아이디어 해석
- 주제·의도
- 시놉시스
- 플롯/인과관계
- 캐릭터 arc
- setup/payoff
- 극단적으로 짧은 단편 또는 screenplay 작성
- narrative QA
- canonical 선택

**출력:** `schemas/narrative-package.schema.json`.

## NVC 책임

`확정된 이야기를 무엇을, 어떤 매체로, 어떤 장면/샷/움직임으로 보여줄 것인가`를 결정한다.

NVC가 하면 안 되는 것:

- plot hole을 몰래 고침
- 결말을 바꿈
- synopsis와 story 사이에서 더 마음에 드는 내용을 임의 선택
- 생성 모델이 만든 설정을 source fact로 승격

## Authority matrix

| Artifact | Authority | Mutability | Conflict rule |
|---|---|---|---|
| canonical_narrative | canonical | immutable | 최우선 |
| planning_synopsis | planning evidence | immutable | hard-intent conflict면 `SRC_005` |
| user constraints | policy | immutable | canonical과 직접 충돌 시 review |
| StoryIR / MeaningSpec | derived | owner가 새 version만 생성 | canonical에 종속 |
| DirectorIR 이후 | production-derived | owner가 새 version만 생성 | semantic upstream에 종속 |

## Story vs Screenplay

둘 다 지원하되 **동시에 canonical authority를 주지 않는다**.

- `story`: 내레이션/내면/서술이 풍부하므로 S01이 event/state/meaning을 구조화한다.
- `screenplay`: scene/action/dialogue가 더 구조화되어 있으므로 그 구조를 S01에서 활용한다.

동일 작품에 story와 screenplay가 모두 있으면 NDS가 하나를 canonical로 지정한다. 다른 것은 reference가 될 수 있으나 NVC core source-of-truth에는 포함하지 않는다.

## Normalized external port

```text
NDS.out.NarrativePackage
        ↓
NVC.in.NarrativePackage
```

이 포트 하나가 시스템 경계다.
