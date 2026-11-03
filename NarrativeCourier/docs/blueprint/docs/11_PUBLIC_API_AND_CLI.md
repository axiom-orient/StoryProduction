# 11. Current CLI and Target Protocols

## Public CLI — Current

`narrative-courier-cli` is the public binary declared in [`src-tauri/Cargo.toml`](../../../src-tauri/Cargo.toml). Current STS, Delivery, `storyboard`, and `animatic` command lists live in the [product README](../../../README.md#sts-cli) and that binary's `--help` output.

The actual Delivery invocations are:

```bash
narrative-courier-cli deliver PROJECT REQUEST_JSON
narrative-courier-cli deliver-review PROJECT REQUEST_ID OUTPUT_ID approve|reject [NOTE]
```

The JSON request is a positional file argument. The executable also accepts `--project-root DIR` for an existing project whose stored logical identity must match `PROJECT`; import creation cannot combine with that option. The README documents the current admission, path, and token rules. This page's external protocols below are Blueprint Target contracts, not additional CLI commands.

## Request storage and commit marker

The product's current user-visible project tree is in the [README](../../../README.md#release와-저장). A Delivery request owns its four-file contract chain:

```text
visual/requests/<request_id>/
├─ delivery-request.json
├─ director-plan.json
├─ asset-index.json
└─ delivery-set.json       # final publish/commit marker
```

Materialized assets use `visual/assets/blobs/<sha256>.<extension>`; imported image inputs use `visual/assets/imports/<sha256>.<extension>`. Rendered output files use `visual/deliveries/<output_id>/` and are hash-bound by their delivery receipt.

The request ID is bound to its content. Reusing an ID with different content fails; exact-content reuse revalidates the source and artifact bytes. Without `delivery-set.json`, the request is incomplete; leftover render/blob files after failure or cancellation are not a successful delivery.

## MediaBackend protocol — Target

The shape authorities are the [request](../../../protocol/media-backend-request.schema.json) and [response](../../../protocol/media-backend-response.schema.json) schemas. Envelopes carry `protocol_version`, `request_id`, `operation`, and `payload`; `operation` selects `DESCRIBE`, `MATERIALIZE`, or `RENDER`.

The closed response echoes `protocol_version`, `request_id`, and `operation` and requires `ok`, `result`, `error`, and `error_class`. `ok=true` requires a typed operation result with null error fields; `ok=false` requires null result, a nonempty error, and `TRANSIENT`, `PERMANENT`, or `CANCELED`.

- `DESCRIBE` has an empty payload and returns a typed capability descriptor. Its `operations` lists only `MATERIALIZE` and/or `RENDER` capabilities; `DESCRIBE` itself is a protocol operation.
- `MATERIALIZE` selects an exact `director_plan` artifact reference, `asset_request_id`, and isolated `workspace`.
- `RENDER` selects exact `director_plan` and `asset_index` artifact references, `output_request_id`, and isolated `workspace`.
- A backend writes only in the isolated workspace; Runtime validates its result before canonical publication.
- Unsupported or ambiguous capability selection fails explicitly. No silent format or backend fallback is allowed.

The protocol schemas describe the target external boundary. Current deterministic Delivery behavior and any executable qualification remain in the product README and Current.

## AgentExecutor protocol — Target

The shape authorities are the [request](../../../protocol/agent-executor-request.schema.json) and [response](../../../protocol/agent-executor-response.schema.json) schemas. A request selects `protocol_version`, `request_id`, `task`, typed hash-bound `inputs`, `output_schema`, and isolated `workspace`.

- `ok=true` requires an artifact object and null `error` and `error_class`.
- `ok=false` requires a null artifact, nonempty error, and one `TRANSIENT`, `PERMANENT`, or `CANCELED` error class.
- The executor returns structured artifacts only; it has no canonical project write or publish authority.

## Failure and retry boundary

Contract, semantic, unsupported-capability, and ambiguous-backend failures are not hidden by blind retry. Retry is allowed only for an explicitly idempotent operation and the same admitted inputs. Cancellation or a partial external effect retains its actual outcome; it cannot be converted to a successful delivery or final commit marker.
