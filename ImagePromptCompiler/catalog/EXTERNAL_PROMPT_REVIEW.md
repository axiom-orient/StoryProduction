# External image-prompt research and product decision

Reviewed 2026-07-30 with emphasis on current official guidance and actively maintained public examples.

## Sources

- OpenAI’s [image generation prompting guide](https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide) recommends structured control over subject, composition, lighting, palette, text, and exclusions.
- OpenAI’s [image evaluation cookbook](https://developers.openai.com/cookbook/examples/multimodal/image_evals) separates prompt intent from observable output criteria.
- The current Codex [imagegen skill](https://github.com/openai/codex/blob/main/codex-rs/skills/src/assets/samples/imagegen/SKILL.md) uses generation/edit intent, a structured prompt, validation, and focused iteration.
- Public repositories such as [awesome-gptimage2-prompts](https://github.com/erickkkyt/awesome-gptimage2-prompts) and [awesome-gpt-image-2](https://github.com/YouMind-OpenLab/awesome-gpt-image-2) are useful large galleries and search corpora.

## Macro conclusion

Large prompt collections optimize discovery and inspiration. promptGen has a different job: convert one user outcome into a typed, auditable generation contract. Treating hundreds of styles as peer categories produced an ontology with incompatible axes and made the user choose implementation vocabulary before stating the result.

The product therefore keeps six outcome cards and moves style into explicit medium, scene, composition, lighting, palette, surface, text, and reference fields. This preserves expressive range while reducing decision cost and eliminates category/style contradictions.

## Operational findings

Recent Codex reports include a [long-running image-generation timeout](https://github.com/openai/codex/issues/29645) and [rate-limit failure](https://github.com/openai/codex/issues/26731). The adapter therefore keeps explicit timeouts, candidate withholding, one focused repair, receipt binding, and fail-closed publication rather than treating a returned artifact as success.

Research informs the taxonomy and safeguards; it does not substitute for repository tests or a prompt-to-artifact receipt.
