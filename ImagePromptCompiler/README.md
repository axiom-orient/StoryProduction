# promptGen 1.5.0

promptGen is an image-only typed prompt compiler and Studio. It keeps one
deterministic contract across the Rust library, CLI, MCP server, and browser
Studio.

## Scope

The catalog contains six primary image outcomes: C1 people and lifestyle, C4
product and brand, C5 campaign and poster, C6 information and UI, C10
illustration and story, and C11 concept and entertainment. Profiles refine these
outcomes without adding catalog categories. See [SPEC](docs/SPEC.md) for the
supported profiles and their boundaries, [IMAGE_PROMPT_SPEC](docs/IMAGE_PROMPT_SPEC.md)
for typed image and render contracts, and [POSE_TRANSFER_PROMPT_SPEC](docs/POSE_TRANSFER_PROMPT_SPEC.md)
for the composite reference workflow.

## Use

```sh
cargo run -p promptgen-cli -- image --input examples/image-logo-identity.json --mode prompt-only
cargo run -p promptgen-cli -- schema image
cargo run -p promptgen-cli -- catalog --family all
```

The workspace requires Rust 1.97, as declared by `Cargo.toml`. A result file uses `--result FILE`;
`--output FILE` is not accepted. `--image-output FILE` is the generated PNG/GIF destination,
and Studio's `serve --output-dir` is a separate directory setting. Repeated options fail.

The six catalog entries and their PNG reference cards live under `catalog/`.
Reference images are evidence for visual direction only; the Codex image
adapter requires an explicitly transported reference and never invents a local
path. Prompt-only compilation works without a provider or authenticated runtime.

The Image API adapter supports native GPT Image 2.6 GIF generation with
`--mode openai-image-generate`, `task_mode=generate`, no references, and
`output.model=gpt-image-2.6-sunburst` or `gpt-image-2.6-flare`.
Use `output.format=gif`, explicit `frame_count=2..16` and
`frame_duration_ms=20..2000` in 10 ms increments. This local profile caps
all decoded frames at 16,777,216 pixels. GIF preview entitlement and
`OPENAI_API_KEY` are required; account access is not inferred from docs.
See [the animated example](examples/image-animated-coffee.json).

`--mode openai-image-edit` sends one base image to `POST /v1/images/edits`;
it accepts GPT Image 2 PNG or GPT Image 2.6 PNG/GIF. It requires exactly one
`references[0]` with `index=1`, `role=base`, and `--reference-image FILE`.
Generation sends JSON to `POST /v1/images/generations`; editing sends multipart.
Both routes require `--image-output FILE`, preserve API keys outside argv,
and publish only validated bytes plus a hash-bound receipt. GIF validation
checks actual LZW raster data, canvas, frame count and frame delays. A PNG
returned for a GIF request fails; it is never relabeled or synthesized into success.
The existing `--mode codex-imagegen` also accepts the GPT Image 2.6 GIF profile
for reference-free generate or one base-PNG edit (`--reference-image FILE`).
It requires a ChatGPT login and a selected CLI with native GIF support, including
the `image_generation_animation` feature (observed in official
`0.168.0-alpha.7`). API key environment variables are removed for this lane.
Use `--codex-binary PATH --codex-home PATH` to select an installed runtime.
Server entitlement remains independent of client version; a known rejection
is a failure. The actual hidden model ID is recorded as unknown. GIF candidates
require separate visual review. Default size policy is exact; explicit
`output.size_policy=provider` retains and records the provider's native canvas.
The legacy GPT Image 2 PNG profile is preserved. No route silently substitutes
another executor. Composite, multi-reference, mask, and Web/MCP image execution
remain unsupported. See [both key-free GIF paths](https://github.com/axiom-orient/StoryProduction/blob/main/docs/GIF_GENERATION.md).

[Official GIF parameters](https://developers.openai.com/api/docs/guides/image-generation#generate-animated-gif-output).

## MCP agent entry point

Build the release server with cargo build --release -p promptgen-cli
--locked --offline, then start the default stdio transport:

```sh
./target/release/promptgen mcp
```

Streamable HTTP is also available on loopback port 4174:

```sh
./target/release/promptgen mcp --transport http
```

An external agent should discover the server, call promptgen_interview with
{"kind":"image","brief":"...","answers":{}}, ask the returned
structuredContent.questions, and retry with the same brief plus answers keyed
by each question id. A ready response exposes the final prompt at
structuredContent.compilation.prompt; invalid is fail-closed. The complete
HTTP/stdio contract, security limits, and runnable examples are in
[docs/MCP.md](docs/MCP.md).

## 정본 문서

Provider generation is a candidate effect. Compilation, PNG validation, visual review,
and Story adoption have different owners. Historical gate results apply to their recorded
source snapshots; current build/runtime/provider qualification is described in Current.

[IDENTITY_AND_EVOLUTION](docs/IDENTITY_AND_EVOLUTION.md) → [SPEC](docs/SPEC.md) → [ARCHITECTURE](docs/ARCHITECTURE.md) → [IMPLEMENTATION_STATUS](docs/IMPLEMENTATION_STATUS.md) → [PLAN](docs/PLAN.md). 상세 경계는 [ANALYSIS](ANALYSIS.md)에서 하위 단위로 연결한다. Current는 실제 관찰, 규범은 확정 계약, PLAN은 남은 구현·결정·검증을 소유한다.
