# 154-Story Field Evidence

This directory is **quarantined evaluation evidence**. It is not loaded by normal STS execution.

The corpus was the user's Korean 001–154 Story Polish v4 collection. Story bodies are intentionally not copied here; the ledger stores IDs, titles, hashes, structural measurements, adaptation-risk lenses and the prior editorial oracle.

Purpose:

1. test whether STS principles correspond to defects that occurred in real short narratives;
2. identify systematic adaptation risks not adequately modeled by STS 4.1;
3. prevent overfitting: corpus-specific observations may become runtime rules only when they generalize to the synopsis→screenplay problem.

Key result from the prior editorial oracle:

- 55 / 154 stories had revisions;
- 41 / 55 revisions directly matched STS semantic/state/causal contracts; 2 additional medium-severity revisions were adjacent craft issues;
- all 16 high-severity revisions and 11 of 13 medium-severity revisions directly mapped to STS semantic contracts; the remaining two medium cases were adjacent dialogue/specificity craft issues;
- the remaining 12 revised stories were prose-level naturalness/line-edit issues and remain outside STS's mission.

This is evidence that the fidelity/state/causality lenses are useful. It is **not** evidence that the current generator would automatically have found every defect without a controlled model run.

## Per-story evidence

- `PREFLIGHT_INDEX.md` routes to 154 individual preflight cards.
- `VALIDITY.md` defines what this retrospective study can and cannot claim.
- Each card leaves Adaptation Pressure unresolved until target duration/format is provided.
