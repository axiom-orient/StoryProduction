# 글투 — 제품 계약

## 필수·선택 기능

핵심은 원문·스토리·시놉시스 authoring, project/scene/fact/timeline/thread 편집, journal/revision, deterministic review/proof, 후보·사용자 결정과 명시적 복구다. voice·refinement·craftpack·writing-type/provider generation·visual admission·StoryPackage·reviewed synopsis export도 공개 capability다. Provider 미설정은 저장된 문서·task·기존 편집이나 순수 prompt 준비 기능을 제거하지 않는다.

## Authoring 계약

문서 종류는 `source`·`story`·`synopsis`다. Source는 실제 입력 텍스트이며 Story/Synopsis는
stable block ID·heading·text, dependency hashes, guidance와 document revision을 가진다.
새 개발 노트·companion·매체 profile을 이미 지원하는 document kind로 취급하지 않는다.

| 공개 작업 | 상태·결과 의미 |
|---|---|
| `start`·`brief` | 실제 원문 저장, brief revision CAS와 downstream stale 표시 |
| `prepare`·`task` | source/guidance/base·선택 block에 결속된 durable task와 조회; 외부 호출 없음 |
| `submit`·`candidate` | untrusted draft·based_on·차이·assumptions/open_questions와 exact candidate hash |
| `review` | 사람이 읽은 hash와 현재 task/source/guidance에 결속된 채택/거절 |
| `edit`·`restore` | document revision CAS로 사람이 전체 본문 편집 또는 과거 문서를 새 revision으로 복원 |
| `cancel` | 이후 task 채택 차단; late draft 보존. 외부 provider 취소의 보장은 아님 |
| `status`·`document`·`history` | 실제 저장본·미결 후보·missing/current/needs_revision·최근/지정 revision 조회 |
| `export`·`bundle` | 현재 채택 Markdown/raw hash 또는 한 saved project revision의 원고 묶음; NVC release는 아님 |
| `handoff` | 사람이 검토한 synopsis revision/raw hash의 immutable producer 인계 |

부분 후보는 선택한 block을 정확히 포함하고 선택 밖의 독립된 편집을 보존한다.
상위 원문·brief·voice가 바뀐 문서를 부분 수정만으로 전체 current로 승격하지 않는다.
복원은 과거 source/guidance binding을 유지하며 과거 승인을 되살리지 않는다.
Human-only 작업은 MCP 목록과 실행에서 거절한다. CLI/Studio actor는 로컬 attribution이다.
정확한 이름·필드는 [authoring tool 정본](../crates/geultu/src/authoring/api/tools.rs)을 따른다.

## 입력·출력·전제조건

| surface | 입력·검증 | 출력·관찰 동작 |
|---|---|---|
| `Application::call(tool, Value)` / CLI `call TOOL JSON` | tool별 DTO·id·revision·command_id | structured JSON 또는 typed error |
| scene/domain write | project/object expected revision·text invariant | accepted revision/events·snapshot projection |
| candidate/changeset review | 필수 actor ID·candidate subject revision/digest; approval에는 current project revision 추가 | 모든 reject/approve가 exact subject-bound HumanDecision을 journal에 기록하거나 stale/conflict로 거절 |
| generation.prepare/generate | project/writing_type/brief·optional stage/parameters/expected_content_revision; generate에는 provider_id | state를 쓰지 않는 plan 또는 untrusted draft/candidate metadata |
| reviewed synopsis handoff | actor/language·검토된 원문·source revision/hash | exact UTF-8 synopsis + producer v1 handoff |
| visual.admit_package | StoryPackage와 existing PASS receipt·capability/budget | NVC route admission; narrative release는 만들지 않음 |
| recover | project/index와 명시 remove_lock 선택 | journal 재생 또는 명시 integrity/lock 오류 |

`authoring.prepare`는 Story/Synopsis에 결속된 durable task packet을 저장하는 mutation이다. 외부 writer를 호출하거나 canonical document를 수정하지 않는다. `expected_project_revision`을 전달하면 Workspace가 project lock 안에서 exact revision을 비교한 뒤 journal에 commit한다. 동일한 `command_id`의 replay는 기존 commit을 반환한다.

