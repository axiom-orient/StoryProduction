# StoryProduction media research — 2026-11-06

## Product decision

StoryProduction develops an author's source into editable narrative, scene/shot
direction and reviewable audiovisual production. Story meaning, spatial
continuity, performance, text and media must remain traceable to their owners.
GIF is an asset/playback format and a preview deliverable, not the product's
identity. A GIF preview cannot carry the character dialogue audio of a film.

This research starts at Story revision `4150b580c1c53e948948e097645285d857535cbc`.
Research findings, prototype outputs and shipped capabilities are separate.
The implementation status must be updated from actual execution, not from this
decision document.

## Observed gaps at the starting revision

* The native animatic `Context::render` in
  `SceneMotionCompiler/engine/crates/smc-animatic/src/animatic.rs` publishes a
  silent WebM, HTML, VTT and source/request receipts. It does not publish a
  whole-scene GIF or synthesized character dialogue.
* `installComposition` in
  `SceneMotionCompiler/packages/production/src/animatic-runtime.js` displays all
  a panel's dialogue simultaneously. It lacks utterance timing and a two-line
  caption layout contract. A readable string is not speech/caption alignment.
* The existing generated interior images do not consistently preserve one
  vehicle topology across cameras. A prompt saying left-hand drive does not
  verify the image. Vehicle left/right and screen left/right are distinct.
* SAC's GIF intake already coalesces native frames, binds source bytes and
  preserves play-once/hold timing. A second general image-provider/atlas engine
  would duplicate that boundary without itself fixing spatial or acting quality.

## Local project analysis

The supplied module directories are filesystem snapshots without their own Git
metadata. Findings are scoped to the inspected files. Their implementation and
licensing must not be inferred from their names or historical README claims.

| Project | Reachable implementation evidence | Decision for this production path |
| --- | --- | --- |
| LiveText | `Sources/LiveTextLayout/InlineLayoutEngine.swift::layout`, preparation/source ranges, Apple shaping/render targets in `Package.swift`; MIT | Keep its stable source identity, immutable layout and grapheme/range principles. Direct SwiftUI/CoreText integration is optional native presentation work, not a prerequisite for the existing Canvas video renderer. Do not import handwriting, selection or Dust effects for ordinary subtitles. |
| PaperScape | `Packages/PaperScape/Sources/PaperScapeCore/Timeline.swift::sample`, `Frame.swift::FrameEvaluator.evaluate`, iOS 26/Metal host | Reference its explicit once/loop/hold semantics and separate frame evaluation. Do not add an iOS wallpaper runtime or its clocks to a scene compiler. New Swift implementation is MIT; individual bundled resources have separate provenance. |
| LiveWallpaper | `packages/liveplayer/src/frame.js::evaluateFrame`, `packages/web-host/src/wave.js::WaveSurface`, strict runtime/compiler split | Its deterministic sprite/transform evaluation is relevant, but SMC already owns that clock and drawing boundary. Avoid a second runtime and whole-frame warps on faces/seats. No root license was located in the inspected snapshot; do not copy source or example art without establishing rights. |
| Beziara | `Beziara-Creative/Sources/VectorLottie/VectorLottieExporter.swift`, raster/vector motion exports and editor workspace | Optional vector/prop authoring, not a generator of coherent bus interiors or character acting. Do not pull its editor/vectorization graph into this runtime. |
| Acornry | `Sources/Acornry` transfer/receipt/reconcile APIs, optional SwiftUI transfer host | Nearby file delivery is unrelated to current single-host scene production. No dependency. |
| MoToolbox | `Libraries/MoTDiscovery/Sources/MoTDiscovery/Discovery.swift`, separate native build/device products | Development tooling, not narrative/media authority. Use existing host tools when needed; do not embed its installer or device products. |
| LayoutSheet | `src/application/pack.ts::encodePack/decodePack`, `src/domain/model.ts::Project`, independent PageAPI | Use the existing optional `.layoutpack` contract when a spatial study is needed. It does not own narrative or approve generated imagery. A geometry/shot package must retain an unapproved revision until actually reviewed. |

## Generation and motion

