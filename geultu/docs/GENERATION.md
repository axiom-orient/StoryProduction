# Generation adapters

Geultu separates writing type from generation provider.

```text
canonical project
    -> WritingAdapter.prepare
    -> GenerationPlan
    -> GenerationProvider.generate
    -> untrusted draft
    -> candidate.create
    -> candidate.subject
    -> changeset.create
    -> changeset.approve with a bound human decision
```

## Built-in types

`geultu-writing` registers four type adapters:

| Type | Default stage | Intended output |
|---|---|---|
| `novel` | `draft` | long-form book or chapter Markdown |
| `fiction` | `draft` | short-fiction Markdown |
| `synopsis` | `outline` | premise, conflict, arc, and ending Markdown |
| `storyboard` | `draft` | shot-oriented Markdown |

The type adapter owns format and writing instructions. A new type adds an
adapter package; it does not add a branch to the domain reducer or storage.
Custom type IDs use the same validated identifier rules as project objects.

## Provider boundary

The engine does not contain API keys, HTTP clients, model SDKs, retries, or
provider-specific response parsing. A `GenerationProvider` receives one
`GenerationPlan` and returns one `GeneratedDraft`:

```json
{
  "content": "draft text",
  "model": "provider-model",
  "request_id": "optional-provider-request-id"
}
```

`geultu-writer-process` is the generic process adapter. Configure the CLI or
Studio with `--writer-program PATH` and repeated `--writer-arg ARG`. The
program receives the plan JSON on stdin and returns one draft JSON object on stdout.
Only `content`, `model` and optional `request_id` are accepted; extra result fields fail explicitly.
The process is deliberately explicit; no shell interpolation or implicit
fallback provider is used.

## Public calls

- `writing.types.list` lists registered type adapters and providers.
- `generation.prepare` builds a type-specific plan without writing state.
- `generation.generate` executes a selected provider and returns an untrusted
  draft plus a ready-to-review candidate payload.

Generation calls are read-only from the canonical workspace perspective. The
source content revision and input hash are returned so callers can create a
candidate and let the existing revision/approval checks decide whether it may
become canonical.

Every candidate/changeset rejection and changeset approval records a human
decision bound to an exact pending candidate. Read its engine-computed subject
after creating the candidate:

```json
{
  "project_id": "book",
  "candidate_id": "candidate-1"
}
```

`candidate.subject` returns an `ArtifactRef`. Pass its `revision` and `digest`,
plus the reviewer's `actor_id`, to `candidate.reject`, `changeset.reject`, or
`changeset.approve`. Approval also requires the current `expected_project_revision`.
Every decision rejects missing claims or a subject mismatch before writing its
event. Approval additionally rejects a stale project revision before changing
canonical text. An actor ID is recorded attribution; it is not login authentication.

```js
const subject = await call("candidate.subject", {
  project_id: "book",
  candidate_id: "candidate-1"
});
await call("changeset.approve", {
  project_id: "book",
  changeset_id: "changeset-1",
  expected_project_revision: currentProjectRevision,
  actor_id: "reviewer-id",
  expected_artifact_revision: subject.revision,
  expected_artifact_digest: subject.digest,
  command_id: "approve-1"
});
```

The provider can use the active bounded voice profile and canonical facts,
timeline, scenes, and threads supplied in the plan. Raw voice samples are not
persisted or included in the profile.

## Pack configuration

Application generation planning enriches a plan only from a successfully loaded craft pack.
Explicit Application paths take precedence over `GEULTU_ASSETS_ROOT`, followed by discovered
repository layouts. A configured missing or corrupt pack fails; it is not replaced by another
location. With neither configuration nor a discovered pack, generation remains available without
a craft reference. Mandatory pack/refinement operations still require their inputs.

## Voice input contract

`voice.compare` accepts `project_id` and the required `candidate_text` only. Voice profile creation
accepts a sample string or an object with `text` and optional `id`; absent IDs are assigned stable
request-local indices. The former `candidate`/`text` comparison aliases and `sample_id` alias are
retired without translation. Voice-specific DTOs reject unregistered fields.

Stored profiles must have bounded counts and fixed-point values consistent with those counts.
Comparison and guide use reject inconsistent profiles; they do not silently recompute stored data.
Sample prose is not persisted. The comparison score describes measured habits and does not grant
approval or prove authorship. The short/long sentence tips point toward the measured reference.

The process adapter owns file-backed request/capture I/O and direct-child timeout/cleanup.
Both stdout and stderr byte limits are checked during polling and after observing child exit,
then capture bytes are decoded under the same output policy. Descendant or remote-effect
cancellation is outside this adapter's guarantee; there is no implicit retry.

## Refinement proposal boundary

Refinement diagnoses current scene text and publishes a pending candidate/changeset only.
The catalog and its declared archives must be checksum-covered; the bytes parsed as catalog are
checked against the verified digest. Supported policy is translation-forbidden, Geultu approval,
and `geultu_polish_patch` output authority. Unknown request fields and raw caller replacement helpers
are not accepted.

The changeset transaction ID is the full SHA-256 of UTF-8 command ID bytes prefixed with
`geultu-refinement-changeset-command-v2` and one NUL byte. The clipped `.cs` derivation is retired;
stored proposals are preserved and receive no automatic translation. After publication, the
producer checks actual stored pending artifacts before returning a proposal. This replay check
excludes only observed creation time; human review still hashes the full stored candidate body.
Publication remains two separate transactions, so partial completion/crash recovery still needs
inspection. Canonical text changes require a separate current-subject human approval and budget
admission.

## Loaded pack snapshots

Loaders capture manifest text under its checksum before parsing. Checksum entries use normalized
relative paths and reject duplicates, observed symlink/non-regular entries, traversal and malformed
records; unlisted package files remain outside the verification set. Archive hashing is streaming.
Craft workflow version/node IDs and agent prompt references must be supported and checksum-covered.

`CraftPack` and `RefinementPack` expose read-only `manifest()`/`workflow()`/`catalog()`/`files()`
accessors as applicable. Constructing or mutating a claimed verified pack through public fields is
retired. `CraftPack::status()` takes no directory and returns a JSON value for the loaded snapshot
and its captured directory; it does not reverify live files. Reload when current filesystem evidence
is needed. Workflow nodes serialize using the original flat `kind` representation. No old field,
status or enum-tag compatibility layer is provided. Assets and legal/source metadata are preserved.
