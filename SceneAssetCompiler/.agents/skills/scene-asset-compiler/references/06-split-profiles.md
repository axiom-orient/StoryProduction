# Split Profiles

**Split only when independent control is required.** Hard split reasons: independent motion/deformation, state swap, relative draw-order change, interaction/hit-test, separate material/emission, independent reuse, consumer requirement, or mask/clip boundary. Otherwise merge.

- **L0 Shot:** whole character/object + only needed occluder/emitter.
- **L1 State/Swap:** head/body plus actual expression/costume/prop swaps. Do not split limbs for hypothetical future use.
- **L2 Interactive 2D:** head, torso/pelvis, upper/lower limbs/hands/feet, actual dynamic hair/costume groups, needed face controls. Joint overlap required.
- **L3 Hero/Performance:** add only genuinely controlled pupils/lids/mouth subparts/dynamic hair/accessories/fingers. L3 is opt-in, not default.

Layer count is a cost, not a quality score.
