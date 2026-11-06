# SceneMotionCompiler

선택 animatic 경로는 `animatic-render/status/cancel/recover` public CLI로
source-bound NVC packet과 SAC PNG pack/review를 소비한다. pinned Chromium의
프레임 합성으로 editable HTML·VTT·VP9 WebM·GIF 후보를 출력한다. 사용법과 승인
경계는 [움직이는 콘티 운영](../docs/ANIMATIC_PRODUCTION.md)을 따른다.

승인 cinematic/visual reference·motion/editorial 후보의 사용은 [StoryProduction 운영 가이드](../docs/OPERATIONS.md), 실제 구현 범위는 [Current](docs/IMPLEMENTATION_STATUS.md)를 참조한다.


SceneMotionCompiler (SMC)는 NarrativeCourier의 완료 release를 검증해 SceneGraph로 projection하고,
SceneAssetCompiler의 승인된 asset handoff를 소비해 moving scene HTML과 Track B release evidence를 만드는
독립 제품이다. 제품 문서·ScreenContract·VisualTargetIR·UI release는
별도 제품 VisualScreenCompiler의 경계이며 이 저장소에는 포함하지 않는다.

## 현재 범위

- 지원·수용 기준: macOS Darwin/arm64
- 입력 권위: Completed NarrativeCourier release
- asset 생성·PSD 의미·rights 결정: SceneAssetCompiler
- SMC 책임: projection, handoff 검증, self-contained HTML, measured evidence, custody-backed release
- 일반 사용자 표면: macOS 네이티브 런처와 한국어 Studio 장면 미리보기

Linux 런타임과 Linux 데스크톱 UI는 현재 지원·검증 범위가 아니다.

## macOS에서 장면 미리보기 열기

개발자 또는 배포 담당자가 저장소에서 한 번 앱을 만든다.

```bash
npm run studio:macos:build
```

그 뒤 일반 사용자는 저장소 루트의 `SceneMotionCompiler Studio.app`을 더블클릭하고 다음만
수행한다.

1. SceneAssetCompiler에서 이미지와 자산을 승인·게시한 프로젝트 폴더를 선택한다.
2. 브라우저에서 **장면 미리보기 만들기**를 누른다.
3. **미리보기 열기**로 이미지·대사·샷 시간을 확인한다.

앱은 프로젝트를 검사하고 Studio를 loopback에서 시작하며 기본 브라우저를 연다. 준비되지
않은 폴더에는 SceneAssetCompiler에서 먼저 해야 할 일을 안내한다. 앱을 종료하면 앱이 시작한 Studio
서버도 종료된다. 앱이 비정상 종료되어도 Node 자식은 부모 사망을 감지해 종료한다. 이미 만든
미리보기가 있으면 재진입 화면에서 바로 열거나 다시 만들 수 있다. 런처는 승인 command를
소유하거나 CLI/MCP에 노출하지 않는다.

`SceneMotionCompiler Studio.app`은 이 checkout과 빌드 당시 Node.js에 묶인 로컬 산출물이다.
다른 Mac에 전달할 서명·공증·Node 번들 배포본은 아직 제공하지 않는다.

외부 provider, 실 PSD, public custody/native helper를 실행하지 못한 경우 성공으로 추정하지
않고 검증 결과에 `NOT_RUN_*`을 남긴다.

## 공개 명령

CLI의 공개 명령과 옵션은 [command registry](packages/application/src/command-registry.js)가 소유한다.
기존 Track B/production 경로는 아래 명령을 사용하고, 일반 장면 확인에는 macOS 앱을 사용한다.

```text
production-prepare
project-narrative-bundle
compile-scene-html
trackb-evidence-measure
trackb-release-prepare
trackb-release-promote
trackb-release-status
animatic-render
animatic-status
animatic-cancel
animatic-recover
```

`compile-scene-html`은 `handoff/scene-motion-asset-handoff.v2.json`만 production 입력으로
받는다. 임의 Markdown, SceneGraph, HTML, PNG, MP4 입력은 production 경로로 승격되지 않는다.
`production-prepare`는 source-bound motion/clip/QC/editorial/OTIO 후보를 준비하며 생성 provider를 실행하거나 최종 release하지 않는다.
release approval은 CLI/MCP에 노출하지 않고 Studio의 정확한 loopback origin/session에서만
사람이 수행한다.

