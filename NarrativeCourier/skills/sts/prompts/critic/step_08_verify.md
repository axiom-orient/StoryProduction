# STEP 8D — Independent Fidelity Comparison

Compare only the Human-approved Source IR and independently recovered Screenplay IR. Do not consult
treatment, Narrative IR, Scene Blueprint, diagnosis or intended transformation ledger.
Treat the Source Coverage Audit as a traceability map, not as extra semantic authority. If a quoted
source span contains material meaning with no corresponding SRC/REL obligation, return `BLOCKED` and
request a Human-approved Source IR revision; do not invent a source ID during STEP 8D.

## Dimensions

Evaluate separately:

- **Fidelity:** source assertions and relations, ending outcome and meaning, decisive actor ownership,
  knowledge/epistemic status, source-form function, invention and transformation discipline.
- **Integrity:** causality, knowledge state, character/relationship state, physical/spatial state, scene
  dependencies, playability and economy.

## Required evidence alignment

For every material SRC and REL, provide exactly one alignment row with these columns:

| Source ID | State | Recovered Screenplay IR IDs | Screenplay scene evidence | Rationale |
|---|---|---|---|---|

Cover every `CORE`/`SUPPORT` source ID and every protected assertion exactly once. `TEXTURE` rows may
be omitted unless protected. Do not add an ID that is absent from the Source IR. Use the recovered
ID prefix that matches the row: `SRC-*` cites `OBS-*`; `REL-*` cites `OREL-*`. Each cited scene must
appear in the cited recovered item’s `evidence=SCN-*` record. Do not place an unescaped `|` inside a
table cell.

Allowed states are `PRESERVED`, `DRAMATIZED`, `COMBINED`, `REORDERED`, `OMITTED`, `DRIFTED`, and
`UNCERTAIN`. A row may cite multiple `OBS-*`/`OREL-*` IDs; one Screenplay IR item may support more than
one source row. Cite the `SCN-*` evidence retained by the independent Screenplay IR.

Only independently recovered Screenplay IR IDs and their screenplay evidence can support a
`PRESERVED`, `DRAMATIZED`, `COMBINED`, or `REORDERED` conclusion. A Source IR ID, source sentence,
remembered synopsis fact, or intended adaptation decision alone does not show that the screenplay
contains the obligation. If no recovered item supports a material SRC/REL, cite `—`, record the
absence or uncertainty explicitly, and do not infer presence from the source.

Do not require literal prose or literal document form when behavior, image, sound, dialogue, VO, text,
or another screen-native carrier preserves the same material function. A semantic coda is preserved
when its material reframe is recoverable from cited screenplay evidence, not when its sentence is
copied.

## Hard failures and decision

Hard failures include protected assertion/relation drift or omission, ending outcome or material
ending-meaning drift, actor reassignment that changes meaning, polarity/epistemic drift, loss of an
ESSENTIAL source-form function without equivalent evidence, undeclared structural invention,
story-changing causal break, impossible transition chain, or material round-trip loss. Material
ambiguity that prevents a safe verdict remains uncertain; craft strength cannot compensate for a
hard semantic failure.

End with exactly one line:

`STS_STATUS: READY` if no hard failure remains and all material uncertainties are acceptable;
or `STS_STATUS: BLOCKED` otherwise.

The program rejects incomplete or duplicate source coverage, unknown recovered IDs, wrong-kind
citations, scene citations absent from the recovered item, and a `READY` verdict with unresolved
uncertainty or protected/CORE omission or drift. Treat these as hard output-contract requirements.
