# Story integration

StoryProduction의 native 공개 실행 파일 `storyctl`과 immutable 교환 계약을 소유한다. 작품 상태·서사 의미·승인 정본은 Geultu/NVC/SAC/SMC에 남는다. 합성 입력을 사용하는 integration E2E는 public 경로의 연결과 파일 무결성을 확인하며 실제 작품 품질·사용자 채택을 인증하지 않는다.

## 실행

```sh
integration/target/release/storyctl doctor --scope text
integration/target/release/storyctl text --workspace /absolute/path/my-story status
integration/target/release/storyctl text --workspace /absolute/path/my-story next
```

`create → author/next → import-handoff → scene → storyboard → package`를 연결한다. `next`는 owner task를 저장할 수 있지만 모델을 호출하지 않는다. `run`은 명시 writer 한 attempt만 실행하고 `recover`는 저장 응답을 처리한다. `export`는 원고만, `package`는 NVC가 검증한 승인 bundle까지 발행한다. [텍스트 운영](../docs/TEXT_PRODUCTION.md)을 따른다.

## Local runtime root

Source checkout의 빌드된 native 실행 파일과 curated resources를 별도 디렉터리에 조립하고 검증할 수 있다. 출력 경로는 새로 비어 있어야 한다.

```sh
STORY_ROOT=/absolute/path/StoryProduction
integration/target/release/storyctl --story-root "$STORY_ROOT" runtime assemble \
  --output /absolute/path/StoryRuntime
/absolute/path/StoryRuntime/bin/storyctl runtime verify
```

PromptGen을 bundle에 포함하려면 `--include-promptgen true`를 선택한다. Runtime manifest는 로컬 파일 무결성·macOS arm64 target을 기록하며, code signing·상업적 재배포 권리·필요한 system media tool의 준비를 확인하지 않는다. 사용자 workspace/capability data는 별도 경로에 둔다.

Curated animatic runtime은 SMC가 byte-pin해 읽는 `SceneMotionCompiler/packages/production/src/animatic-runtime.js`와 `SceneMotionCompiler/packages/contracts/src/trackb-navigation-guard.js`를 SMC producer resource로 포함한다. 이 두 JavaScript 파일은 data resource이며 이 경로에서 Node interpreter를 실행하지 않는다. Chromium, macOS font, FFmpeg/ffprobe 준비는 SMC가 소유하고 bundle의 local-integrity verify만으로 보장되지 않는다.

`storyctl` 자체와 기본 텍스트 owner entrypoint는 native Rust 실행 파일이다. Cargo·Rustc는 개발 빌드/테스트 도구다. 선택 기능은 각 owner가 가진 Python, Node, Swift 및 시스템 자원 요구를 유지하며 Story CLI가 이를 다른 구현으로 대체하지 않는다. 로컬 검증은 `scripts/verify-local.sh`로 실행한다. 이 전체 gate는 선택 animatic 미디어 도구도 사용한다. 기본 텍스트 검증만 실행할 때는 `STORY_REQUIRE_MEDIA_TESTS=0`을 지정한다. source 실행에서 설치 위치가 다르면 전역 `--story-root PATH`, `--geultu-bin`, `--nvc-bin`을 지정한다. Story root는 계약·NVC STS 자료 위치이며 컴파일 당시 개발 경로를 사용하지 않는다.

## 선택 경로

| 공개 명령 | 책임 |
|---|---|
| `runtime assemble --output PATH [--include-promptgen true|false]` / `runtime verify` | 이미 빌드된 macOS arm64 실행 파일과 curated resource로 새 local runtime root를 만들거나 integrity를 재검증 |
| `text animatic` | native NVC direction/packet → SAC asset inspection/review → SMC render/status/cancel/recover; [운영](../docs/ANIMATIC_PRODUCTION.md) |
| `run-promptgen [--capability-root PATH]` | 한 PromptGen compile/generate/edit attempt; candidate-only result/job. Source 모드는 실행기를 명시하고 Installed 모드는 포함된 실행기만 기본 선택 |
| `capability validate-candidate [--capability-root PATH] --current-source FILE --result FILE` | 읽기 전용 source/result envelope와 capability root의 실제 artifact 검증; 승인·채택은 caller |
| `scene-asset request --project ID --smc-project-root ROOT --receipt REL --receipt-hash HASH` | SMC receipt를 SAC RunRequest로 구성; 이미지 실행·승인 아님 |
| macOS SEMI provider | 고정 workspace의 4 reads와 별도 승인된 brief/task/candidate/review 변경 |

