# StoryProduction SEMI provider — module ownership

## Boundary

This is a macOS helper that exposes a constrained authoring capability provider. It owns the SEMI
adapter lifecycle and fixed-workspace process boundary; it calls StoryProduction's public `storyctl`
entrypoint for every read or mutation. It does not read or write Geultu/NVC state files itself and
does not decide canonical adoption outside geultu's owner API.

## Module map

| File | Owner | Boundary |
|---|---|---|
| `ProviderConfiguration.swift` | Parse fixed launch arguments, resolve configured paths once, anchor the selected workspace directory by descriptor/device/inode, and prepare child working-directory actions | A request cannot choose or redirect the workspace after startup |
| `StoryCLI.swift` | Invoke only `storyctl text author <operation>` with bounded input/output/time, cancellation and process-group teardown | Delivers raw owner stdout to `StoryProvider`; does not interpret domain JSON |
| `StoryJSON.swift` | Decode bounded JSON with duplicate-key rejection and retain number tokens until exact UInt64 conversion; encode those tokens without rounding | Adapter-local transport validation, never a shared domain core or producer-schema owner |
| `StoryProvider.swift` | Advertise the closed read/mutation capability set, validate input/plan/idempotency and the preview-bound project identity, prepare exact owner commands, reserve bounded operation state, and preserve `not_applied`/`uncertain` outcomes | Story status/candidate/task/doc facts come from the public owner CLI |
| `main.swift` | Bind the exact executable identity, register with SEMI, reconnect with bounded backoff, and drain active work on disconnect or termination | Registration and peer cleanup remain explicit host lifecycle state |

`StoryProvider.ownerRead` is the sole JSON object decoder for owner read responses. The previous
`StoryCLI.read` duplicate parse has been removed; the CLI transport checks process success/output
presence, and the provider converts one parsed object into the SEMI result. Read and mutation paths
remain distinct because mutation dispatch can end in uncertainty after a lost owner result.

The adapter validates revisions from original number tokens, not `NSNumber.int64Value` or
`doubleValue`. This preserves the Geultu UInt64 range, including values above 2^53, and rejects
booleans, negative/fractional values and overflow before preparing a CAS operation. Foundation's
object decoder can round a fractional or exponent token before a later Decimal check, so the local
codec also rejects duplicate keys at every depth. Brief/draft/review input types and candidate review
content are checked without replacing malformed owner fields with empty approval data.

A zero process exit and a JSON object do not establish a mutation result. Success requires the
public Geultu `commit` and operation-specific `result` shape, the approved project identity, exact
UInt64 commit metadata with a positive revision and a Boolean idempotency observation. Prepared
task instructions/selections must match approval and remain active; submitted candidates must
match the approved task, draft and lineage. Reviewed decisions must match the approved hash,
decision and note, and the receipt candidate's draft/task/based_on must equal the captured preview.
A brief receipt must retain every explicitly provided field; omitted fields may
use owner defaults. Malformed, unrelated or terminal failure output cannot become success.
Known `not_applied`/`not_started` responses remain failed;
unconfirmed cancellation or effects remain uncertain. Startup requires a regular executable file;
directories, FIFOs and invalid explicit paths are rejected without fallback. Only the invocation owning the transition
from ready to dispatch releases its cancellation flag. Duplicate reads/preparations/executions cannot
share an active operation ID or clear another invocation's cancellation state.

## Evidence boundary

The Swift package is macOS 26+ and depends on the workspace-local `SEMIProviderSDK` plus pinned
`swift-subprocess`. On 2026-10-03, native Debug/Release builds and 25 Swift tests (94 cases) passed
with Swift 6.4. The release executable's ad-hoc signature and exact signing identifier were also checked.
These tests import the actual provider target. Bounded shell fixtures exercise malformed inputs,
precise CAS, receipt validation, operation collisions, cancellation and replay. A separate scratch
workspace uses actual native `storyctl` and Geultu public CLIs, with no interpreter on PATH, to
check prepare without mutation, brief CAS/stale rejection, task preparation, untrusted submission,
rejection without adoption, separate acceptance and descriptor project-identity change before
dispatch. A stale CAS is conservatively uncertain when the owner adapter cannot establish an
unapplied receipt; the unchanged owner state is checked independently. No actual user workspace,
preferences, LaunchAgent or XPC registration is changed. Production signing trust,
same-team/ad-hoc XPC connection, real SEMI consent and human approval remain **NOT_RUN**.

Reproduce after building native `integration/target/release/storyctl` and
`geultu/target/release/geultu`:

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
  STORY_PROVIDER_TEST_STORYCTL="$PWD/integration/target/release/storyctl" \
  swift test --package-path integration/macos-provider --jobs 4
```

`STORY_PROVIDER_TEST_STORYCTL` and `STORY_PROVIDER_TEST_GEULTU` select explicit alternate native
binaries. Missing binaries fail the native test; they are not silently skipped. Rust native owner
tests separately exercise the directory FD and public Geultu call after workspace pathname replacement.
