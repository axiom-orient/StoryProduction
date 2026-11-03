# Production Recipes — Best Known SceneAssetCompiler Method

These are workflow recipes, not fixed visual styles. Use the smallest recipe that satisfies the asset's real purpose.

## 1. Recurring Character Anchor

Goal: establish identity authority before scene production.

1. Define identity invariants: face proportions, silhouette, age, hair shape, distinctive marks, canonical palette/costume construction.
2. Use a neutral full-body reference view first: low-distortion perspective, readable focus depth, broad neutral/motivated light, no dramatic color cast, no scenery unless identity requires it.
3. Keep the first generation simple. Do not mix action pose, environment, typography, complex props, and hero lighting into the identity anchor.
4. Review identity, silhouette, palette, material treatment, crop/padding, and style system.
5. Approve one anchor.
6. Derive side/back/3-quarter/expression views individually when needed, using the approved anchor as IDENTITY reference.
7. Assemble a character sheet for review; do not make a single multi-cell sheet the only canonical source.

Best failure response: edit only the drifting identity feature; full regeneration only when the base identity broadly collapses.

## 2. Prop Anchor

Goal: stabilize a recurring object's design/material language.

- neutral readable angle and silhouette,
- exact dimensions/proportional cues when narratively relevant,
- concrete material response,
- no hand/character unless needed to establish scale,
- separate emission/movable parts only when the runtime needs them.

Approve before using the prop in key visuals.

## 3. Reusable Environment

Goal: create a world asset, not a one-off beauty shot.

1. Start from protected world facts and navigation/architectural logic.
2. Define foreground/midground/background and important occluders.
3. Avoid characters and temporary story clutter unless the environment asset explicitly owns them.
4. Treat lighting state separately when the same location must support day/night or emotional variants.
5. Preserve stable landmarks/materials across variants.

Environment reference role is `ENVIRONMENT`, not STYLE or COMPOSITION unless it actually controls those axes.

## 4. Key Visual / Story Scene

Recommended order:

```text
narrative function
→ emotional intent
→ first read / focal hierarchy
→ blocking and composition
→ camera / lens behavior / focus depth
→ motivated light
→ color-script beat / value hierarchy
→ material + atmosphere
→ role-isolated approved references
→ constraints / avoid
```

Use approved character/prop anchors as explicit `TASK_OUTPUT` references. A dependency alone must never silently become an image input.

At thumbnail size the intended first read must survive. If it does not, remove competing contrast/detail before adding detail.

## 5. Photoreal / Cinematic Still

Specify physical photographic decisions only when useful:
- shot size and camera position,
- perspective/lens behavior rather than arbitrary lens-number fetishism,
- focus depth / depth of field,
- believable motivated sources and source size/softness,
- surface response: roughness, wear, weave, scratches, moisture, translucency,
- plausible asymmetry/imperfection when the concept calls for lived reality.

Do not rely on “cinematic, 8K, masterpiece.” The image should be cinematic because the shot is directed, not because the adjective is present.

## 6. Comic / Anime / Watercolor / Stylized Illustration

Style is a system:
- shape abstraction,
- edge/line hierarchy,
- value/shading grammar,
- texture/medium behavior,
- palette,
- light logic,
- material abstraction,
- camera/composition behavior,
- detail density.

Do not accidentally request photoreal material cues that contradict the chosen abstraction. For watercolor, for example, pigment granulation/paper tooth may be relevant while photographic micro-specular realism is not.

## 7. Layered PSD / Interactive 2D Asset

Generation is construction, not segmentation.

1. Choose the lowest valid split profile L0–L3.
2. Split only independent motion/deformation/state/z-order/interaction/material/reuse/mask boundaries.
3. Use exact semantic leaf IDs.
4. For L2/L3, construct hidden joint overlap inside the asset so motion does not reveal holes.
5. Keep shadow/emitter/effects separate only when independently controlled.
6. Validate actual PSD structure and then visually verify semantic separation; layer names alone cannot prove correct pixels.

## 8. Revision Ladder

When a candidate is mostly correct, preserve it. Repair in the narrowest applicable axis:

1. identity,
2. narrative state,
3. composition/first read,
4. lighting/color,
5. material,
6. semantic structure/articulation,
7. production usability.

The order is not mandatory; only failed axes are edited. Never reroll already-passing decisions simply to “see if it gets better.”

## 9. Exploration vs Production

Exploration may generate a few genuinely different compositions. Once a candidate satisfies the core brief, switch to edit mode. Production rewards locked decisions and local repair, not endless variation.
