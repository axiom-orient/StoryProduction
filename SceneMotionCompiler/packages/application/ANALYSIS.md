# SMC application — module analysis

## Responsibility boundary

The application package composes SMC commands and custody-backed release operations. It owns command sequencing and application-level immutable candidates. Narrative meaning stays with the imported NarrativeAuthority/SceneGraph contracts, visual approval stays with SceneMotionCompiler Studio, and durable locks, immutable writes, and pointer CAS stay with the store/custody owner.

## Module map

| Module | Responsibility | State/effect boundary |
|---|---|---|
| `src/index.js` | Explicit application entry point for command metadata/registries, CLI name resolver, product version, and command service consumed by CLI/MCP/Studio | Release operations stay in service/operation owners; approval dispatch, concrete error class, Studio frame contract, implementation registry, and derived command maps stay out of the entry point |
| `src/command-registry.js` | One command definition owner; derives the CLI/MCP registries and Studio allowlist; resolves CLI names from the canonical CLI registry | Implementation identity remains application-internal; duplicate command-name maps and callerless registry lookup helpers are removed |
| `src/trackb-commands.js` | Defines the three application-owned Track B handlers consumed by service composition | Exports implementation names, not a second command→handler map; service incorporates these handlers directly and joins them with the canonical registry; no duplicate forwarding functions |
| `src/product-version.js` | Reads the single package version for CLI, MCP server metadata, and Studio health output | No client-specific version literal or second version owner |
| `src/trackb-release-contract.js` | Studio request route and authenticated frame/body validation | The Studio host imports this defining module directly; it is not a general application-entrypoint export |
| `src/service.js` | Public command service, command-to-application routing, and translation of adapter-owned release errors into Application envelopes | Command registry and request dispatch; preserves adapter error code/message/details/retryability at the consumer boundary |
| `src/studio-dispatcher.js` | Studio-only release approval dispatch | Requires the authenticated Studio execution context; no CLI/MCP approval alias |
| `src/command-error.js` | Stable application error value | Error code/message/retryability mapping, no state |
| `src/trackb-application-primitives.js` | Bounded Track B diagnostics and plain-object shape recognition | Pure application-local primitives used by command service, Studio contract, binding, measurement, and release callers; no command/error-code policy |
| `src/trackb-measure-runtime.js` | Public measured-evidence command lifecycle: normalized input, measurement session, browser invocation, receipt publication, and guaranteed session close | Orchestrates one measurement operation; no pathname filesystem access or release approval |
| `src/trackb-measure-evidence.js` | Custody response bytes/limits, NarrativeAuthority and provenance binding, measured receipt/result cross-checks | Evidence admission over bytes returned by the measurement-purpose session; no browser or pointer write |
| `src/trackb-measure-support.js` | Measurement-specific error-code allowlist, fallback message, mapping, and close-failure attachment | Error policy only; preserves `ApplicationCommandError` boundary and measurement fallback |
| `src/scene-asset-compiler-handoff.js` | Stable orchestration adapter from the exact SAC handoff to verified SceneGraph/assets/manifest/provenance | Calls the existing NVC projection and owner registries; returns the same verified tuple to `compileHtml` |
| `src/scene-asset-compiler-handoff-contract.js` | Projection identity, requirement coverage, audit evidence, rights/kind/crop invariants | Pure producer-claim comparison; no project reads or image byte decoding |
| `src/scene-asset-compiler-handoff-files.js` | Root/path safety, producer schema reads, byte-hash/size/MIME checks, generated AssetManifest validation | Filesystem/schema effects; raw digest uses the core byte-hash owner; registry promises start on first actual handoff use, not module import |
| `src/trackb-release-input-contract.js` | Exact seven-byte-set validation, raw-byte parsing, and cross-owner authority/provenance qualification | Pure input contract; no filesystem reads or candidate publication |
| `src/trackb-release-bindings.js` | Narrative/projection/provenance/measured-evidence equality, semantic qualification, typed rejection mapping | Pure source/hash binding rules; consumes application primitives and performs no filesystem or custody calls |
| `src/trackb-release-candidate.js` | Canonical in-memory ReleaseCandidate construction and validation | Candidate bytes and hash output only; it does not persist or approve |
| `src/trackb-release-runtime-prepare.js` | Read exact candidate inputs, bind the verified current base, and stage candidate bytes | Custody-backed ingress; no stable pointer publication |
| `src/trackb-release-runtime-approve.js` | Record the exact Studio human decision as an immutable receipt | Requires trusted Studio execution context; no promotion |
| `src/trackb-release-runtime-promote.js` | Stage DeliverySet and publish the current pointer through CAS | Custody owns durable writes and post-rename uncertainty |
| `src/trackb-release-runtime-status.js` | Read current pointer/candidate/approval projection | Read-only session; no durable writes |
| `src/trackb-release-runtime-registry.js` | Load the exact six-schema Track B registry and pinned SMC/SAC artifact registries | Schema source identity and registration only |
| `src/trackb-release-runtime-artifacts.js` | Read project artifact envelopes, preserve raw bytes, and parse canonical snapshots | Bounded bytes/hash/identity checks; no approval or pointer state |
| `src/trackb-release-runtime-custody.js` | Open/close custody sessions and validate typed response snapshots/writes through the store public barrel | Process/session/protocol lifecycle; no narrative or approval authority |
| `src/trackb-release-runtime-approval.js` | Validate Studio context, candidate authority, and immutable approval receipt | Approval subject and receipt semantics; Studio remains the trusted human ingress |
| `src/trackb-release-runtime-delivery.js` | Build DeliverySet/pointer bytes, verify current base, and derive candidate status | Durable-state projection and CAS subject; writes remain custody-owned |
| `src/trackb-release-runtime-support.js` | Release-specific typed error translation and runtime value helpers | Error taxonomy, exact keys, and hash paths; bounded messages and object-shape checks come from application primitives |

