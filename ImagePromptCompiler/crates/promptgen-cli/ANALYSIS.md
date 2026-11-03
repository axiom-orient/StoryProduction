# promptgen-cli — module ownership

## Identity and authority

`promptgen-cli` owns the `promptgen` executable grammar and adapts requests to the core compiler,
Codex effect, OpenAI edit effect, MCP service, and loopback Web service. It owns no prompt semantics,
catalog data, image-provider credentials, persistent story state, or human approval state.

## Module map

| Module | Owner |
|---|---|
| `main.rs` | The single executable entrypoint and top-level command dispatch. |
| `options/mod.rs` and `options/*.rs` | One `OutputOptions` value owner and shared `--result`/`--force` grammar; each command family owns its remaining option grammar and typed DTOs. |
| `commands/image.rs` | Image compile/mode dispatch and Codex execution composition. |
| `commands/image/openai.rs` | OpenAI edit preflight, provider call, receipt projection, provider path-alias call, and CLI I/O error mapping. |
| `commands/interview.rs` | Typed interview execution and JSON/text result projection. |
| `commands/metadata.rs` | Schema, catalog, and LUT listing commands. |
| `commands/service.rs` | MCP/Web service startup and transport-specific error mapping. |
| `output.rs` | Bounded stdin/file JSON input, stdout formatting, and result-file publication. |
| `paths.rs` | CODEX_HOME selection and lexical output identity for the Codex command. |
| `error.rs`, `help.rs` | CLI exit/error contract and command help text. |

## Output contract

`--result FILE` is the sole file destination for JSON/text results from `image`, `interview`,
`schema`, `catalog`, and `lut-presets`. `options/mod.rs` owns the shared `OutputOptions` value, result
path parsing, duplicate admission, and force flag; command parsers do not duplicate those fields or
grammar. The old generic `--output FILE` synonym is removed. The distinct `--image-output FILE` names
generated image bytes; `serve --output-dir DIR` names the Web service's artifact directory. These
options retain separate meanings and owners.

`output.rs` rejects oversized/non-UTF-8 input and publishes result files through a same-directory
temporary file. `--force` controls replacement of the result file only; it does not relax input,
reference, or provider checks. The OpenAI command maps provider path-inspection I/O errors at the CLI
boundary while using the OpenAI adapter's canonical alias algorithm.

## Current source-only change

The `--output FILE` result synonym was removed from the five result-producing option parsers and their
help text. Repeated `--result`/`--force` parsing and their values now share one `OutputOptions` owner.
The CLI boundary case was rewritten to require `--output` rejection and repeated `--result` rejection.
No test, build, formatter, lint, CLI/provider runtime, or source-snapshot check was run after this
clean break; the preserved source manifest has not been refreshed.
