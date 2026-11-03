# Changelog

## Unreleased

- Added screenplay export (`sts export-fountain|export-fdx`, MCP `export_screenplay`): the COMPLETED canonical screenplay round-trips into Fountain 1.1 (force-marked `.SCN-` headings and `@` cues so Korean cast names survive uppercase heuristics) and a minimal Final Draft `.fdx`. Deterministic bytes; golden tests assert cross-format structural parity and the Completed-only gate. The MCP tool anchors exports inside the project output directory (bare filenames only).
- Documented deliberate non-feature: scene dependency DAG / character relationship graphs stay out until a machine-readable SceneIR relation contract exists — declared dependency fields live only in agent-authored step_06 markdown, and parsing freeform narrative prose would constitute Narrative interpretation.
- Added interactive scene graph viewer for JSON graph deliveries: `ui/graph-viewer.js` renders the deterministic graph projection as layered SVG (zero dependencies, CSP-safe, deterministic depth layout) with pan/zoom, node selection showing canonical content, and a Raw JSON toggle. Layout logic is mechanically asserted in `verify-all.sh` via `ui/graph-viewer.test.js`.
- Added screenplay ingestion (`sts import`, MCP `import_screenplay`): deterministic Fountain 1.1 subset and Final Draft `.fdx` parsing in `crates/sts/src/fountain/import/{fountain,fdx}.rs` — format-level normalization only. Imported screenplays land as project references (`input/references/*.txt`) with structural summary (scenes, INT/EXT, cast); no narrative IR is fabricated and the synopsis remains a required human-approved input. Golden tests cover cross-format parity (fountain ↔ fdx), boneyard/notes exclusion, duplicate rejection and the empty-`output/` authority guard.
- Added `crates/mcp` (`narrative-courier-mcp`): MCP stdio server exposing read + mechanical-delivery tools (`list_projects`, `project_status`, `doctor`, `delivery_context`, `list_deliveries`, `run_delivery`, `cancel_delivery`) over the canonical engines, with explicit tool annotations. STS gate decisions and delivery review are intentionally not exposed; approval stays human-only via Tauri UI / gated CLI. Protocol tests cover handshake, tools/list annotations policy, a full HTML delivery e2e for a released project and the invalid-request error path.
- Fixed STS integration-test fixtures to satisfy manifest-required domain markers (`step_00` `## Adaptation Preflight`; `step_06` `Necessary function`/`Cinematic value`); all 10 workflow tests pass.
- Made `tools/verify_delivery_source.py` needles whitespace-insensitive (rustfmt-proof, semantics unchanged) and switched the `asset_inputs` check to semantic consumption matching; fixed `tools/verify_contract.py` to read the rustfmt-wrapped `SOURCE_FORMATS` array.
- Applied `cargo fmt --workspace`; resolved all clippy warnings under `-D warnings` (explicit `.truncate(false)` on lock files, `div_ceil`, slice `contains`, merged identical branches, needless `return`, test-module placement).
- Restored `src-tauri/icons/` via the official `tauri icon` CLI and pinned `bundle.icon` in `tauri.conf.json`; added `app-icon.png` source for regeneration. Cold `cargo check` no longer fails on the missing default window icon.

## 0.12.1 — Asset/Reuse/Review/Process Closure

- Added optional provenance-bound `DeliveryRequest.asset_inputs[]` for USER/DERIVED assets and deterministic promotion into `DirectorPlan.asset_requests[]` / `AssetIndex`.
- Mapped asset inputs to explicit `SCN-*` units only; no new Narrative interpretation was added.
- Embedded resolved raster USER/DERIVED assets in HTML/PDF delivery while keeping active/vector content non-executable.
- Changed render identity to output-local `DeliveryPlan + relevant assets + backend fingerprint + renderer version`, enabling safe cross-request reuse of unaffected outputs after technical revalidation.
- Added Human `NEEDS_REVIEW → PASS|BLOCKED` lifecycle through existing `DeliverySet` verification layers, exposed in Tauri UI and CLI without a sixth durable contract.
- Moved Codex readiness probes onto the same bounded Runtime process primitive as execution and made stdin delivery concurrent with timeout/cancel observation.
- Added Rust regression tests for blocked-stdin timeout and the asset→render→review vertical slice; execution remains `NOT_RUN` when the pinned Rust toolchain is unavailable.
- Added WEBP to the existing Deliveries UI selector.

## 0.12.0 — STS + Loss-Controlled Delivery

- Added released `NarrativePackage` as the only STS→Delivery semantic boundary.
- Added the five canonical shared DTOs and executable cross-document invariants.
- Added deterministic exact-`SCN-*` DirectorPlan for `PRESENTATION` + `FULL|SELECTION|EXCERPT`; unsupported semantic reinterpretation fails explicitly.
- Added descriptor-driven HTML/JSON/PNG/WEBP/PDF/MP4 routing without a separate capability registry.
- Added request-scoped immutable contract chains under `visual/requests/<request_id>/`, shared asset blobs, and content-addressed delivery artifacts.
- Added semantic preflight, persisted hash-chain verification, asset/artifact revalidation, path-segment traversal protection, stale-package rejection, idempotent request identity, and Runtime-owned artifact publication.
- Linearized cancellation vs final `DeliverySet` publication using one atomic active-lease race.
- Added one Deliveries UI surface and matching CLI inspect/run/list/cancel commands.
- Added external renderer smoke for HTML/PDF/PNG/WEBP and exact 6-frame MP4.
- Rust `cargo check/test/clippy` remains a separate required release gate and is not claimed when the toolchain is unavailable.
- Hardened incomplete-run recovery: orphan render bytes are regenerated, verified asset blobs self-repair from hash-checked provenance, and final artifact/asset hashes are rechecked before DeliverySet commit.
- Hardened verification: HTML/JSON semantic PASS is proven from artifact bytes, DeliverySet top-level status must exactly aggregate child statuses, and aspect-ratio overflow is rejected.

## 0.11.1-phase1-hardened

- Unified all NarrativeCourier/STS package licensing under MIT and added an explicit `LICENSE`.
- Moved STS-owned `Event`, `GenerationPolicy`, `DomainIdentity`, and `FileFingerprint` out of the shared runtime crate.
- Kept the shared runtime limited to reusable path/hash/lock/atomic-write/document/process/token primitives.
- Bundled Blueprint v1.2 contract/invariant hardening as the next target specification.
- Made unsupported Windows atomic replacement/process-tree behavior fail explicitly instead of weakening canonical publish/cancel invariants.
- Added a single verification entrypoint that reports Rust verification as `NOT_RUN` when the toolchain is absent rather than claiming PASS.

## 0.11.0 — Runtime Phase 1

- Removed legacy `crates/workflow-core` trunk crate.
- Added `crates/runtime` for domain-neutral path, lock, hash, atomic-write, and document-ingestion primitives.
- Added `crates/sts` as the unchanged STS domain facade/orchestrator.
- Rewired Tauri/CLI dependency from `screenplay-workflow` to `sts-workflow`.
- Preserved STS 4.2 workflow, prompts, schemas, 154-story regression evidence, audit pipeline, screenplay export, and Codex policy.
- Updated static contract verification to assert runtime domain neutrality and the new workspace boundary.
- Added the canonical STS + DirectorIR implementation blueprint under `docs/blueprint/`.

Rust compile/runtime: environment-dependent and not claimed when unavailable.
