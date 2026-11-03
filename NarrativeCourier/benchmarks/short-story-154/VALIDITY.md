# Field Study Validity

## Question

Does STS focus on failure classes that actually occur when compact stories are prepared for faithful screen adaptation, and did the 154-story corpus expose missing contracts worth adding?

## Design

This is a **retrospective oracle mapping + story-by-story adaptation preflight**, not a prospective model benchmark.

- Corpus: Korean Story Polish v4, IDs 001–154.
- Canonical package recorded by the prior validation: `story_new_korean_001-154_story-polish-v4.zip`, SHA-256 `4fe26d8931d0115793c47f78e1edcb0b179a48466df46df7b1a0f770f223f874`.
- Oracle: the prior v4 editorial ledger, produced before this STS 4.2 field revision.
- Per-story application: `story-field-ledger.csv` + `preflight/001.md` … `preflight/154.md`.
- Corpus story bodies are not copied into this benchmark directory; hashes bind each row/card to the inspected source.

## Supported conclusions

1. **Defect-class relevance is supported.** 41 of 55 prior revisions directly map to STS semantic/state/causal contracts. All 16 high-severity cases and 11 of 13 medium-severity cases are direct matches; the other two medium cases are adjacent dialogue/specificity craft.
2. **Scope discipline is supported.** Twelve revised stories are primarily prose-naturalness/line-edit changes. They remain outside STS rather than being used to broaden the system.
3. **Repeated modeling gaps were found.** WORLD_RULE facets, knowledge acquisition, semantic ending/form preservation and target-dependent Adaptation Pressure recur across independent stories and justify STS 4.2 contracts.
4. **Short source length is not a routing signal.** Target duration/format must be supplied before expansion pressure is resolved.

## Unsupported conclusions

This study does **not** establish:

- prospective precision/recall of an automatic STS defect detector;
- that STS would have discovered every oracle defect without human/editorial evidence;
- that a screenplay generated with STS 4.2 is better than one generated with 4.1;
- audience preference, production quality, box-office quality or director-like quality;
- causality between STS principles and historical v4 edits.

## Threats to validity

- Mapping is retrospective and not blinded.
- `KEEP` is not a negative-label gold standard.
- Risk lenses and STS value are analyst judgments, not automated metrics.
- The corpus is compact prose, not a controlled set of synopsis→screenplay pairs.
- The prior editorial lineage already used related continuity/state reasoning, so the historical oracle is not fully independent of the broader design lineage.

## Release rule

Only recurrent observations that generalize to **synopsis→screenplay fidelity** may enter runtime contracts. Corpus-specific story details remain quarantined. New rules must also pass adversarial schema/eval tests.
