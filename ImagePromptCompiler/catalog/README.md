# Image catalog v4

`image_catalog.json` is the runtime authority for six representative output outcomes. A user selects exactly one outcome. Medium, lighting, palette, surface, composition, text, and references remain typed request fields; they are not parallel catalog categories.

| ID | Outcome | Covers |
|---|---|---|
| C1 | 인물·라이프스타일 | portrait, fashion, profile, lifestyle photography |
| C4 | 제품·브랜드 | product hero, package, brand application, material study |
| C5 | 광고·포스터 | campaign key visual, poster, social banner |
| C6 | 정보·UI | infographic, diagram, presentation, UI mockup |
| C10 | 일러스트·스토리 | illustration, comic, sticker, storyboard |
| C11 | 컨셉·엔터테인먼트 | concept art, cinematic key art, game world |

## Runtime contract

- `schema_version` is `4`.
- `entries` contains exactly the six IDs above and every entry is `tier_1=결과물`.
- Each entry has three executable `prompt_directives` and one PNG `asset_path`.
- `taxonomy.patterns` accepts no runtime selection. A legacy style ID fails closed and the guided interview offers only removal.
- Category-specific interview answers are pruned whenever the outcome changes.

## Files

- `image_catalog.json`: source taxonomy and prompt directives.
- `reference_scenarios.json`: one deterministic representative brief per outcome.
- `assets/category/*.png`: six visually reviewed reference cards.
- `CHATGPT_IMAGE_CATALOG_MANIFEST.json`: byte size, SHA-256, and dimensions for each PNG.
- `REPRESENTATIVE_IMAGE_QA.json`: criterion-level manual visual review bound to final PNG hashes.
- `IMAGE_GENERATION_PROMPTS.md`: generated reference-generation payloads.
- `IMAGE_CATALOG_ANALYSIS.md`: per-card visual QA.
- `EXTERNAL_PROMPT_REVIEW.md`: research and the product-scope decision.
- `EXTENSION_PROPOSALS.md`: evidence required before adding an outcome.

## Update and verification

After editing the taxonomy or scenarios:

```bash
python3 scripts/renew-image-catalog.py
python3 scripts/audit-catalog-assets.py --write
python3 scripts/renew-image-catalog.py --check
python3 scripts/audit-prompts.py --check
python3 scripts/audit-catalog-assets.py
python3 scripts/audit-representative-image-qa.py
```

The authoring script regenerates prompts and the prompt audit report atomically. The asset audit rejects missing, extra, duplicate, corrupt, or manifest-drifted PNGs.
