# SceneAssetCompiler 운영 가이드

## 결론

SceneAssetCompiler은 생성 계획·증거·QA·승인·게시를 소유합니다. 실제 픽셀 실행기는 두 가지입니다.

| 작업 | 실행기 |
| --- | --- |
| 참조 없는 PNG text-to-image | Scene Provider v2 (`scene-provider-v2`) |
| 참조 이미지, edit target, cutout 보정, PSD | Codex `$imagegen` |

SceneAssetCompiler 자체에는 OpenAI API client, API key 처리, 배경 제거, matting, PSD 조립 코드를 넣지 않습니다.

## Provider CLI

`execute-scene-provider` runs one reference-free PNG attempt through the pinned v2 provider.
`probe-scene-provider` performs a bounded protocol probe without generating an image. These are
the canonical command names accepted by the CLI. References, edits, cutouts, and PSD work stay on
the `$imagegen` path above.

## 1. 스킬 발견

Standalone bundle을 추출한 뒤 bundle root에서 다음 값을 먼저 절대경로로
확정합니다. 이후 이 문서의 provider, PerfectPixel, lock, product, run 경로는
이 값만 사용합니다.

```bash
BUNDLE_ROOT="$(pwd -P)"
PRODUCT_ROOT="$BUNDLE_ROOT/SceneAssetCompiler"
SCENE_ASSET_COMPILER="$PRODUCT_ROOT/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py"
PROVIDER_BIN="$BUNDLE_ROOT/bin/scene-provider-v2"
PERFECTPIXEL_BIN="$BUNDLE_ROOT/bin/perfectpixel"
COMPONENT_LOCK="$PRODUCT_ROOT/contracts/runtime-component-lock.json"
RUN_ROOT="$PRODUCT_ROOT/.scene-assets/runs"
```

Source checkout을 Codex 프로젝트 루트로 열거나, source checkout의 canonical
skill을 사용자 skill root에 연결할 때는 다음처럼 실제 경로를 resolve합니다.
기존 대상이 없을 때만 실행합니다.

```bash
SOURCE_ROOT="$(cd "/path/to/StoryProduction" && pwd -P)"
SOURCE_PRODUCT_ROOT="$SOURCE_ROOT/SceneAssetCompiler"
mkdir -p "$HOME/.agents/skills"
ln -s "$SOURCE_PRODUCT_ROOT/.agents/skills/scene-asset-compiler" \
  "$HOME/.agents/skills/scene-asset-compiler"
```

새 task를 시작해야 skill catalog가 다시 로드됩니다. 중복 `name: scene-asset-compiler` package를 여러 active root에 만들지 않습니다.

## 2. Scene Provider v2 준비

Scene Provider 소스에서 package boundary 이름의 provider를 빌드합니다.

```bash
cd /path/to/scene-provider-source
cargo build --offline --locked --release --bin vergerail-upagent-provider
cp target/release/vergerail-upagent-provider "$BUNDLE_ROOT/bin/scene-provider-v2"
```

세 외부 입력을 명시합니다.

```bash
export VERGERAIL_CODEX_PACKAGE="/absolute/path/to/official-codex-0.150.1-package"
export VERGERAIL_MODEL="gpt-5.6-luna"
export VERGERAIL_WORKSPACE="$PRODUCT_ROOT"
```

운영 호스트에서 선택한 Scene Provider의 검증된 runtime root와 bundle provider를
준비합니다. `VERGERAIL_WORKSPACE`는 위에서 resolve한 실제 product root입니다.

```bash
export VERGERAIL_CODEX_PACKAGE="/absolute/path/to/pinned-scene-provider-runtime"
export VERGERAIL_MODEL="gpt-5.6-luna"
export VERGERAIL_WORKSPACE="$PRODUCT_ROOT"

"$PROVIDER_BIN" \
  --vergerail-runtime-verify "$VERGERAIL_CODEX_PACKAGE"
```

