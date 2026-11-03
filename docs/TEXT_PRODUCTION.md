# StoryProduction — 원문에서 제작 문서까지

이 문서는 현재 텍스트 집필 경로의 사용 계약이다. 명령은 저장소 루트에서 실행한다.
`/absolute/path/my-story`는 사용자 작업 공간 예시이며 실제 원문 경로로 바꾼다.
`complete`는 현재 원본·승인·파일 결속이 갖춰졌다는 결과이며 작품 품질 승인이 아니다. 현재 소스의 미검증 범위와 과거 실행 결과는 [Current](IMPLEMENTATION_STATUS.md)에 둔다.

## 준비

- POSIX(macOS/Linux), 빌드된 `integration/target/release/storyctl`. Python·pip는 실행 전제가 아니다.
- 외부 writer `run`/`recover`와 계약 검증은 Rust에 내장된 JSON Schema validator를 사용한다.
- 새 소스가 포함된 `geultu`와 `narrative-courier-cli` 바이너리.
- 자동 작성을 사용할 때만 사용자가 선택한 외부 작성 프로그램.

바이너리는 각 제품 Cargo workspace의 현재 소스에 맞춰 준비한다. 기본 경로는
`geultu/target/release/geultu`와 `NarrativeCourier/target/release/narrative-courier-cli`다.
경로가 다르면 다음처럼 명시한다. 모든 `text` 공통 옵션은 하위 명령 앞에 둔다.
`--`로 시작하는 값을 전달할 때는 `--writer-arg=--model`처럼 `=`로 결속한다.
값이 빠진 옵션이 다음 옵션 이름을 본문으로 소비하지 않는다. `--help`/`-h`는 owner를 호출하지 않는다.

```sh
integration/target/release/storyctl text \
  --workspace /absolute/path/my-story \
  --geultu-bin /absolute/path/geultu \
  --nvc-bin /absolute/path/narrative-courier-cli status
```

본문의 짧은 예제는 기본 바이너리 경로를 사용한다. 프로젝트 소유자가 저장물을
관리하므로 `.geultu`나 STS의 `state.json`을 직접 편집하지 않는다.

## 1. 원문 저장

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story create \
  --project my-story --title "내 이야기" \
  --source /absolute/path/original.md --actor author --language ko
```

입력의 UTF-8 본문을 그대로 저장한다. 원문을 잘라 예시로 바꾸거나 자동 개작하지
않는다. 동일한 create 인수는 중단된 초기화를 이어갈 수 있다. 다른 원문이나
제목으로 같은 workspace를 초기화하면 충돌이며, 기존 작품을 덮어쓰지 않는다.
새 intake 참조는 `intakeSourceSha256`으로 첫 원문에 결속한다. Source JSON 요청의 16 MiB
한도는 owner 초기화 전에 확인한다. 생성 재개는 같은 요청의 기록을 확인하고 snapshot·인덱스를
복구하며, 재개 도중 이미 수정된 원문이 관측되면 `source_changed`를 반환한다.
해시가 없는 과거 intake 참조를 자동 변환하거나 다른 입력으로 덮어쓰지 않는다.

작업 공간은 다음 책임으로 나뉜다.

```text
my-story/
  story-project.json             프로젝트 참조와 최초 입력 identity
  writing/.geultu/                geultu 원문·문서·태스크·후보 journal
  narrative/my-story/             NarrativeCourier STS 씬·대본·콘티
  attempts/<attempt>/             외부 작성 요청·응답·저장 결과
  .story-attempt-<attempt>.claim    중복 실행 방지용 영속 예약
  .story-attempt-<attempt>.submit.lock  run/recover 제출 잠금
```

한 CLI 호출은 workspace와 writing/narrative/project/attempt directory를 열린 FD로
결속한다. 실행 중 pathname이나 descriptor의 project ID가 바뀌면 다른 작품을 재선택하지
않는다. Dispatch 전에 확인된 충돌은 거절하고, 이미 시작한 효과를 관찰하지 못하면
`outcome_unknown`으로 남긴다. 숨김 claim/lock을 지워 재시도하는 방식으로 복구하지 않는다.

## 2. 제작 브리프와 글투

`authoring.status`에서 brief_revision을 읽는다. 예를 들어 아래 JSON을 `brief.json`에
저장하고 `author authoring.brief --input brief.json`으로 넘긴다. 값은 작품에 맞게
사용자가 정한다. project_id는 integration이 현재 workspace에 결속한다.

```json
{
  "command_id": "brief-001",
  "expected_revision": 0,
  "brief": {
    "audience": "성인 독자",
    "genre": "생활 미스터리",
    "tone": "건조하지만 인물에게 다정하게",
    "point_of_view": "원문의 시점 유지",
    "voice_notes": "감정 이름을 직접 설명하기보다 행동과 짧은 문장으로 표현",
    "preserve": ["원문의 결말", "주인공과 누나의 관계"],
    "avoid": ["갑작스러운 악역 독백", "근거 없는 반전"]
  }
}
```

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story \
  author authoring.brief --input /absolute/path/brief.json
```

