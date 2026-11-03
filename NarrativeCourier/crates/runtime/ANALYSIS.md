# NVC Runtime — 구현 분석

## Verdict·책임 경계

domain-neutral path/document/file/process effect primitives. caller는 STS/Delivery → runtime paths/store/process helpers. 방향 **IMPROVE**. reachable wiring 확신도 높음, 실환경 검증 확신도 낮음(NOT_RUN).

상위 제품의 정체성·공개 계약을 변경하지 않는 material unit이다. 이 문서는 해당 state/I/O 경계를 소유하고 상위 ANALYSIS는 통합 위치만 요약한다.

## 공개 surface·입출력·호출·활성 조건

crate root owns the domain-neutral `Error`/`Result` return contract. Effect functions and public
values are accessed through their owning `document`, `paths`, `process`, `store`, or `token` module;
the former root aliases for `extract_document_text` and `RuntimePaths` were removed. Callers now use
`document::extract_document_text` and `paths::RuntimePaths` directly.

`paths` owns safe relative-path/segment grammar and RuntimePaths project/domain composition;
`store` owns locks, hashing, and durable file operations. The project-name and generic path-segment
checks share one ASCII grammar while keeping their distinct length limits and error messages.
`store::open_lock_file` owns common parent creation/open options for workflow and agent-run lock
acquisition. `agent_run_is_active` observes only an existing lock file, so a status/cancel probe does
not create a project directory; blocking acquisition, nonblocking acquisition, and active-state
observation stay with their respective operations.

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| 공개 호출·상위 composition에서 활성 | STS/Delivery → runtime store/process helpers | safe path/file bytes, lock, command stdin/logs/cancel marker; Result | 아래 owner 구분 | 정상 result 또는 명시적 오류 | `src/store.rs::atomic_write/atomic_publish_file; src/process.rs::run_piped_command` |

## 실제 흐름

caller validates domain → runtime lock/private files → process group + writer thread → poll timeout/cancel → terminate/reap → result → domain caller

## 현재 state·contract·I/O owner

file descriptors, temporary paths, process group와 lock만 소유. source semantics·node status는 소유하지 않는다. max time/log policy와 process configuration은 caller의 명시 입력.

## 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| 위 공개 surface | REACHABLE | PARTIAL | NOT_RUN: Rust/실제 provider/제품 runtime; 정적 호출만 확인 | `src/store.rs::atomic_write/atomic_publish_file; src/process.rs::run_piped_command` |
| 독립 호출 library 확장 | PUBLIC_LIBRARY | UNKNOWN | NOT_RUN: 외부 caller 미조사 | public exports/상위 composition |

## 의존성·findings

**NR1.** 문제 → reaped leader 뒤 descendant의 열린 stdin 때문에 scoped join이 bounded wait 밖에서 멈출 수 있었다. 근거 → `src/process.rs::run_piped_command/terminate_process_tree`. 원인 → leader 종료와 group I/O 종료를 동일시. 영향 → timeout/cancel hang. 방향 → stdin channel 완료까지 deadline/cancel을 유지하고 group liveness로 TERM/KILL escalation한다. stdin 오류/자식 stdin 부재도 cleanup 경로로 보냈다. 새 Unix 회귀는 NOT_RUN이다.

**NR2.** 문제 → capture helper는 stdout/stderr 전체 파일을 read하며 호출별 output quota가 없다. 근거 → `run_piped_command_capture`. 원인 → bounded time과 bounded output을 분리하지 않음. 영향 → hostile 출력의 disk/memory 압박. 방향 → public 출력 필요량을 확인한 뒤 effect policy 확장; 이번 변경에서 임의 한도를 넣지 않았다. atomic helper의 directory sync 실패 처리도 실환경 fault 검증이 남는다.

## 실패·취소·재시도·복구와 남은 gap

fs2 lock은 운영체제 lock이며 자동 polling expiry가 아니다. nonUnix process path는 명시 unsupported. document extraction의 모든 형식/악성 입력 fuzz와 real ffmpeg/browser는 미검증.

## 판단·검증 범위

이 문서는 도달 가능한 소스와 caller를 기준으로 Current를 기록한다. `SATISFIED`는 명시한 정적 계약 범위의 충족이며 실제 제품 환경 검증을 뜻하지 않는다. 검사 결과는 별도로 표시한다. 실행용 Markdown·fixture·schema·vendor는 이름만으로 제거하지 않는다. 상위 요약과 남은 조치는 각 제품의 `docs/IMPLEMENTATION_STATUS.md`, `docs/PLAN.md`가 소유한다.

[제품 Current](../../docs/IMPLEMENTATION_STATUS.md) · [남은 PLAN](../../docs/PLAN.md)

