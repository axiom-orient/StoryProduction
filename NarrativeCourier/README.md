# NarrativeCourier 0.12.1

STS에서 검토한 시놉시스를 씬·대본으로 확정하고, 승인된 서사를 Delivery·텍스트 콘티·별도 연출 문서로 투영하는 로컬 도구다. 서사 의미의 정본은 STS이며 renderer나 agent가 새 authority를 만들지 않는다. 현재 STS domain은 [workflow.json](skills/sts/workflow.json)의 `4.3.0`이다.

## 제작 경로

```text
검토 시놉시스 → STS agent task·Human gates → Completed release
  ├─ 텍스트 콘티 task·후보·검토 → Markdown/JSON bundle
  ├─ DeliveryRequest → DirectorPlan → AssetIndex → renderer/review → DeliverySet
  ├─ cinematic continuity·shot draft·review → downstream projection
  └─ 승인 text board → animatic 연출 draft·review → 자산/합성용 packet
```

StoryProduction의 기본 순서는 [텍스트 운영](../docs/TEXT_PRODUCTION.md), 움직이는 콘티는 [animatic 운영](../docs/ANIMATIC_PRODUCTION.md), 기존 shot/visual 경로는 [미디어 운영](../docs/OPERATIONS.md)을 따른다. 이미지·영상 도구는 기본 텍스트 제작의 필수 의존성이 아니다.

## STS CLI

현재 소스에 맞는 `narrative-courier-cli` 바이너리를 준비한다. Cargo package와 binary는 [src-tauri/Cargo.toml](src-tauri/Cargo.toml)에 선언돼 있다. Asset root와 사용자 project root를 명시적으로 선택한다.

```sh
STS_ASSETS_ROOT=/absolute/path/NarrativeCourier \
STS_PROJECTS_ROOT=/absolute/path/projects \
/absolute/path/narrative-courier-cli agent-task my-story
```

| 명령 | 의미 |
|---|---|
| `import-reviewed-synopsis PROJECT --handoff-root DIR --handoff FILE` | geultu producer identity·approval claim·raw synopsis bytes를 검증해 새 STS 프로젝트 시작 |
| `agent-task PROJECT`·`source-binding PROJECT`·`artifact PROJECT NODE` | 현재 obligation/token·인계 identity·저장 artifact 조회 |
| `submit PROJECT ARTIFACT_FILE --task-token TOKEN` | 현재 task의 실제 결과 제출. File은 project-relative regular file |
| `decide PROJECT approve\|revise\|pause --gate-token TOKEN` | 현재 gate를 사람이 결정. Revise에는 실제 step/feedback을 전달 |
| `run PROJECT`·`cancel PROJECT`·`resume PROJECT` | 명시 Codex 실행·취소·paused/Completed 재개 |
| `status PROJECT`·`doctor PROJECT` | 상태/현재 gate 또는 명시 integrity 검사 |
| `export-fountain PROJECT [PATH]`·`export-fdx PROJECT [PATH]` | Completed screenplay export |
| `deliver-context`·`deliver`·`deliver-review`·`deliver-list`·`deliver-cancel` | 별도 Delivery 요청·검토·상태·취소 |

기존 프로젝트는 `agent-task my-story --project-root /absolute/path/my-story`처럼 선택할 수 있다. `PROJECT`는 선택한 state의 논리 ID와 일치해야 한다. executable은 symlink 없는 directory handle을 열어 process cwd를 고정하고, 상대 경로로 읽기·쓰기를 계속하므로 선택 후 directory 이름이 바뀌어도 다른 프로젝트로 이동하지 않는다. import 생성 명령과 `--project-root`의 병용은 dispatch 전에 거절한다. `STS_PROJECTS_ROOT=.`는 선택한 invocation cwd의 namespace를 유지하며 같은 경계를 import 생성에도 적용한다. 이 선택은 단일 executable transport가 소유한다. 동시 library 작업에서 process cwd를 바꾸는 API로 사용하지 않는다.

옵션은 한 번씩 받으며 `--`는 literal positional arguments의 시작이다. `--name=VALUE`로 옵션처럼 보이는 값을 전달할 수 있다. 빈 root·traversal/symlink artifact·stale task/gate는 거절한다. Synopsis import는 screenplay나 final release 승인이 아니다. 일반 screenplay import도 reference를 저장할 뿐 서사 IR이나 사람 승인을 만들지 않는다.

## Release와 저장

Completed 이후 `NarrativePackage v2`, `NarrativeSceneMap v2`, `NvcReleaseBundle v1`을 발행한다. Bundle은 현재 state·source·원본 파일과 hash를 결속한다. Completed resume는 동일 승인 입력을 검사하고 누락 파생 결과만 복구하며 손상 결과를 자동 덮어쓰지 않는다.

