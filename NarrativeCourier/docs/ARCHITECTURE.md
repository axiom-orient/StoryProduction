# NarrativeCourier — 아키텍처

## 경계·책임·의존 방향

`Human + STS → NarrativePackage/release → deterministic DirectorPlan → AssetIndex → renderer → verification → DeliverySet`의 한 방향 권한 흐름을 따른다. STS만 서사 의미를 결정하며 delivery는 승인된 의미를 projection한다.

| 단위 | 소유 | 소유하지 않음 |
|---|---|---|
| `crates/sts` | source snapshot, node/task/gate, Human 결정, Completed, package/map/bundle, Fountain/FDX·DOCX 경계 | delivery request/render 상태 |
| `crates/contracts` | 계약별 DTO·cross-field/cross-document invariant, 기존 crate-root exports | process·UI·workflow 실행 |
| `crates/delivery` | DirectorPlan, descriptor routing, assets/render orchestration, verification, review | 서사의 요약·재해석·새 생성 |
| `crates/runtime` | path/hash/JCS/lock/process/cancel/atomic publish primitive | STS/DirectorPlan/renderer 정책 |
| cinematic authoring tools | ContinuityBible/CinematicDirectorPlan/Approval producer, explicit review·immutable publication | 기존 delivery DTO·renderer·자산 approval |
| `crates/authoring` · native `storyboard` | Completed release에 결속된 text-board task/candidate/review/export | STS scene 의미 변경·사람 결정 대행·Delivery DTO·media generation |
| `crates/authoring` · native `animatic` | 승인 board에서 plan/review/caption packet, frame·layer·pose·asset 요구 | 실제 PNG 생성·자산 승인·media renderer·final release |
| `crates/app-config` | assets/projects root 해석과 명시 설정 | domain state·silent fallback |
| `crates/mcp` | engines로 위임하는 agent transport | Human gate 결정·delivery review 노출 |
| `src-tauri`/`ui` | CLI/desktop composition·presentation | 별도 domain state |

Domain은 contracts/runtime에 의존하고 Runtime은 domain에 의존하지 않는다. CLI가 src-tauri package에 있어도 별도 서사 owner로 간주하지 않는다.

Tauri/headless packaging을 바꾸려면 실제 caller와 lifecycle owner를 확인하는 별도 설계 결정이 필요하다. 현재 두 surface는 같은 STS/Delivery engines를 사용한다.

## 실행·상태 모델

STS는 source hash, task token, Human gate token을 재검증하며 final approval의 artifact 집합을 봉인한다. NarrativePackage 생성, Completed 저장, release-bundle 발행의 의미를 구별한다. Completed 재개는 새 승인이나 state rewrite가 아니라 검증된 동일 승인 결과의 파생 산출물 복구여야 한다. 누락과 손상을 구분한다.

Delivery는 Completed API에서만 입력을 받아 immutable request→plan→assets→render→verification을 수행한다. semantic BLOCK은 render 전에 종료한다. 같은 request id의 다른 content는 거절하고 같은 completed content는 bytes를 재검증한 뒤 재사용한다. `delivery-set.json`은 request의 마지막 commit marker다.

별도 cinematic branch는 `Completed release→ContinuityBible→explicit shot draft→plan validation→operator approval`이다. output은 원본 raw hash 및 content identity로 봉인하고 같은 output의 다른 content를 덮어쓰지 않는다. NVC schema가 producer이고 SMC copy는 pin으로 동기화한다. 새로운 전역 DB/공유 mega-domain은 두지 않는다.

텍스트 콘티 branch는 `Completed release→source-bound scene/semantic task→candidate board→separate human review→text export/bundle`이다. 공개 `narrative-courier-cli storyboard`는 `crates/authoring`으로 위임한다. `project`가 held directory FD·shared STS lock·current source capture·immutable publication을, `codec`이 strict JSON과 기존 pretty/JCS bytes를, `schemas`가 embedded producer admission을, `storyboard`가 lifecycle·coverage·render·diff를 소유한다. `animatic`은 같은 release·approved board 관찰을 조합한다. 이 결과는 NVC-owned 문서이며 새로운 scene/narrative truth나 media renderer가 아니다.

Storyboard는 현재 release를 한 번 capture해 task·후보·review·bundle에서 사용하며 response/diff와
bounded UTF-8 bytes를 준비한 뒤 불변 발행한다. 취소 tombstone도 seal·effect를 판정한다.
동일 bytes 재사용, 발행 후 오류의 unknown outcome과 승인 전 preview를 구분한다.
Authoring의 Python-compatible insertion-order pretty JSON은 private ordered DOM이 소유한다.
Schema/typed contract admission에서만 serde_json::Value로 변환하며 기존 workspace의
serde_json map·serializer feature를 변경하지 않는다. Raw publication bytes와 integer-only JCS는 별도 profile이다.

Animatic은 같은 source lock 아래 승인 board·plan·review를 다시 읽어 원문 caption text와
soundNotes를 export한다. Packet은 NVC producer, PNG pack/review는 SAC producer, render result는
SMC producer다. Integration은 public CLI와 불변 packet으로 연결하며 이 owner를 합치지 않는다.

## persistence·transaction·동시성·복구

STS output과 `visual/requests/<request_id>/`를 분리한다. cancel과 final publish는 `.delivery-active`의 atomic ownership claim으로 순서를 정한다. output-local BuildKey는 plan·관련 asset provenance·backend fingerprint·renderer version으로 묶고 매 재사용 전에 검증한다. USER/DERIVED 입력은 exact path/hash를 가진 SCN-scoped 자산으로 materialize하며 **delivery 내부에** 새 visual semantic planner를 만들지 않는다. cinematic intent는 Completed release에서 갈라지는 별도 NVC stage이며 동일 delivery request state를 공유하지 않는다.

프로세스 stdin·wait·probe·timeout/cancel·cleanup은 Runtime 경계 안에 둔다. POSIX process group/atomic replacement가 지원되지 않으면 조용히 약한 구현으로 전환하지 않는다. `NEEDS_REVIEW`는 기존 DeliverySet에 대한 명시 Human review이며 기술 PASS가 자동 human approval은 아니다.

Runtime의 Unix lifecycle은 nonblocking stdin·leader observation·group liveness·capture monitoring을
한 loop에서 처리한다. Private capture는 exclusive directory/files, per-stream 8 MiB monitoring과
bounded reads를 사용한다. Poll 사이 overshoot가 가능하며 kernel disk quota는 아니다.
Path/hash/JCS/token·store publication은 domain-neutral owner에 남는다. 실제 host·fault qualification은
[Current](IMPLEMENTATION_STATUS.md)에 따로 둔다.

## 코어·선택 기능·확장

필수 의미는 [SPEC](SPEC.md), delivery field 계약은 [Blueprint](blueprint/README.md)가 소유한다. SUMMARY/MOOD·audience/locale 재서술·free-form semantic constraints·procedural generation은 별도 의미 권한 없이 추가하지 않는다. scene dependency/character relation DAG는 선언 Markdown을 추측 파싱해 확정하지 않는다. 공식 machine contract가 없는 확장은 결정 필요다. 외부 renderer·MCP·UI는 교체 가능하되 release/승인/실패 의미를 보존한다.
