# StoryProduction — 구현 현황

현재 Story 공개 실행 계층은 native storyctl로 연결된다. Integration의 현재 검증과 미실행 qualification은 [구현 현황](../integration/docs/IMPLEMENTATION_STATUS.md), 제품 계약은 [SPEC](../integration/docs/SPEC.md), owner/effect 경계는 [아키텍처](../integration/docs/ARCHITECTURE.md)에 둔다. 아래 내용은 전체 저장소의 owner 경계와 source-specific 과거 근거를 요약한다.

## 주 제작 경로와 소유자

기본 경로는 원문 → geultu story/synopsis 후보·채택 → NVC STS 씬·대본 → NVC 텍스트 콘티 후보·검토 → immutable package다. 선택 animatic은 별도 NVC → SAC → SMC 후보 흐름이다. Integration은 public entrypoint를 조합하고 후보·사람 승인·current source/hash를 구분한다. 완료 marker나 candidate는 작품 품질 승인이 아니다.

| Owner | 권위 상태 | 코드·책임 분석 | 현재 구현·qualification |
|---|---|---|---|
| geultu | 원문·브리프·본문·시놉시스·후보와 사람의 결정 | [ANALYSIS](../geultu/ANALYSIS.md) | [Current](../geultu/docs/IMPLEMENTATION_STATUS.md) |
| NarrativeCourier | STS 서사 정본·Human gate·Completed release·텍스트 콘티 | [ANALYSIS](../NarrativeCourier/ANALYSIS.md) | [Current](../NarrativeCourier/docs/IMPLEMENTATION_STATUS.md) |
| integration | public owner 조합·attempt/recovery·불변 export/package·SEMI adapter | [SPEC](../integration/docs/SPEC.md) · [ARCHITECTURE](../integration/docs/ARCHITECTURE.md) | [Current evidence and NOT_RUN](../integration/docs/IMPLEMENTATION_STATUS.md) |
| ImagePromptCompiler | typed prompt compile/interview/catalog와 provider별 adapter | [ANALYSIS](../ImagePromptCompiler/ANALYSIS.md) | [Current](../ImagePromptCompiler/docs/IMPLEMENTATION_STATUS.md) |
| SceneAssetCompiler | 자산 attempt·source binding·Human review·handoff | [ANALYSIS](../SceneAssetCompiler/ANALYSIS.md) | [Current](../SceneAssetCompiler/docs/IMPLEMENTATION_STATUS.md) |
| SceneMotionCompiler | 승인 입력 projection·합성·측정·media candidate·custody pointer | [ANALYSIS](../SceneMotionCompiler/ANALYSIS.md) | [Current](../SceneMotionCompiler/docs/IMPLEMENTATION_STATUS.md) |

2026-10-04 Story integration 실행 gate와 Swift provider 테스트는 [integration Current](../integration/docs/IMPLEMENTATION_STATUS.md)에 **PASS** 근거와 정확한 범위가 기록되어 있다. 이는 actual user source, model/provider 품질, production SEMI/XPC·consent, human adoption, 최종 publication을 검증한 결과가 아니다. 선택 확장의 상세 잔여 범위와 qualification은 각 owner의 linked Current/PLAN이 정본이며 Story native dispatcher 전환은 해당 owner의 Python/JavaScript 실행기나 개발 toolchain을 대체하지 않는다.

## 이전 source별 실행 근거

아래 기록은 당시의 입력·코드 snapshot에만 적용한다. 현재 Story integration 실행 상태는 [integration 구현 현황](../integration/docs/IMPLEMENTATION_STATUS.md)을 따른다.

- [2026-10-02 animatic implementation](research/2026-10-02-animatic-implementation.md): NVC/SAC/SMC의 실제 fixture/media 경로와 해당 snapshot의 boundary evidence. 합성 PNG와 review를 사용했으므로 실제 자산 생성·작품 품질·Human approval·최종 release를 인증하지 않는다.
- [2026-10-01 current-tree revalidation](research/2026-10-01-live-use-revalidation-current-tree.md) 및 [follow-up](research/2026-10-01-live-use-followup.md): 당시 synthetic owner chain과 각 gate의 성공·실패·NOT_RUN 기록.
- [2026-10-01 owner-cleanup snapshot](research/2026-10-01-live-use-validation-after-owner-cleanup.md) 및 [2026-09-30 source validation](research/2026-09-30-current-source-real-use-validation.md): 해당 source의 blocked fixture와 합성 screenplay fidelity 결과.

각 문서에 연결된 transcript, hash, manifest와 실패 근거는 research 원본에서 보존한다. 과거 PASS를 이후 source나 현재 product qualification에 자동 적용하지 않는다.