## 상태·오류·수용 기준

`validate → lock/load → decide → pure evolve/schema preflight → journal publication → snapshot/index → result`로 진행한다. Command event 묶음은 기존 NDJSON prefix와 stage한 뒤 atomic replace한다. Invalid input·revision conflict·active lock은 반영 전 거절한다. Journal 실패를 accepted command로 보고해서는 안 된다. Journal 반영 후 snapshot/index 실패는 `committed_snapshot_failed`와 반영 revision을 반환한다. 같은 accepted command_id는 중복 event를 만들지 않고 projection 복구 후 응답한다. 손상된 과거 자료를 자동 해석·변환하지 않는다.

Workspace metadata·Project snapshot·journal envelope의 현재 schema version은 2다.
지원하지 않는 v1/version과 unknown stored fields는 거절하며 자동 migration하지 않는다.

candidate metadata의 audit revision과 canonical content revision은 다르다. `candidate.subject`는 pending candidate의 engine-computed ArtifactRef를 반환한다. `candidate.reject`, `changeset.reject`, `changeset.approve`에는 actor ID와 exact subject revision/digest가 필수다. approval은 source/project/operation revision도 다시 확인한다. claim-less review는 받지 않는다. generation만으로 canonical 원문을 쓰거나 자동 승인하지 않는다. Actor ID는 기록용 attribution이지 인증 증명이 아니다. registry의 동일 identity가 모호하면 특정 provider를 임의 선택하지 않는다.

## 호환성과 변경 경계

Project 원문·journal 이력·revision, 후보와 사람 결정, 기존 public tool names·`GenerationProvider`·handoff schema는 의도된 제품 계약이다. 저장 형식 v2는 지원하지 않는 버전과 unknown fields를 거절하며 자동 변환하지 않는다. 새 저장 형식이나 public contract에는 별도의 호환성 결정이 필요하다. 내부 모듈·UI·adapter·provider·projection 구현은 Geultu의 state owner와 승인·commit·오류·복구 의미를 보존하는 범위에서 바꿀 수 있다. 유효한 선택 capability와 실제 사용자 기록은 정리 대상으로 간주하지 않는다.

## transport·한도

일반 id는 1–64 ASCII alphanumeric/dot/underscore/hyphen을 따르고 project path의 literal `.`·`..`는 거절한다. Core text는 16 MiB, Studio/MCP JSON transport는 128 MiB다. Voice sample은 1–32개, 각 2 MiB/합계 20 MiB이며 raw prose는 저장하지 않는다. 한도는 각 core/transport owner가 소유한다.

MCP는 현재 stdio initialize/ping/tools/list/tools/call 계약을 따른다. Studio는 loopback-only, fresh256bit bearer와 origin 검증을 사용하며 CSPRNG 실패 시 시작을 거절한다. Content-Length/HTTP framing 모호성을 허용하지 않는다.

## visual admission 예

`story.export`의 실제 `package.source` ArtifactRef와 이미 발행된 PASS receipt를 사용한다. 아래 설명용 `{}`는 실행 가능한 artifact가 아니다.

```json
{"project_id":"novel","run_id":"run-1","stage":"S00","package":{},"route_admission":{"route_id":"NVC","command_id":"adm-1","input_refs":{"text_master":{},"export_spec":{}},"required_receipts":[{"validator_id":"geultu-review","subject_name":"text","required_outcome":"PASS"}],"capabilities":["render"],"budget":{"time_ms":1000,"output_bytes":1000}}}
```

S-stage admission의 route는 `NVC`다. 다른 route를 편의상 변환하지 않는다.

## intentional contract 원본

[producer handoff schema](../contracts/reviewed-synopsis-handoff-v1.schema.json), [embedded canonical bundle](canonical/), `crates/geultu/src/model.rs`, `application.rs`, `generation.rs`의 공개 타입/도구, [GENERATION](GENERATION.md)을 따른다. schema/fixture 번들은 일반 설명 문서처럼 삭제하지 않는다. 저장형식의 현재 불완전성과 개선 제안은 [Current](IMPLEMENTATION_STATUS.md)/[PLAN](PLAN.md)에만 둔다.
