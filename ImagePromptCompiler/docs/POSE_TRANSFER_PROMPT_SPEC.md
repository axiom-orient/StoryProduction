# Pose-transfer prompt contract

## 1. Category boundary

`pose_transfer` generates a deterministic prompt for recreating one reference image's visible pose with one character sheet's identity. It is a workflow category expressed by `render_profile`, not a new visual-output taxonomy ID. The existing C1/C4/C5/C6/C10/C11 category still describes what the final image is; `pose_transfer` describes how reference authority is divided.

The canonical fixture is [`examples/image-pose-transfer.json`](../examples/image-pose-transfer.json).

## 2. Required input

The request must use `task_mode=composite` and exactly two ordered references:

1. `role=pose`: the pose source. It owns visible full-body geometry, body-local left/right, foot contact, weight, joint positions, hand and foot shape, body/head facing, eye gaze, camera, perspective, overlap, and occlusion.
2. `role=subject`: the character sheet. It owns character identity, face geometry, base expression, proportions, hair, wardrobe, colors, materials, line work, and rendering style.

The two roles are exclusive. Do not copy the pose source person's face, clothing, or style. Do not copy the character sheet's default stance, head angle, or gaze.

Exactly one output subject with `count=1` is required. `change_contract.change_only` states the identity/style transfer; `change_contract.preserve` repeats both references' invariants and the output locks.

## 3. Authority and conflict rules

- When written pose text and visible pose-source pixels conflict, the visible pose-source geometry wins.
- Structured scene, output, and safety fields remain authoritative outside pose geometry.
- Body-local left/right must never be converted to viewer-left/viewer-right or mirrored.
- The nearest limbs, visible body side, foreshortening, overlap, and hidden joints are inferred from the source camera, not normalized to a front view.
- The character sheet is final authority for eyes, facial features, and base expression. Only head rotation and gaze direction are transferred from the pose source.
- If a detail is genuinely hidden in the pose source, preserve the visible silhouette and occlusion instead of inventing a more readable canonical technique.

## 4. Compiled prompt structure

The compiler emits these sections in order:

1. purpose and task mode;
2. ordinal input references and exclusive responsibilities;
3. change-only and preserve contract;
4. pose fidelity contract;
5. selected visual outcome directives;
6. scene, subject, composition, camera, light, color, constraints, and output.

The pose fidelity section is compiler-owned. User strings are quoted as data and cannot replace its headings or relax its locks.

## 5. Fidelity gates

Evaluation and correction use the first failing gate. A later gate cannot turn an earlier failure into a pass.

### Gate 1 — whole action

Compare the full silhouette before local details:

- both feet's location, contact, angle, and spacing;
- stance width and depth;
- support leg and front/rear weight emphasis;
- pelvis and torso rotation and lean;
- full-body scale, center, and action phase.

If this gate fails, strengthen joint-chain, stance/center, and silhouette constraints. Do not spend the correction on face or rendering polish.

### Gate 2 — limb and hand/foot detail

After Gate 1 passes, compare:

- shoulder → elbow → wrist → hand chains;
- hip → knee → ankle → foot chains;
- executing limb and supporting limb roles;
- hand shape, palm/back-of-hand orientation, fist/open state;
- toe direction, knee tracking, overlap, and foreshortening.

If this gate fails, correct only the mismatching chain or terminal shape while preserving the accepted stance and silhouette.

### Gate 3 — facing and gaze

After Gates 1 and 2 pass, compare:

- body-facing angle and visible side;
- head rotation and tilt;
- eye-gaze direction;
- source-camera view and nearest limbs.

A correct technique with the wrong facing or gaze still fails. Correct direction and gaze without mirroring the body or changing the accepted limb geometry.

### Identity lock

Identity is checked alongside every gate:

- face and base expression match the character sheet;
- body proportions, hair, wardrobe, colors, and rendering style remain consistent;
- pose correction must not drift identity;
- identity correction must not reset the pose to the sheet's default stance.

## 6. Prompt-authoring checklist

Before compilation, record observable facts rather than technique names alone:

- action phase: preparation, impact, completed technique, recovery;
- character-local left/right for front foot, executing side, hand, and limb;
- support and weight emphasis;
- foot placement, stance depth/width, and toe direction;
- shoulder/elbow/wrist/hand and hip/knee/ankle/foot geometry;
- hand shape and target height;
- body-facing, visible body side, head angle, eye gaze, and nearest limbs;
- framing, full-body inclusion, scale, camera, and output constraints.

Technique labels may clarify intent but never replace visible geometry. Avoid vague instructions such as "same pose", "dynamic", or "looking sideways" when the structured fields can describe observable alignment.

## 7. Correction strategy

Keep the best accepted candidate and revise one axis at a time:

1. wrong action or stance → joint-chain lock, then silhouette/center lock;
2. correct action but wrong limb, hand, or foot → targeted executing/supporting limb detail lock;
3. correct pose and detail but wrong facing or gaze → body-facing, head, gaze, camera, and body-local left/right lock;
4. identity drift → character-sheet identity lock while preserving all accepted pose gates.

Never request a broad "make it closer" rewrite. State the earliest failed gate, the observable difference, the single correction, and the locks that must remain unchanged.

## 8. Execution boundary

```bash
promptgen image \
  --input examples/image-pose-transfer.json \
  --mode prompt-only \
  --format text
```

The current Codex adapter has no verified reference-image transport. Therefore this category compiles and validates prompts but `codex-imagegen` execution remains fail-closed. Supplying local image paths inside prompt text does not attach those images and does not satisfy the reference contract.

## 9. Verification

```bash
cargo test -p promptgen-core image:: --locked --offline
cargo test -p promptgen-cli --test cli --locked --offline
python3 scripts/audit-prompts.py --check
./scripts/verify.sh
```
