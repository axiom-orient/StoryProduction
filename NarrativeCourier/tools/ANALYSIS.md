# NarrativeCourier tools — 폴더 책임 분석

`tools/` is a set of maintenance and optional authoring commands, not one runtime service. STS under
`crates/sts` remains the sole durable narrative authority; these tools validate, inspect, or publish
their own explicitly named derived artifacts. Static gate output does not promote a candidate or
replace a Human decision.

| Owner | Responsibility and effect |
|---|---|
| `verify_contract.py` | Schema/fixture/source-boundary CLI and diagnostic aggregation; read-only over project authority. |
| `tauri_wiring.py` | Pure lexical owner for UI invoke, `generate_handler!` registration, `#[tauri::command]` definition, and human gate-token field mapping. |
| `verify_narrative_scene_map_v2.py` + `scene_map_v2_validator.py` | Stable CLI/fixture harness and map-level occurrence/identity orchestration; no product-state writer or producer contract authority. |
| `scene_map_v2_contract.py`, `scene_map_v2_shape.py`, `scene_map_v2_identity.py` | Fixture-only bounded values, schema/shape rules, and integer-only JCS identity owner composed by the whole-map validator; raw SHA-256 delegates to `digest.py`. |
| `verify_delivery_source.py` | Delivery source/wiring guard composes Tauri/CLI consumers with backend types/discovery/routing, DeliveryEngine, and renderer source boundaries; raw schema-byte hashing delegates to `digest.py`. It does not own DeliveryRequest or DeliverySet state. |
| `sts_audit.py` | Stable audit CLI and three-stage orchestration. It owns workflow/resource hash framing, external Codex invocation and output-directory publication; raw bytes/file hashing belongs to `digest.py`. The screenplay reconstruction stage remains source-blind. |
| `sts_audit_inputs.py` | Bounded plain-text, Fountain, FDX, DOCX and PDF extraction; input-specific tests import this owner directly rather than the audit CLI. It produces text for the audit only; it does not rewrite source files. |
| `sts_audit_results.py` | Returned JSON schema, IR identifier/coverage, verdict-coherence checks and Markdown report projection. It does not create STS state or approval receipts. |
| `cinematic.py` + `cinematic_contracts.py` | One captured Completed-release snapshot for continuity facts plus explicitly authored cinematic-plan/approval artifacts. Native text-board/animatic authoring has its own NVC Rust capture boundary in `crates/authoring`; it does not import this Python module. |
| `digest.py` | NVC-tools-only lowercase raw SHA-256 for in-memory bytes and incrementally read files; no canonicalization, prefix, identity, or story-state policy. |
| `source_manifest.py` | NVC source-manifest generation or read-only verification composed from `digest.py`; no runtime story state. |
| `verify_geultu_handoff.sh` | Cross-component public-CLI regression; its embedded byte-integrity checks call the NVC raw digest owner instead of implementing SHA-256 inline. |
| `smoke_renderers.py` | Optional renderer-facing smoke; external renderer availability is a separate capability. |
| `test_sts_audit.py`, `verify-all.sh` | Regression cases and gate composition, not product runtime entrypoints. |

## Tauri command-wiring guard

The Tauri composition root registers module-qualified handlers from `src-tauri/src/commands/`.
`verify_contract.py` keeps CLI/report aggregation; `tauri_wiring.py` reads Rust sources under
`src-tauri/src/` and relates UI `invoke` names, `generate_handler!` registrations, and
`#[tauri::command]` definitions. Every UI invoke must be registered, and each registered command
must resolve to a definition. The token boundary check reads `gate_token` from the STS command owner
while retaining the UI's `gateToken` mapping. These lexical checks describe source wiring; they do
not qualify the native window or replace compilation/runtime evidence. This extraction is source-only
and has **NOT_RUN** tests/build/lint.

## SceneMap v2 fixture verifier

