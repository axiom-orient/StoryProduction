# Copy/Paste GPT PSD Generation Profile

Use this block in the generation capsule when the task output is PSD.

## Single asset — ORGANIC default

```text
PSD delivery:
- Return a real editable Adobe Photoshop PSD.
- RGB, 8 bits/channel, embedded sRGB IEC61966-2.1, exact requested canvas.
- Keep the approved asset pixels/identity/pose/style unchanged. This is packaging/editability work, not redesign.
- Use one native pixel layer per required semantic layer with the exact machine layer name.
- Preserve ordinary RGBA transparency. Do not use Photoshop pixel/user layer masks, and do not use a layer vector mask as a substitute for the formal Saved Path.
- Do not use Smart Objects or adjustment layers unless explicitly requested.
- Do not trim the layer.
- For each cutout layer, create a clean Work Path from the final cutout boundary, then immediately SAVE/RENAME it as `clip.<layer_id>` in the Paths panel.
- The Named Saved Path is authoritative; do not leave the only boundary as an unsaved Work Path.
- Organic Work Path tolerance target: about 1.5 px; use smooth Bezier curves and the fewest clean anchor points that preserve the silhouette.
- After saving/renaming the Work Path, the Named Saved Path is canonical; no surviving Work Path copy is required.
- Set the primary Named Saved Path as the document clipping path.
- Leave clipping-path Flatness blank/default. Do not invent a numeric value unless a downstream print contract explicitly requires one.
- Do not rasterize the path into a layer mask.
```

## Single asset — SOFT_ALPHA

Change only these lines:

```text
- Preserve soft alpha for hair/fur/translucency as visual truth.
- Work Path tolerance target: about 3.0 px; the Saved Path is a coarse semantic selection envelope.
- Do NOT set a document clipping path.
```

## Multi-layer scene PSD

```text
PSD delivery:
- Return a real editable Adobe Photoshop PSD.
- RGB, 8 bits/channel, embedded sRGB IEC61966-2.1, exact locked scene canvas.
- Use approved locked layer pixels exactly; do not redraw, restyle, relight, recompose, inpaint or invent during packaging.
- One native pixel layer per required semantic asset, exact machine IDs, exact z-order/registration.
- Ordinary layer transparency only; no Photoshop pixel/user layer masks.
- No Smart Objects, no trim.
- Only for cutout/path layers explicitly declared by the SceneProductionPlan, create a Work Path from the final boundary and SAVE it as `clip.<layer_id>`. Do not create meaningless paths for fixed backplates/opaque layers.
- Do not leave unsaved Work Paths as the only boundary authority.
- Use the per-layer tolerance target: HARD about 0.75 px, ORGANIC about 1.5 px, SOFT_ALPHA envelope about 3.0 px; preserve smooth curves with minimal anchors.
- Do NOT set a document clipping path on a multi-layer scene PSD.
- After Saved Paths exist, no Work Path is required.
- Preserve soft alpha for hair/fur/translucent layers; their Saved Path is only an editing envelope.
```
