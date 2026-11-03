# Image Executor Handoff

SceneAssetCompiler has two executor lanes. It never contains an Images API client.

- Follow the installed official `$imagegen` skill for resolved references, edits, cutouts, PSDs, or unsupported Scene Provider sizes.
- Use `scene-asset-compiler execute-scene-provider` only for reference-free PNG text-to-image attempts. See `34-scene-provider-executor.md`.

For each ready task using `$imagegen`:
1. Begin an attempt.
2. Read `generation-capsule.md` and use it as the sole visual brief. `prompt.md` exists for audit/debugging and must not be pasted into the image request.
3. Read `attempt.json.resolved_references`; these are the only visual inputs. Workflow dependencies that are not explicit references must not be sent to `$imagegen`.
4. For local raster edit targets, make them visible to Codex as required by `$imagegen` before editing.
5. Call `$imagegen` once for this asset/variant with the capsule content and only the resolved visual inputs. Do not prepend or append workflow explanations, Doctor reports, JSON, scoring language, or project-meta prose.
6. Inspect the result before accepting it as a candidate.
7. Move/copy selected project-bound files from Codex's generated-image storage into the attempt `outputs/`.
8. Immediately create an execution draft from observed metadata and run `record-execution`. Use `UNKNOWN` rather than inventing a model snapshot or provider request ID; label unverified metadata `EXECUTOR_RECORDED`.
   `submitted_prompt_path` must be canonical and relative to the attempt directory. Absolute paths, `..`, symlinks, external files and post-receipt hash drift are rejected. Older absolute-path field receipts are historical evidence only and do not satisfy the hardened gate when revalidated.
9. Never overwrite the output or receipt.
10. Never fabricate a PSD. PSD generation is supported when `$imagegen` actually returns one; if a particular call returns only raster output, record that result and use fallback only when the task explicitly allows it.

Approved anchors are critical, but dependency and reference are different concepts. `depends_on` gates readiness; a `TASK_OUTPUT` reference explicitly selects which approved dependency output is passed to `$imagegen`. A downstream recurring-character task must use the selected `CHARACTER_ANCHOR`, never chat memory or an unapproved candidate.

The Scene Provider lane fails before dispatch when `resolved_references` or an edit target exists. It must never silently drop a visual input to force a text-only request.


## Context-isolation invariant

The image executor must not infer visual content from surrounding workflow conversation. `generation-capsule.md` is deliberately free of SceneAssetCompiler/Doctor/QA vocabulary. If an output renders dashboards, diagrams, JSON, workflow labels, or other conversation-meta content that was not requested, record `EXECUTOR_CONTEXT_LEAKAGE:` in `output-audit.json.global_observations` and run Doctor.