## Process deadline 파트 — 소스 정리 완료, 실행 미검증

`run_piped_command`가 만든 하나의 시작 시각을 child wait와 stdin delivery wait에 전달한다.
기존에는 child waiter가 별도의 시작 시각을 만들어 같은 실행의 timeout 기준이 둘이었다.
Cancellation-file 해석, Unix process-group TERM/KILL, direct-child wait, stdin task 종료와
capture cleanup은 같은 runtime owner에 유지한다. 새로운 retry/fallback/성공 판정을 추가하지
않았으며 tests/build/lint/runtime은 NOT_RUN이다.

## Runtime document extraction — source-only closure

Document extraction captures one held regular non-symlink file with metadata/actual read bounds and
size consistency. MD/TXT remain exact UTF-8 strings instead of blanket newline normalization. PDF
extraction consumes those captured bytes through the existing library memory entrypoint, eliminating
a second path open. The input limit is 25 MiB; extracted text and DOCX XML publication are bounded
at 64 MiB. Empty, unsupported, invalid UTF-8 and changed-size inputs fail explicitly. The final PDF
output cap does not qualify the third-party decoder's peak allocation or CPU behavior.

DOCX has a private body extraction owner over the captured archive bytes. Exactly one word/document.xml
part is required; reported and actual uncompressed XML sizes are bounded. Namespace-aware parsing
accepts the standard/strict WordprocessingML document and one direct body. Only body text elements
contribute content; tabs, line breaks and paragraph boundaries remain explicit. Text/CDATA/entities
preserve decoded source content, while invalid numeric/named entities, DTD declarations, undeclared
prefixes, invalid XML 1.0 text, incomplete structure and tracked revisions/embedded body alternatives
fail rather than disappearing into a partial successful projection. This is body-text extraction, not
full DOCX layout, annotations/footnotes, or semantic approval. The existing entity fixture now uses the
real producer namespace; no tests were added or executed.

STS reference packet and Tauri synopsis ingress consume the same public entrypoint; Delivery PDF
admission retains extraction failures as explicit preview-required warnings. Durable references,
Narrative authority and original file hashes remain caller-owned. No source document/user project
was extracted or mutated during this work. Hostile filesystem replacement and document fidelity are
unqualified. Tests/build/fmt/lint/PDF/DOCX/runtime and source-manifest qualification are **NOT_RUN**.

## Runtime paths/project-root composition — source-only closure

Runtime owns one platform-independent relative wire-path grammar: forward-slash normal components
only, no backslash/NUL/colon, empty/dot/parent components, trailing dot/space aliases, control/forbidden
filename characters, or reserved device names. Bounded ASCII project/path-segment IDs use the same
portable identity rule. Domain path composition consumes that grammar rather than a separate native
components implementation; domain paths are declared UTF-8. Absolute OS roots remain PathBuf values
and can retain non-UTF-8 filesystem names. No canonicalization or physical containment claim is made
by this lexical grammar; symlink/regular-file/containment checks remain with actual I/O owners.

RuntimePaths refuses empty roots, captures one CWD for relative root anchoring, and rejects roots
that remain drive-relative. STS Engine::new/with_roots now return Result instead of attributing every
root failure to an infallible static domain path. CLI/MCP/Tauri propagate the error through their
existing boundaries. Existing STS/MCP fixtures consume the fallible constructors; no fixture/test
was run. STS path_parts and Delivery storage remove duplicate lexical validation while retaining
component-by-component filesystem admission. Producer schema bytes and integration pins are unchanged;
this is stronger runtime path admission, without migration or a legacy alias branch.

User roots/projects and environment were not read, created, normalized on disk, or removed. Tests/
build/fmt/lint/platform path/runtime and source-manifest qualification are **NOT_RUN**.

## Runtime durable storage/publication/lock primitives — source-only closure

Store remains the stable public facade over private digest, publication and lock owners. Digesting
holds one regular non-symlink source, checks opened identity on Unix, and refuses growth/size changes
instead of returning an unqualified mixed-size hash. Raw byte digest and timestamp entrypoints remain
unchanged. Generic primitives carry no STS/Delivery DTOs or approval state.

Atomic writes use exclusive process/clock/checked-sequence staging with native filenames; no filename
placeholder or pre-epoch default is used. The stage guard is acquired only after create_new succeeds,
normal failures combine write and owned cleanup outcomes, and Drop is a best-effort unwind backstop.
Canonical targets must be regular or absent. Staged publication syncs a held regular source, checks
Unix source identity and source/target file aliasing, then syncs destination and distinct source parent
directories after rename. Post-rename sync failure explicitly reports that publication occurred with
unconfirmed durability; it never becomes success or claims rollback. Caller-owned staged sources are
not deleted on failed publication. Unsupported Windows publication fails before any I/O or staging.

