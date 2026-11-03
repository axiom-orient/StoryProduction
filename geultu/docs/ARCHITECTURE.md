# 글투 — 아키텍처

## 경계·책임·의존 방향

`CLI/MCP/Studio → Application facade → application/dispatch owner handler → domain/review/generation → Workspace/effect adapter`를 따른다. `application/requests`는 tool DTO, `application/tools`는 공개 tool schema, `application/dispatch` 하위 모듈은 도메인별 caller 연결을 소유한다. WritingAdapter는 순수 계획을, GenerationProvider는 외부 draft를 만든다. 원문 상태를 쓰는 owner는 Workspace/domain이며 provider/UI가 이를 대신하지 않는다.

| concern | authoritative owner | projection/소비자 |
|---|---|---|
| project·scene·fact·timeline·thread 모델 | `geultu::model` | UI/Markdown/export |
| invariant·Command→Event·evolve | `geultu::domain` (`command`, `validation`, `reducer`) | Application and Workspace persistence |
| events·snapshot·index·lock·recovery | `geultu::store` (`journal`, `file_io`, `lock`, `recovery`, `handoff`, `history`) | Workspace가 commit을 조정하고 recovery가 projection을 재구성 |
| text-run budget·snapshot gate | `admission` (`budget`, `snapshot`) | Domain effect admission and stored Project conformance |
| tool DTO/schema/dispatch | `application::requests`, `application::tools`, `application::dispatch/*` | CLI/MCP/Studio |
| source/story/synopsis authoring | `authoring` (`api`, `mutation`, `freshness`, `read`, `conformance`) | Geultu project journal; no automatic candidate adoption |
| generation plan/draft/registry | `geultu::generation` (`registry`, `hash`) | writing/process adapters; plans remain non-canonical |
| craft-pack discovery/load/status | `craftpack` (`paths`, `loader`, `status`, `reference`) plus `integrity` (`parse`, `verify`) | verified pack data cited by generation; never narrative authority |
| review/proof·voice·refinement | `review/{diagnostics,proof,text}`, `receipts/{subject,validation,decision,timestamp}`, `voice/{profile,measurements,compare,guide}`, `refinement/{catalog,diagnose,proposal}` | 후보·receipt·guide; review/proof는 읽기 전용 projection, active voice profile은 Project state |
| visual admission/StoryPackage | `visual_run`/`story_package` | NVC 권한을 발명하지 않음 |
| Studio browser draft/review presentation | `adapters/geultu-studio/web/app.js` | 장면 편집 buffer·미제출 수정안·dirty flag는 일시적인 UI 상태다. 저장과 결정은 공개 Application command로 전달하며 owner 결과 재읽기에 실패하면 명시적 재확인 전까지 결정을 막는다. |
| process lifetime·HTTP framing | 각각 process writer/Studio | 원문 commit 권한 없음 |

## 실행·상태 모델

쓰기의 순서는 `validate → project lock → load → decide → pure evolve/schema preflight → journal commit → snapshot/index projection → result`다. `authoring.prepare`의 선택적 `expected_project_revision` 비교는 같은 lock 안에서 idempotent replay 확인 뒤, journal append 전에 수행한다. canonical content revision과 audit revision을 구별하며 candidate/changeset metadata만으로 원문 revision을 올리지 않는다. command identity는 같은 반영을 중복 생성하지 않아야 한다.

## persistence·권한·복구

journal이 state의 기준이고 snapshot/index는 복구 가능한 projection이다. commit 후 projection 실패는 반영 revision을 포함한 오류로 구별한다. 복구는 명시 호출이며 lock을 시간만으로 자동 탈취하지 않는다. `remove_lock=true`는 살아 있는 writer가 없다는 운영 전제에서만 사용한다. handoff export도 동일 project state를 읽어야 하며 consumer는 raw bytes/hash를 다시 검증한다. 여러 파일/이벤트의 실제 crash 범위는 Current와 PLAN에서 공개한다.

## 값의 분류

text/identifier/schema version은 Constant, ProcessPolicy/승인·budget 규칙은 Policy, workspace/program/args는 Configuration, user 선택은 해당 요청의 입력이다. revision/candidate/lock은 State다. persisted Settings나 Feature Flag owner가 필요하다는 근거가 없으면 새 저장소를 만들지 않는다.

## 집필 최신성과 journal 발행

`authoring::Freshness`는 한 immutable Project를 빌려 guidance/document hash와 currentness를
요청 안에서만 재사용한다. Task input/base는 content identity인 block 값을 정확히 대조하고,
selection은 block index를 사용한다. Review와 export는 현재 source/guidance를 같은 owner에서 판단한다.

Journal publisher는 held regular file의 identity/length를 대조해 기존 NDJSON prefix와 새
command event 묶음을 같은 stage에 쓴 뒤 atomic replace한다. Snapshot·index는 commit 뒤의
projection이며 동일 command/create 재개도 공통 projection writer로 복구한 뒤 응답한다.
전체 prefix 복사의 I/O와 과거 손상/no-op 복구 한계는 [Current](IMPLEMENTATION_STATUS.md)에 둔다.

## 코어·선택 기능·확장

