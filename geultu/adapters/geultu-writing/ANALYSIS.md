# geultu-writing — built-in plan adapters

## Verdict and boundary

This crate contributes deterministic writing-plan adapters to Geultu's generation registry. It owns
neither canonical state nor external writer effects. Its only product entrypoint is
`builtin_adapters()`, which returns four closed built-in `PromptAdapter` values for novel, fiction,
synopsis, and storyboard work.

## Module ownership

| Owner | Responsibility | Effect boundary |
|---|---|---|
| `src/adapter.rs::PromptAdapter` | Bind one stable adapter ID/type, default stage, Markdown output format, and type-specific instructions; reject a mismatched `WritingType` before preparing a plan | Pure value projection; no filesystem, process, provider, or persistence access |
| `src/builtins.rs::builtin_adapters` | Register the four ordered built-in plan adapters | The Application/GenerationRegistry chooses and invokes adapters |
| `src/prompt.rs::render` / `render_context` | Project source revision, scenes, facts, timeline, story threads, and voice guidance into planner context text | Read-only projection of the supplied `GenerationContext` |

`PromptAdapter::prepare` applies the request stage when supplied, otherwise its configured stage;
it serializes request parameters, renders current canonical context, adds type instructions and
review-before-adoption language, and computes `context_digest(input)` for identity. It sets
`craftpack: None`: optional craft-pack enrichment is owned by the Geultu Application layer, not this
adapter.

The plan is an instruction for a selected `GenerationProvider`; it is not a generated draft, approval,
or canonical write. Source revision and request identity remain bound in `GenerationPlan` for the
registry/provider boundary to recheck.

## Evidence boundary

The `geultu-writing` crate has a public facade and private adapter, builtin and prompt owners, with no independent durable state or CLI. The current
source-only repository refactor did not run Rust tests, build, lint, or provider execution.

## Generation 파트 정리

Context always starts with the actual source revision, even when scenes/facts/timeline/threads are
empty. The unreachable generic empty-context message is removed. Four adapter IDs, default stages,
format and craft instructions, source order, voice guidance and review-before-adoption rules are
preserved. This change was not tested, built or exercised with a provider.

## Writing adapter 파트 — source only

`lib.rs`는 기존 public PromptAdapter/builtin_adapters facade만 소유한다. Adapter identity와
stage selection, closed builtin definitions, prompt rendering은 각각 private module로 나눴다.
Stage는 requested/default 값을 빌려 사용하고 owned plan에 넣을 때만 복제한다. Prompt는 하나의
String에 context와 instructions를 직접 써 scene/fact/timeline/guideline별 임시 format String과
전체 intermediate context String을 제거했다. JSON parameter serialization의 오류는 그대로 반환한다.

기존 adapter IDs/order/default stages/Markdown format/type instructions와 context field projection을
유지했다. Output rule에 source prose/parameters가 tool·approval instruction이 아닌 data임을 명시했다.
이는 provider의 실제 prompt-injection 저항이나 글 품질을 검증한 결과가 아니다. Core input digest,
craftpack enrichment와 별도 human review 권한은 기존 owner에 있다. 기존 test imports만 새 facade에
맞췄으며 tests/build/fmt/lint/provider runtime은 **NOT_RUN**이다.