Animatic 네 명령은 operator CLI에만 있으며 MCP/Studio command로 노출하지 않는다.
Render는 approved NVC packet과 SAC pack/review를 요구하고 `--motion full|reduced`를 받는다.
Animatic producer는 owner-local Rust `smc-animatic`이며 Node/Python을 실행하지 않는다.
기존 SMC Node application의 네 명령은 같은 native executable에 전달한다.
locked Chromium/font, FFmpeg/ffprobe가 필요하다. 결과는 `finalApproval:false`,
`audio:none`인 WebM·렌더 GIF 후보·HTML 합성 원본·VTT이며 기존 custody release promotion이 아니다.
같은 attempt의 다른 입력은 거절하고 cancel/recover가 renderer를 자동 재시작하지 않는다.
Direct SMC CLI는 전달 packet/asset bytes를 재검사한다. 실제 현재 NVC 원고의 재관찰은
StoryProduction integration 경로가 별도로 수행한다.

```bash
cargo build -p smc-animatic --release --locked
target/release/smc-animatic render --project PATH --attempt ID \
  --packet REL --assets REL --asset-review REL --motion full
```

`status|cancel|recover --project PATH --attempt ID`는 기존 기록을 관찰하며 render를
재시작하지 않는다. `SMC_CHROMIUM_PATH`, `SMC_FONT_PATH`, `SMC_FFMPEG_BIN`,
`SMC_FFPROBE_BIN`으로 실행 자원을 명시할 수 있다. schema와 browser drawing/guard
asset은 native binary에 포함된다. `--owner-root PATH`를 전달하면 그 루트의 schema·drawing·guard
bytes를 제한된 크기로 읽고 binary의 계약 pin과 대조해 사용한다. 임의 resource drift는 거절한다.
기존 Node application에서 binary 위치를 바꾸려면
`SMC_ANIMATIC_BIN`을 명시한다. interpreter fallback이나 실행 중 자동 build는 없다.

계약 schema는 폐쇄 집합으로 관리한다. SMC active set은 AssetManifest와 NarrativeCourier
package/bundle/map/continuity/cinematic plan/approval/projection/SceneGraph, SceneAssetCompiler consumer set은 binding manifest, asset
pack, audit, approval, handoff와 visual reference pack/approval이다. production draft/candidate schema도 닫힌 별도 집합이다. `evaluation-report.schema.json`은 archival 보존만 하며
runtime lookup에서 거부된다. 각 집합 밖의 schema와 active `$ref` drift는 계약 검증을
실패시킨다. Animatic producer/consumer는 별도 closed schema set이다.
생성되는 AssetManifest의 subject 필드는 `approvalSubjectHash` 하나만 사용한다.

## Track B 흐름

```text
Completed NarrativeCourier release + continuity + approved cinematic plan
  → project-narrative-bundle
  → durable NarrativeSceneProjection + SceneGraph
  → approved SceneAssetCompiler SceneMotionAssetHandoff
  → compile-scene-html
  → pinned Chromium renderer evidence
  → custody durable receipt
  → candidate → Studio human approval → DeliverySet/pointer CAS
```

각 단계는 앞 단계의 schema, raw/canonical hash, identity, path containment를 다시 검증한다.
SMC는 screenplay Markdown을 production HTML에서 다시 parse하지 않으며, SceneAssetCompiler bytes의
MIME/hash/rights/lineage를 추론하지 않는다.

## 검증

```bash
npm ci
npm run verify
```

`npm run verify`가 전체 aggregate 검증 진입점이다. pnpm을 쓰는 환경에서는
`SMC_PACKAGE_MANAGER=pnpm pnpm run verify`를 실행한다. manifest 대상 source나 문서를
바꿨다면 `npm run source:manifest` 또는 `pnpm run source:manifest`로
`SOURCE_MANIFEST.json`을 갱신한 뒤 aggregate 검증을 실행한다.

테스트 prerequisite, runner 제한, 현재 검증 범위는 [Current](docs/IMPLEMENTATION_STATUS.md)에 둔다.


## 정본 문서

[IDENTITY_AND_EVOLUTION](docs/IDENTITY_AND_EVOLUTION.md) → [SPEC](docs/SPEC.md) → [ARCHITECTURE](docs/ARCHITECTURE.md) → [IMPLEMENTATION_STATUS](docs/IMPLEMENTATION_STATUS.md) → [PLAN](docs/PLAN.md). 상세 경계는 [ANALYSIS](ANALYSIS.md)에서 하위 단위로 연결한다. Current는 실제 관찰, 규범은 확정 계약, PLAN은 남은 구현·결정·검증을 소유한다.

[제품 상세 계약](docs/PRODUCT_CONTRACT.md) · [NVC 경계](docs/BOUNDARY_NARRATIVE_COURIER.md) · [상위 registry](../integration/contracts/registry.json)