[sprite-gen](https://github.com/aldegad/sprite-gen), inspected at
`c30322bc6a51a86c26d0d9aadcc2e7db7345027e`, provides component rows, reference
identity, frame extraction, atlas manifests, face layers and optional video
processing. Its code is Apache-2.0. The full skill also contains provider routing,
key-based paid alternatives, video tools and optional RIFE installation.

Adopt the useful production constraints: one identity/geometry anchor, explicit
motion states, per-frame curation, contact/occlusion checks, source frame hashes,
and a deliberate distinction between loops and irreversible events. Existing
SAC extraction and SMC pose tracks should consume the selected frames. Do not
install its whole provider engine, automatically switch to an API key, or claim
that an atlas alone makes motion anatomically natural. Voice-linked face motion
and phoneme accuracy need their own actual validation.

Rive/Lottie are suitable for authored vector rigs. They do not turn existing
painted scene images into correct articulated performances and would add a new
asset/renderer graph here. Rhubarb offers audio-to-mouth cues, but Korean
recognizer quality and an aligned face pack remain unqualified; it is not a
default dependency. See [Rhubarb's own documentation](https://github.com/DanielSWolf/rhubarb-lip-sync).

## Deferred research: free, key-free character speech

The user explicitly deferred voices for this version. The following research and
actual local auditions are retained outside the public source tree. No TTS
provider, model weights, Python inference dependency or voice track is included
in the current release scope.

| Candidate | Evidence and limits | Decision |
| --- | --- | --- |
| Qwen3-TTS 1.7B CustomVoice + local MLX | Official Qwen model supports Korean, fixed speaker IDs and a separate performance instruction. Apache-2.0 weights; MLX-Audio implementation MIT. A pinned 4-bit conversion can run on Apple Silicon. | First actual audition candidate. Verify memory, pronunciation, pacing, character separation and output WAVs locally. No API key, hosted inference or new paid account. |
| Qwen3-TTS 0.6B | The official implementation explicitly discards `instruct` on 0.6B. | Not a substitute for requested acting control. |
| Supertonic | Local ONNX, multilingual/Korean, smaller and speed focused; inspected upstream is archived. | Useful speed baseline, not the default emotional-performance choice. Do not generalize hosted Sona features to local Supertonic. |
| ElevenLabs / Cartesia / Supertone hosted services | Commercial offerings expose expressive speech, voices and/or dialogue. Availability, free tiers and licensing vary; account/API paths do not satisfy this user's no-key/no-account constraint. | No runtime dependency or paid/API fallback. |
| Fish Speech and larger local models | Expressive generation is relevant, but exact model licensing, hardware fit and Korean performance need model-specific checks. | No speculative dependency on this 8 GB host. |

If resumed, the runtime should be an optional, isolated local provider process. NVC owns
the spoken source, cast/delivery direction and timing; SAC owns admitted audio
assets/review; SMC owns frame/sample alignment and output encoding. TTS must not
rewrite dialogue, silently drop words, add an announcer or replace a failed take
with silence. A numeric waveform check does not approve acting or pronunciation.

Primary sources:
[Qwen model and API](https://github.com/QwenLM/Qwen3-TTS),
[official CustomVoice behavior](https://github.com/QwenLM/Qwen3-TTS/blob/main/qwen_tts/inference/qwen3_tts_model.py),
[MLX-Audio](https://github.com/Neywa-Labs/mlx-audio),
[selected conversion](https://huggingface.co/mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-4bit),
[Supertonic](https://github.com/supertone-oss-archive/supertonic),
[ElevenLabs performance tags](https://elevenlabs.io/docs/help-center/product/core-capabilities/text-to-speech/how-do-audio-tags-work-with-eleven-v3-and-v4).

## Captions and text effects

Use one source ID/range and one frame clock for each spoken caption cue. Keep
speaker identity, original text and actual audio duration. Bound line count,
font size, safe area and overflow; never silently truncate. Text effects should
support reading/emphasis and respect reduced motion, rather than distort words
or invent an independent playback clock.

[WebVTT](https://www.w3.org/TR/webvtt1/) provides timed cues and positioning and
fits the existing browser/sidecar workflow. [IMSC](https://www.w3.org/TR/ttml-imsc1.1/)
is a distribution profile to consider when an actual delivery needs it.
[libass](https://github.com/libass/libass) is a mature ASS/SSA renderer, but the
installed FFmpeg does not expose its subtitles/ass filters. Adding another
renderer is unnecessary before fixing SMC's actual cue/layout behavior. Keep
exported captions editable and validate burned-in output against the same cues.

### Format and motion choices

| Method | Useful property / limitation | Current choice |
| --- | --- | --- |
| Native generated GIF | Service-produced sequential pixels, but provenance and actual frame content must be inspected | Keep the working ChatGPT UI intake path; a gated subscription endpoint is not a successful generator. |
| Rendered GIF | Ubiquitous image preview; palette colors and centisecond delays, no dialogue audio | Add a bounded 6 fps / maximum 640 px preview from the exact captured PNGs. Label it rendered, never native generation. |
| PNG pose sequence | Lossless frames, exact raw hashes, selective replacement and explicit event order | Keep as canonical admitted visual material. Discrete poses remain an animatic; do not call them fully animated acting. |
| APNG | Lossless color/alpha and frame delay/disposal/blend controls in the current PNG specification | Suitable optional short transparent motion deliverable. No current consumer requires it, so do not add another exporter now. |
| Animated WebP | Lossy/lossless color and alpha, often smaller than GIF; quality and decode cost need host-specific measurement | Optional future web delivery. No universal size/speed claim or automatic conversion in this release. |
| WebM / editable HTML | Existing exact-frame video path and inspectable frame seeking/play/pause | Keep full-frame video and source composition; GIF is a companion preview. Voice remains excluded. |
| Layer transform / modest camera move | Cheap deterministic composition when real separated layers exist | Existing SMC clock owns it. Do not warp the entire cabin or invent depth from a flattened plate. |
| Optical-flow / RIFE interpolation | Can synthesize intermediate frames between observed endpoints | Do not install by default. Interpolation does not establish hand contact, seat geometry or a missing action state. Such use would require actual occlusion/foot/door testing. |
| Vector rigs / face layers | Useful with authored components and reviewed articulation | No new Rive/Lottie/editor runtime for this painted scene. No voice/lip-sync dependency this version. |

Primary format references: [GIF89a](https://www.w3.org/Graphics/GIF/spec-gif89a.txt),
[PNG third edition / APNG](https://www.w3.org/TR/png/),
[WebP tradeoffs](https://developers.google.com/speed/webp/faq), and
[RIFE paper](https://arxiv.org/abs/2011.06294).
The quality/architecture choices in the final column are project decisions,
not performance guarantees from those specifications.

### Subtitle effects and legibility

Ordinary dialogue uses a reserved lower band, two shaped lines, word-preferred
wrapping with grapheme-safe fallback, a locked Korean font and a short cue fade.
It never covers the hand/food-container or foot/step evidence. Typewriter reveal,
per-glyph bouncing, particle dissolution, scrolling dialogue and karaoke are
not useful defaults for this quiet scene; without voice they would imply an
unverified performance rhythm. Decorative titles can be separately authored
later without changing dialogue source ranges or their timing authority.

An editable composition starts paused and exposes pause/seek controls. Reduced
motion removes decorative transforms/fades while retaining the sequence of
story action states. A downloadable GIF cannot enforce host pause controls or
OS motion preferences; use the controlled HTML/video for review. These choices
follow the distinction between [automatic motion controls](https://www.w3.org/WAI/WCAG20/Understanding/pause-stop-hide.html)
and [interaction motion preferences](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html);
they are not a claim of complete accessibility certification.

## Spatial continuity

For a Korean left-hand-drive coach, use vehicle coordinates: windshield/front,
front-left driver, wheel/dashboard ahead of the driver's forward-facing chair,
front-right passenger entrance/stairs, forward-facing passenger rows and a real
aisle. Every camera is evaluated against that topology. Establish it in a
geometry reference before generating new interior shots. Do not reuse the old
interior layout as an image-generation anchor or approve it by prose alone.

The manufacturer's [Korean Universe catalogue](https://www.hyundai.com/ccontents/carmng/CP00000011/universe-catalog4.pdf)
is an observed cockpit/cabin reference, not artwork to redistribute or a claim
that the fictional vehicle is a particular branded trim.

## Acceptance for implementation

1. Explicit, mechanically verified GIF preview and silent video outputs.
   Preserve native-generation versus rendered-preview provenance.
2. Voices remain deferred. Preserve research/audition evidence without connecting
   an unused speech runtime to the current version.
3. Original dialogue appears in individually timed, readable cues. Missing or
   overflowing material fails explicitly. Future speech must use the same cue
   authority rather than invent another clock.
4. Corrected interior anchor and dependent shots pass actual spatial/visual
   review. Loops never reverse a one-way action such as opening/closing doors.
5. Actual scene E2E, cancellation/failure/unknown evidence and human review remain
   separate. No framework, legacy branch or optional tool is added merely because
   it exists in another repository.

## Implementation evidence at this revision

NVC now validates exact source-range caption cues; SMC uses the resolved browser
cue array for both drawing and VTT. The native renderer publishes a probed GIF
preview with separate rendered provenance alongside its existing video/source
artifacts. No package dependency or generation framework was added.

The full SMC suite passed 229 tests after a real GIF probe failure was fixed:
non-seekable input reports zero stream duration, so the implementation observes
decoded frame delays and timestamps. Story's complete native local gate also
passed, including actual owner/media boundary execution with explicit mechanical
test pixels and simulated test-only reviews. These tests do not certify the
artwork, human approvals or a corrected EP01 S01 film.

The actual corrected Korean-bus anchor was generated without an API key and
adopted by the user only as a spatial reference. The optional LayoutSheet pack
was actually imported in its browser; two unsupported SVG font variants were
retained as failed attempts before a compatible generic SVG imported. New shot
pixels and changed direction remain separate human review subjects. Current
single-scene production status belongs in its private evidence, not in a fake
public completion marker.
