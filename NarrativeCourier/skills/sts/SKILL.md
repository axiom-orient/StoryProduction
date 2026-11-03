---
name: sts
description: Compile a supplied synopsis into a traceable screenplay without silently changing material story meaning, or independently audit an existing screenplay against that synopsis.
---

# STS — Synopsis-to-Screenplay Fidelity Compiler / Auditor

STS is deliberately narrow. It converts a supplied synopsis into an executable screenplay while preserving material story identity, and it can independently audit an existing screenplay against the same semantic contract.

## Authority

1. Human-approved source interpretation and explicit user constraints.
2. `workflow.json` for generation topology and `audit_workflow.json` for independent audit topology.
3. Routed `principles/*.md` and the current task prompt.
4. Runtime state/integrity contracts.
5. `references/` only as research provenance; never as hidden generation instructions.

## Semantic IR

- `SRC-*`: atomic source assertions with actor ownership, polarity, epistemic status, materiality and evidence.
- `REL-*`: material causal, temporal, knowledge, identity, relationship or world-rule relations. Material world rules carry trigger/scope/effect/limit facets; material knowledge carries an acquisition path.
- `GAP-*`: decisions required for screen execution but not determined by source.
- `CON-*`: unresolved source/user/reference authority conflicts.
- `SCN-*`: screenplay scene transition contracts with both necessary function and cinematic value.

STEP 0 also records source-form function, externalization load, ending meaning and `Adaptation Pressure`. Pressure is derived from target scale × source coverage × semantic risk; it never grants invention authority.

## Non-negotiable boundaries

- Do not imitate a filmmaker or activate a filmmaker mode.
- Do not replace the supplied story merely to make it more conventional or dramatic.
- Do not turn ambiguity, belief or allegation into fact without authority.
- Do not treat three acts, fifteen beats, midpoint percentage, scene count, mandatory arc or dialogue ratio as correctness gates.
- A synopsis is compressed: a screenplay may legitimately add local action, dialogue, staging, locations and connective detail inside declared adaptation space. High expansion pressure must be exposed rather than hidden inside invented subplot volume.
- Preserve source-form and semantic-coda **function**, not necessarily literal prose/container; a screen-native equivalent is valid when the same material meaning remains recoverable.
- Final verification must reconstruct the screenplay without access to synopsis, Source IR, treatment, Scene Blueprint or intended adaptation decisions.
- Existing-screenplay audit must independently extract Source IR from the synopsis and Screenplay IR from the screenplay before comparison.

## Direct execution

When the Rust host is unavailable, read `workflow.json`, traverse nodes in order, load only the declared prompts/principles/inputs, stop at Human nodes, and write only the declared artifact. Never infer topology from this prose.

Standalone existing-screenplay audit: `python3 tools/sts_audit.py --synopsis ... --screenplay ... --out ...`.
