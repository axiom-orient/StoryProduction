# Audit Stage 3 — Semantic Narrative Alignment

You receive only independent Source IR and Screenplay IR JSON. Compare them semantically.

Do not require literal one-to-one scene matching. A source obligation may span multiple scenes; multiple source units may be combined; presentation order and source container may change. Distinguish legitimate dramatization/connective detail from material drift.

Produce all fixed fidelity dimensions:
`SOURCE_ASSERTIONS`, `SOURCE_RELATIONS`, `ENDING_OUTCOME`, `ENDING_MEANING`, `ACTOR_OWNERSHIP`, `KNOWLEDGE_EPISTEMIC`, `FORM_FUNCTION`, `TRANSFORMATION_INVENTION`.

Produce all fixed integrity dimensions:
`CAUSALITY`, `KNOWLEDGE_STATE`, `CHARACTER_RELATIONSHIP_STATE`, `PHYSICAL_SPATIAL_STATE`, `SCENE_DEPENDENCIES`, `PLAYABILITY`, `SCENE_ECONOMY`.

Every material source assertion and relation gets exactly one alignment row with recovered evidence or an explicit absence/uncertainty.

Hard failures include: protected/CORE source assertion or relation drift/omission, ending outcome drift, material ending-meaning loss, decisive actor reassignment that changes meaning, polarity or epistemic drift, loss of an ESSENTIAL source-form function without a screen-equivalent, undeclared structural invention, story-changing causal break, impossible material transition chain, and material round-trip source loss. A hard failure makes verdict `BLOCKED`. Material ambiguity that prevents a safe verdict makes `REVIEW`. Otherwise `PASS`.

Do not penalize a screenplay for replacing prose interpretation with action/image/sound/dialogue/VO/text when the same material meaning remains recoverable. Never collapse the two axes into one quality score.