- `VERGERAIL_CODEX_PACKAGE`는 선택한 Scene Provider revision의 pinned runtime과 일치해야 합니다.
- 계정은 Codex에서 이미 ChatGPT로 로그인돼 있어야 합니다.
- 이 경로에는 `OPENAI_API_KEY`가 필요하지 않으며 SceneAssetCompiler은 credential을 읽지 않습니다.
- bundle provider 파일명은 정확히 `scene-provider-v2`여야 합니다. upstream Cargo
  target은 `vergerail-upagent-provider`일 수 있지만 package 단계에서 canonical 파일명으로
  복사해야 하며, 이름이 바뀌지 않은 binary는 runtime 입력으로 거부됩니다.

`VERGERAIL_*` 환경변수와 `--vergerail-runtime-verify` 플래그는 upstream provider 실행
계약이므로 adapter가 그대로 전달합니다. 이 upstream ABI 보존은 provider binary filename,
tool identity, receipt/evidence 이름의 별칭 허용을 뜻하지 않습니다. Runtime은
`scene-provider-v2`, `scene-provider-*`라는 canonical identity만 받습니다.

## 3. Run과 작업 생성

repository root에서 bytecode 생성을 끄고 canonical CLI를 사용합니다.

```bash
export PYTHONDONTWRITEBYTECODE=1
export SCENE_ASSET_COMPILER="$PRODUCT_ROOT/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py"

RUN_DIR="$RUN_ROOT/<run-id>"
python3 "$SCENE_ASSET_COMPILER" init "$PRODUCT_ROOT/request.json" "$RUN_DIR"
python3 "$SCENE_ASSET_COMPILER" validate-authority "$RUN_DIR"
python3 "$SCENE_ASSET_COMPILER" compile-plan "$RUN_DIR"
python3 "$SCENE_ASSET_COMPILER" ready "$RUN_DIR"
python3 "$SCENE_ASSET_COMPILER" begin "$RUN_DIR" <task-id>
```

재사용 투명 자산의 운영 형식은 PNG로 고정합니다. 현재 decoded alpha·canvas·margin gate는 PNG에만 완전하게 적용됩니다.

## 4. Scene Provider 사전 검사와 실제 생성

먼저 비용 없는 dry run을 실행합니다.

```bash
python3 "$SCENE_ASSET_COMPILER" execute-scene-provider "$RUN_DIR" <task-id> 1 \
  --provider-bin "$PROVIDER_BIN" \
  --perfectpixel-bin "$PERFECTPIXEL_BIN" \
  --component-lock "$COMPONENT_LOCK" \
  --quality medium \
  --dry-run
```

`READY`와 함께 size, background, quality, capsule byte count/hash를 확인합니다. 그다음 같은 attempt를 실제로 한 번만 실행합니다.

```bash
python3 "$SCENE_ASSET_COMPILER" execute-scene-provider "$RUN_DIR" <task-id> 1 \
  --provider-bin "$PROVIDER_BIN" \
  --perfectpixel-bin "$PERFECTPIXEL_BIN" \
  --component-lock "$COMPONENT_LOCK" \
  --quality medium
```

성공 시 다음 파일이 한 번에 생성됩니다.

- `scene-provider-dispatch.json`
- `scene-provider-process.json`
- `outputs/generated.png`
- `execution-receipt.json`
- `result.json`
- `mechanical-review.json`

`scene-provider-dispatch.json`은 provider 실행 전에 먼저 기록됩니다. process/host가 중간에 종료돼도 같은 attempt를 다시 보내지 않게 하는 안전장치이며, provider binary·capsule·정확한 request frame의 SHA-256을 보존합니다. `scene-provider-process.json`은 전용 process group의 PID/PGID와 monotonic deadline·teardown budget을 기록합니다. controller가 살아 있는 동안 timeout이 나면 live child leader의 PGID를 signal 직전에 확인하고 best-effort로 group을 정리하며, leader reap과 no-survivor 관찰을 receipt에 남깁니다. POSIX에서는 확인과 signal이 원자적이지 않으므로 아주 좁은 PID/PGID 재사용 race가 남습니다.

