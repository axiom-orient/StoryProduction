# STEP 0 — Typed Source IR + Adaptation Preflight

Read the synopsis, config and authorized references. Recover the canonical source interpretation for Human approval. Do not improve the story.

Required sections:

1. `## SRC Assertions` — `SRC-001...`: assertion, actor/target, `AFFIRM|NEGATE`, `FACT|BELIEF|CLAIM|UNKNOWN`, `CORE|SUPPORT|TEXTURE`, evidence, protected yes/no, narrative function.
2. `## REL Relations` — `REL-001...`: `CAUSES|ENABLES|PREVENTS|BEFORE|AFTER|KNOWS|BELIEVES|IDENTITY|RELATIONSHIP|WORLD_RULE`, endpoints, materiality, evidence.
   - A material `WORLD_RULE` must state known facets: trigger, scope, effect, cost/tradeoff, limits/exceptions, reversibility/duration, and who knows it. Unknown facets remain `UNKNOWN`.
   - A material `KNOWS/BELIEVES` relation records acquisition when the source supplies it: `SOURCE_GIVEN|OBSERVED|TOLD|INFERRED|REMEMBERED|UNKNOWN`.
3. `## GAP Open Adaptation Space` — only decisions needed for screen execution that source does not determine.
4. `## CON Authority Conflicts` — conflicting or materially ambiguous source/reference/user claims.
5. `## Ending Contract` — event outcome, decisive actor(s), semantic reframe, and residue/open question. Separate event from interpretation.
6. `## Adaptation Preflight`
   - source form: `NEUTRAL|SUPPORTING|ESSENTIAL` and its narrative function;
   - interiority/externalization load: `LOW|MODERATE|HIGH`;
   - world-rule, epistemic, physical/institutional and ending sensitivity;
   - target-duration pressure: `LOW|MODERATE|HIGH|EXTREME` based on target duration × source coverage × semantic risk;
   - likely screen carriers for material non-observable meaning;
   - expansion/compression risks. High pressure never authorizes invention.
7. `## Fidelity Risks` — concrete places where dramatization could alter identity, knowledge, rule scope, agency, form function or ending meaning.
8. `## Source Coverage Audit` — a lossless, human-reviewable map from every source clause to its SRC/REL obligations.

## Coverage rules

Treat the approved synopsis as the only source for narrative SRC/REL obligations; config and authorized references provide context but cannot silently add story facts. If a reference contributes a new or conflicting story claim, record it in CON and leave it unresolved until a Human-approved source update. Inventory the approved synopsis as evidence, not as a prompt to summarize. Before writing the compact IR, split every sentence into its smallest meaning-bearing claims and relations. In particular:

- Preserve attributed **beliefs and claims** as beliefs/claims by their holder. Do not convert an accusation into an established event or omit it because the plot can proceed without it.
- Preserve explicit **unknowns, denials, absences, negation, and unresolved causes or agents** as source meaning. An explicit unknown is an `SRC` assertion with epistemic kind `UNKNOWN`; it is not an adaptation `GAP` and never authorizes a guess. Mark it `CORE` and protected when it defines the mystery, blame, ending, or a material relationship.
- Split coordinated statements when they express distinct obligations. A paragraph may contain a belief, two times, two independent unknowns, and an ending; one broad SRC line does not stand in for those separate claims.
- Keep each SRC/REL's actor, target, polarity, epistemic kind, materiality, protection and ending consequence explicit. Attach one exact source span to each declaration through `evidence=SPAN-001` metadata.

The coverage audit table is part of the reviewed Source IR:

| Source span | Exact source text | SRC/REL ID | Unmapped reason |
|---|---|---|---|
| SPAN-001 | one exact, contiguous quote copied from the synopsis | SRC-001 | — |

Use a new `SPAN-###` for each smallest meaning-bearing source span. Each row maps exactly one atomic SRC/REL; when a phrase supports two obligations, quote it in two rows with distinct span IDs. Quotes must match the synopsis exactly as UTF-8 text; if wording repeats, include enough surrounding text to identify one unique occurrence. Escape `|` as `\\|` and a literal backslash as `\\\\` for the Markdown table. Cover every non-whitespace source character. Map each SRC/REL exactly once to its evidence span. The only unmapped spans allowed are literal Markdown heading lines, marked `MARKDOWN_HEADING`; narrative text always needs an SRC/REL ID. This table exposes omissions to the Source IR Human gate and lets the runtime reject unsupported citations or uncovered source text. It does not replace the typed SRC/REL inventory.

Write each SRC and REL declaration on its own line with the identifier at the start. Every row must
carry `materiality=CORE|SUPPORT|TEXTURE` and `evidence=SPAN-###`. Each SRC also requires
`polarity=AFFIRM|NEGATE`, `epistemic=FACT|BELIEF|CLAIM|UNKNOWN`, and `protected=yes|no`. Each REL
requires `relation=CAUSES|ENABLES|PREVENTS|BEFORE|AFTER|KNOWS|BELIEVES|IDENTITY|RELATIONSHIP|WORLD_RULE`.
For example: `SRC-001 — ...; polarity=AFFIRM; epistemic=BELIEF; materiality=CORE;
protected=yes; evidence=SPAN-001`. This keeps the approved obligation inventory distinct from prose
and lets the release comparison match IDs without guessing.

Do not propose a better plot. Do not resolve GAP/CON or structural invention without Human authority.
