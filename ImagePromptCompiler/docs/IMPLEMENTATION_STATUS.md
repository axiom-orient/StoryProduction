# ImagePromptCompiler / promptgen — 현재 구현

2026-10-04 문서 갱신에서 현재 CLI/core/provider 경계와 유지 문서를 다시 대조했다.
`scripts/audit-doc-links.py`는 87 links **PASS**, `scripts/test-source-snapshot.py`는 **PASS**다.
`scripts/source_snapshot.py`의 read-only inventory는 계산했지만, 이 출력은 qualification receipt가 아니다.
이 docs-only 변경에서 Rust tests/build/fmt/Clippy, CLI/Web runtime, actual provider, full `scripts/verify.sh`,
현재 source identity qualification은 **NOT_RUN**이며 `VERIFICATION.json`은 쓰거나 갱신하지 않았다.

## 현재 연결

| 범위 | 직접 읽은 근거 | 현재 의미 |
|---|---|---|
| compiler | [core](../crates/promptgen-core/src/lib.rs) | Image-only typed compile/interview/catalog, 여섯 outcome과 category profile |
| CLI | [main](../crates/promptgen-cli/src/main.rs)·[options](../crates/promptgen-cli/src/options/mod.rs) | image/interview/schema/catalog/LUT/serve/mcp; `--result` 단일 result destination |
| Codex effect | [Codex facade](../crates/promptgen-codex/src/lib.rs) | Effect 전 request 재compile·prompt 일치, bounded process/event/artifact/fidelity lifecycle |
| OpenAI effect | [execution](../crates/promptgen-openai/src/execution.rs) | Explicit one-base edit·PNG/receipt, 실제 API 가용성은 별도 |
| MCP·Studio | [MCP](../crates/promptgen-mcp/src/lib.rs)·[Web](../crates/promptgen-web/src/lib.rs) | 같은 compiler 계약·loopback/stdio/HTTP·generation guard와 artifact 표시 |
| identity packaging | [source_snapshot.py](../scripts/source_snapshot.py) | 현재 source와 maintained docs를 inventory; gate 이후 verification receipt와 비교 |

CLI/output·image validation·interview/inference·render section·catalog/LUT와 각 provider의 책임을
정리했다. Buffered raw-byte SHA-256 중복은 core primitive로 모았고 effect/state/오류는 provider에
남겼다. 자세한 module/변경 근거는 [ANALYSIS](../ANALYSIS.md), public 계약은 [SPEC](SPEC.md)를 따른다.
기존 root SOURCE_MANIFEST/SOURCE_SNAPSHOT은 retired output 이름이며 새로 생성하지 않는다.
검증 없는 `VERIFICATION.json`이나 historical image evidence를 만들지 않는다.

## 남은 한계

실제 Codex/OpenAI 인증·endpoint/model 지원·reference 전송·PNG/fidelity 품질과 descendant·대용량·
Studio/MCP 종료·race는 별도 qualification이다. Compiler success와 실제 이미지·SAC/Story 승인은
다르다. 과거 image-evidence 원본과 verification receipt의 source identity 공백은 문서 수정으로
해결되지 않는다. [PLAN](PLAN.md)에서 후속 범위를 관리한다.

## 과거 실행 근거

아래는 당시 source graph의 결과다. 263/273 workspace tests, release/quality/smoke 통과와
historical evidence 누락에 의한 official gate PARTIAL/FAIL을 함께 보존한다. 각 entry의 후속
source-only 변경은 이전 PASS로 인증하지 않는다. 상위 실행 기록은 StoryProduction의
`docs/research/`가 소유하며 component 문서는 내부 정본 링크를 유지한다.

<details>
<summary>과거 source snapshot의 실행 기록</summary>

## 2026-09-30 post-split compile finding and source-only cleanup

The latest owner-split verification stopped at `cargo check`: `category_contracts/questions.rs`
contained unused imports of `CategoryContractSpec` and `parse_subjects`. The parent workspace
also observed that the historical
`evals/dogfood/historical/1.4.0/image/image-evidence.json` input is absent. The unused imports were
removed and nested relative paths to inference/question/category owners were replaced with explicit
crate-qualified paths. This is source-only cleanup; tests, check, fmt, Clippy, docs/release builds,
CLI/Web runtime, source-snapshot qualification, and receipt publication after it are **NOT_RUN**.
The exact prior run and independent audits are in the parent StoryProduction workspace at
`../../docs/research/2026-09-30-live-use-validation.md`.