controller 또는 host 자체가 강제 종료된 경우 자동 orphan 정리는 아직 없습니다. 기록된 PID/PGID 숫자만 보고 `kill`하지 마십시오. PID가 재사용되면 무관한 process를 종료할 수 있습니다. process identity와 외부 provider 상태를 수동 확인하고, custody를 증명하지 못하면 해당 attempt는 `UNKNOWN`으로 보존합니다. dispatch journal 때문에 같은 attempt 재실행은 계속 차단됩니다.

`READY_FOR_VISUAL_REVIEW`는 실제 PNG 형식, canvas, receipt binding이 통과했다는 뜻입니다. alpha는 `transparent_background=true`일 때, foreground core와 여백은 해당 재사용 asset contract가 요구할 때만 추가로 통과합니다. 의미·정체성·미술 방향 승인까지 뜻하지 않습니다.

v2 preflight는 provider SHA-256과 `misegen.provider/2`/schema version 2를
고정하고, 실제 PerfectPixel의 `schema`를 실행해 CLI version, standalone
role, `perfectpixel.asset-inspection/1`, `schema`/`inspect` 명령과 source
manifest pin을 확인합니다. PerfectPixel inspect 결과는 성공 PNG의 hash,
byte count, dimensions, alpha bound evidence만 attempt에 수입합니다.

현재 Scene Provider lane의 제한은 다음과 같습니다.

- PNG만 지원
- 참조 없는 `GENERATE` attempt만 지원
- 크기 `1024x1024`, `1536x1024`, `1024x1536`
- image 한 장, 자동 재시도 없음

참조·edit·PSD 작업은 attempt의 `generation-capsule.md`와 `resolved_references`만 `$imagegen`에 넘긴 뒤 기존 `record-execution → manifest → review` 흐름을 사용합니다.

## 5. 시각 검토와 승인

실제 PNG를 열어 확인한 다음 rubric을 만듭니다.

```bash
python3 "$SCENE_ASSET_COMPILER" visual-template "$RUN_DIR" <task-id> 1
# visual-review.json의 모든 판정과 실제 관찰 근거를 작성
python3 "$SCENE_ASSET_COMPILER" validate-visual "$RUN_DIR" <task-id> 1
python3 "$SCENE_ASSET_COMPILER" approve "$RUN_DIR" <task-id> 1
```

통과하지 못하면 `audit-template → validate-audit → doctor → reconstruction-plan → reconstruct` 순서로 가장 작은 실패 축만 고칩니다. 같은 attempt의 파일이나 receipt를 덮어쓰지 않습니다.

## 6. 실패와 재시도

| 상태 | 조치 |
| --- | --- |
| `PROVIDER_FAILED` | receipt를 유지하고 원인을 진단한 뒤 새 attempt 결정 |
| `MECHANICAL_REVIEW_FAILED` | 반환 PNG를 보존하고 Doctor로 alpha/canvas/content 실패 분리 |
| `OUTCOME_UNKNOWN` | 과금 요청 결과가 불명일 수 있으므로 같은 attempt 재실행 금지 |
| `POSTFLIGHT_INCOMPLETE` | dispatch journal·반환 bytes·오류 기록을 보존하고 같은 attempt 재실행 금지 |

`resolution_required`, timeout, 응답 유실, malformed response는 자동 retry 권한이 아닙니다. 외부 완료 여부를 먼저 확인하고 안전할 때만 새 immutable attempt를 만듭니다.

## 7. 게시와 실제 소비

모든 required task가 승인되면 generic pack을 게시합니다.

```bash
python3 "$SCENE_ASSET_COMPILER" status "$RUN_DIR"
python3 "$SCENE_ASSET_COMPILER" publish "$RUN_DIR"
```