Source 모드의 `run-promptgen`은 `--promptgen-bin PATH`를 반드시 받아야 한다. 누락된 옵션은 실행 claim이나 provider dispatch 전에 거절하고, 지정 파일이 없거나 실행 가능한 regular file이 아니면 다른 debug/release binary나 PATH 항목으로 대체하지 않는다. Verified Installed 모드는 manifest가 기록한 `ImagePromptCompiler/target/release/promptgen`을 사용할 수 있으며, 그 실행기가 bundle에서 빠졌으면 외부 실행기를 찾지 않는다. Source 모드의 capability files는 기본 Story root 아래에 둔다. Installed 모드는 기존 외부 디렉터리를 `--capability-root`로 지정해야 하며 이 root는 immutable runtime과 겹칠 수 없다. Request/current-source/result JSON 파일은 기존 read-only path 또는 stdin 입력을 사용한다. Request payload, image reference, 결과 artifact와 새 output은 capability root에 결속된다. PromptGen image generation은 호출자 요청과 별도의 provider/전송 조건을 따른다.

```sh
STORY_ROOT=/absolute/path/StoryProduction
integration/target/release/storyctl --story-root "$STORY_ROOT" run-promptgen \
  --request "$STORY_ROOT/integration/contracts/examples/capability-request.prompt-compile.json" \
  --output-dir "$STORY_ROOT/.story-runs/promptgen/compile-001" \
  --promptgen-bin "$STORY_ROOT/ImagePromptCompiler/target/release/promptgen"
```

위 명령은 `ImagePromptCompiler/target/release/promptgen`을 미리 빌드한 뒤 local `image.prompt.compile` fixture를 실행한다. image generation/provider 호출은 하지 않는다. 각 실행은 새 output directory를 사용한다.

`--story-root`는 `integration/contracts/registry.json`이 있는 Story source root이며, Source 모드에서 `--output-dir`는 이 root 아래의 새 execution directory로 해석된다. Installed 모드에서는 `--output-dir`가 `--capability-root` 아래에 있어야 한다. `runtime assemble`은 기존 release executable/resource만 새 output root에 복사하고, `runtime verify`는 저장된 manifest의 local file integrity를 확인한다. 이 기록은 코드 서명·상업적 재배포 권리·외부 도구 준비 상태를 인증하지 않는다. `doctor --scope authoring|text|promptgen|animatic`은 기능별 실행 파일/자원의 존재만 확인한다. 필수 항목이 모두 있으면 `PASS`, 누락이 있으면 `PARTIAL`과 `missingRequired`를 보고하며, 작품 완료·실행 준비·provider 실환경 qualification을 인증하지 않는다. `not_started`는 doctor 상태가 아니라 개별 명령이 effect dispatch 전에 멈춘 경우의 결과다. Animatic의 Chromium/font와 FFmpeg/ffprobe는 SMC 소유 자원이며 Story doctor 범위 밖이다.

## 구조와 보존 계약

`story-integration-core`는 JSON·I/O·clock·producer domain 없이 식별자·결과·capability 전이만 소유한다. `storyctl`은 CLI, 좁은 owner ports, workflow, owner별 adapter, contracts, POSIX effects로 나눈다. Core 역의존과 owner implementation crate 연결을 금지한다.

`story-project.json`은 project/intake 참조이며 attempt는 실행/복구 기록이다. 같은 경로는 같은 bytes만 재사용하고 unknown 효과를 자동 retry하지 않는다. 후보 저장·사람 승인·완료 package는 구분한다. Producer schema/hash 규칙과 [registry](contracts/registry.json)의 byte pin은 유지한다.

현재 제품 계약은 [SPEC](docs/SPEC.md), owner·effect 경계는 [ARCHITECTURE](docs/ARCHITECTURE.md), 최신 검증·NOT_RUN과 날짜별 source evidence는 [구현 현황](docs/IMPLEMENTATION_STATUS.md)에 둔다.