`verify_narrative_scene_map_v2.py` stays the maintenance CLI. It loads the NVC-owned schema and
positive example, then exercises a closed in-memory negative corpus. `scene_map_v2_contract.py`
owns bounded JSON value guards and immutable field sets; `scene_map_v2_shape.py` owns dialogue, scene,
locator, entry and schema-shape checks; `scene_map_v2_identity.py` owns the integer-only RFC 8785/JCS
subset and raw object identity hashes. `scene_map_v2_validator.py` composes action/dialogue occurrence
bijections, contiguous directive occurrence indexes, and whole-map identity checks, with `validate_map` as
its only maintained module entrypoint.
The CLI's only maintained public entry is `main`; it imports the few fixture helpers it needs directly
from their owners, while hash rebinding and negative-case builders stay private. The old helper-import
facade is retired. These modules only validate fixtures; they do not write project state, publish
producer contracts, or become STS semantic authority. The validator also enforces contiguous directive
occurrence indexes, and the negative corpus carries a rehashed gap case. Tests, build, lint, CLI/runtime,
`verify-all.sh`, and source-manifest verification were not run after these source-only edits.

## Existing-screenplay audit decomposition

`sts_audit.py` remains the stable CLI/composition facade. It owns workflow identity and prompt loading,
audit-bundle framing, the three isolated Codex process calls, audit output-directory lifecycle/cleanup, and
the source-blind stage ordering. Extraction/result helpers remain imported from their owner modules where
the CLI calls them; raw SHA helpers are accessed through private `_digest` and are not re-exported as a
facade compatibility surface.

`sts_audit_inputs.py` owns size-bounded byte-to-text extraction for UTF-8, Fountain, FDX, DOCX, and PDF.
It does not write to the input files or choose story semantics. `sts_audit_results.py` owns JSON result
schema checks, ID and complete-coverage invariants, verdict coherence, fixed audit dimensions, and
Markdown projection. Neither module creates STS state, review receipts, or cross-component DTOs. The
facade sends only the synopsis to the source stage, only the screenplay to the reconstruction stage,
and only the two IR values to the comparison stage; the moved owners do not change this information
boundary.

This split is source-only. No NVC tests, build, lint, CLI/runtime, or source-manifest verification was
run after the move. The source manifest is regenerated separately as metadata only.

## Cinematic authoring

### Verdict·책임 경계

**IMPROVE / 높은 확신도(구조·실제 CLI), 중간 확신도(전체 제품 통합).** 승인된 NarrativePackage/SceneMap으로부터 continuity 사실을 추출하고, 외부에서 명시적으로 작성한 영화 연출 초안을 검증·봉인한다. caller는 로컬 제작자 또는 초안 작성 도구다. 가치는 shot 결정권을 renderer에서 제거하고, scene→N shots와 각 semantic occurrence의 담당 shot을 고정하는 데 있다.

`tools/cinematic.py`는 manifest 없는 실제 Python entry다. Rust `crates/contracts::DirectorPlan`/`crates/delivery::build_plan`을 대체하지 않는다. 별도 제품·서비스·DB가 아니라 NVC 내부의 독립 분석 대상이다. Rust STS/desktop/MCP에 이 authoring CLI가 자동 등록된 것은 아니다. 기존 5개 delivery DTO 규칙과 신규 cinematic artifact의 관계는 [루트 ADR](../../docs/adr/0003-cinematic-authority.md)에 고정했다.

### 공개 surface·입출력·호출·활성 조건

| Surface·활성 조건 | Caller→Handler | Input·검증 | State/Effect owner | Output·Failure | 근거 |
|---|---|---|---|---|---|
| `continuity` / Completed release 필요 | operator → `main` → `load_completed_release` → `extract_continuity` | bundle/state/package/map raw hash, derived ID, semantic occurrence | NVC facts; immutable file publication | `ContinuityBible` v1 / REJECTED | `cinematic.py`, `cinematic_contracts.py` |
| `plan` / 명시적 draft 필요 | proposer/operator → `validate_plan` → `publish` | closed schema, source/bible hash, unique IDs, positive frames, scene/shot order, occurrence bijection | cinematic intent은 작성자와 NVC; 컴파일러는 결정을 발명하지 않음 | `CinematicDirectorPlan` v1 / REJECTED | `validate_plan`, `contracts/cinematic-director-plan-v1.schema.json`(제품 root 기준) |
| `approve` / 별도 operator action | reviewer → `validate_approval` → `publish` | `--reviewer`, `--expected-plan-hash`, exact current inputs | 승인 receipt 한 곳 | 승인 v1 / stale·reviewer conflict | `cinematic.py::main` |
| `inspect` / 읽기 전용 | operator → 재검증 | current source + 선택적 approval | 별도 상태 없음 | VALID + scene/shot 수 / REJECTED | `cinematic.py` |
| importable helper | 동일 NVC 도구/검사 → canonical/path/schema helpers | duplicate JSON key, UTF-8, integer domain, no symlink/traversal | 순수 계약 + 로컬 filesystem boundary | value/hash / `CinematicError` | `cinematic_contracts.py` |

