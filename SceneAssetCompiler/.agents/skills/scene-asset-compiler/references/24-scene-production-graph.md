# Scene Production Graph — One Scene = Many Generation Units

## Core decision

A scene is not one image-generation task. It is a graph of decisions and assets.

```text
Scene VisualBrief
  ↓
Composition candidates (3–5 typical)
  ↓ COMPOSITION_LOCK
Integrated target candidates (1–3 typical)
  ↓ TARGET_LOCK
Optional wireframe layout
  ↓ LAYOUT_LOCK
One PLACEHOLDER_PHOTO per slot
  ↓ one PLACEHOLDER_LOCK per slot
  ↓ PLACEHOLDER_SET_LOCK
SceneCoordinateContract
  ↓ COORDINATE_LOCK
Independent semantic asset candidates
  ├─ background/environment
  ├─ each character
  ├─ each prop
  ├─ foreground occluders
  ├─ effects/emission
  └─ shadow/light helpers
  ↓ one LAYER_LOCK per required layer
GPT `$imagegen` PSD_PACKAGING unit
  ↓ returned editable PSD
  ↓ structural/path validation + COMPOSITE REVIEW / DOCTOR
Targeted layer edit/regeneration
  ↓
DELIVERY_LOCK
```

A `GenerationUnit` is exactly one image-executor invocation. A complex scene may use 20–40+ units. The count is a search ceiling, not a quota: stop as soon as the locks pass.

## Why this boundary

Modern multi-reference generators still fail through subject omission, identity degradation, attribute swapping, dominance and blending as subject/reference count grows. SceneAssetCompiler therefore reduces simultaneous uncertainty instead of asking one call to solve identity, composition, environment, lighting, props and layer separation at once.

## Lock ladder

1. `COMPOSITION_LOCK` — blocking, framing, camera and first-read are approved.
2. `TARGET_LOCK` — the integrated scene is visually approved.
3. `LAYOUT_LOCK` — optional wireframe slot geometry is approved.
4. `PLACEHOLDER_LOCK.<slot_id>` then `PLACEHOLDER_SET_LOCK` — every optional slot photo is independently approved and sealed.
5. `COORDINATE_LOCK` — canvas and semantic layer order are frozen.
6. `LAYER_LOCK.<layer_id>` then `ASSET_LOCK` — every required semantic layer is independently approved and sealed.
7. `COMPOSITE_LOCK` — returned GPT-generated PSD passes structure/path policy and target-vs-PSD visual review.
8. `DELIVERY_LOCK` — PNG/PSD package is immutable.

A downstream failure reopens the earliest responsible lock only. A hand defect does not reopen composition. A wrong camera does.

Every lock command requires a review receipt whose `artifact_sha256` matches the artifact at lock time. Sealing rehashes the composition, target, coordinate, every layer artifact, and every review receipt. A filename, prior review, or existing lock record alone is never proof that current bytes were approved.

Every downstream lock revalidates prerequisite lock schema, scene identity, local path containment, artifact/review hashes, and the complete placeholder/asset seals. Layout scenes cannot lock semantic layers, seal assets, or enter PSD packaging without current `LAYOUT_LOCK` and `PLACEHOLDER_SET_LOCK` evidence. Scene generation capsules carry the exact numeric canvas and output format; layer capsules also state opaque-backplate versus genuine-alpha behavior and prohibit trimming/rescaling locked registration.

## Composition exploration

Prefer 3–5 genuinely different hypotheses. Variation priority:

1. blocking/action
2. camera position
3. shot size/framing
4. lighting

Do not waste four calls on nearly identical lighting variants.

`ABSTRACT_BLOCKING` is the default when several recurring characters are present: solve spatial composition before identity binding. Use `IDENTITY_AWARE` only when silhouette/appearance materially determines blocking.

## Integrated target

The approved composition becomes a `COMPOSITION` reference. Add only the minimum approved identity/prop/style anchors needed for the scene. Generate 1–3 target candidates. Once one passes, freeze it.

The target render is a visual authority for scene placement and harmony. It is **not** the canonical source pixels for PSD layers.

## PSD assetization

After target lock, generate each required semantic layer independently. Each layer call receives:

- approved target as `COMPOSITION`/`POSE` reference,
- owner identity/prop anchor only when needed,
- scene light/color contract,
- semantic layer ID and occlusion policy,
- transparent-background/full-canvas requirement.

Do not crop pixels from the target as the release asset. Once every semantic layer is locked, run one `PSD_PACKAGING` GenerationUnit. `$imagegen` creates the PSD, performs any required cutout/path packaging, and must preserve locked pixels/registration. SceneAssetCompiler only validates the returned file.

## Parallelism

Parallelize only units whose required locks are already satisfied and which do not depend on each other's pixels. Composition candidates and independent layer candidates are naturally parallelizable. Executor parallelism is a capability setting, not a semantic rule.