원문은 매 task에 집필 자료로 포함된다. 별도의 글투 관찰을 원하면 기존
`voice.profile.create`를 같은 `author` 경로로 호출한다. 인수는
`expected_project_revision`, `samples: [{"id":"sample-1","text":"작가의 실제 글"}]`,
`command_id`다. API의 한도와 보존 범위는 [geultu 문서](../geultu/docs/SPEC.md)를 따른다.
글투 profile은 제한된 관찰이며 원작자의 문학적 정체성 자체가 아니다.

## 3. 에이전트가 실제 후보 쓰기

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story next
```

`owner=geultu`, `status=agent_task`이면 `task_id`와 `packet`을 에이전트에 준다.
packet에는 원문, 상위 문서, 집필 brief, 글투 지침, 변경 범위와 출력 스키마가 있다.
모델을 호출하지 않고 사용할 수 있는 작업 패킷을 저장하는 명령이다.

에이전트는 다음 구조의 **실제 본문**을 작성한다. 아래는 형식 설명이며 실제
사용자 원고나 완료 예제가 아니다. `text`에 안내 문구 대신 완성한 글을 넣어야 한다.

```json
{
  "blocks": [{"id": "opening", "heading": "첫 장", "text": "완성한 본문"}],
  "change_summary": "무엇을 보존하고 어디를 확장하거나 다듬었는지",
  "assumptions": [],
  "open_questions": []
}
```

후보 저장은 `authoring.submit`이다. 입력 JSON은 다음 필드로 구성한다.

```json
{
  "command_id": "submit-story-001",
  "candidate_id": "story-001",
  "task_id": "next가 반환한 실제 task_id",
  "draft": {
    "blocks": [{"id": "opening", "heading": "첫 장", "text": "완성한 본문"}],
    "change_summary": "구체적인 변경 요약",
    "assumptions": [],
    "open_questions": []
  }
}
```

`author authoring.submit --input submit.json`의 결과는 candidate_hash, 이전/이후
블록과 채택을 막는 이유를 포함한다. open_questions는 채택 전에 해결해야 할
중요한 결정만 담는다. 과거 원문에서 작성된 후보도 비교 자료로 저장되지만
현재 문서에 바로 채택되지는 않는다.

## 4. 검토·직접 수정·부분 재작성

`authoring.candidate`에는 `{"id":"story-001"}`를 넘긴다. 실제 내용을 검토한 사람이
다음 입력을 `authoring.review`에 전달한다. 검토 전 해시를 자동으로 읽어 승인하는
루프를 만들지 않는다.

```json
{
  "command_id": "review-story-001",
  "candidate_id": "story-001",
  "expected_candidate_hash": "실제로 검토한 후보의 candidate_hash",
  "accept": true,
  "actor": "author",
  "note": "검토 메모"
}
```

거절은 `accept:false`다. 후보는 삭제되지 않는다. 후보를 직접 고쳤다면 새
candidate_id와 command_id, 원래 task_id, `based_on:"story-001"`로 다시 제출한다.
채택은 직접 고친 새 후보의 hash에 결속된다.

부분 재작성은 `authoring.prepare`에 `target:"story"`, `selection:["opening"]`,
`instruction`, 새 task_id/command_id를 준다. 결과는 선택한 block id를 빠짐없이
포함해야 한다. 다른 블록의 최신 사용자 편집은 보존한다. 상위 자료가 바뀐 문서는
부분 수정만으로 전체가 최신이 됐다고 표시하지 않는다.

현재 문서의 수동 편집은 `authoring.edit`의 `kind`, `blocks`, `expected_revision`,
`actor`, `command_id`다. 전체 문서의 새 내용과 순서를 명시하는 사람용 명령이다.
MCP에서는 review/edit/restore/handoff 실행을 거절한다. CLI 호출은 신뢰된 로컬
운영 경계이며 actor 문자열 자체가 신원 인증은 아니다.

스토리 채택 후 `next`는 시놉시스를 준비한다. 시놉시스도 같은 후보·수정·채택
흐름을 따른다. `status`는 원문·스토리·시놉시스의 missing/current/needs_revision을
구별하고 변경 이유를 알려준다.

## 5. 이전 문서와 복원

`authoring.history`에 `{"kind":"story"}`를 넘기면 최근 20개 revision, 전체 버전
수, 생략된 개수를 돌려준다. `revision`을 추가하면 그 저장본의 실제 document를
읽는다. 오래된 버전도 revision으로 직접 주소를 지정할 수 있다.

`authoring.restore`는 `kind`, 복원할 `revision`, 현재의 `expected_revision`,
`actor`, `command_id`를 받는다. 이전 내용을 새 revision으로 저장하고 최신
사용자 편집과 충돌하면 거절한다. 복원본의 과거 source/guidance binding을 유지하므로
원문이 바뀐 뒤 옛 본문을 복원하면 여전히 갱신 필요 상태일 수 있다.

## 6. 시놉시스를 STS 씬으로 인계

`authoring.export`에 `{"kind":"synopsis"}`를 주고 실제 markdown, revision,
markdown_sha256을 확인한다. 검토한 값으로 `authoring.handoff`를 호출한다.

```json
{
  "actor": "author",
  "language": "ko",
  "expected_revision": 1,
  "expected_markdown_sha256": "검토한 synopsis의 raw SHA-256"
}
```

이 명령은 원문/스토리 전체가 아닌 **채택한 시놉시스만** 기존 producer 계약으로
내보낸다. 결과의 handoff_root를 사용한다. 작업 공간의 다른 경로를 추정하지 않는다.
인계 도중 중단됐다면 같은 actor·language·검토 revision·hash로 다시 요청한다.
현재 원고와 기존 검토 입력이 같을 때만 저장된 승인과 동일 본문을 재사용하며,
다른 바이트나 승인은 덮어쓰지 않는다. 완료된 인계에는 임시 검토 기록을 남기지 않는다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story import-handoff \
  --handoff-root /actual/handoff/root --duration 30

integration/target/release/storyctl text --workspace /absolute/path/my-story next
```

