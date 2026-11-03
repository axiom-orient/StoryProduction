# GPT PSD Path / Work Path / Clipping Path Contract

## Product rule

The PSD producer is `$imagegen`.

SceneAssetCompiler supplies settings and validates the returned file. It never generates the boundary itself.

## Photoshop semantics

- Work Path is temporary/unsaved editor state.
- A production boundary must be promoted to a **Named Saved Path**.
- Saved Path names use `clip.<layer_id>`.
- A PSD can hold many Saved Paths.
- Photoshop has one document-level clipping-path name; it is not a per-layer clipping feature.

Adobe's file-format specification stores path geometry in 26-byte path records, Saved Paths in image resource IDs 2000–2997, and the document clipping-path name in resource 2999.

## Canonical prompt settings

### Document
- PSD
- RGB
- 8-bit/channel
- sRGB IEC61966-2.1
- exact requested canvas
- no trim
- native pixel layers
- no Smart Objects
- no pixel/user layer masks

### Work Path → Saved Path

For each cutout leaf:
1. create the final clean cutout with ordinary RGBA transparency;
2. create a Work Path from the cutout boundary;
3. clean/simplify it;
4. **save/rename it immediately** to `clip.<layer_id>`;
5. do not leave Work Path as the only path authority.

Adobe permits Make Work Path tolerance from 0.5 to 10 px. SceneAssetCompiler prompt defaults:

| Edge | Target tolerance | Reason |
|---|---:|---|
| HARD | 0.75 px | close rigid/product contour |
| ORGANIC | 1.5 px | smooth body/clothing silhouette with fewer noisy anchors |
| SOFT_ALPHA envelope | 3.0 px | coarse selection/edit envelope only |

These are prompt targets, not proof. The returned path still requires inspection.

The embedded ICC resource must be a structurally valid RGB monitor/input profile whose description identifies sRGB; an `acsp` signature alone is insufficient. Device-link, Adobe RGB, Display P3, malformed tag tables and unidentified RGB profiles fail the sRGB contract.

A path resource needs at least one non-degenerate subpath with valid Photoshop 26-byte selector sequencing. Name and record alignment alone do not prove that a path follows the visible alpha edge. Pixel/path alignment remains `STRUCTURE_ONLY` unless render-back or an external path-overlap receipt is available.

Required pixel leaves also require structurally valid channel payloads: exact raw byte count, valid PSD/PSB PackBits row tables and decoded row widths, or ZIP output matching the layer dimensions and bit depth. This rejects truncated or junk channel declarations but still does not replace visual render-back.

## Single asset

For HARD/ORGANIC:
- Named Saved Path required;
- the Saved Path is canonical; the temporary Work Path does not need to remain after saving;
- set the primary Saved Path as the document clipping path;
- clipping Flatness: **leave blank/default when supported**;
- do not invent a numeric Flatness unless a downstream print contract explicitly requires one; Flatness is not a PSD editability/quality control.

Adobe recommends leaving Flatness blank to use the printer default; lower values increase curve approximation detail. This metadata is mainly a PostScript output concern, not an image-quality knob.

## Soft alpha

Hair, fur, smoke, translucent cloth:
- preserve soft RGBA alpha as visual truth;
- save a simplified semantic envelope path;
- no document clipping path;
- do not harden alpha merely to satisfy a path.

## Scene PSD

- Saved Path per cutout layer;
- no document clipping path;
- no requirement to keep an unsaved Work Path;
- path names must be unique and stable.
