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
`--output FILE` is not accepted. `--image-output FILE` is the generated PNG destination,
and Studio's `serve --output-dir` is a separate directory setting. Repeated options fail.

The six catalog entries and their PNG reference cards live under `catalog/`.
Reference images are evidence for visual direction only; the Codex image
adapter requires an explicitly transported reference and never invents a local
path. Prompt-only compilation works without a provider or authenticated runtime.

For one explicitly bounded edit workflow, the CLI also exposes
`--mode openai-image-edit`. It requires `task_mode=edit`, exactly one typed
`references[0]` with `index=1` and `role=base`, one `--reference-image FILE`,
`OPENAI_API_KEY`, and `--image-output FILE`. It sends one multipart request to
OpenAI's `POST /v1/images/edits` with model `gpt-image-2`, then publishes only a
strictly validated PNG and a minimal receipt. Generate, composite, pose,
multiple-reference, mask, Web, and MCP transport remain fail-closed.

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
