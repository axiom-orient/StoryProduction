# NarrativeCourier — 제품 계약

## 필수·선택 기능

핵심은 검토 synopsis 입력, STS node·task token·Human gate, Completed release와 producer package/map/bundle의 무결성이다. 텍스트 콘티는 현재 씬에서 후보·부분 수정·별도 검토·문서 bundle을 제공한다. Delivery, cinematic, animatic은 서로 다른 투영/연출 계약이다. Fountain/FDX import/export, DOCX, MCP, desktop UI와 외부 renderer도 공개 기능이다.

## 입력·출력·오류·전제조건

| 경계 | 입력·전제 | 출력·거절 조건 |
|---|---|---|
| reviewed import | exact producer handoff schema/raw synopsis hash·approval claim | bound source project; hash/path/schema drift 거절 |
| agent submit | current task/source token·declared output contract | node completion/blocked; stale result 거절 |
| Human decide | current gate token·allowed decision·artifact identity | 공개 CLI approve/revise/pause; 자동 human 결정 없음 |
| release read/export | Completed·current source/package/map/state binding | exact v2 package/map + v1 bundle; 불완전 입력 거절 |
| Completed resume | 동일 승인 source/package·bundle 존재/무결성 | state 변경 없이 검증, 누락 파생 bundle만 복구; 손상 덮어쓰기 없음 |
| cinematic | Completed source + current continuity + explicit shot draft + 사람 review hash | 별도 bible/plan/approval; duplicate/unknown/stale refs·duration/order·semantic coverage 위반 거절 |
| text storyboard | current Completed source·씬/semantic occurrence·task/base·exact hash review | 후보/diff·문서 export·전체 승인 bundle. 취소·stale·선택/coverage 오류 거절 |
| animatic direction | current approved board·별도 plan draft·review·원문 caption IDs | source/submit/review/export packet. 기간·keyframe·panel/asset coverage·unresolved 승인 거절 |
| delivery | explicit released package·typed request·descriptor-supported capability | plan/AssetIndex/DeliverySet 또는 명시 unsupported/block/error |
| review/cancel | current request/result·사람 결정 또는 cancel request | review 기록 / publish ownership 결과 |

## 관찰 가능한 동작·수용 기준

NVC는 유일한 Narrative semantic owner다. downstream는 screenplay를 새 의미로 해석하지 않는다. SUMMARY/MOOD·번역/locale 재서술·미지원 renderer를 근사 성공으로 처리하지 않는다. unsupported는 explicit failure다. `NEEDS_REVIEW`와 technical PASS를 Human approval과 구분한다.

request id와 content를 결합해 중복 반영을 막고 source/artifact drift를 소비 전·최종 publish 전에 검증한다. 취소와 publish의 경쟁은 한 owner가 결정하며 늦은 결과가 최신 상태를 덮어쓰지 않는다. 오류 후 상태·이미 발생한 effect를 명시한다. Human actor 문자열과 raw hash는 무결성/승인 주장이지 cryptographic identity 증명은 아니다.

텍스트 콘티 task의 `cancel`은 `prevent_future_adoption` tombstone을 발행한다. 이미 저장된
board·Human review receipt는 증거로 보존하지만, 그 task의 board를 사용하는 새 후보/승인 export,
전체 bundle, animatic source/submit/review/export는 거절한다. 기존 승인 기록을 삭제하거나
과거 발행물을 자동 rollback하지 않는다.

cinematic plan은 scene마다 명시적인 여러 shot을 허용하되 source semantic occurrence를 정확히 배정하고 shot duration/camera/reference 필요성을 결정해야 한다. 추출 불가능한 visual 사실은 unresolved다. 기존 delivery DirectorPlan/다섯 DTO를 새 cinematic contract로 덮어쓰지 않는다. 구체적 권한 분리는 [ADR0003](../../docs/adr/0003-cinematic-authority.md)을 따른다.

## 호환성과 owner 경계

사용자 원문·reference provenance, 유효한 optional capabilities와 기존 producer/public contracts는 각 owner의 입력과 계약이다. Candidate·review·approval 및 Delivery의 publish 결과는 분리된 상태로 남긴다. Transport, UI, renderer, provider와 내부 effect modules는 source authority·approval·idempotency·failure/recovery 의미를 보존하는 범위에서 바꿀 수 있다.

## intentional contract 원본

[contracts](../contracts/), [process protocols](../protocol/), [active STS workflow](../skills/sts/), [Delivery Blueprint](blueprint/README.md), [SceneMap v2](NARRATIVE_SCENE_MAP_V2.md)를 원본으로 참조한다. consumer reviewed-synopsis schema는 글투 producer pin을 따른다. MCP에는 Human gate decision·delivery review를 노출하지 않는다.

현재 STS domain version은 `workflow.json`/`audit_workflow.json`의 `4.3.0`이다. STEP 0은 typed
SRC/REL inventory와 exact-source span coverage를 요구한다. STEP 8D는 승인 Source IR와 source-blind
recovered Screenplay IR을 대조한다. 구조 guard는 모델의 의미 정확성이나 Human 승인을 대신하지 않는다.
다섯 canonical Delivery DTO와 각 producer artifact schema를 구분하고 consumer copy는
[상위 registry](../../integration/contracts/registry.json)에서 pin한다.