### 실제 흐름

`Completed release → 원본 bytes와 semantic occurrences 검증 → deterministic continuity → 명시적 초안 → closed schema/semantic assignment 검증 → 새 plan 파일 → 명시적 review(expected hash) → 새 approval 파일 → SMC 네 입력 bundle`.

2 scenes / 5 shots fixture에서 NVC **실제 Python CLI**가 continuity/plan/approval을 만들고 SMC가 투영한다. 이 fixture의 narrative 입력·연출·reviewer는 검사 데이터이며 실제 STS 실행/사람의 미학적 승인 증거가 아니다. 코드 caller는 SMC `tests/cinematic-production.test.js`, native 전체 caller는 `integration/tests/story_pipeline_e2e.py`다.

### Current source-only slice — shared release and validation owners

`load_completed_release` returns one named internal snapshot with the verified package, SceneMap, state,
source binding, and exact raw release-bundle hash. The text-storyboard and cinematic callers consume it
without reopening bundle/state files. `validate_shape` caches each immutable producer schema validator and
selects the first path-ordered error without retaining the full error list. Approval retries reuse a valid
existing receipt without constructing or validating a second candidate. Producer schemas, approval meaning,
wire artifacts, and CLI commands remain unchanged. Tests/build/fmt/lint/CLI/runtime/source-manifest are
**NOT_RUN**.

The Python SceneMap consumer now also rejects gaps in repeated emotion, lighting-transition, and camera-movement
occurrence indices in the same validation order as the Rust contract owner. Entry hashes are checked after
source-locator/source-text binding; manifest and map identities are checked after coverage.

### 현재 state·contract·I/O owner

`contracts/*cinematic*`, `continuity-bible-v1`은 NVC 정본, SMC 파일은 byte-pinned consumer snapshot이다. `contentHash`는 자신을 제외한 정수 한정 canonical JSON hash, 파일 ref hash는 raw bytes다. 이 둘을 혼용하지 않는다. safe integer 범위와 UTF-16 key order를 Python/Node에서 맞춘다. 일반 부동소수점 JCS 구현이라는 주장은 하지 않는다.

Continuity entity ID는 kind와 원본 identity에서 파생한다. environment identity와 scene time은 다른 concern이며 같은 location의 시간 변화 때문에 environment를 중복 생성하지 않는다. 등장인물/장소/소품/scene time만 원본에서 추출하고 외모는 `UNRESOLVED_VISUAL`로 남긴다. wardrobe·prop possession·screen direction·eyeline·공간 지리·승인된 shot endpoint는 아직 이 schema의 상태 모델이 아니다.

plan/approval은 mutable current-pointer DB를 갖지 않는다. caller가 current revision 경로를 선택한다. 동일 경로의 동일 bytes 발행은 멱등, 다른 bytes는 거절한다. temp+fsync+exclusive link+directory fsync는 **단일 파일 게시**의 기법이며 여러 파일 전체 transaction이나 공격자 directory-swap sandbox는 아니다. 읽기 16 MiB 한도는 NVC I/O policy이고 SMC 소비 한도는 8 MiB다.

### 기능 현황

