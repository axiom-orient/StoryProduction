# SceneMotionCompiler — 제품 계약

## 필수·선택 기능

필수는 Completed NVC release와 승인 cinematic 입력의 검증, 정확한 SceneGraph v3 projection, SAC asset handoff의 검증, scene HTML/provenance, locked measurement, candidate와 사람 approval/custody release 분리다. provider-neutral motion request·clip/QC·editorial 후보와 OTIO interchange는 별도 typed capability다. 실제 provider 생성·최종 영상 release를 이 후보 command의 암묵적 효과로 만들지 않는다.

## 입력·출력·오류·전제조건

| surface | 입력·전제 | 관찰 출력·오류 |
|---|---|---|
| `scene.projectNarrativeBundle` | Completed bundle + current ContinuityBible + CinematicDirectorPlan + CinematicApproval | v3 graph/receipt; source/hash/schema/approval/occurrence drift 거절 |
| `scene.compileHtml` | exact SAC handoff v2, 검증된 source/runtime bytes·권리, 유효 dimensions | 움직이는 HTML + non-executable provenance; partial publication을 typed failure로 보고 |
| `production.prepare` | 닫힌 production draft, 현재 plan/approval/source/media/QC | 해당 단계 candidate/evidence/interchange; generation·approval·release 권한 없음 |
| `trackb.evidence.measure` | current HTML/provenance/authority와 locked runtime | renderer-only measured evidence; 임의 browser fallback 금지 |
| release prepare/approve/promote/status | current candidate subject/authority/receipt + 사람 review + exact CAS base | 승인된 DeliverySet/pointer 또는 실패/uncertainty; 승인과 promote는 별도 |
| animatic render/status/cancel/recover | approved NVC packet·SAC pack/review·실제 PNG·명시 attempt/motion·locked render 환경 | exact-frame WebM/HTML/VTT candidate 또는 cancelled/failed/outcome_unknown. Final approval 없음 |

Animatic은 별도 operator CLI이며 MCP/Studio command로 노출하지 않는다. 동일 attempt의
sourceBinding이 다르면 거절한다. 첫 terminal receipt가 승리하며 취소 뒤 late candidate로 바꾸지
않는다. Recover는 기존 결과/worker identity를 읽거나 unknown을 고정하고 렌더를 재실행하지 않는다.
Candidate의 `finalApproval:false`·`audio:none`과 실제 output hash·frame count/canvas/fps를 결속한다.
Status는 과거 attempt 기록이며 현재 NVC 원고에서의 adoption eligibility를 보장하지 않는다.

## 관찰 가능한 동작·수용 기준

shot 수·ID·순서·durationFrames·camera intent·narrative/semantic refs를 plan대로 보존한다. 미지원 연출을 renderer의 임의 decision으로 보충하지 않는다. 모든 대사 occurrence를 전달하며 고정 4초나 숨은 추가 delay로 approved timing을 바꾸지 않는다.

raw bytes hash와 canonical content identity를 구별한다. capsule/source/graph를 재검증하고 narrative authority의 release/package/map hash를 모두 대조한다. self-hash는 서명이나 human identity 증명이 아니다. current graph 및 승인 이후 바뀐 media/receipt/review evidence는 재검토 없이는 채택하지 않는다.

QC는 media contract와 visual/motion/narrative 관찰을 분리하며 미검토를 PASS로 처리하지 않는다. OUTCOME_UNKNOWN은 reconciliation 전 자동 재시도하지 않고, CANCELLED는 새 실행 권한 없이 다시 생성하지 않는다. timeline은 QC PASS여도 candidate다. 실제 media/audio/transition 지원 여부는 명시적으로 검증해야 하며 지원하지 않는 export는 오류다.

`packages/application/src/command-registry.js`가 application/CLI/MCP/Studio surface와 권한의 단일 owner다. `trackb.release.approve`는 Studio exact loopback/origin/one-shot session에서만 노출한다. 임의 SceneGraph·Markdown·HTML·MP4를 권위 있는 입력으로 우회 수용하지 않는다. native publish는 descriptor 재검증/fsync/exclusive publish/CAS를 따르고 pre-rename 실패와 post-rename durability unknown을 구별한다.

## intentional contract 원본

선택 animatic은 별도 [closed contract set](../contracts/schemas/animatic/)과
[운영 계약](../../docs/ANIMATIC_PRODUCTION.md)을 소비한다. SMC는 렌더 attempt와
media candidate를 소유한다. NVC board/plan 승인과 SAC 자산 승인을 대체하지
않으며 Human release approval을 생성하지 않는다.

[전체 schema](../contracts/schemas/), [NVC boundary](BOUNDARY_NARRATIVE_COURIER.md), [production schema](../contracts/schemas/production/production-v1.schema.json), [registry pins](../../integration/contracts/registry.json)가 세부 계약을 소유한다. NarrativeSceneMap v2, SAC handoff v2, Track B authority/approval/delivery/pointer v2 및 candidate v4는 SceneGraph v3와 독립적인 현재 계약이다. `evaluation-report.schema.json`은 AGENTS에 따라 archival 보존하되 active lookup에 넣지 않는다.