duration은 STS의 명시적인 목표 분량이며 실제 영상 생성 시간이 아니다.
`import-handoff`는 새 씬 프로젝트를 만들기 전에 인계의 작품·project revision·본문을
현재 채택 시놉시스와 대조한다. 공개 NVC 진입점에는 캡처한 같은 바이트만 전달하고,
NVC가 저장한 인계 hash를 확인한 뒤 호출용 사본을 제거한다. 호출 중 원고가 바뀌면
`source_changed`와 비정상 종료 코드를 반환하며, 이미 만들어진 씬 프로젝트는 보존한다.
STS는 기존 source graph→서사/인물→treatment→씬 설계→대본 흐름과 Human gates를
유지한다. `next`는 공개 entrypoint에서 task token과 자료가 들어 있는 task_text를
읽는다. 에이전트 작성물을 STS 프로젝트 내부의 파일에 저장한 뒤 제출한다.

첫 STEP 0은 typed Source IR과 `Source Coverage Audit`을 함께 만든다. 각 SRC/REL은 승인된
시놉시스의 고유한 정확 인용 span에 연결하고, 감사 표는 모든 비공백 원문을 덮어야 한다.
제출기는 누락/모호/잘못된 인용이나 연결되지 않은 source ID를 차단한다. 표는 의미 분해의
정답을 자동 판정하지 않으므로 Source IR 승인자는 특히 인물의 믿음·주장, 부정, 명시적
unknown이 각각 obligations로 적혔는지 원문과 대조한다. 명시적 unknown은 adaptation GAP가
아니며, 원인을 추정해 채우지 않는다.
SRC 행에는 `polarity`와 `epistemic` 분류, materiality/protection 및 span ID가 필요하고, REL 행에는
닫힌 관계 종류와 materiality/span ID가 필요하다. 이 typed inventory가 Human 승인 뒤 STEP 8D의
비교 기준이 되며 coverage quote는 별도 source authority가 아니다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story scene submit \
  input/my-draft.md --task-token ACTUAL_TASK_TOKEN

integration/target/release/storyctl text --workspace /absolute/path/my-story scene decide \
  approve --gate-token ACTUAL_REVIEWED_GATE_TOKEN
