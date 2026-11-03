# SceneAssetCompiler

Use `$scene-asset-compiler` for reusable story visual assets/scenes.

Non-negotiable:
- pixels come only from Codex `$imagegen` or the typed Scene Provider v2 provider executor;
- Scene Provider is limited to reference-free PNG text-to-image attempts; references, edits, cutouts, paths and PSDs stay with `$imagegen`;
- never add local OpenAI image API clients;
- never add local PSD assembly, segmentation, matting or path-tracing code to the canonical workflow;
- use one selected-executor call per independently reviewable asset/variant;
- one scene may require dozens of GenerationUnits;
- validate returned PSD structure and path metadata before approval;
- retry only failed assets/properties.
