# SceneAssetCompiler

Native GIF intake and exact-frame handoff: [Story GIF workflow](../docs/GIF_GENERATION.md). GIF generation stays in promptGen; SAC decodes and inspects existing bytes.


외부에서 만든 실제 시각 자료를 task/attempt·검사·선택·사람 검토·AssetPack/scene handoff로 관리하는 CLI와 Codex skill이다. 일반 제작은 Python skill CLI, animatic PNG pack은 Rust `sac-animatic`이 소유한다. NVC 서사 정본이나 SMC 렌더/release를 대신 소유하지 않는다. 로컬 픽셀·matting·path tracing·PSD assembly로 생성 실패를 우회하지 않는다.

## 공개 경로

| 경로 | 입력 → 결과 | 진입점 |
|---|---|---|
| 일반 asset workflow | RunRequest/VisualAuthority → tasks/attempts → 반환 artifact → mechanical/visual review → selection → Human policy → AssetPack | active skill CLI |
| Scene release | current SMC projection/rights → pack/audit/binding → subjectHash approval → handoff/v2 | `scene-release prepare/approve/validate/publish` |
| Visual references | current graph/shot/entity·실제 image → reference/keyframe candidate → 별도 approval → derived index | `visual-reference prepare/approve/inspect/index` |
| Scene/PSD | GenerationUnit·placeholder/layer locks → 외부 PSD → structural/packaging evidence | scene/PSD CLI |
| Animatic PNG pack | approved NVC packet·실제 PNG → assets candidate → exact-hash review | `sac-animatic submit/inspect/review` |

기본 StoryProduction 텍스트 제작은 SAC를 요구하지 않는다. 움직이는 콘티의 자산은 [animatic 운영](../docs/ANIMATIC_PRODUCTION.md), 기존 shot/visual/scene 경로는 [미디어 운영](../docs/OPERATIONS.md)을 따른다.

## 시작과 실행 조건

```sh
python3 .agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py --help
cargo build --release
./target/release/sac-animatic --help
```

예시는 component root 기준이다. 일반 skill CLI는 Python standard library를 사용한다. `sac-animatic` 실행에는 Python·pip·Cargo·Rustc가 필요하지 않다. 바이너리 빌드와 개발 검증에는 Rust toolchain을 사용한다. 독립 Python 개발/release 도구의 의존성은 [requirements-dev.txt](requirements-dev.txt)를 따른다.

설치한 바이너리는 `--owner-root /absolute/path/SceneAssetCompiler` 또는 `SAC_OWNER_ROOT`로 SAC producer·pinned NVC schema 위치를 받는다. 입력 경로와 artifact root는 `--project-root`로 별도로 선택한다. 일반 제작 pipeline의 Python 소유 기능을 `sac-animatic`으로 우회하지 않는다.

픽셀은 설치된 imagegen 또는 제한된 `misegen.provider/2` executor가 만든다. Scene Provider는 reference-free PNG text-to-image만 담당하고 refs/edit/cutout/Path/PSD는 imagegen 경계다. 실제 provider/PerfectPixel의 binary·lock·권한이 필요하며 미설정일 때 성공이나 자동 fallback을 만들지 않는다.

Animatic의 자산 root는 실제 작업 디렉터리이며 입력과 output은 그 아래의 명시 상대 경로다. PNG 크기·raw hash·실제 alpha·asset coverage를 검사한다. RGBA 선언만으로 cutout이라고 표시하지 않고 전체 투명 PNG도 거절한다. 생성 provenance는 선언이며 인증 증명이 아니다.

선택한 root와 parent directory는 열린 FD에 결속한다. 실행 중 root의 이름이 바뀌고 같은
경로에 다른 디렉터리가 생겨도 입력·receipt 존재 조회·동일 review 재사용·발행은 원래 root에
남는다. 저장된 SAC hash는 기존 compact UTF-8 profile과 integral-float dimension 표현을 보존한다.

## 검토·복구

생성·mechanical 검사·visual review·선택·Human 승인·발행은 다른 단계다. Current bytes가 검토 subject와 다르면 재검토한다. `PREFERRED_OUTPUT_MISSING`은 해당 호출 실패이며 PSD의 전역 미지원 판정이 아니다. `SUBMITTER_ATTESTED` packaging 비교는 독립 render-back 관측이 아니다.

일반 workflow와 scene-release·visual-reference·animatic review의 DTO/subject를 바꾸어 쓰지 않는다. Unknown dispatch를 새 유료 attempt로 자동 retry하지 않는다. 실제 원본 PSD·provenance가 없는 G09 evidence 공백은 합성 golden이나 문서로 채우지 않는다.

## 문서와 source metadata

[문서 색인](docs/README.md) · [정체성](docs/IDENTITY_AND_EVOLUTION.md) · [SPEC](docs/SPEC.md) · [ARCHITECTURE](docs/ARCHITECTURE.md) · [Current](docs/IMPLEMENTATION_STATUS.md) · [PLAN](docs/PLAN.md) · [분석](ANALYSIS.md).

[OPERATIONS](docs/OPERATIONS.md)는 실행·실패·복구, [RELEASE_GATES](docs/RELEASE_GATES.md)는 qualification 의무, [RESEARCH](docs/RESEARCH.md)는 설계 근거다. Root AGENTS와 active skill의 provider 정책이 정본이다.

`tools/source_manifest.py`가 skill/docs/release scope의 metadata를 생성·검증한다. Rust animatic 검증 결과는 [Current](docs/IMPLEMENTATION_STATUS.md)에 기록한다. Source archive를 Standalone binary bundle의 통과 근거로 사용하지 않는다.
