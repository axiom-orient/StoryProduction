# Audit Stage 1 — Synopsis → Source IR

Read only the synopsis. Extract typed source obligations without knowledge of the screenplay. Keep the source compressed: do not invent missing scene detail.

Return JSON conforming to the supplied schema.

- `assertions` preserve actor/target, polarity, epistemic status, materiality, protection, evidence and narrative function.
- `relations` preserve material causal/temporal/knowledge/identity/relationship/world-rule links.
  - `WORLD_RULE` relations record source-supported trigger, scope, effect, cost, limits/exceptions, reversibility/duration and knowledgeable entities; unknown facets stay unknown.
  - `KNOWS/BELIEVES` relations record source-supported acquisition mode.
- `source_form` records whether the source container is `NEUTRAL|SUPPORTING|ESSENTIAL`, its narrative function, and material non-observable meaning that will need cinematic externalization.
- `ending` separates event outcome, decisive actor(s), semantic reframe and residue/open question.
- Record open adaptation space and ambiguity rather than resolving it.