```

수정 요청은 `revise --step N --feedback "구체적인 수정" --gate-token ...`,
일시 중지는 `pause --gate-token ...`, 재개는 `scene resume`다. STS의 기존 source
계약과 허용 결정만 적용된다. synopsis handoff가 STS 최종 확정을 대신하지 않는다.

새 시놉시스와 이전 STS source가 달라지면 `source_changed`가 표시된다. 기존 STS
입력을 직접 덮어쓰지 않는다. `text --workspace ... --narrative-project my-story-v2
import-handoff ...`처럼 새 대상 이름을 명시하고 이후 같은 공통 옵션으로 작업한다.

## 7. 씬을 텍스트 스토리보드로 만들기

STS 최종 확정 뒤 다음 명령으로 콘티 작업을 준비한다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story storyboard prepare \
  --task-id board-001 --brief "원문의 절제된 감정을 구도와 시선으로 보여준다" \
  --output storyboards/tasks/board-001.json
```

반환된 draftSchema, scenes, semanticOccurrences를 에이전트가 읽는다. 각 씬에는
intent와 panels, 패널에는 panelId, composition, staging, sourceRefs, soundNotes,
transition을 작성한다. sourceRefs는 주어진 semanticId/entryHash를 그대로 쓴다.
씬의 모든 occurrence를 연결해야 한다. 원문 행동·대사는 내보내기에서 실제 자료로
해결하며, 새 사건은 콘티에 몰래 넣지 않는다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story storyboard submit \
  --task storyboards/tasks/board-001.json --draft input/board-draft.json \
  --output storyboards/boards/board-001.json
```

결과에는 실제 Markdown 콘티와 이전 후보 대비 diff가 있다. 사람이 이를 읽고
확인한 contentHash로 별도 검토 기록을 남긴다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story storyboard review \
  --board storyboards/boards/board-001.json --decision approved --reviewer author \
  --expected-hash sha256:ACTUAL_REVIEWED_HASH --output storyboards/reviews/board-001.json
```

거절은 `--decision rejected`다. 부분 수정은 새 `prepare`에
`--base storyboards/boards/board-001.json --scene SCN-003`을 넣는다. 선택하지 않은
씬은 그대로 보존된다. 제출한 draft는 선택한 씬을 원래 순서대로 정확히 포함한다.
전체 프로젝트 패키지는 모든 씬의 콘티가 있을 때만 완성 상태로 내보낸다.

`storyboard cancel --task ...`은 해당 작업의 이후 채택을 막는다. 외부 생성 요청의
취소를 보증하지 않는다. 다른 작업으로 수정할 때는 새 task-id와 output을 사용한다.
이미 채택한 콘티의 승인 기록은 유지한다. 명령과 맞지 않는 옵션, 중복 옵션과 축약 옵션은
거절한다. 미리보기·diff·응답을 준비한 뒤 후보를 저장하며, 저장 후 I/O 실패나 결과 전달 실패는
`outcome_unknown`이다. 후보를 다시 생성하기 전에 기존 output과 현재 검토 기록을 확인한다.

## 작성 프로그램과 자동화

사용 중인 에이전트가 packet을 읽고 직접 submit해도 된다. 외부 작성 프로그램을
연결할 때는 아래처럼 한 번의 명시적 attempt를 실행한다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story run \
  --attempt write-001 --writer-program /absolute/path/my-writer --timeout 600
```

프로그램은 stdin의 [text-writer-request](../integration/contracts/story-text-writer-request-v1.schema.json)를
받고 stdout에 [text-writer-result](../integration/contracts/story-text-writer-result-v1.schema.json)
JSON 하나를 반환한다. geultu/콘티 작업의 content는 draft JSON을 문자열로 담고,
STS 작업의 content는 실제 Markdown 본문이다. 진단은 stderr로 쓴다.

```json
{
  "schemaVersion": "story.text-writer-result/1",
  "attemptId": "write-001",
  "status": "succeeded",
  "content": "실제 작업 결과",
  "error": null
}
```

실패·취소·미확정 결과는 해당 status, `content:null`, 구체적인 error를 반환한다.
외부 모델/API 선택과 자격 증명은 작성 프로그램이 소유한다. 비밀은 기록되는
writer 인수에 넣지 않고 그 프로그램의 인증 경로를 사용한다.

`run`은 한 번 호출하고 결과를 후보로 저장하며 자동 채택하지 않는다. 다음 실행은
다음 task 또는 같은 task의 명시적인 새 수정 작업이다. pending candidate가 있으면
모델을 다시 호출하지 않고 검토 대상으로 반환한다. 콘티 작업은
`--storyboard-task storyboards/tasks/board-001.json`을 추가한다.

timeout은 1..7200초다. JSON 입력은 16 MiB, stdout/stderr의 합도 별도로 16 MiB로 제한한다. `storyctl text`는
provider host의 정상 취소 신호(SIGTERM)를 cleanup 경로로 보내고, 실행 중단 결과는
`outcome_unknown`으로 남긴다. Owner task 준비 전에
attempt 디렉터리를 독점 예약하며 request/response/result를 남긴다. 응답이 없는
기존 attempt는 outcome_unknown으로 유지한다. 동일 attempt에 다른 프로그램을
지정하면 충돌이다. source/task가 바뀌었다고 자동으로 새 공급자에 재시도하지 않는다.

응답은 저장됐지만 submit 단계에서 끊겼다면 다음 명령은 생성 호출 없이 기존
응답만 복구한다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story recover --attempt write-001
```

