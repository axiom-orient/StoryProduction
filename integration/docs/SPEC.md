# Story integration — 제품 계약

## 필수·선택 기능

producer/consumer schema pin, Story capability request/job/result, pure state transition, current source/artifact adoption preflight가 공통 계약이다. promptgen process adapter와 SMC→SAC request composer는 선택 경계다. 범용 image-core·전역 approval/job database는 비목표다.

## 텍스트 제작 계약

native `storyctl text`는 geultu와 NVC public entrypoint를 조합한다. Python runtime API는 Story 실행 entrypoint로 사용하지 않으며 저장 DTO는 보존한다. Descriptor는 최초
`intakeSourceSha256`과 project 참조를 저장하며 작품 승인 정본이 아니다. Source 요청의
UTF-8/16 MiB 한도를 초기화 전에 수용하고 같은 intake intent만 재개한다.

`next`는 owner 상태에 맞는 기존 task 또는 새 durable task를 반환하며 provider를 호출하지
않는다. `run`은 task 준비 전에 한 attempt를 독점 예약하고 writer request/result schema·현재
source/token을 적용한다. `recover`는 저장 응답과 실제 owner 결과를 관찰하며 provider를 재실행하지 않는다.
새 writer task envelope에는 `storyProjectId`, NVC task에는 `narrativeProjectId`도 저장한다.
이는 운반용 라우팅 증거이며 승인 정본이 아니다. 과거 request/response/result의 bytes를
변환하지 않는다. Legacy `schemaVersion`은 해당 버전의 read contract로 처리하며 다시 쓰지 않는다. 저장된 public commit 등으로 project를 입증하지 못하는 legacy attempt는
`outcome_unknown`과 점검 안내를 반환하며 자동 submit하지 않는다. Task ID나 같은 본문만으로
다른 project에 제출할 권한을 추정하지 않는다. 제출 전 current task의 immutable source와
저장 task를 대조하고, 취소된 후보를 보존할지 여부는 owner가 판단한다.

일반 pathname과 SEMI 상속 FD 모두 호출 context가 열린 workspace directory를 보유한다.
writing root, narrative namespace와 선택한 NVC project, attempt directory도 별도로 결속한다.
Story의 draft 저장과 owner process cwd는 같은 project FD를 사용한다. NVC의 공개
`--project-root`는 논리 project ID를 유지한 채 선택한 디렉터리를 전달하는 transport 옵션이다.
Import는 namespace에서 수행하고 project-root override를 받지 않는다. 경로 교체로 다른
디렉터리를 재선택하지 않으며 dispatch 뒤 identity 변화는 unknown으로 보존한다.

인계는 현재 작품·revision·채택 synopsis bytes와 producer approval을 대조한 뒤 NVC로 보낸다.
호출용 captured copy는 정리하고 NVC 저장 binding과 원고를 다시 관찰한다. `source_changed`나
`outcome_unknown`이 이미 생성된 owner 프로젝트를 자동 삭제/덮어쓰는 뜻은 아니다.

`export`는 coherent writing snapshot, `package`는 원고 3개와 NVC의 정확한 6개 승인 artifact를
대상으로 한다. 모두 검사한 뒤 열린 output directory FD로 파일을 쓰고 final owner 재관찰,
directory identity 및 저장된 exact bytes 재확인 후 마지막 manifest를 발행한다.
같은 bytes만 불변 재사용한다. Status의 완료 관찰과 실제 package 발행, 작품 품질 승인은 구분한다.

## 선택 animatic 계약

