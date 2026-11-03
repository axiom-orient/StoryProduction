# SceneAssetCompiler — 제품 계약

## 필수·선택 기능

필수는 source/VisualAuthority·reference 정규화, task/attempt 및 generation capsule, 실제 반환 artifact의 검증, 선택·승인·AssetPack 발행이다. SceneProductionPlan/placeholder/layer locks/PSD postflight, SMC scene-release, 제한된 typed provider는 각각 독립 capability다. 로컬 이미지/PSD writer는 비목표다.

## 입력·출력·전제조건

| 경계 | 입력·검증 | 관찰 출력·오류 |
|---|---|---|
| init/compile-plan | RunRequest·authority/anchor/dependency/reference | compiled task set; 실패를 생성 중으로 표시하지 않음 |
| begin/execution | 현재 refs/edit target와 capsule·provider route | immutable attempt/receipt; 유효하지 않으면 dispatch 안 함 |
| review/approve | current returned bytes·mechanical/visual/layout required axes | selection 또는 blocked/repair; 해시 drift 거절 |
| human-approve/publish | request 정책·정확한 선택 전체·reviewer | 같은 선택에 bound approval/portable pack; stale approval 거절 |
| scene-release | `SMC_NARRATIVE_SCENE_PROJECTION` receipt/root/SHA와 asset rights/lineage | approved scene asset handoff/v2 |
| visual-reference | current SMC projection v3 + 실제 entity/keyframe bytes·reference roles + explicit review | hash-bound pack/approval·derived index; drift/unknown entity/missing keyframe 거절, 자동 generation 없음 |
| typed provider | strict `misegen.provider/2`, locked binaries, reference-free PNG | bound PNG/inspection result 또는 UNKNOWN; fallback/retry 금지 |
| PSD | 실제 반환 PSD·profile·canvas/layer/alpha/ICC/path evidence | structural findings와 별도 pixel/render-back 검증 범위 |
| animatic assets | approved NVC packet·명시 asset 요구·실제 PNG·exact packHash review | SAC assets/review producer artifact; hash/크기/alpha/coverage·approval/packet mismatch 거절 |

Animatic의 submit은 후보 발행, inspect는 실제 파일의 계약 검사, review는 사람이 검토한
packHash/packetHash의 결정이다. RGBA 형식 선언과 실제 cutout alpha를 구분하고 전체 투명
PNG를 거절한다. Scene-provider-v2 provenance에는 opaque PNG만 허용한다. 이미지 생성·
matting·NVC 연출 변경·SMC 렌더·최종 release는 이 public entrypoint의 effect가 아니다.

## intentional 동작·수용 기준

선택·승인·최종 발행은 별도 경계다. 승인 후 내용이 바뀌면 재검토·재승인한다. `human_final_review=false`라는 유효 request 정책은 보존하되 후에 request를 바꿔 승인을 우회할 수 없어야 한다. human receipt의 schema를 임의 개정하지 않는다.

실제 파일이 PNG라면 PSD라는 prompt/확장자만으로 성공하지 않는다. `PREFERRED_OUTPUT_MISSING`은 해당 호출의 실패이며 PSD의 전역 미지원 판정이 아니다. 승인된 raster+exact retry lock+bound `SUBMITTER_ATTESTED` 비교 선언으로 허용된 PSD-only retry는 최대1회다. 제출자의 비교 선언을 독립 실행/인증 provenance로 격상하지 않는다.

reference/identity/STYLE의 역할과 허용 preserve axes를 구별한다. reusable transparent object의 양수 per-edge margin, wireframe slot layout/source/output binding, exact locked canvas·leaf layers·ordinary alpha·Named Saved Path·clipping policy를 유지한다. local matting/tracing/PSD assembly로 실패를 숨기지 않는다.

visual reference 승인 subject는 pack 내용과 narrative/plan/graph/continuity, shot/entity, actual media hash를 결합한다. memory index는 승인 pack에서 파생하며 별도 character truth를 만들지 않는다. rich visual identity 개발은 원본 narrative 사실과 구별하고 승인 없이 확정하지 않는다.

## 계약 원본

[active skill](../.agents/skills/scene-asset-compiler/SKILL.md), [active schemas](../.agents/skills/scene-asset-compiler/schemas/), [consumer/release contracts](contracts/), [PSD profile](../.agents/skills/scene-asset-compiler/references/32-gpt-psd-generation-profile.md), [render contracts](../.agents/skills/scene-asset-compiler/references/33-render-contracts.md)가 필드별 원본이다. tests/fixtures의 부재나 현재 미검증은 규범을 완화하는 이유가 아니다.