[GENERATION](GENERATION.md)의 public trait와 JSON process protocol을 확장점으로 사용한다. 외부 결과는 승인 전 후보이며 type-specific reducer·provider SDK·retry engine·두 번째 UI state store를 코어에 넣지 않는다. repository trait, actor framework, HTTP framework는 단일 local storage/동기 transaction의 정확성을 높이는 근거 없이 추가하지 않는다.

[채택 결정](adr/0001-contract-adoption.md) · [계약](SPEC.md)

## Domain engine의 명령과 replay 경계

`domain::decide`의 exhaustive dispatch는 명령별 typed owner function으로 바로 이동한다.
Owner마다 전체 Command를 다시 받거나 잘못된 owner 경로를 runtime 오류로 유지하지 않는다.
Command/Event serde와 public facade는 유지한다. Bulk scene import는 내부 중복 ID를 거절하며
changeset 중복 판정은 borrowed identifiers를 사용한다.

Reducer는 신규 records를 덮어쓰지 않는다. Receipt/HumanDecision의 identical replay만 별도
immutable-record 정책으로 수용한다. Project/content/scene revision increment는 checked 연산이며
overflow를 revision 고정 성공으로 반환하지 않는다. Workspace transaction의 pure evolve는
journal append 전에 실행한다.

## Typed contract 정의와 검증 책임

`contracts`는 public type facade다. ArtifactRef와 flat story package의 정의는 전용 private
module, 외부 effect receipt는 `effects.rs`가 소유한다. Visual pending effect는 기존 EffectReceipt
schema의 typed 값이며 COMMITTED/FAILED/UNKNOWN_OUTCOME을 보존한다. Text pending decision은
object/null에 한정한다. 이는 provider 실행이나 upstream adoption 권한을 추가하지 않는다.

Version grammar는 정확히 세 numeric components를 요구하고 selector/validator의 suffix 차이는
각 owner가 정한다. 언어 subtag 폭도 story package와 reviewed synopsis의 producer 계약대로
명시한다. Missing optional field와 금지된 null을 구분하며 schema enum의 underscore를 보존한다.
Producer schema와 cross-component byte pins는 각각의 producer와 registry 정본을 따른다.

## Admission과 snapshot 수용 경계

Budget의 positive-unit/terminal/checked subtraction policy는 command admission과 journal
reducer가 공유한다. Snapshot gate는 내부 Project의 revision/event-count, map/entity identity,
run ownership과 candidate/changeset/visual lineage를 수용하며 embedded contracts와 voice profile을
각 owner의 validation으로 검사한다. 완전한 project-state wire document를 만들거나 snapshot과
과거 journal의 cryptographic equivalence를 주장하지 않는다. Gate는 snapshot write/load뿐 아니라
snapshot 없이 또는 tail을 적용하여 복원한 최종 Project에도 적용한다.

run.open의 command replay는 lock 아래 저널의 opening payload와 결속한다. 이후 예산이 줄어도
동일 opening 요청의 replay는 가능하며 응답은 현재 stored run을 사용한다. 다른 payload나 무관한
command ID reuse는 conflict다. Pivot budget은 선언값만 기록한다.

## Reviewed handoff와 Markdown transfer

Whole-project reviewed handoff는 리뷰한 `project.export_markdown`의 project revision과 raw UTF-8
SHA-256을 필수 입력으로 받는다. Claim을 export 시 자동 발급하지 않으며 project lock 아래 현재
source와 대조한다. Adopted authoring synopsis는 독립 source/export owner를 유지하되 approval DTO
조립만 공유한다. Immutable reuse는 project/title/path/actor/language/reason/source/hash와 실제
regular-file bytes를 대조한다. Partial body나 symlink는 자동 repair/overwrite하지 않는다.

Markdown import는 fenced code 속 heading을 본문으로 유지하고 중복 suffix를 stable base에서
만든다. Order 계산은 checked이며 Application은 읽은 project revision의 CAS로 import한다.
주 authoring handoff는 `.review-intent.json` → exact synopsis bytes → public receipt 순서로
불변 발행하고 같은 검토 intent만 재개한다. 완료 뒤 matching intent를 제거하며 cleanup 오류는
숨기지 않는다. Whole-project 고정 file pair의 crash atomicity는 별도 한계로 남는다.
실행 qualification은 [Current](IMPLEMENTATION_STATUS.md)에 둔다.

## Application tool 등록과 input admission

Static catalog는 tool metadata, owning handler와 human-only policy를 한 번 구성한다. 같은 등록부가
public tool listing, exact-name dispatch와 MCP human-only filtering을 제공한다. Authoring도 등록된
이름으로만 진입하며 whole-project reviewed handoff approval은 human-only다. Local CLI/Studio는
기존 human entry를 유지한다. Cache는 metadata 전용이며 workspace/provider 결과를 저장하지 않는다.

Application.call은 catalog schema의 최상위 object/properties/required에 맞는 입력만 전달한다.
Nested contracts와 상태/effect policy는 기존 typed owner의 책임이다. with_writing_adapters는 기존
configured registry에 추가하며 providers를 초기화하지 않는다.