NVC plan/review/packet, SAC assets/review, SMC render result는 각 producer 소유다.
Integration은 public CLI로 전달하고 렌더 전후 live NVC packet을 관찰한다.
`story.animatic-observation/1`은 원래 SMC receipt를 `render`에 보존하고 sourceStatus를
current/changed/unobserved로 구분한다. Producer contentHash를 재작성하지 않는다.
후보·취소·실패·source stale·unknown은 자동 승인/재시도로 바꾸지 않는다.
설치된 native animatic 경로는 SMC가 byte-pin한 두 브라우저 JavaScript 자원을 data로 포함할 수 있지만 Node interpreter를 실행하지 않는다. SMC가 확인하는 Chromium, font, FFmpeg/ffprobe는 별도 시스템 요구다.
Render 옵션과 필요한 실행 파일은 packet 발행 전에 검사한다. asset root를 열린 FD에
결속하고 SAC/SMC는 그 cwd에서 상대 경로를 사용한다. Candidate receipt는 pinned schema,
requested attempt, 동일 attempt의 public status receipt와 대조한다. `not_started` receipt도
attempt ID 일치 검사를 생략하지 않는다. receipt identity가 없거나 status와 어긋나면 기존
producer receipt를 보존한 `outcome_unknown`이다. 실제 dispatch 전 missing executable만
`not_started`로 남긴다. Current source가 명시적으로 달라진 경우 changed, 읽지 못한 경우
unobserved다. Root 교체나 최종 관찰 실패는 이미 얻은 producer receipt를 보존하면서 unknown을 반환한다.

## SEMI local provider

`macos-provider/`는 `SEMIProviderSDK`의 인증 transport 뒤에서 process 시작 때 지정된 단일 workspace에 접근한다. 요청 schema에 workspace 경로나 executable 경로를 받지 않는다. 읽기는 `authoring.status`, `authoring.document`, `authoring.candidate`, `authoring.task`를 호출한다. 변경은 brief revision CAS, `authoring.prepare` task creation, untrusted candidate submit, preview-bound exact candidate review를 연다. Provider는 prepare 중 owner가 반환한 후보 revision을 캡처하므로 모델 입력이나 approval preview에 raw revision hash를 요구하거나 노출하지 않는다. 각 mutation은 no-effect provider prepare와 별도 SEMI approval을 통과한다. Preview에 담긴 `project_id`는 owner 호출에도 `expected_project_id`로 전달되고, StoryIntegration은 workspace descriptor의 현재 ID와 비교한 뒤에만 geultu를 호출한다. 중간 task/candidate read도 앞서 관찰한 project ID에 결속한다. Write mismatch는 geultu mutation 전에 `not_applied`로 반환하고, 준비 중 read mismatch는 plan 생성을 중단한다. Task preparation의 no-effect plan은 current project revision을 preview에 묶고, execute는 geultu Workspace lock 안의 revision CAS를 요구한다. `authoring.prepare`는 durable task packet만 만들며 writer를 호출하거나 canonical document를 바꾸지 않는다. `authoring.review`는 실행 시 geultu가 같은 후보 내용과 현재 source/guidance blocker를 다시 판단한다. Candidate submit은 canonical document를 적용하지 않는다.

provider는 임의 CLI dispatcher가 아니며 `text next`, source create, writer execution, handoff와 package publication을 노출하지 않는다. 특히 `text next`는 저장된 task를 만들 수 있으므로 read로 분류하지 않는다. 승인된 `story.authoring.task.prepare`는 그 좁은 task creation effect를 owner revision CAS로 실행한다. Candidate submit은 draft만 저장하고 채택하지 않는다. geultu의 journal `command_id`가 operation UUID에 결속된 owner replay identity이며, post-dispatch 응답 유실은 `uncertain`으로 남겨 자동 retry하지 않는다.

## 입력·출력·전제조건

| 공개 surface | 입력·검증 | 출력·실패 |
|---|---|---|
| verify-contracts | [registry](../contracts/registry.json) exact producer/copy/hash | drift/schema/example failure 또는 PASS |
| native capability envelope | request hash·허용 transition·cancel/failure/result invariants | 새로운 job value; 무효 전이 거절 |
| run-promptgen | Story resource root의 pinned schema, read-only request file, CapabilityRoot의 payload/ref/permission/mode·새 output. Source 모드는 필수 `--promptgen-bin PATH`, verified Installed 모드는 포함된 PromptGen path 기본값 | candidate result/job 또는 explicit terminal failure/unknown |
| capability validate-candidate | read-only current-source/result file 또는 stdin, expected request hash, succeeded candidate, CapabilityRoot의 실제 artifact | verified paths; 승인/실제 채택은 caller |
| scene-asset request | SMC receipt path/root/SHA·명시 human policy | producer RunRequest; 누락·형식 오류 거절 |

