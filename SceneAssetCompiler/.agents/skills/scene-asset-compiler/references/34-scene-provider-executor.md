# Scene Provider v2 Executor

## Purpose

Use the Scene Provider as SceneAssetCompiler's headless executor only for one reference-free PNG text-to-image attempt. SceneAssetCompiler still owns task state, immutable evidence, alpha/canvas/margin QA, visual review, approval, and publication. The provider owns authentication, network dispatch, and one bounded image response.

This is not a fallback that weakens the task. If the attempt needs a reference image, edit target, mask, cutout repair, WebP, or PSD, use the installed `$imagegen` skill.

## Prerequisites

- Apple silicon macOS.
- An executable named exactly `scene-provider-v2` at the bundle boundary.
- The official Codex runtime package pinned by the selected Scene Provider revision.
- An existing workspace and an already authenticated ChatGPT account selected by Codex.
- `VERGERAIL_CODEX_PACKAGE`, `VERGERAIL_MODEL`, and `VERGERAIL_WORKSPACE` set to explicit values. Do not set or copy an API key for this lane.

SceneAssetCompiler validates the executable name, required environment, task/attempt lineage, empty output directory, PNG preference, supported size, and absence of references before dispatch. Scene Provider independently validates the runtime package and request contract.

Preflight hashes the exact provider first, then runs a bounded side-effect-free
`protocol_probe` against that same executable. The observed response must be a
single v2 JSON envelope (`schemaVersion=2`, correlated request ID,
`invalid_request`, `retryable=false`) and the owned process group must be
reaped with no survivors. A v1, malformed, non-v2, or timed-out response stops
before dispatch. The resulting observed probe response and custody record are
stored in `scene-provider-dispatch.json`; lock metadata is an expectation, never
the proof of the running binary.

The upstream `VERGERAIL_*` environment variables remain because they are part
of the external provider's runtime contract. Historical attempt evidence is
read-only. New bundle binaries and attempt evidence use the neutral Scene
Provider names.

Controller diagnostics and receipt notes use the `SCENE_PROVIDER_*` namespace.
The upstream `VERGERAIL_*` names below are environment variables only, not
local diagnostic codes.

The only CLI commands are `execute-scene-provider` and
`probe-scene-provider`. Former `misegen` and `vergerail` command spellings are
rejected by the ordinary argument parser.

## Execution

Run the examples from the extracted Standalone bundle root. Resolve every
runtime path before invoking the controller:

```bash
BUNDLE_ROOT="$(pwd -P)"
PRODUCT_ROOT="$BUNDLE_ROOT/SceneAssetCompiler"
SCENE_ASSET_COMPILER="$PRODUCT_ROOT/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py"
PROVIDER_BIN="$BUNDLE_ROOT/bin/scene-provider-v2"
PERFECTPIXEL_BIN="$BUNDLE_ROOT/bin/perfectpixel"
COMPONENT_LOCK="$PRODUCT_ROOT/contracts/runtime-component-lock.json"
RUN_DIR="$PRODUCT_ROOT/.scene-assets/runs/<run-id>"
```

Always preflight first:

```bash
python3 "$SCENE_ASSET_COMPILER" execute-scene-provider "$RUN_DIR" <task-id> <attempt-no> \
  --provider-bin "$PROVIDER_BIN" \
  --perfectpixel-bin "$PERFECTPIXEL_BIN" \
  --component-lock "$COMPONENT_LOCK" \
  --quality medium \
  --dry-run
```

`--dry-run` performs no provider call and writes no attempt evidence. Inspect its size, background, quality, prompt byte count, and prompt hash.

Execute exactly once:

```bash
python3 "$SCENE_ASSET_COMPILER" execute-scene-provider "$RUN_DIR" <task-id> <attempt-no> \
  --provider-bin "$PROVIDER_BIN" \
  --perfectpixel-bin "$PERFECTPIXEL_BIN" \
  --component-lock "$COMPONENT_LOCK" \
  --quality medium
```

Before starting the provider, the command writes `scene-provider-dispatch.json` with the request ID, exact request-frame/capsule hashes, provider binary hash, options, and timeout. Its presence permanently blocks a second dispatch of that attempt, including after process or host failure. The provider starts in a dedicated process group; `scene-provider-process.json` records the leader PID, PGID, monotonic deadline, and teardown budgets before input is sent.

The command then sends the attempt-local `generation-capsule.md` without added prose, requests one PNG, validates the typed response/base64/byte count/PNG metadata with a bounded decoded-size limit, writes `outputs/generated.png` atomically, and creates:

- `scene-provider-dispatch.json`
- `scene-provider-process.json`
- `execution-receipt.json`
- `result.json`
- `mechanical-review.json`

`READY_FOR_VISUAL_REVIEW` means transport and mechanical QA passed. It does not mean visual approval. Inspect the actual PNG, create and complete `visual-review.json`, validate it, then approve.

## Limits

The current Scene Provider process contract supports only `1024x1024`, `1536x1024`, and `1024x1536`, one image, and no reference inputs. `background=transparent` is a request preference; approval depends on SceneAssetCompiler decoding the returned PNG and proving actual transparent pixels, a near-opaque subject core where required, and the contracted per-edge margin.

The provider model is the `gpt-image-2` alias. The execution receipt therefore records `model_snapshot=UNKNOWN`; never claim pixel reproducibility. Preserve approved bytes and hashes instead of assuming that rerunning a recipe reproduces them.

## Failure and recovery

- Typed provider failure: receipt outcome `FAILED`; diagnose before creating a new attempt.
- `resolution_required`, provider timeout, lost response, nonzero transport, stderr, or malformed response after possible dispatch: receipt outcome `UNKNOWN`.
- Timeout/cancellation cleanup checks that the live child leader owns the recorded PGID immediately before best-effort signalling, waits for graceful exit, may force-stop that numeric group, reaps the leader, and records whether any group survivor remains. POSIX does not make the identity check and `killpg` atomic; a narrow PID/PGID-reuse TOCTOU remains. Missing pre-signal check or no-survivor observation remains `UNKNOWN`.
- A controller or host crash after dispatch still blocks redispatch, but automatic orphan reaping is not implemented. The recorded PID/PGID is evidence, not safe signal authority: PID reuse can target an unrelated process. Inspect process identity and external provider state manually; preserve the attempt as `UNKNOWN` when custody cannot be proven.
- `UNKNOWN` is not retry authority. Do not execute the same attempt again. Resolve possible external completion, then reconstruct a new immutable attempt only when safe.
- Mechanical failure after a valid PNG: keep the returned bytes, run Doctor, and repair the smallest failed axis.
- Local receipt/result/review finalization failure: status `POSTFLIGHT_INCOMPLETE`; retain `scene-provider-dispatch.json`, any returned bytes, and `scene-provider-postflight-error.json`. Do not redispatch that attempt.

Never delete or overwrite a failed/unknown attempt to make it runnable again.