```text
projects/<project>/
  input/                         synopsis와 참조 입력
  output/                        STS authority artifacts·scene-map
  state.json                     workflow state
  narrative-package.json         Completed package
  release-bundle.json            cross-component release binding
  storyboards/                   명시 output 경로의 task/board/review
  animatic/                      별도 연출 plan/review/packet
  visual/requests/<request_id>/   DeliveryRequest/Plan/AssetIndex/DeliverySet
```

Delivery의 마지막 `delivery-set.json`이 없는 request와 취소·실패 뒤 남은 blob/render 파일은 성공 결과가 아니다. 사용자 프로젝트 state를 직접 편집하지 않는다.

## 텍스트 콘티와 animatic

공개 `narrative-courier-cli storyboard`는 `prepare/task/submit/review/export/cancel/scenes/bundle`을 제공한다. Current Completed release·씬·semantic occurrence와 base를 묶고 STS와 같은 lock을 쓴다. 후보·diff와 검토한 hash, 전체 씬 승인 bundle을 구분한다. `--candidate` export는 검토 전임을 표시하며 review와 함께 사용할 수 없다. `--project-root DIR`은 invocation directory 기준으로 해석되며 이후 파일 접근은 열린 project FD에 결속한다.

공개 `narrative-courier-cli animatic`는 `source/submit/review/export`로 승인된 board의 순서·panel coverage·duration/keyframes·자막 semantic ID·asset 요구를 묶는다. 현재 총 연출 길이는 10분 이내이며 unresolved direction을 승인하지 않는다. 실제 자막 문자열은 현재 scene-map에서 export한다. 그림 검사와 렌더는 각각 SAC·SMC의 별도 owner다.

두 native lane은 `crates/authoring`이 소유하며 producer schema와 craft를 binary에 포함한다. Python/pip나 개발 checkout을 실행 요건으로 요구하지 않는다. 기존 task/board/review의 schema, contentHash와 immutable bytes를 읽으며 same-byte replay만 허용한다. 실패는 stderr JSON의 `not_applied`/exit 2 또는 `outcome_unknown`/exit 3으로 구분한다. `storyboard --help`와 `animatic --help`가 명령 문법을 제공한다. 별도 `tools/cinematic.py`의 shot-plan 기능과 개발 검증 Python은 독립 선택 경로다.

## Delivery 지원 범위

Canonical durable DTO는 정확히 `NarrativePackage`, `DeliveryRequest`, `DirectorPlan`, `AssetIndex`, `DeliverySet`이다. SceneMap·release bundle·콘티·cinematic·animatic은 목적이 명시된 producer artifact이며 이 다섯 DTO를 범용 plan으로 바꾸지 않는다.

현재 deterministic planner는 `PRESENTATION`, `FULL|SELECTION|EXCERPT`, canonical `SCN-*`의 exact `PRESERVE`를 지원한다. USER/DERIVED asset은 project-relative path·raw hash·SCN scope를 지정한다. 출력 형식은 HTML/JSON/PNG/WEBP/PDF/MP4이며 실제 backend가 필요한 renderer capability를 확인한다.

`SUMMARY`, `MOOD`, audience/locale 재서술, free-form semantic constraints와 generative/procedural assets는 미지원이다. Silent fallback 없이 `UNSUPPORTED_CAPABILITY`로 거절한다. 기술 PASS는 사람의 의미·시각 승인이 아니며 필요한 결과는 `NEEDS_REVIEW` 뒤 별도 approve/reject를 거친다. Output 재사용은 plan·관련 자산·backend/version binding과 실제 bytes를 다시 검사한다.

## 어댑터·플랫폼·문서

`crates/mcp`는 동일 STS/Delivery engine의 stdio agent transport다. Human STS decide와 Delivery review는 도구로 노출하지 않는다. Tauri/기존 UI는 별도 state owner가 아니다. Unix-like(macOS/Linux)의 process group·lock·atomic publication을 사용하며 지원되지 않는 플랫폼에서 약한 fallback을 하지 않는다.

[제품 계약](docs/SPEC.md) · [ARCHITECTURE](docs/ARCHITECTURE.md) · [Current](docs/IMPLEMENTATION_STATUS.md) · [PLAN](docs/PLAN.md) · [모듈 분석](ANALYSIS.md) · [Delivery Blueprint](docs/blueprint/README.md).

공식 로컬 gate는 `verify-all.sh`이고 `tools/source_manifest.py`는 source metadata의 무결성을 생성·검증한다. Manifest 갱신은 제품 실행 qualification을 대신하지 않는다. 날짜별 gate 범위와 남은 미실행 항목은 [Current](docs/IMPLEMENTATION_STATUS.md)에 기록한다. MIT와 별도 attribution은 `LICENSE`·`NOTICE.md`를 따른다.
