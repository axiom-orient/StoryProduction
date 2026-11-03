# Geultu 5

로컬 원문·집필 문서·후보·사용자 결정을 journal에 저장하는 Rust 글쓰기 엔진이다. StoryProduction에서는 `authoring.*`로 원문 → 스토리 → 시놉시스를 만들고 검토된 시놉시스를 NarrativeCourier에 인계한다. STS 씬·대본의 서사 정본은 NarrativeCourier가 소유한다.

## 사용

Rust workspace의 최소 버전은 [Cargo.toml](Cargo.toml)의 1.85다. `geultu/`에서 현재 소스에 맞는 바이너리를 준비하고 사용자 workspace를 지정한다. 아래 명령은 CLI 사용 예시이며 실행 qualification을 뜻하지 않는다.

```sh
cargo build --manifest-path Cargo.toml -p geultu-cli --bin geultu --release --offline --target-dir target
./target/release/geultu --workspace /absolute/path/writing init
./target/release/geultu --workspace /absolute/path/writing project-create my-story "내 이야기"
./target/release/geultu --workspace /absolute/path/writing call authoring.status \
  '{"project_id":"my-story"}'
```

원문 저장·브리프·후보 작성·부분 수정·사람 채택·history/restore·인계의 순서는 [StoryProduction 텍스트 운영](../docs/TEXT_PRODUCTION.md)을 따른다. Generic entrypoint는 `call TOOL JSON`이며 JSON 파일을 stdin으로 보낼 때 마지막 인수는 `-`다. Workspace/writer 옵션은 명령 앞에 둔다.

`authoring.prepare`는 durable task를 저장하지만 writer나 canonical document 변경을 실행하지 않는다. `authoring.submit`은 후보를 저장하며 채택하지 않는다. `authoring.review`는 실제 검토한 candidate hash와 현재 source/guidance를 결속한다. 취소·오래된 후보도 비교 자료로 보존한다.

## 패키지와 진입점

| 패키지 | 책임 |
|---|---|
| `crates/geultu` | Application·authoring/domain·journal·review·voice·refinement·공개 계약 |
| `crates/geultu-cli` | `geultu` CLI와 stdio MCP |
| `adapters/geultu-writing` | novel/fiction/synopsis/storyboard의 순수 generation 계획 |
| `adapters/geultu-writer-process` | 명시적인 외부 writer의 GenerationPlan/GeneratedDraft JSON process 경계 |
| `adapters/geultu-studio` | loopback HTTP·token 기반 기존 Studio |

어댑터는 Application을 통해 요청하고 Workspace/domain이 상태를 commit한다. Generation 계획/외부 draft와 authoring task/candidate는 서로 다른 계약이다. 자세한 provider 입력은 [GENERATION](docs/GENERATION.md)에 둔다.

```sh
./target/release/geultu --workspace /absolute/path/writing mcp
./target/release/GeultuStudio --workspace /absolute/path/writing
```

MCP는 initialize 후 `notifications/initialized`가 필요하며 stdin 종료로 닫는다. Authoring review/edit/restore/handoff 및 candidate/changeset의 사람 결정은 MCP 목록과 실행에서 거절한다. CLI·Studio의 actor는 로컬 attribution이며 신원 인증 증명이 아니다. Studio 바이너리는 별도 `geultu-studio` 패키지로 준비한다.

## NarrativeCourier 인계

주 집필 경로의 `authoring.handoff`는 현재 **채택 시놉시스만** 내보낸다. Export가 보여준 document revision·raw Markdown SHA-256을 사람이 검토해 전달하며, 같은 project revision·actor·language·본문의 중단된 인계만 재개한다. 반환된 `handoff_root`를 사용한다.

Whole-project `export-reviewed-synopsis`는 `project.export_markdown` 전체 결과와 project revision/hash를 검토하는 별도 공개 경로다. 두 경로의 입력을 바꾸어 쓰지 않는다. 둘 다 [producer handoff schema](contracts/reviewed-synopsis-handoff-v1.schema.json)를 사용하며 NVC의 import는 STS 프로젝트 시작까지만 한다. STS final-release gate는 별도다.

## 상태·운영 문서

[제품 계약](docs/SPEC.md) · [ARCHITECTURE](docs/ARCHITECTURE.md) · [Current와 실행 근거](docs/IMPLEMENTATION_STATUS.md) · [남은 qualification](docs/PLAN.md) · [모듈 분석](ANALYSIS.md) · [Licensing](docs/LICENSING.md).

저장 형식은 schema v2이며 v1이나 unknown stored fields를 자동 migration하지 않는다. 공개 API·호환성 경계는 SPEC, 실제 코드와 날짜가 붙은 qualification은 Current에 둔다. 공식 로컬 gate는 `scripts/verify.sh`, 생성물 정리는 `scripts/clean.sh`가 소유한다. 원문·user workspace·vendor·계약·legal 자료는 보존되는 입력이다.
