# Audit Stage 2 — Screenplay → Source-Blind Screenplay IR

Read only the screenplay. Reconstruct the story and internal state actually evidenced on the page. Do not guess what synopsis it might be adapting.

Return JSON conforming to the supplied schema. Use stable audit-local scene IDs and evidence excerpts.

Recover:
- actor ownership, polarity and fact/belief/claim distinctions;
- causal/temporal/knowledge/identity/relationship/world-rule relations;
- material knowledge acquisition events;
- world-rule behavior actually demonstrated on screen;
- ending event, decisive actor(s), recoverable semantic reframe and residue;
- material screen-form functions (e.g. UI/text/VO/document/chat/montage) when they carry narrative meaning;
- material scene transition state and internal contradictions.

Report uncertainty rather than filling gaps. Never infer source intent.
