# Public source distribution

This snapshot preserves the existing component boundaries and their licenses.
Publication of source is not a claim that all release or live-provider qualification gates pass.

- `geultu/LICENSE`: MIT engine; reserved author corpus/profile are excluded.
- `NarrativeCourier/LICENSE` and `NOTICE.md`: MIT.
- `ImagePromptCompiler/LICENSE` and `NOTICE.md`: BSD-2-Clause. Catalog cards are recorded as project-generated assets in `catalog/CHATGPT_IMAGE_CATALOG_MANIFEST.json`.
- `SceneMotionCompiler/LICENSE`: MIT.
- SAC native and integration manifests declare MIT. No new blanket license is assigned to previously unlicensed files.
- Vendored Rust dependencies retain their upstream legal files. Refinement archives retain their Attribution License, original source and origin records.

The following remain local and are excluded from every public commit:

- `external/screenwriting-skills/`: supplied README permits personal study; public redistribution is unconfirmed.
- `geultu/assets/craft-packs-scriptforge-v4/`: noncommercial/share-alike declaration exists, but upstream attribution and redistribution provenance are insufficiently identified. The optional ScriptForge craft-pack fixture/gate requires this private input. The MIT baseline craft pack and executable engine remain included.
- Author corpus/current profile, credentials, caches and machine metadata.

GitHub Actions are disabled for this repository. Existing workflows inside vendored crates are inert upstream source, outside the root `.github/workflows` location, and have not been modified or run. Verification is local.

Known qualification gaps at initial publication include missing authentic IPC historical image evidence and SAC G09 PSD provenance. Neither gap is filled with generated or mock evidence. SEMI is optional and requires its separately supplied SDK; LayoutSheet is optional and communicates only through `.layoutpack`.
