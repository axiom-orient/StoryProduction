# Research Decisions

## Adopted

### Semantic narrative alignment
Pial et al., *Analyzing Film Adaptation through Narrative Alignment* (EMNLP 2023) aligns source units and screenplay scenes without requiring literal one-to-one correspondence. STS therefore evaluates semantic lineage and supports one-to-many, many-to-one and reorder transformations. https://aclanthology.org/2023.emnlp-main.962/

### Full-screenplay world representation
STAGE (2026) represents full screenplays with provenance-linked entities, events, relations and character-centric state. STS therefore reconstructs a source-blind Screenplay IR instead of relying on a single prose summary. https://arxiv.org/abs/2601.08510

### Stateful transition constraints
ConWriter (2026 preprint) uses static requirements, dynamic narrative state, scene transition constraints and risk-aware repair. STS adapts this narrowly as `SCN-*` precondition/action-turn/postcondition/dependency contracts and risk-first review. https://arxiv.org/abs/2608.05169

### Long-form consistency / world-state verification
MAGNET/ATLAS (2026 preprint) reports gains from shared world state and graph-based consistency verification in long-form generation. STS keeps one canonical state/causal model rather than asking each draft part to remember the whole story implicitly. https://arxiv.org/abs/2607.00918

### Reverse summarization is fallible
SummScreen (ACL 2022) shows screenplay/transcript summarization is a long-context inference problem where relevant details are distributed and indirect. STS therefore records evidence/confidence and compares structured IRs; a reverse synopsis alone is not proof. https://aclanthology.org/2022.acl-long.589/

### Screenplay input formats
Fountain’s public syntax defines scene headings, character, dialogue and other screenplay elements in plain text. STS accepts Fountain directly and preserves semantic element boundaries where possible. https://fountain.io/syntax/


### Structured audit execution
STS uses ordinary `codex exec` with a strict JSON Schema, not `codex exec review`. OpenAI's current structured-output examples use ordinary `codex exec --output-schema`; an open Codex issue dated 2026-08-14 reports that `exec review` can accept but ignore `--output-schema` while exiting 0. STS therefore keeps its audit stages on ordinary `exec` and validates the returned JSON again locally. https://github.com/openai/codex/issues/38545

## Rejected or bounded

- Director modes or signature imitation: rejected; provenance is not execution style.
- Universal beat/act/scene quotas: bounded to optional heuristics.
- Literal source-to-scene equality: rejected; synopsis compression makes this systematically wrong.
- One aggregate quality score: rejected; strong craft cannot cancel source drift.
- Evaluator access to generation treatment/blueprint: rejected as answer-key leakage.
- Automatic self-evolution of craft rules: rejected; it can silently mutate source-fidelity semantics. STS domain changes are versioned and Human-reviewed.
- Naive whole-screenplay summary comparison: rejected; reverse inference itself may hallucinate or omit distributed plot evidence.

### 154-story field study (2026-08)
The user's Korean 001–154 microfiction corpus was used as quarantined regression evidence. Of 55 prior editorial revisions, 41 were direct semantic/state/causal contract matches; all 16 high-severity and 11 of 13 medium-severity corrections were direct matches, while the remaining two medium revisions were adjacent dialogue/specificity craft issues. The study added explicit world-rule facets, knowledge acquisition, Adaptation Pressure, source-form function, semantic-coda externalization, and scene `Necessary function` vs `Cinematic value`. Corpus text is not runtime prompt material. See `benchmarks/short-story-154/`. This is a retrospective oracle mapping plus per-story preflight, not a blinded estimate of automatic detection accuracy or an end-to-end screenplay-quality benchmark; see `benchmarks/short-story-154/VALIDITY.md`.

### Adaptation pressure and short-form variability
Sundance Collab's current adaptation course frames adaptation as preserving the source's soul while constructing a cinematic work that stands on its own; its short-film curriculum explicitly treats short form as having varied structures and lengths. STS therefore does not infer target topology from source word count. https://collab.sundance.org/catalog/Writing-Screenplay-Adaptations/July-2026 https://collab.sundance.org/catalog/Crafting-Your-Short-Film-Screenplay/January-2026

### Necessary scene vs desired cinematic scene
Céline Sciamma describes separating scenes the story logically needs from scenes she desires to make, then requiring needed scenes to become desired. STS adopts only the generalized craft consequence: every Scene IR entry records both its necessary story function and the cinematic value of its chosen realization. No stylistic imitation is encoded. https://www.bafta.org/media-centre/press-releases/screenwriters-lecture-series-2019-celine-sciamma/

### Interior source meaning needs cinematic translation
Peter Straughan describes adaptation as re-imagining what prose does through cinema rather than reproducing it exactly. STS therefore treats interiority, source-form function and semantic codas as translation obligations, not as text that must either be copied or deleted. https://static.bafta.org/uploads_pre_202411/transcripts/24_oct_2010_screenwriters_lecture_by_peter_straughan.pdf

### Dynamic cinematic world modeling
FilmWorld (2026 preprint) models novel-to-film generation as construction and evolution of persistent cinematic world entities and evaluates both novel fidelity and cross-scene consistency. STS adopts the narrower implication that literary-to-cinematic conversion must bind source obligations to explicit evolving screen state. https://arxiv.org/abs/2607.19038