## Earlier verified source snapshot

Current workspace check, all 273 Rust tests, doc tests, fmt/Clippy, documentation build, and release
build passed. Release CLI and loopback Web smokes also passed, along with documentation links,
catalog asset/prompt, representative-image-QA, Rust-structure, and source-snapshot regression checks.
This predates the later category and route-inference owner split and does not qualify that source.
The detailed older cross-repository scope is in the parent workspace report at
`StoryProduction/docs/research/2026-09-30-real-use-validation-cancellation-hardening.md`.

The official `scripts/verify.sh` remains **PARTIAL** because the checkout lacks
`evals/dogfood/historical/1.4.0/image/image-evidence.json`; the gate stopped there before source
identity publication. No historical evidence or verification receipt was created. Source-snapshot
regression tests passed, but current production source identity was not sealed. External Codex/OpenAI
providers, actual browser/client sessions, and generated-image quality remain **NOT_RUN**.
Cross-repository runtime results are recorded in the parent workspace report at
`StoryProduction/docs/research/2026-09-30-real-use-validation-cancellation-hardening.md`.

## 2026-09-24 current-source qualification — render and inference owners

The latest render-section, subject-inference, and brief-field module graph passes all 263 workspace
tests, doc tests, fmt/Clippy quality checks, documentation build, and release build. Its official
`scripts/verify.sh` remains PARTIAL because the checkout lacks
`evals/dogfood/historical/1.4.0/image/image-evidence.json`; no evidence or `VERIFICATION.json` receipt
was fabricated. Release CLI and loopback Web smoke pass, the source-snapshot tests pass, and seven
representative image example outputs are byte-identical to the preceding verified binary. During
qualification, explicit child-module paths, a moved `quote` import, stale private re-exports, and
formatting were corrected without changing prompt ordering. This component-local status does not
link outside the ImagePromptCompiler tree; the workspace report records the full cross-component run.

## 2026-09-24 last verified graph — before render, subject, and field splits

The current workspace passes check, tests, doc tests, fmt/Clippy quality checks, docs build, and
release build. `scripts/verify.sh` is PARTIAL because `audit-image-evidence.py` requires the absent
`evals/dogfood/historical/1.4.0/image/image-evidence.json`. Release CLI and loopback Web smokes pass;
`test-source-snapshot.py` passes, but `VERIFICATION.json` is absent and its identity check cannot pass.
No historical evidence or receipt was created. External provider execution remains NOT_RUN; source identity remains unqualified.

## 2026-09-24 OpenAI adapter owner split — current-source partial verification

`lib.rs` is now a stable facade re-exporting the same crate-root `execute_image_edit`, config, receipt,
and error types. `api.rs` owns those public DTOs, `execution.rs` owns the ordered provider effect,
`support.rs` owns shared internal error/hash projection, and the existing `tests` module moved to
`tests.rs`; request/reference/transport/response/files/limits owners remain focused. The CLI caller and
one-base edit contract are unchanged.

Current source passed the ImagePromptCompiler check, 263 workspace tests, doc tests, fmt/clippy quality
gate, documentation build, and release build. The official scripts/verify.sh is PARTIAL: it stops at
audit-image-evidence.py because evals/dogfood/historical/1.4.0/image/image-evidence.json is absent from
this checkout. Separate scripts/smoke.sh and scripts/smoke-web.py runs against the release binary passed.
Later catalog audits and source-snapshot write/verify steps were not reached. The checked-in
VERIFICATION.json predates this graph and remains unrefreshed.

OpenAI API/provider execution is NOT_RUN; adapter unit/integration tests use their bounded fake transport.

## 검증

`scripts/test-source-snapshot.py`: **PASS**. 새로운 Codex compilation mismatch 회귀 및 전체 core/provider/Web/CLI tests는 **NOT_RUN**(Rust 부재). integration fake process29의 PASS는 실제 promptgen binary 또는 OpenAI/Codex generation PASS가 아니다.

</details>
