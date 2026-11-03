# Changelog

## Unreleased

- Removed the old provider-binary filename input alias. The upstream Cargo target name may remain
  vergerail-upagent-provider, but packaging must place those pinned bytes at the canonical
  scene-provider-v2 path; runtime identity checks accept only that name. VERGERAIL_* environment
  values and the upstream runtime-verification flag remain because they are the provider ABI.

## 0.13.0

- Renamed the active bundle provider executable to `scene-provider-v2`; the
  upstream Cargo target remains `vergerail-upagent-provider` and is copied to
  the canonical package filename.
- Locked the provider boundary to `misegen.provider/2` and the standalone
  `perfectpixel.asset-inspection/1` contract, with no v1 fallback.
- Kept provider v1 responses terminal `UNKNOWN`, with no fallback or retry, and
  retained process-group teardown/no-survivor evidence.
- Added bounded PerfectPixel schema framing and source-manifest pinning, with
  regressions for pretty JSON, concatenated documents, trailing data, and
  duplicate keys.
- Added actual provider/PerfectPixel preflight sealing and deterministic 0.13
  source and bundle evidence.

## 0.12.0

- Clean-break renamed the product, skill, CLI, schema directory, environment prefix, and hidden run root to `SceneAssetCompiler`, `$scene-asset-compiler`, `scene_asset_compiler.py`, `SCENE_ASSET_COMPILER_`, and `.scene-assets`.
- Locked the typed image provider to strict v2 / `schemaVersion: 2`; v1 responses are terminal `UNKNOWN` with no fallback or retry.
- Sealed provider and PerfectPixel executable SHA-256/protocol identities in preflight and added fake/packaged protocol probes.
- Imported only successful PNG PerfectPixel read-only inspection evidence after validating the actual PNG dimensions, alpha, byte count, and input hash; stale or mismatched evidence is terminal.
- Added a reproducible standalone bundle containing the canonical product and pinned runtime components.

## 0.11.0

- Added a positive per-edge transparent fit invariant for reusable character/prop outputs, with an exact output-canvas pixel margin in prompts and canvas-fit-only repair instructions; mixed margin/content failures now fail closed before repair creation.
- Added wireframe source/output-hash-bound slot layout evidence, including separate source/output label states, observed rotation, and z-order; evidence is mandatory even when the label/guide policy is omitted and defaults to `PRESERVE_IF_PRESENT`, while legacy non-layout v0.9 reviews remain valid.
- Re-hash every result artifact at approval, bind wireframe review evidence to exactly one current PRIMARY, and fail closed when an approved selection later goes missing or drifts; publication retains its independent re-hash gate.
- Added one bounded PSD-only packaging retry for `PREFERRED_OUTPUT_MISSING`; its execution receipt exactly mirrors the attempt lock, preservation binds exactly one PRIMARY PSD, and acceptance requires a `SUBMITTER_ATTESTED`, hash-bound submitted comparison attestation with declared zero-delta pixel/alpha metrics. Mechanical acceptance checks internal consistency and file binding, not authenticated provenance or independently reproduced comparison. A second missing PSD stops without redraw or local assembly and retains both attempts' previews/evidence.
- Added immutable 2026-08-26 live transparent-prop evidence: the genuine-alpha original remains PARTIAL at the 64 px threshold, and two RGB checkerboard canvas-fit edits are rejected and never promoted.

## 0.10.0

- Added the optional `render_contract` for orthogonal capture, genre, semantic-role, and wireframe layout constraints.
- Added deterministic photoreal, Japanese manga, fantasy, picture-book, background, and object prompt lines with strict style-reference boundaries.
- Added wireframe `LAYOUT_LOCK` and independent per-slot `PLACEHOLDER_PHOTO` GenerationUnits before existing semantic-layer and external PSD packaging stages.
- Added conditional `genre_fidelity` and `layout_fidelity` review gates while preserving legacy v0.9 ten-axis reviews.
- Added render-contract schema/example, focused regression coverage, and reference documentation.
- Repaired legacy ten-axis JSON Schema parity, prompt-profile/render-contract conflict enforcement, slot-ID and nonblank schema parity, and structured STYLE preserve-axis safety.
- Made emitted layout/placeholder scene units validate against the public GenerationUnit schema and added immutable `LAYOUT_LOCK`, per-slot `PLACEHOLDER_LOCK`, aggregate `PLACEHOLDER_SET_LOCK`, asset-seal binding, and PSD-entry drift checks.
- Clarified that `PREFERRED_OUTPUT_MISSING` is an invocation-specific PSD result, not live-success proof or a global unsupported declaration.

## 0.9.0

- Added immutable generation execution receipts binding the submitted prompt, compiled capsule, references/edit target, requested contract, and returned output hashes.
- Added executable photoreal scene, character-anchor, and layer-asset prompt profiles plus a mandatory photorealism review axis.
- Replaced PNG header-only checks with CRC/filter/pixel decoding, real-alpha proof, exact canvas validation, opaque-core checks, alpha bounds, clipping detection, and a 2% reusable-asset safe-margin default.
- Routed PSD tasks through strict postflight: canvas, ICC header, required layers/bounds, path-record sequencing, saved/clipping-path policy, and explicit render-back status.
- Bound every scene lock to a reviewed artifact hash and revalidated all artifact/review receipts at asset sealing.
- Added field-tested repair guidance from observed visual/mechanical failures. The live transparency edit test caught a baked checkerboard; a separate live PSD request returned PNG and was recorded as an invocation failure, not as evidence that PSD generation is unsupported.
- Closed independent-review bypasses: indexed/16-bit alpha measurement, all-254 background rejection, attempt-local prompt evidence, task-bound photoreal review profiles, downstream scene-seal revalidation, exact scene canvas/format capsules, identifiable sRGB ICC checks, nonempty path structure, and native pixel-leaf layer checks.
- Expanded regression coverage to 71 tests after reviewer repairs, including compression-aware PSD channel payload validation.

## 0.8.0

- Clean break: GPT/Codex `$imagegen` is now the only PSD producer.
- Removed local cutout, matte, path tracing, PSD writing and deterministic PSD assembly from the canonical codebase.
- Added `PSDGenerationProfile` and prompt compiler rules for Saved Path / Work Path / document clipping path.
- Added a dedicated `PSD_PACKAGING` GenerationUnit after all scene layer locks.
- Scene PSD uses one Named Saved Path per cutout and no document clipping path.
- Single-asset HARD/ORGANIC PSD promotes the temporary Work Path to a Named Saved Path; that Saved Path may be the one document clipping path. No surviving Work Path copy is required.
- SOFT_ALPHA preserves RGBA alpha and forbids document clipping path.
- Default path-prompt tolerances: HARD 0.75 px, ORGANIC 1.5 px, SOFT_ALPHA 3.0 px, MIXED scene 2.0 px.
- Clipping flatness is left unset/Photoshop output-device default; v0.8 does not hard-code a numeric print Flatness.
- PSD parser/validator remains as independent postflight evidence.