SceneMotionCompiler 소비는 별도 `scene-release` 계약입니다. 각 asset은 명시적인 `kind`, `rights`, `sourceRegionId`, source artifact와 distinct runtime derivative가 필요합니다. 같은 path/hash를 source와 runtime으로 중복 선언할 수 없으며 SceneAssetCompiler이 metadata를 추측하지 않습니다.

즉 Scene Provider PNG 한 장만 생성됐다고 곧바로 SMC handoff가 되는 것은 아닙니다. 승인된 source와 승인된 runtime derivative를 준비하고 다음을 실행합니다.

```bash
python3 "$SCENE_ASSET_COMPILER" scene-release prepare "$RUN_DIR" "$PRODUCT_ROOT/scene-release-request.json"
python3 "$SCENE_ASSET_COMPILER" scene-release approve "$RUN_DIR" --reviewer <human-id>
python3 "$SCENE_ASSET_COMPILER" scene-release validate "$RUN_DIR"
python3 "$SCENE_ASSET_COMPILER" scene-release publish "$RUN_DIR"
```

최종 `handoff/scene-motion-asset-handoff.v2.json`만 SceneMotionCompiler production input입니다.

## 8. 재현성과 검증

이미지 생성은 확률적입니다. model alias와 같은 prompt를 다시 써도 동일 픽셀이 보장되지 않습니다. 승인된 실제 bytes와 SHA-256을 재사용 authority로 삼습니다.

변경 검증:

```bash
cd "$PRODUCT_ROOT"
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s "$PRODUCT_ROOT/tests" -v
PYTHONDONTWRITEBYTECODE=1 python3 "$PRODUCT_ROOT/.agents/skills/scene-asset-compiler/scripts/verify_skill.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$PRODUCT_ROOT/tools/source_manifest.py" verify
PYTHONDONTWRITEBYTECODE=1 python3 "$PRODUCT_ROOT/tools/verify_release.py"
```

fixture provider test는 wire/receipt/QA 연결만 증명합니다. 실제 생성 기능은 opt-in live qualification에서 `Scene Provider call → genuine alpha PNG → mechanical PASS` receipt를 별도로 남겨야 합니다.

현재 설치 상태를 실제 호출로 검증하려면, 사용량을 소비할 수 있음을 확인한 뒤 존재하지 않는 새 evidence 경로를 지정합니다.

```bash
export SCENE_ASSET_COMPILER_LIVE_SCENE_PROVIDER=1
export SCENE_ASSET_COMPILER_LIVE_PROVIDER="$PROVIDER_BIN"
export SCENE_ASSET_COMPILER_LIVE_OUTPUT_DIR="/private/tmp/scene-asset-compiler-live-scene-provider-evidence"

PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  tests.test_scene_provider_executor.SceneProviderExecutorTests.test_live_provider_returns_mechanically_valid_transparent_png -v
```

이 검사는 실제 provider·계정·네트워크를 사용하며, 반환 PNG가 진짜 alpha·canvas·core·margin mechanical gate를 통과해야 PASS합니다. 반복 실행 시에는 새 output directory를 사용합니다.

2026-08-31 현장 검증은 `PARTIAL`입니다. 첫 호출은 실제 alpha PNG를 반환했지만 요청한 square 대신 portrait canvas여서 정확히 거부됐고, 새 portrait attempt는 HTTP 429로 실패했습니다. 429 후 추가 호출은 하지 않았습니다. 근거는 `field-tests/v0.12-vergerail-live/FIELD_TEST.md`에 보존돼 있습니다.

## Visual reference/keyframe

새 `visual-reference prepare/approve/inspect/index`는 현재 SMC v3 projection과 실제 reference image를 사용한다. 자동 이미지 생성이나 기존 scene-release 승인의 대체가 아니다. 명령·draft 원본·현재 제약은 [상위 운영 가이드](../../docs/OPERATIONS.md)를 따른다.
