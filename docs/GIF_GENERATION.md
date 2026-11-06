# Native GIF generation

StoryProduction supports native GPT Image 2.6 animation through the existing
promptGen adapter. NVC retains narrative/direction authority; LayoutSheet remains
an optional `.layoutpack` input. SAC and SMC do not acquire image API clients.

## Generate

Use `image.generate` with `provider.kind=openai`,
`provider.model=gpt-image-2.6-sunburst` or `gpt-image-2.6-flare`,
`networkPolicy=required`, and an expected `image_candidate` with
`mediaType=image/gif`. The typed payload uses `task_mode=generate`, no references,
and this output profile:

```json
{"model":"gpt-image-2.6-sunburst","width":768,"height":512,"quality":"medium","background":"opaque","format":"gif","frame_count":9,"frame_duration_ms":200}
```

See the complete [prompt example](../ImagePromptCompiler/examples/image-animated-coffee.json)
and [capability template](../integration/contracts/examples/capability-request.gif-generate.json).
Replace example source/authorization identities with actual project authority
and authorization before provider dispatch.

```sh
STORY_ROOT=/path/to/StoryProduction
"$STORY_ROOT/integration/target/release/storyctl" --story-root "$STORY_ROOT" \
  run-promptgen --capability-root /path/to/capability-root \
  --request /path/to/current-gif-request.json \
  --output-dir /path/to/capability-root/runs/unique-attempt \
  --promptgen-bin "$STORY_ROOT/ImagePromptCompiler/target/release/promptgen"
```

Existing capability-root selection, source/hash and authorization rules apply.
`OPENAI_API_KEY` must be configured in the invoking environment, and the account
needs GIF preview access. No key is copied into argv or artifacts. A missing key
returns `not_started` before a job directory or provider process is created.

For an approved still reference, use `image.edit`, `task_mode=edit`, exactly one
typed reference at index 1 with role `base`, and its hash-bound artifact in the
capability envelope. Generation sends JSON to `/v1/images/generations`; editing
sends the actual image bytes as multipart to `/v1/images/edits`.
Neither route silently substitutes another model or a PNG executor.

The adapter validates actual GIF LZW raster data, canvas, frame count and delays
before publishing `image.gif`. PNG bytes and mismatched timing/dimensions fail.
The local profile requires 2–16 frames, 20–2000 ms per frame in 10 ms increments,
and at most 16,777,216 decoded pixels across all frames. These conservative
local limits do not claim provider entitlement or its complete size limits.

## Use a generated GIF

SAC fully decodes an existing GIF with the already-required FFmpeg/FFprobe tools.
It does not create new motion. An intake draft contains:

```json
{"input":"runs/unique-attempt/image.gif","assetPrefix":"door-opening","fps":12,"playback":"once-hold","executionEvidence":"runs/unique-attempt/capability-result.json"}
```

```sh
"$STORY_ROOT/integration/target/release/storyctl" --story-root "$STORY_ROOT" \
  text --workspace /path/to/workspace animatic assets --asset-root /path/to/assets \
  import-gif --packet animatic/packets/current.json --draft requests/gif-intake.json \
  --output animatic/door-opening/receipt.json
```

`SAC_FFMPEG_BIN` and `SAC_FFPROBE_BIN` select installed decoders when needed.
SAC verifies source/evidence hashes, decodes disposal-aware opaque PNG frames,
and publishes an immutable candidate receipt and assets. Failed, cancelled or
unknown execution declarations are refused. Story results must bind a successful
GIF artifact to the same NVC packet. Imported frames have `user-supplied`
provenance with the original GIF/evidence hashes; this classification does not
claim Codex generated the GIF or authenticate the declared provider identity.

The receipt's `planFragment` provides NVC asset requirements and ordered poses.
Adopt these through NVC's public direction workflow, approve the changed plan,
export its packet, then use ordinary SAC submit/review and SMC render. Prior PNG
approval does not approve new direction or pixels. No NVC state is rewritten by
intake, and no candidate is a release.

Timing uses `ceil(cumulative milliseconds × fps / 1000)` to avoid early actions.
If destination FPS would lose a source frame, intake refuses. GIF loop metadata
is ignored: play once, then hold the last pose. Longer holds belong to the approved
NVC plan. Door opening, descent and closure remain separate story actions.
The existing SMC exact-frame pose renderer consumes these frames without an
autonomous browser GIF clock or another animation authority.

For read-only recovery, use the same `animatic assets` prefix with
`inspect-gif --packet ... --pack animatic/door-opening/receipt.json`.
It rechecks receipt, source/evidence and all retained frame bytes. Partial intake
returns `outcome_unknown`; inspect retained effects before another attempt.

## Evidence limits

Codec fixtures and scripted provider seams test mechanics, not real generation
or production E2E. A ChatGPT animation export is an actual media artifact, but
prompt text does not prove a hidden model identifier or requested canvas. Retain
downloaded bytes and observations separately from API execution and scene approval.

Official sources: [GIF output](https://developers.openai.com/api/docs/guides/image-generation#generate-animated-gif-output),
[generation API](https://developers.openai.com/api/reference/resources/images/methods/generate),
[edit API and preview access](https://developers.openai.com/api/reference/resources/images/methods/edit).
