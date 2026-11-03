# NarrativeCourier Tauri adapter — folder analysis

## Responsibility boundary

This folder is the native desktop transport and composition layer for NarrativeCourier. It wires
Tauri app paths and UI commands to the STS `Engine` and `DeliveryEngine`; it does not own narrative,
release, delivery, or approval state. STS remains the only narrative authority, and delivery state
remains under `crates/delivery`.

| Owner | Responsibility |
|---|---|
| `src/lib.rs` | Creates `AppState`, resolves resource/data/export roots, loads the STS manifest, and registers the exact module-qualified handler allowlist. |
| `src/commands/sts.rs` | File selection and STS project, task, artifact, human-decision, resume, and document-export adapters. |
| `src/commands/delivery.rs` | Approved-asset selection plus delivery context, run, cancel, review, list, and preview-read adapters. |
| `src/delivery_preview.rs` | UI preview DTO, text/base64 encoding over bytes returned by DeliveryEngine's bounded, hash-checked artifact read. |
| `src/main.rs` | Desktop binary entrypoint delegating to the library `run()` composition. |
| `src/bin/narrative-courier-cli.rs` | Separate CLI entrypoint; it shares engines but is not a Tauri command adapter. |

## Data and effect flow

`UI invoke → registered command adapter → STS Engine / DeliveryEngine → owner-produced result`.
Command modules may select files and translate transport errors, but they do not reinterpret
NarrativePackage meaning or publish approval on their own. `decide_review` forwards the gate token to
STS; delivery review forwards the explicit decision and current request/output identity to
DeliveryEngine. Long-running agent and delivery calls use Tauri's blocking-task boundary, while their
cancellation and durable-state semantics remain with the engine owners.

The app-state root is intentionally shared by child command modules, but its fields remain private
outside this adapter. The handler list in `lib.rs` is the single Tauri registration point. The
`tools/tauri_wiring.py` source guard, called by `tools/verify_contract.py`, follows that list into the
command source files; it is a source-wiring assertion, not proof that a native window has run.

## Current structural status

STS/file commands and DeliveryEngine commands have separate private owners. `lib.rs` now contains
composition rather than command implementations; `delivery_preview.rs` keeps UI-only read/encoding
behavior separate from DeliveryEngine policy. Public command names and UI argument mappings are
intended to remain unchanged. Tests, build, formatter, lint, native UI, and runtime were not run after
the current source-only edits under the user's instruction.

## Current source-only slice — Delivery preview byte integrity

`read_delivery_preview` calls `DeliveryEngine::read_artifact_bytes` with the UI's 64 MiB limit. Delivery owns
current package/set admission, project-relative artifact resolution, and verification of the same captured
bytes returned to Tauri. The adapter only chooses text versus base64 presentation and preserves the public
command arguments. Tests/build/fmt/lint/native UI/runtime and manifest qualification are **NOT_RUN**.
