# Research — SceneAssetCompiler 0.9

Date: 2026-08-24

## 0. Evidence policy

Official specifications and vendor documentation define file/runtime contracts; peer-reviewed or primary research informs failure axes; community issues are field signals only. A model or tool's general capability never proves a particular attempt. Each attempt needs its own immutable execution receipt and artifact inspection.

OpenAI's current Image Generation guide states that GPT Image 2 supports flexible sizes, high-fidelity image inputs, and preview transparent backgrounds when `background="transparent"` is used with PNG/WebP. Therefore transparency is requested explicitly and then verified from decoded output pixels, not inferred from an RGBA color type.

Sources:
- https://developers.openai.com/api/docs/guides/image-generation
- https://developers.openai.com/api/docs/models/gpt-image-2

Fine-grained evaluation research repeatedly finds that holistic quality/alignment scores miss object count, spatial relations, attribute binding, and anatomy failures. SceneAssetCompiler therefore keeps atomic claims and a dedicated photorealism axis instead of using “looks realistic” as a single verdict.

Sources:
- https://arxiv.org/abs/2303.11897 (TIFA)
- https://arxiv.org/abs/2310.11513 (GenEval)
- https://openaccess.thecvf.com/content/WACV2026/html/Jain_HumanBench_Two_Heads_No_Legs_But_Mostly_Human_the_State_WACV_2026_paper.html
- https://openaccess.thecvf.com/content/WACV2026W/WVAQ/html/Kas_We_Still_See_Broken_Limbs_Towards_Anatomical_Realism_in_GenAI_WACVW_2026_paper.html

## 1. Codex `$imagegen`: one narrow visual responsibility per call

OpenAI's current system `imagegen` skill uses the built-in image-generation tool by default. For multi-asset work it explicitly loops one built-in call per asset or variant, labels every input image by role, inspects the actual output, and recommends a single targeted change on each edit iteration.

Architecture consequence:

```text
one GenerationUnit
= one independently reviewable visual responsibility
= one `$imagegen` call
```

This supports SceneAssetCompiler's scene graph instead of a monolithic “generate the whole film/asset pack” prompt.

Sources:
- https://github.com/openai/skills/blob/main/skills/.system/imagegen/SKILL.md
- https://github.com/openai/skills/blob/main/skills/.system/imagegen/references/image-api.md
- https://github.com/openai/skills/blob/main/skills/.system/imagegen/references/cli.md

## 2. Photoshop Work Path → Saved Path → optional document clipping path

Adobe documents that converting a selection to a Work Path accepts tolerance `0.5–10 px`; larger values use fewer anchors and produce a smoother path. Adobe also documents the actual clipping-path workflow: create/convert the Work Path, **save it as a path**, then select that saved path as the clipping path when needed.

Photoshop's file-format specification stores:
- Work Path as image resource 1025,
- Saved Paths as resources 2000–2997,
- one document clipping-path name as resource 2999,
- ICC profile as resource 1039.

Therefore SceneAssetCompiler does not treat a temporary Work Path as release authority. The generation prompt asks GPT to promote it to a stable Named Saved Path `clip.<layer_id>`.

Sources:
- https://helpx.adobe.com/kr/photoshop/using/converting-paths-selection-borders.html
- https://helpx.adobe.com/kr/photoshop/using/placing-photoshop-images-applications.html
- https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/

## 3. Tolerance defaults

Adobe's valid Work Path tolerance range is broad; there is no universal “best” value. SceneAssetCompiler therefore uses stage/edge-aware prompt targets rather than pretending the number is a deterministic renderer parameter:

| Edge | Default target | Intended trade-off |
|---|---:|---|
| HARD | 0.75 px | accurate rigid/product silhouette |
| ORGANIC | 1.5 px | smoother body/clothing contour with fewer noisy anchors |
| SOFT_ALPHA envelope | 3.0 px | coarse editing/selection envelope; alpha remains visual truth |

These are **prompt targets**. The returned Saved Path still requires visual/structural review.

## 4. Clipping-path Flatness

Adobe recommends leaving clipping-path Flatness blank to use the output device default. It is primarily a PostScript curve-approximation/printing control, not a visual-quality knob for the PSD itself. SceneAssetCompiler therefore requests `blank/default`. It does not invent a numeric Flatness unless a downstream print contract explicitly requires one.

## 5. Per-layer path policy

Not every scene layer needs a path. Forcing a Saved Path on a full-canvas backplate or opaque fixed environment creates meaningless metadata and extra failure modes.

Scene layers therefore declare:

```text
path_boundary = NONE | SAVED_PATH | SOFT_ENVELOPE
edge_class    = HARD | ORGANIC | SOFT_ALPHA
```

Only declared cutout/path layers require `clip.<layer_id>`.

## 6. Soft edges

Hair, fur, smoke, translucency and similar boundaries cannot be faithfully represented by a hard vector clipping boundary alone. For `SOFT_ALPHA`:
- ordinary RGBA alpha remains the visual truth,
- the Saved Path is a simplified semantic editing envelope,
- document clipping path is forbidden.

## 7. One clipping path per document, many Saved Paths

Photoshop can store many Saved Paths but only one document-level clipping-path name. Therefore:
- **single asset HARD/ORGANIC**: a primary Saved Path may be designated as the document clipping path;
- **multi-layer scene PSD**: one Named Saved Path per declared cutout, **no document clipping path**.

This is a format constraint, not a stylistic preference.

## 8. Layer-native design and multi-subject generation

Recent 2026 research strengthens the same architectural direction:
- **UniWorld-Design (2026-08-04)** treats semantic RGBA layers as atomic generation/editing units and emphasizes complete semantic objects rather than visible-pixel partitions.
- **MultiCompose (2026-08-04)** reports identity degradation and per-subject attribute misalignment when multiple personalized subjects are composed without explicit spatial separation.

Architecture consequence: produce/approve reusable semantic assets in separate generation units, then use a locked target/composition for the final PSD packaging pass.

Sources:
- https://arxiv.org/abs/2608.03971
- https://arxiv.org/abs/2608.03708

## 9. Final v0.9 boundary

Earlier internal experiments proved that Saved Path, Work Path and clipping-path resources can be read and verified from a real PSD. v0.8 deliberately removes the local producer.

```text
GPT/Codex `$imagegen`
  owns pixels + cutout + layer creation + paths + PSD save

SceneAssetCompiler
  owns authority + generation settings + locks + receipts + PSD postflight + Doctor
```

This preserves provider replaceability and prevents a fallback implementation from silently becoming the real product.

## 10. PSD qualification status

The user has directly observed Codex/$imagegen returning real PSD files when the generation brief is sufficiently explicit. Treat this as valid field evidence that the capability exists, not as proof that every call or current tool surface will return PSD. Adobe's format specification remains the postflight authority: ICC resource 1039, Saved Paths 2000–2997, and clipping-path name 2999 are parsed and checked. Qwen-Image-Layered is useful supporting research for semantic RGBA decomposition, but its official repository explicitly says released weights are mainly image-to-layer and have limited text-to-multi-layer control; it is not a drop-in replacement for SceneAssetCompiler's `$imagegen` PSD boundary.

Sources:
- https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/
- https://github.com/QwenLM/Qwen-Image-Layered
- https://arxiv.org/abs/2512.15603
- https://arxiv.org/abs/2601.15507
- https://arxiv.org/abs/2608.03971
