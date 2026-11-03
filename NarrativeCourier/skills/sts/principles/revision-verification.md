# Revision / Verification

- Diagnose before rewriting. Rank `CRITICAL`, `MAJOR`, `MINOR` by story impact rather than checklist count.
- Repair a dependency-aware change-set: owning scene, changed state, protected source obligations and downstream closure.
- Re-run checks on every downstream scene whose precondition consumes a changed postcondition.
- Final reconstruction is source-blind. Infer only what is evidenced by the screenplay and attach scene evidence/confidence.
- Final source comparison occurs only after source and screenplay representations exist independently.
- Alignment claims must point to recovered Screenplay IR IDs and screenplay scene evidence. A Source IR ID, source sentence, or intended adaptation decision alone is not evidence that the screenplay contains a material assertion or relation.
- If the independently recovered screenplay has no evidence for a material source obligation, record absence/uncertainty instead of filling the gap from memory. Protected/CORE omissions and material function loss block `READY`.
- Hard semantic failures cannot be averaged away by craft strength.