| 기능 | 연결 상태 | 계약 충족 상태 | 검증·적용 범위 | 근거 |
|---|---|---|---|---|
| deterministic continuity facts | REACHABLE | SATISFIED | PASS: 실제 NVC CLI→SMC fixture; 원본에 있는 사실 범위 | `extract_continuity`, SMC cinematic tests |
| explicit multi-shot plan·occurrence binding | REACHABLE | SATISFIED | PASS: 18개 cinematic 검사 안의 positive/negative·cross-language 경계 | `validate_plan`, SMC `cinematic-production.test.js` |
| plan review receipt·stale rejection | REACHABLE | PARTIAL | PASS: source/plan/reviewer binding; signer 인증 N/A(로컬 trust model) | `validate_approval`, `publish` |
| automated director proposal provider | ABSENT_IN_SCOPE | NOT_IMPLEMENTED | NOT_RUN: native agent/provider wiring 없음; 초안은 외부 입력 | `main --draft` 필수 |
| cinematic Rust/desktop/MCP 자동 흐름 | ABSENT_IN_SCOPE | NOT_IMPLEMENTED | NOT_RUN: 새로운 Python entry만 연결 | 기존 STS/delivery composition |
| rich temporal continuity | ABSENT_IN_SCOPE | NOT_IMPLEMENTED | NOT_RUN: wardrobe/spatial/endpoint schema 없음 | bible v1 원본 |

### 실패·취소·복구

preflight가 실패하면 새 artifact를 게시하지 않는다. 취소·timeout이 필요한 외부 generation을 이 도구는 수행하지 않는다. 따라서 provider job 상태머신을 여기에 발명하지 않는다. interrupted publication의 private temp가 남을 수 있으며 이를 승인으로 읽지 않는다. in-place plan 수정은 금지하고 새 경로/revision으로 다시 검토한다. reviewer 문자열은 local operator attestation이지 보안 서명 또는 사람 존재 증명이 아니다.

### Findings·gap·검증·근거

**NC1 — 잘못된 shot authority 제거.** 문제 → SMC가 scene당 4초 shot을 생성. 근거 → 원본 `buildSceneGraphFromNarrativeSceneMap`, 현재 신규 `validate_plan`/SMC exact projector. 원인 → narrative scene과 shot의 의미 혼동. 영향 → 길이·coverage·연출 손실. 방향 → **IMPROVE**, 승인 가능한 명시적 CDP를 NVC가 소유.

**NC2 — semantic text와 cinematic interpretation은 다르다.** 문제 → ref/hash가 맞아도 `subjectAction`의 자연어가 원래 의미와 다른지 결정적으로 알 수 없음. 근거 → 자유형 `composition/subjectAction/performanceBeat`와 exact semantic refs. 원인 → 구조 검증과 의미 평가의 차이. 영향 → 승인자가 검토하지 않은 의미 추가 위험. 방향 → 원문 occurrence 보존 + operator narrative review, model QC는 증거만 제공. 자동 의미 무결성 보장을 주장하지 않는다.

**NC3 — rich continuity 미완료.** 문제 → 현재 사실 추출과 승인된 시각 reference 사이 시간 상태 모델 부재. 근거 → `extract_continuity`의 entities/sceneStates. 원인 → 최소 P0 계약부터 구현. 영향 → 장편 wardrobe·prop·공간 연속성 자동 판정 불가. 방향 → [제품 PLAN](../docs/PLAN.md)의 source-backed state transition 설계 후 확장, 추론으로 시각 사실 확정 금지.

**NC4 — native authoring 자동 연결 미완료.** 문제 → 새 CLI가 존재해도 기존 STS UI에서 자동 실행되지 않음. 근거 → `src-tauri`/MCP handlers에는 신규 handler 없음. 원인 → 독립 도구 단계로 도입. 영향 → 운영자가 명시적으로 호출해야 함. 방향 → 자동 제안은 선택 기능으로, 기존 delivery planner에 넣지 않는다.

`cinematic_contracts.publish_bytes` is the immutable filesystem publication owner for cinematic JSON
and storyboard Markdown. `publish` only encodes JSON before calling it. Identical bytes are
idempotent, different bytes conflict, and temporary-file/fsync/link/cleanup policy has one owner.
The former audit CLI input re-exports and unused imported helpers are removed. NOT_RUN.
