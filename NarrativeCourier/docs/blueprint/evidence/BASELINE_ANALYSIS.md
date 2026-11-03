# Baseline Analysis

## Trunk
`STS-NarrativeCourier-0.10.0-STS-4.2.0-field-validated.zip` → STS authority/runtime/UI baseline.

## Reference
`Narrative-Visualization-Compiler-2.0.0-release-verified.zip` → visualization architecture evidence only.

## Reuse decision
STS 4.2 already owns source/narrative/scene semantics. Recomputing NVC StoryIR/MeaningSpec/SceneBundleIR would create double interpretation and drift risk. Therefore DirectorIR consumes STS authority refs directly.

Phase 1 code confirms the first clean-break step: shared technical primitives were extracted without moving STS-specific workflow semantics into Runtime.