`run`과 `recover`의 제출은 같은 attempt 잠금으로 직렬화한다. 이미 저장된 후보나
실패·취소 결과는 그대로 반환한다. STS token이 이미 바뀐 경우에는 현재 시놉시스와
artifact의 node/path/version과 저장된 응답의 정확한 bytes를 함께 관찰한다. 공백이나 개행을
보정하지 않으며 다르면 미확정 상태를 유지한다. 실제 외부 요청 상태를 관찰할 정보가 없다면 새
attempt로 우회하지 말고 기록된 요청을 먼저 확인한다.
Owner가 matching commit/candidate 또는 artifact acknowledgment를 반환해야 `candidate_stored`다.
Writer가 실행된 뒤 owner 호출이 시작되지 못해도 전체 attempt는 `outcome_unknown`으로 남는다.
새 writer request의 opaque task에는 `storyProjectId`와, NVC 경로라면
`narrativeProjectId`도 담는다. 이를 writer가 수정하거나 승인 근거로 해석하지 않는다.
Submit/recover는 project identity와 저장 task의 immutable source/brief/instruction을
current public task에 대조한다. 라우팅 증거가 없는 과거 attempt는 기존 bytes를 유지한 채
unknown과 점검 안내를 반환한다. 필요하면 저장 응답을 검토하고 정확한 owner project에
명시적으로 후보를 제출한다. Provider를 다시 실행하지 않는다. 취소 이후 새 성공 결과를
발행하지 않으며, 취소된 task의 후보 보존·채택 판단은 owner가 수행한다.

## 8. 실제 문서 파일 내보내기

`export --output DIR`은 현재 원문·스토리·시놉시스만 내보낸다. 전체 제작물 완료를
의미하지 않는다. 전체 결과는 다음과 같이 명시적으로 요청한다.

```sh
integration/target/release/storyctl text --workspace /absolute/path/my-story package \
  --board storyboards/boards/board-001.json --review storyboards/reviews/board-001.json \
  --output /absolute/path/my-story-edition-1
```

`source.md`, `story.md`, `synopsis.md`, `scene-blueprint.md`, `screenplay.md`,
`scene-map.json`, `storyboard.md`, `storyboard.json`, `storyboard-review.json`을 만든다.
마지막 `story-package.json`이 파일·hash·owner source를 결속하는 완료 marker다.
이미 있는 경로에는 같은 bytes만 재사용한다. 다른 개정은 새 출력 폴더에 저장한다.
원고 3개와 NVC 산출물 6개의 정확한 파일 집합·hash를 모두 확인한 뒤 파일을 쓴다.
완료 marker를 쓰기 전에 공개 owner 진입점에서 원고와 승인된 콘티 묶음을 다시 읽는다.
출력 디렉터리는 열린 FD에 고정하고, 최종 owner 조회 뒤 실제 저장 파일의 bytes도 다시 확인한다.
그동안 내용·revision·승인·파일·출력 디렉터리 identity가 바뀌면 이번 판본의 완료 marker를 발행하지 않는다.

콘티만 내보낼 때는 `storyboard export --board ... --review ... --output storyboards/export/board.md`를
쓴다. 검토 전 미리보기는 `--candidate`를 명시하며 문서에 후보 상태를 표시한다.
Fountain/FDX는 NVC의 기존 공개 `export-fountain`/`export-fdx` 명령을 사용할 수 있다.

## 상태를 읽고 이어가기

`text status`는 현재 원고와 NVC source가 일치하고 NVC 공개 bundle이 전체 콘티 승인을 확인한
경우에 `complete`를 반환한다. 이는 package가 이미 디스크에 발행됐다는 뜻은 아니다.
실제 판본은 `package` 응답과 마지막 `story-package.json`으로 확인한다.

`source_changed`이면 현재 시놉시스를 검토·인계하고 새 `--narrative-project`로 이어간다.
`outcome_unknown`이면 attempt 요청·응답과 owner 상태를 먼저 읽는다. 자동 재생성이나
승인으로 해결하지 않는다. 선택 영상 제작은 [animatic 운영](ANIMATIC_PRODUCTION.md)을 따른다.
