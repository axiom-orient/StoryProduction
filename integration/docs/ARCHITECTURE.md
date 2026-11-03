# Story integration 아키텍처

`integration/Cargo.toml`은 독립 workspace다. `storyctl` 실행 crate가 `story-integration-core`에 의존한다. Core는 unsafe·JSON·파일·프로세스·network·clock과 producer 구현 dependency를 갖지 않는다. 식별자, 결과 구분, capability 전이만 소유한다. 제품 동작 계약은 [SPEC](SPEC.md), 최신 실행 근거와 qualification은 [구현 현황](IMPLEMENTATION_STATUS.md)에 둔다.

| 코드 | 책임 |
|---|---|
| `storyctl/src/cli` | 옵션 수용, structured JSON, 종료 코드 |
| `ports` | writing/narrative 공개 호출의 좁은 인터페이스 |
| `adapters/owners` | Geultu/NVC argv·환경·owner 응답 및 명시 거절 해석 |
| `workflows/text` | workspace identity, intake, current source, next, captured handoff |
| `workflows/attempts` | task 전 예약, 단일 writer, 저장 응답과 owner 상태 복구 |
| `workflows/publication` | coherent writing export, exact nine-file package, 마지막 marker |
| `workflows/animatic` | producer receipt 보존과 live source observation |
| `adapters/promptgen` | provider mode별 snapshot·실행·receipt 검증 |
| `contracts` | 폐쇄 schema registry와 producer/consumer byte pin |
| `effects` | strict JSON bytes, anchored file/FD, 불변/atomic 발행, bounded process |

Owner 구현은 Cargo로 연결하지 않고 공개 CLI로 호출한다. 각 producer의 type·hash·검토 의미는 owner별 adapter와 producer 계약에 남는다. 새 provider가 narrative/approval state를 core에 추가하지 않는다.

파일 효과는 재사용 가능한 `effects::files::Directory`의 열린 디렉터리 기준 `openat`/`O_NOFOLLOW`, regular-file identity/mtime/size, bounded read를 적용한다. 불변 발행은 temporary file fsync → exclusive hard-link → parent fsync이며 기존 bytes와 다르면 거절한다. Mutable capability job만 atomic replace한다. Workspace binding은 호출 context가 소유하며 전역 FD map을 만들지 않는다. 처음 관찰한 descriptor project ID도 context에 결속한다.

Attempt는 workspace의 `.story-attempt-<id>.claim`으로 실행을 예약한다. Submit은 같은 root의 `.story-attempt-<id>.submit.lock`과 기존 nested `submission.lock`을 함께 잡는다. Attempt parent가 rename돼도 같은 실행을 다시 예약하거나 다른 namespace에서 submit하지 않는다. Request/response/result는 held attempt directory로 읽고 쓴다. Cleanup은 claim/child inode를 대조하며 lock 해제는 fork가 잠시 FD를 보유해도 즉시 적용한다.

일반 workspace pathname과 SEMI의 `/dev/fd/N` 모두 Directory FD를 보유하고 원래 path/dev/inode를 재확인한다. Writing root, narrative namespace와 project leaf도 개별 FD에 결속한다. Darwin에서 devfs 하위 path open은 실제 디렉터리와 다르므로 owner child는 `fchdir(fd)` 후 상대 경로를 사용한다. Geultu는 writing cwd + `--workspace .`, NVC는 selected project cwd + 공개 `--project-root .`를 받는다. STS의 논리 project ID와 source/token 판단은 NVC가 유지한다. Import만 namespace cwd를 사용한다. Story draft 저장도 같은 project FD를 사용하며 내부 FD 경로를 wire artifact path로 내보내지 않는다.

Process 효과는 explicit executable, 입력/combined output 16 MiB, 요청한 process timeout 1..7200초 상한, 양쪽 pipe drain, process group cleanup을 소유한다. 이 timeout은 cleanup 완료까지의 엄격한 end-to-end wall-clock 보장이 아니다. Spawn 전 실패는 `not_started`, dispatch 뒤 관찰 불가는 `outcome_unknown`이다. Owner nonzero exit의 structured refusal/terminal receipt는 adapter가 해석한다. Local cleanup은 외부 취소 증명이 아니다.

`StoryResourceRoot`는 manifest를 registry 탐색보다 먼저 확인한다. manifest가 있으면 전체 integrity 검증에 실패한 상태를 source checkout이나 CWD로 우회하지 않는다. 실행 파일이 현재 실행 중인 canonical `bin/storyctl`인데 최종 manifest가 없으면 incomplete install로 거절한다. 명시한 manifest-less source root는 유효하다. Installed root에서는 text workspace, export/package 출력, animatic asset/output 경로, capability data가 runtime subtree에 들어가지 않도록 effect 전에 경계를 확인한다. CapabilityRoot는 기존 외부 디렉터리를 사용하며 runtime root와 상호 포함될 수 없다. Source mode의 기존 same-root capability 저장은 유지한다.

`runtime assemble`은 이미 빌드된 executable와 curated contracts, owner skill/resource, 현재 source에 있는 notices만 새 destination에 복사하고 manifest를 마지막에 발행한다. `runtime verify`는 파일 상대경로·길이·SHA-256·Unix mode·target을 재검증한다. 이 manifest는 local integrity 확인일 뿐 commercial redistribution 권리, signing identity, 외부 media/runtime readiness를 인증하지 않는다.

Curated runtime에는 SMC native animatic 경로가 byte-pin해 읽는 두 data file, `SceneMotionCompiler/packages/production/src/animatic-runtime.js`와 `SceneMotionCompiler/packages/contracts/src/trackb-navigation-guard.js`가 포함된다. 이 resource 복사는 animatic 경로에 Node interpreter를 추가하지 않는다. Chromium, macOS font, FFmpeg/ffprobe는 계속 외부 시스템 요구이며 runtime manifest가 그 readiness를 주장하지 않는다.

Handoff는 캡처한 bytes를 소비 owner에게 전달하고 저장 binding·현재 source를 재관찰한다. Package는 coherent writing과 NVC 승인 bundle을 capture하고 열린 output directory FD로 파일을 발행한다. 마지막 owner 조회 뒤 directory identity와 실제 저장된 bytes를 다시 대조한 다음 같은 FD로 manifest를 발행한다. 이는 cross-owner atomic transaction이 아니다. 기존 workspace·attempt·receipt schema는 유지하며 Python 실행 fallback은 없다.

Swift SEMI provider는 인증 transport·동의·mutation approval·workspace anchor를 소유하고 native `storyctl text author`만 호출한다. 서명·XPC 실환경 qualification은 [provider 문서](../macos-provider/README.md)에 분리한다.