Locks require an existing regular project/control directory and regular non-symlink markers. Acquiring
a lock no longer creates a phantom project directory. Active-run observation never creates marker or
root, treats only genuine absence as inactive, and preserves other errors. Unix opened-marker identity
is checked; RAII file closure remains the lock lifetime owner. Existing domain callers already create
their project/control directories explicitly. Source guards now read actual child owners and Windows
pre-effect failure branches rather than mistaking owned stage cleanup for canonical deletion; none
were executed. Public import paths, schema bytes and registered integration pins remain unchanged.

User roots/files/locks were not operated on during this work. Same-size concurrent mutation, hostile
namespace replacement, ancestor-directory creation durability, crash recovery and multi-file domain
transactions are not qualified by these primitives. Tests/build/fmt/lint/storage/platform/crash/race/
runtime and source-manifest qualification are **NOT_RUN**.

## Runtime token/JCS identity grammar — source-only closure

Runtime uses one canonical hash entrypoint; the separate hash_json serialization hash is retired
without a compatibility alias. The schema-adapted integer profile rejects serialized floating
representations and integers outside ±(2^53-1) instead of allowing cross-language rounding. UTF-16
property ordering, string bytes and valid in-profile canonical hashes are preserved. The profile is
not arbitrary RFC-number support, a wire parser, schema validation, or input-adaptation certification:
custom serializer normalization/duplicate source fields remain caller-owned. Numeric design rationale
follows [RFC 8785 input/number rules](https://www.rfc-editor.org/info/rfc8785/).

STS owns typed task/gate v2 identity payloads rather than unstructured BTreeMap<Value> assembly. Both
bind domain identity, source snapshot, full node configuration and current version. Gate identity also
binds revision count, current review actions and source digest; decision admission consumes the same
review-actions owner. Task file identity uses admitted project/assets UTF-8 relative namespaces, with
canonical roots captured once per token, not lossy presentation paths or an absolute-path fallback.
Missing prompts/state and file I/O errors remain errors; input PathBuf clones/redundant is_file checks
are removed. Display text remains a separate presentation concern. Prior task/gate tokens become
stale; clients must retrieve the current packet/token, and active work is not grandfathered into
approval. Persisted source/state and five durable contracts are not migrated or merged.

The Python SceneMap identity consumer applies the same safe-integer profile and rejects non-string
object keys instead of coercing them into aliases. Existing Runtime fixtures consume hash_jcs; their
source setup/names are updated but no test was run. Producer schema bytes/pins remain unchanged.
Standards reading was analysis only; no canonicalizer, task, gate, project or provider was executed.
Tests/build/fmt/lint/RFC vectors/cross-language/runtime and source-manifest qualification are **NOT_RUN**.

## Runtime process stdin/output/cancellation lifecycle — source-only closure

The stable process facade delegates Unix lifecycle/capture to separate private owners and refuses
non-Unix calls before any capture I/O. Runtime now uses the already-locked libc package directly for
Unix ABI constants, checked process-group IDs and fcntl nonblocking stdin. One execution loop owns
stdin progress, leader observation, timeout, cancellation and capture monitoring; the borrowed blocking
writer thread/scope join is retired. Progress drains without an artificial sleep per write; only idle
iterations wait. Input completion closes stdin, and group liveness is checked before accepting a reaped
leader. Remaining owned-group descendants are explicit failure, followed by termination/reap.

Child custody covers error/unwind cleanup. Signal, wait and cleanup errors remain explicit unknown
outcomes instead of being discarded into success. Cancellation marker reads are bounded, regular,
non-symlink and UTF-8; only missing marker/root is absence. Positive timeout and bounded run ID are
admitted before launch. Capture uses exclusive 0700 directories/0600 files, checked clock/sequence,
live 8 MiB per-stream budget monitoring and bounded captured reads. Monitoring can overshoot between
polls; it is not a hard kernel disk quota. Normal cleanup outcomes are combined with process outcomes,
and completed-process cleanup failure is still an error. Renderer execution uses this private capture
path to avoid leaking child stdout into CLI/MCP protocol output and retains failure stderr.

Cargo/lock metadata names the existing libc dependency; no package installation or Cargo command was
run. Existing lifecycle fixture setup and source guards consume the new owners/marker error boundary
but were not executed. Public call signatures, state ownership and schema pins are unchanged. Escaped
process sessions, PID reuse, custom pre-exec/spawn latency and OS-uninterruptible reaping are outside
this owned-group qualification; raw caller-provided stdout/stderr sinks remain caller-owned. No process,
signal, marker, provider, test or user project was operated on during this work. Tests/build/fmt/lint/
process/cancellation/performance/runtime and source-manifest qualification are **NOT_RUN**.
