# promptgen-cli

Image-only command line surface:

`image`, `interview`, `serve`, `mcp`, `schema`, `catalog`, `lut-presets`, and
`version`.

Use `image --mode prompt-only` for deterministic provider-free compilation.
`codex-imagegen` is an optional runtime path and remains fail-closed when a
reference transport or provider artifact is unavailable.

`image --mode openai-image-edit` is the sole direct GPT Image 2 edit slice. It
requires one `--reference-image FILE`, a compiled `task_mode=edit` request with
exactly `index=1, role=base`, `OPENAI_API_KEY`, and `--image-output FILE`. The
adapter performs one `POST /v1/images/edits` request and publishes one strict
PNG only after response, dimension, and atomic-output checks. Generate,
composite, pose, multiple-reference, mask, Web, and MCP transport are not
implemented in this mode and fail closed.
