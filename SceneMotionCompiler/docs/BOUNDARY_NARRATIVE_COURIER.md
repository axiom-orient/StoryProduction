# NarrativeCourier → SceneMotionCompiler 경계

독자: NVC producer·SMC adapter와 외부 CLI/MCP caller. 승인 narrative와 shot 의미의 교환 계약이며 screenplay/synopsis Markdown을 재파싱해 새 연출을 생성하는 API가 아니다.

## 입력과 활성 조건

`scene.projectNarrativeBundle`은 `releaseBundlePath`, `continuityBiblePath`, `directorPlanPath`, `directorApprovalPath` 네 입력을 요구한다. CLI는 각각 `--bundle`, `--continuity`, `--director-plan`, `--director-approval`이다. Completed state/package/scene-map raw hash·source snapshot과 bible/plan/approval 관계가 모두 일치해야 한다. producer 원본은 NVC `contracts`, consumer copy는 SMC `contracts/schemas`, pin은 상위 integration registry에 둔다.

## projection 계약

```text
NVC Completed release + ContinuityBible v1 + approved CinematicDirectorPlan v1
 → NarrativeSceneProjection v3
 → staged SceneGraph v3
 → SAC SceneMotionAssetHandoff v2 또는 visual-reference pack v1
```

NVC scene에서 entity/semantic occurrence를 추출하지만 shot은 **승인 plan에서만** 온다. plan shot 하나는 graph shot 하나다. sceneCount와 shotCount를 구별하고 ID/order/frame duration/camera/narrative refs를 보존한다. 1 scene=1 shot, duration=4초 fallback은 없다. 기존 delivery `DirectorPlan`과 cinematic plan은 다른 계약이다.

`sourceNarrativeId`에는 NVC `narrative_id` 원문을 그대로 둔다. canonical NVC ID가 downstream 길이·문자 제한을 만족하면 SceneGraph/projection ID에서 그대로 사용하고, 그 외에는 전체 raw ID의 SHA-256으로 bounded `narrative.nvc-…` ID를 도출한다. lossy slug/truncation이나 constant `untitled` fallback은 없다. 예전 noncanonical ID 변환에 기대는 저장 projection은 호환 경로를 두지 않고 Completed release에서 다시 projection해야 한다.

receipt `projection/narrative-scene-projection.v3.json`은 staged graph를 먼저 만든 뒤 마지막에 publish한다. capsule에는 scene-map/bible/plan/approval의 원본 bytes와 raw hash가 들어간다. consumer는 capsule을 검증하고 expected graph를 다시 만들어 현재 graph와 비교한다. capsule은 media 전체 보관 store가 아니다.

## downstream·오류·권한

SAC handoff v2 loader는 projection/graph/asset pack/audit/binding/approval/source/runtime bytes 및 rights를 재검증한다. Track B authority의 release-bundle/package/scene-map hash도 receipt와 모두 대조한다. stale/tampered/symlink/traversal/권리 미승인은 실패다. hash 서명과 인증된 사람 identity를 혼동하지 않는다.

CLI/MCP는 projection과 후보 준비를 노출하지만 final human approval은 Studio의 exact loopback/origin/one-shot session만 제공한다. 영상 clip/QC/editorial candidate는 기존 HTML custody의 final release subject가 아니며 자동 승격하지 않는다. SceneGraph v2 compatibility wrapper는 없고 내부 caller는 v3로 함께 전환한다. 외부 v2 consumer 존재 여부는 UNKNOWN이며 전환 권한을 추정하지 않는다.

사용 명령은 [상위 운영 가이드](../../docs/OPERATIONS.md), 남은 qualification은 [Current](IMPLEMENTATION_STATUS.md)와 [PLAN](PLAN.md)을 따른다.
