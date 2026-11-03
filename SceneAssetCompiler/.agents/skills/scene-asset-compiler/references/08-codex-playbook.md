# Codex Playbook

When `$scene-asset-compiler` triggers:

1. Read only the reference docs needed for the current phase.
2. If no run exists, initialize from RunRequest.
3. If VisualAuthority is missing, derive it from narrative evidence + explicit user direction; mark inventions as visual choices, never narrative facts.
4. If GenerationPlan is missing, plan anchors first, then reusable assets, then scenes. Keep task boundaries independently repairable.
5. Run deterministic validators before any image executor.
6. Process `ready` tasks. Use the Scene Provider only for eligible reference-free PNG `GENERATE` attempts and `$imagegen` for all other pixel work; make one executor call per asset/variant.
7. Persist selected outputs to the exact attempt directory.
8. Mechanical review first, then visual review.
9. On failure, issue targeted repair and preserve everything else.
10. Approve task selections only after both review gates pass.
11. Publish only after every required task is approved.
12. Report paths, failures, skipped checks, and whether PSD/other capability actually existed.

Do not ask for user confirmation between routine repair iterations unless a true creative decision or destructive choice is required.

## When an image looks "good but wrong"

Do not praise or reroll first.

1. Re-open `task.json`, `attempt.json`, and actual PRIMARY output.
2. Finish `visual-review.json` with evidence.
3. Run `audit-template`.
4. For each atomic claim, write what is actually visible; do not infer hidden intent.
5. Mark missing/wrong primary subject, setting, narrative facts, identity, or required structure as MAJOR.
6. Mark a small visual deviation MINOR only when the asset remains usable and the canonical function still reads.
7. Run Doctor.
8. Follow the chosen boundary:
   - local → edit,
   - global → fresh generation from authority,
   - reference conflict → ablation experiment,
   - repeated identity failure → anchor diagnosis/reseed.
9. Re-run the exact same audit categories on the new attempt.

Never let aesthetics hide semantic mismatch.
