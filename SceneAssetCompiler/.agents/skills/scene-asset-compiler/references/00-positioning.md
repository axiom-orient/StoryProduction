# Product Positioning

SceneAssetCompiler is a **Codex App skill, workflow, and deterministic pipeline** that compiles narrative intent into verified reusable visual assets. It is not an image model, image API wrapper, rigging tool, or scene editor.

Pixel work is always delegated to an external executor. The typed Scene Provider lane handles only one reference-free PNG text-to-image attempt; `$imagegen` handles references, edits, cutouts, WebP, paths, and PSDs. SceneAssetCompiler owns planning, contracts, dependency/anchor management, deterministic handoff, QA, targeted repair, lineage, and AssetPack publication.

The repo-local skill is the canonical source. Codex discovers it when `scene-asset-compiler` itself is the project root, or through one canonical user-skill link when a parent directory such as `SMA` is opened. Long-term external distribution can wrap this skill as a Plugin; do not redesign the workflow for packaging.