Candidate construction and validation remain with `trackb-release-candidate.js`; durable prepare
uses the custody-backed runtime and exact-byte input contract. The unused path-based candidate
workflow and file reader have no product caller and were removed, avoiding a second non-custody
read path. This does not add a second schema authority or shared cross-product state store.

The application entry point no longer wildcard-reexports the concrete error class or Studio request
contract, forwards unused registry aliases, or re-exports prepare/promote/status runtime operations.
The command service imports each release operation from its defining owner; Studio approval is
imported directly from its defining module and remains absent from the application entry point.
Studio consumes its frame contract from the defining module; CLI resolves names through the canonical
CLI registry, while MCP and Studio consume their owner registries directly.

The Track B binding, measurement, and release runtime callers share bounded diagnostic and
plain-object primitives through `trackb-application-primitives.js`. Measurement and release retain
their own error-code allowlists and fallback wording; runtime support no longer depends on the
binding module for generic message handling.

The derived `TRACK_B_HANDLERS` command-name map has been removed. It duplicated
`APPLICATION_COMMAND_REGISTRY` plus `TRACK_B_HANDLER_IMPLEMENTATIONS` and had no product caller;
white-box tests resolve the declared implementation for a command or call the single measurement
implementation directly.

## Preserved invariants

- A source receipt, handoff, HTML, provenance, measured-evidence receipt, and SceneGraph must all be present as the exact bounded input set. There is no recursive discovery or silent artifact inference.
- `buildTrackBReleaseCandidate` remains in-memory. Candidate creation is not approval or promotion.
- Each command-specific runtime module obtains project state and writes only through the custody session. Studio Human approval and pointer promotion remain separate operations.
- Meaning, source provenance, visual evidence, and durable release state are compared by their declared owner-bound hashes; this package does not become another narrative or renderer authority.

## Current refactor status

Binding/candidate construction and runtime registry/artifact/custody/approval/delivery duties have
separate internal owners. The Application index keeps only command-facing entrypoints and version
metadata; release operations remain direct to service/operation owners. The redundant
`trackb-release-runtime.js` forwarding module and the unconsumed path-based candidate workflow/file
reader have been removed. The SAC handoff loader remains split into a composition facade,
producer-claim contract, and filesystem/schema/MIME adapter; its registry loading is lazy to the
handoff operation. The application primitive consolidation and earlier verification evidence are
recorded in the current root report. These clean breaks are source-only and unverified. Packaged
native custody execution and the app-to-helper release cycle remain separate gaps.

Application public config contains repository/project roots only. `inputPaths` is not a launch
option and is explicitly rejected; protected paths are derived from verified handoff artifacts by
the HTML command owner. CLI/Studio/MCP callers no longer supply an ignored empty list. This
intentional config clean break is source-only and NOT_RUN.
