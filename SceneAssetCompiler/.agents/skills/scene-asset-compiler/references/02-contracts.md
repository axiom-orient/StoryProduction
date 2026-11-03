# Contract Map

Authority flows only forward:

`RunRequest → VisualAuthority → GenerationPlan → task.json → attempt.json → result.json → reviews → selection.json → AssetPack`.

- **RunRequest** owns desired product/output/review policy.
- **VisualAuthority** owns project style and character identity invariants.
- **GenerationPlan** owns asset inventory, dependencies, split level, references, output expectations and acceptance rubric.
- **Task** is immutable after compilation.
- **Attempt** is immutable generation intent for one try.
- **Result** identifies actual files by SHA-256.
- **MechanicalReview** owns deterministic checks only.
- **VisualReview** owns perceptual/semantic judgment.
- **Selection** is the sole approved pointer for a task.
- **AssetPack** contains only selected artifacts.

Prompt prose is compiled from Task/Attempt and never becomes authority.

## SceneMotionCompiler boundary contracts

The generic `AssetPack` v1 and `asset-pack.json` emitted by `publish` are
standalone SceneAssetCompiler output. They are not accepted by SceneMotionCompiler.
Production scene delivery uses the producer-owned six schemas in
`docs/contracts/` and the fixed filenames under a run's `handoff/` directory:

| Contract | Producer responsibility |
|---|---|
| `scene-asset-release-request-v1` | Declare source/runtime files, four requirement types, and `kind`/`rights`/`sourceRegionId` for every asset and binding. |
| `scene-asset-pack-v2` | Deterministic normalized asset and requirement package bound to the imported NVC projection. |
| `scene-asset-pack-audit-v1` | Complete PASS evidence for every source, runtime derivative, and requirement binding. |
| `binding-manifest-v1` | Repeat the exact requirement-to-asset metadata bindings consumed downstream. |
| `scene-asset-release-approval-v1` | Human APPROVED receipt whose subject has exactly four hashes: SceneGraph, pack, audit, and binding. |
| `scene-motion-asset-handoff-v2` | Final hash envelope consumed by `SceneMotionCompiler`, including all semantic projection pins. |

`kind` is a descriptive medium (`illustration`, `photo`, `texture`,
`product`, `icon`, or `vector-decoration`); it is not inferred from a file
extension. `rights.status` is one of `USER_OWNED`, `LICENSED`,
`PUBLIC_DOMAIN`, or `CC0`, with explicit holder and source evidence.
`sourceRegionId` is a stable producer-owned region identifier (use a full-frame
identifier when no crop exists). The downstream consumer preserves these
declarations and does not upgrade or infer them.

The imported NVC semantic manifest keeps its producer-owned `source_locator`
snake_case fields. The staged SceneGraph is the SMC canonical graph: its
`sourceLocator` uses `sceneId`, `field`, `occurrenceIndex`, and optional
`character`/`dialogueIndex`. The adapter preserves locator values exactly while
renaming only these keys; the handoff pins both the original source semantic
manifest hash and the normalized projected semantic manifest hash.

For the three scene-release contracts that carry `crop` (`SceneAssetReleaseRequest
v1`, `SceneAssetPack v2`, and `SceneAssetBindingManifest v1`), `crop` is a
source-artifact pixel rectangle with exactly `x`, `y`, `width`, and `height`.
All four values are JSON integers: `x` and `y` are zero-based coordinates with
minimum `0`; `width` and `height` are positive dimensions with minimum `1`.
Fractions, booleans, negative coordinates, and zero dimensions are invalid
before publication. This integer-only rule is part of the cross-runtime hash
boundary. The scene-release contract's canonical domain is objects, arrays,
strings, booleans, null, and integers; it is not a general RFC 8785/JCS
implementation.

### Trusted-local threat model

This boundary assumes one cooperative local writer owns a run directory at a
time. The producer and consumer re-check regular files, project-relative paths,
raw hashes, sizes, MIME declarations, and supported file signatures; a
concurrent or hostile local writer is outside that trust boundary. These checks
are integrity and lineage checks, not identity proof: cryptographic signatures,
key ownership, reviewer identity, and hostile-local-writer authenticity are
NOT PROVIDED. Human approval records the exact four-hash subject but does not
turn a trusted-local receipt into a signed authorization.

## Diagnostic contracts (0.5)

### OutputAudit
Authority: **observed evidence about actual returned pixels**, not generation intent.

Maps canonical input claims to:
- expected,
- actual,
- PASS / MINOR / MAJOR / NA,
- concrete visual evidence.

`UNKNOWN` blocks Doctor.

### DoctorReport
Authority: **diagnostic decision**, derived from completed OutputAudit + mechanical review + optional metric evidence.

Owns:
- gap vector by visual axis,
- LOCAL / REGIONAL / GLOBAL scope,
- stable gap signature,
- causal hypotheses with confidence,
- recommended recovery strategy,
- preserve/rewrite axes,
- diagnostic experiments.

Doctor hypotheses are not narrative or visual authority. They are disposable diagnostic state.

### ReconstructionPlan
Authority: **how the next attempt is allowed to differ**.

Owns:
- edit vs regeneration boundary,
- exact edit target when applicable,
- preserved axes,
- rewritten axes,
- reference policy,
- experiment plan.

A ReconstructionPlan never changes VisualAuthority. If the authority itself is wrong, stop and revise authority explicitly rather than hiding the change in reconstruction.

### MetricEvidence
Optional advisory evidence from external evaluators. It never becomes a universal quality score and never overrides a failed hard claim.
