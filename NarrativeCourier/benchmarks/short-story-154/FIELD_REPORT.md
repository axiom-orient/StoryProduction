# STS 154-Story Field Study

## Decision

STS 4.1's central direction is validated, but the corpus exposed four missing adaptation contracts: **adaptation pressure, explicit world-rule facets, knowledge acquisition, and semantic-form/ending externalization**. STS 4.2 adds those contracts without turning into a prose editor or director-style simulator.


## Validity boundary

This is a retrospective field study, not a blinded end-to-end generation benchmark. The 16/16 high-severity and 11/13 medium-severity figures measure **contract-class correspondence to a prior editorial oracle**, not prospective automatic detection accuracy. `KEEP` stories are not counted as true negatives. The corpus validates where STS should look and exposed missing contracts; final screenplay-quality gain still requires controlled synopsis→screenplay pairs. See `VALIDITY.md`.

## What actually held up

The prior editorial oracle contains concrete defects rather than preference-only notes. Its high/medium failures repeatedly fall into the exact classes STS was designed to protect:

- world-rule scope/timing;
- causal ownership;
- character knowledge boundaries;
- physical/institutional continuity;
- timeline/counting invariants;
- ending meaning.

Examples include 002 (sleep-rule scope + causality), 005 (83-second rule), 008 (private knowledge), 067 (loop knowledge), 076 (operational permission), 092 (age-bank rule), 111 (plate accounting/safety), 123 (lighthouse causal logic), 129 (timeline), 133 (time arithmetic), and 146 (physical continuity).

Across the full v4 editorial ledger, all 16 high-severity revisions and 11 of 13 medium-severity revisions directly map to STS semantic/state/causal contracts. The remaining two medium cases are adjacent dialogue/specificity craft issues. Twelve low-severity revisions are prose naturalness/line editing and should remain outside STS.

## What STS 4.1 missed or under-modeled

### 1. Short source does not determine target workflow

The corpus is microfiction, but the same source can be adapted into a 5-minute short or a feature. Routing by source length would be wrong. STS therefore models `Adaptation Pressure = target scale × source coverage × semantic risk` and treats high pressure as an invention-authority warning.

### 2. WORLD_RULE was too coarse

Many stories depend on one exact speculative rule. A generic `WORLD_RULE` edge is insufficient. The executable contract now tracks trigger, scope, effect, cost, limits, reversibility/duration, and who knows the rule.

### 3. Knowledge needs an acquisition path

A character having correct information is not enough. Material knowledge must be traceable to `SOURCE_GIVEN | OBSERVED | TOLD | INFERRED | REMEMBERED | UNKNOWN`. This directly addresses defect classes seen in 008, 067 and 126.

### 4. Prose meaning must be externalized, not deleted

Many stories end with an interpretive reframe. Earlier editorial work had already shown that a blanket “show, don't tell” rule damages some endings. STS now preserves `event outcome + agency owner + semantic reframe + residue`, while allowing the screenplay to carry the reframe through behavior, image, sound, dialogue, text or justified voice-over.

### 5. Source form can be semantically material

Terms of service, reports, chat, monologue and pseudo-commercial formats can control reveal order or tone. STS now distinguishes `NEUTRAL | SUPPORTING | ESSENTIAL` source-form function. An ESSENTIAL container may change only if a screen-native equivalent performs the same function.

### 6. A necessary scene is not automatically a good scene

Scene IR now records both `Necessary function` and `Cinematic value`. This implements the reusable lesson behind scene-as-desire approaches without copying a filmmaker's style.

## Research cross-check

- Pial et al., EMNLP 2023, *Analyzing Film Adaptation through Narrative Alignment*: source units and screenplay scenes support one-to-many/many-to-one semantic alignment; literal scene equivalence is not required. https://aclanthology.org/2023.emnlp-main.962/
- Chen et al., ACL 2022, *SummScreen*: screenplay plot information is scattered through dialogue and long context, and abstractive systems can invent facts. https://aclanthology.org/2022.acl-long.589/
- Li et al., 2026, *ConWriter*: static requirements, dynamic narrative state, transition constraints and risk-aware repair reduce accumulated long-form inconsistencies. https://arxiv.org/abs/2608.05169
- Tian et al., 2026, *STAGE*: full screenplays benefit from a shared world representation of events, relations and character state. https://arxiv.org/abs/2601.08510
- Zuo et al., 2026, *FilmWorld*: literary-to-cinematic conversion benefits from explicit world construction/evolution and state verification. https://arxiv.org/abs/2607.19038
- Sundance Collab, *Writing Screenplay Adaptations*: adaptation must preserve the source's soul while building a cinematic structure that stands alone. https://collab.sundance.org/catalog/Writing-Screenplay-Adaptations/July-2026
- Sundance Collab, *Crafting Your Short Film Screenplay*: short films vary in structure/length and still require scene craft, behavior, openings/endings and visual storytelling. https://collab.sundance.org/catalog/Crafting-Your-Short-Film-Screenplay/January-2026
- BAFTA, Céline Sciamma Screenwriters' Lecture: logically needed scenes must become scenes worth making; scene count/rhythm is material-specific rather than a universal quota. https://www.bafta.org/media-centre/press-releases/screenwriters-lecture-series-2019-celine-sciamma/
- BAFTA, Bong Joon-ho Screenwriters' Lecture: characters emerge through specific actions/situations; process varies by material. https://www.bafta.org/media-centre/press-releases/screenwriters-lecture-series-2019-bong-joon-ho/
- BAFTA, Peter Straughan Screenwriters' Lecture: an adaptation often must re-imagine interior source material through cinematic means rather than reproduce it literally. https://static.bafta.org/uploads_pre_202411/transcripts/24_oct_2010_screenwriters_lecture_by_peter_straughan.pdf

## Non-adoptions

The field study does **not** add:

- microfiction prose polishing;
- director modes;
- three-act/beat quotas;
- automatic shortening because the source is short;
- mandatory visual-only endings;
- corpus-specific story examples to normal prompts;
- self-mutating rules learned automatically from this corpus.

Those would either broaden the mission or overfit the benchmark.
