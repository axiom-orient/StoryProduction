# Track B custody helper — module analysis

## Responsibility and authority

`smc-trackb-custody` is a bounded POSIX process/file-descriptor bridge for Track B release state. It accepts newline-delimited JSON requests and does not own narrative, visual, or human approval decisions. The release store remains the durable owner; this helper enforces descriptor-relative reads, lock ownership, compare-and-swap publication, and explicit uncertainty.

## Module ownership

| Module | Responsibility | Owned state or effect |
|---|---|---|
| `src/main.rs` | POSIX process entry, bounded JSON-lines loop, and non-POSIX refusal entry | Process lifecycle only; no session state or wire-record DTOs |
| `src/error.rs` | Typed input, storage, lock-recovery, and I/O error envelope | Custody failure code, message, retryability, and `Result` alias |
| `src/startup.rs` | Startup option parsing and purpose selection | `Config` and `SessionPurpose` before session creation |
| `src/protocol.rs` | Request-line bounds, protocol version, response framing, canonical JSON, hashes, UTC timestamps, and bounded base64url payload codec | Stable line-protocol primitives |
| `src/session.rs` | Session construction, operation dispatch, session-owned state, and ordered project-ingress sequence | `Session`, read ledgers, directory handles, and state initialization |
| `src/session/lifecycle.rs` | Initialize, lock acquisition, snapshots, ordinary reads, immutable writes, and close | Narrative-session lifecycle and write bound |
| `src/session/project_ingress.rs` | Ordered project-artifact reads before narrative initialization | Project-ingress admission and read sequence |
| `src/session/measurement.rs` | Fixed-order measurement reads and bounded receipt publication | Measurement-session sequence and kind-specific byte limits |
| `src/session/pointer.rs` | Strict pointer/DeliverySet validation, compare-and-swap publication, and reconciliation errors | Release-pointer effect and pointer byte bound |
| `src/session/storage.rs` | Session-local directory handles and descriptor-relative storage-parent resolution | Storage traversal within the retained project descriptor |
| `src/lock_release.rs` | Lock-record serialization, stale-owner proof, acquisition, and exact release | Lock record, lock handle, owner identity, probes, and lock bound |
| `src/release_records.rs` | Pointer and DeliverySet DTO decoding, hash binding, and CAS path derivation | Persisted release-record interpretation; no lock effects |
| `src/record_fields.rs` | Exact JSON-object key-set comparison | Pure field-set comparison shared by record owners |
| `src/process_identity.rs` | PID liveness and process-start identity probes, including Darwin `proc_pidinfo` | Owner classification distinguishing live, dead, reused, and unknown owners |
| `src/artifacts.rs` | Stable file snapshots, snapshot-token admission, byte bounds, and artifact-read/snapshot response projections | Snapshot DTO and bounded artifact bytes |
| `src/artifacts/path.rs` | Project artifact reads, measurement-receipt parent traversal, and one checked directory-edge opener | Absent project parents remain absent; receipt parents are created only under the explicit create policy |
| `src/filesystem.rs` | Private composition of filesystem child modules | No forwarding helpers or path policy |
| `src/filesystem/admission.rs` | Project-root admission, safe path grammar, request fields, opaque IDs, and fixed Track B storage namespace | Filesystem ingress rules, path/ID bounds, and canonical namespace segments |
| `src/filesystem/descriptors.rs` | Descriptor-relative open/fstatat/mkdir/link/rename/unlink/fsync, file-mode classification, and inode identity | POSIX descriptor operations and `FileIdentity` |
| `src/filesystem/publication.rs` | Owned-temporary identity checks, cleanup, and publication sequencing | Temporary-file effects and uncertain-residue preservation |
| `src/session/tests.rs` | Session-owned unit fixtures and transition tests | Exercises private lifecycle state beneath its Session owner; no production API exposes test access |

The binary root now only composes startup, session, error, and protocol owners. Persisted lock and release records live beside their parsers; `FileIdentity` lives with descriptor operations; `Snapshot` lives with artifact reads; and `Session` alone constructs and mutates session state. Consumers import the owner directly instead of relying on names defined in `main.rs`.

## Preserved contracts and effects

- Request operation names, JSON-lines fields, response error codes, byte bounds, protocol version, canonical hashes, and timestamp rules remain unchanged.
- Startup purpose selection continues to gate project ingress, measurement, and narrative operations independently.
- The admitted project descriptor remains the authority after startup. Later reads and writes do not reopen the startup pathname.
- Lock acquisition/release, inode checks, stale-owner proof, CAS order, fsync points, temporary cleanup, and uncertainty classification remain with their existing effect owners.
- Pointer and DeliverySet records remain strict closed DTOs. Candidate state does not become an approval or promoted release by parsing alone.
- The helper remains POSIX-only. Non-POSIX builds use the explicit refusal entrypoint rather than implying support.

## Validation boundary

No build, test, formatter, lint, runtime, or manifest-freshness check was run after this source-ownership refactor, following the user's instruction to continue analysis and improvement without verification. The source manifest was regenerated as package metadata only; that generation does not verify this module graph. Previous execution evidence applies to the prior source graph only.