Story JSON 입력은 strict UTF-8 object여야 한다. 잘못된 JSON, decode 후 중복되는 key, `NaN`·`Infinity`, non-object body는 거절한다. Story가 저장하는 JSON은 기존 sorted-key·2-space·UTF-8·newline bytes 형식을 보존하며, raw file hash와 producer canonical content hash는 구별한다. Process byte budget과 deadline은 [아키텍처](ARCHITECTURE.md)의 effect 경계를 따른다.

## 상태·오류·수용 기준

terminal은 succeeded/failed/cancelled/outcome_unknown이며 서로 바꾸어 쓰지 않는다. succeeded에만 resultId가 있고 active job에는 failure가 없어야 한다. cancelRequested 뒤 success는 거절한다. provider effect를 이미 시작했지만 결과를 알 수 없으면 outcome_unknown으로 남기며 retryable을 성공 가능성의 추정으로 설정하지 않는다.

CLI와 owner 경계에서도 `not_started`, `not_applied`, `failed`, `cancelled`, `outcome_unknown`, `source_changed`, `source_stale`는 서로 다른 결과다. 취소나 local cleanup을 성공으로 바꾸지 않는다.

Source 모드의 `run-promptgen`은 호출자가 `--promptgen-bin PATH`를 명시해야 한다. 누락되면
claim/job/provider effect 전에 거절하며, 선택한 경로가 유효한 executable regular file이 아니어도
debug/release binary나 PATH로 fallback하지 않는다. Verified Installed 모드에서는 manifest가
기록한 curated 실행기를 선택할 수 있고, 해당 실행기가 bundle에서 빠졌으면 다른 binary를 찾지
않는다. `--capability-root`는 installed mode에서 필수이며 기존 사용자 디렉터리여야 한다.
설치 root와 이 capability root가 같거나 서로를 포함하면 effect 전에 거절한다. Source 모드는
CapabilityRoot를 생략할 때 Story root를 기본으로 사용하고 외부 root를 명시할 수도 있다.
Read-only request/current-source/result envelope file은 기존 경로 또는 stdin에서 읽는다.
Request payload, referenced input, artifacts/receipt, execution claim과 job/result snapshot은
CapabilityRoot 아래에 결속한다. 새 output은 전용 빈 directory에만 만들고 existing output을
성공으로 재사용하거나 `--force`로 지우지 않는다. payload/ref snapshot과 exact result binding,
expected output kind/media type이 일치해야 한다. 실패 시 저장된 job/result와 이미 생긴 효과를
구분한다.

PromptGen은 held output FD에서 payload/ref snapshot·claim·job/result를 관리한다. Producer stdin은
captured bytes이며 cwd와 artifact 경로는 같은 CapabilityRoot에 결속한다. 실행 뒤 snapshot이나
root identity가 바뀌면 unknown이다. Compile-only에도 configured process timeout·signal·출력
유실은 unknown이며, 관찰된 무효 receipt는 failed다. IPC payload/compilation과 SAC RunRequest는
producer schema의 byte-pinned consumer copy로 검증한다. Optional receipt도 실제 bytes/hash를 확인한다.

공개 CLI error의 기존 stderr·exit code는 유지한다. Unknown error에 유효한 owner JSON object가
있으면 `observedOutput`으로 보존하며 성공으로 해석하지 않는다. 전체 JSON 출력 한도를 넘으면
`observedOutputOmitted`로 생략 사실을 표시한다.

Text CLI는 성공 결과 0, 실패 결과 3, 요청 거절 또는 transport error 2를 반환한다. PromptGen은 `succeeded`일 때만 0을 반환하며 다른 결과는 2다.
Story가 소유한 CLI 문법 오류와 필수 인자 누락은 owner/provider effect 전에 거절한다.

Story capability result의 성공은 `candidateOnly=true`다. Text package 완료나 animatic result에 이 별도 envelope 필드를 임의 추가하지 않는다. TransferAllowed/hash declaration은 필요한 admission이지 actor 인증·network sandbox의 충분조건은 아니다. Actual adoption 시점에 current source와 파일 identity를 재검사하는 책임은 caller에게 있다.

## intentional contract 원본

[Story-owned schemas](../contracts/)와 각 producer schema를 따른다. core와 CLI에 없는 persistence/approval을 발명하지 않는다. source hash/byteLength와 cancellation 의미를 optional provider마다 조용히 바꾸지 않는다.
