# Agent Contract

## Truth
`user request → 5 canonical schemas → actual code/tests → these docs → historical archives`.

## Architecture invariants
- STS owns Narrative meaning.
- DirectorIR owns projection only.
- Runtime stays domain-neutral.
- Renderer/Agent never publish canonical project state directly.
- Never add a durable contract/stage/interface for hypothetical variation.
- Never hide capability mismatch with silent fallback.

## Change bar
Every implementation change must identify owner, contract, call/wiring, failure path, and regression.
