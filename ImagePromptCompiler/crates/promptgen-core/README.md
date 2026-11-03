# promptgen-core

Pure image prompt domain library. It owns the six-entry image catalog, typed
category-bound profiles, strict JSON decoding, validation, deterministic
rendering, and the image guided interview.

The public image surface is `ImagePromptRequest`, `compile_image_prompt`,
`image_schema`, `validate_image_request`, `available_lut_presets`, `run_interview`,
and the catalog lookup API (`catalog`, `catalog_json`, `find_entry`, and
`CatalogFamily::parse`/`as_str`). The MCP catalog filter uses the core-owned family parser, which
accepts only canonical family identifiers. Raw LUT resolution structs, catalog-selection diagnostics,
selection metadata builders, and prompt-renderer display formatting remain core implementation details.
Logo identity is an opaque PNG concept contract, and `app_icon` is
a C4-bound generate-only contract for one centered textless subject and one
opaque 1024×1024 PNG master concept. Textless `app_web_ui` requests keep only
visual structure, navigation, state, and component relationships. Reference
images must be explicitly transported by the caller.
