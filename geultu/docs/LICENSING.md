# Geultu licensing scope

The repository's [LICENSE](../LICENSE) defines three separate material scopes; the absence of Cargo's `license` package metadata does not change or replace that file.

- **Section A — MIT:** the reusable engine, executable, skills, tests, release tooling, configuration, documentation, and the listed resources. This is an explicit license for the Section A material, not a license for every file in the repository.
- **Section B — reserved rights:** `corpus/` and `profiles/author-current.json`. Those materials are not authorized for redistribution, commercial use, adaptation, or model training without separate written permission; personal, non-commercial inspection of repository structure and behavior is permitted. Section A software may be used with a user's own corpus and profile, subject to that user's rights.
- **Section C — third-party software:** files under `vendor/` retain their upstream copyright notices and licenses; the Geultu license does not relicense them.

Of Geultu craft-pack inputs, the runtime bundle at the 2026-10-06 r2 qualification includes only the separately authored [Story baseline pack](../assets/craft-packs-story-baseline-v1/LICENSE) under its own MIT license. It does not include the original NC craft pack or the language-refinement archives; their omission from the bundle is not a new license decision for the source-tree materials.

Geultu's Section A license is explicit. **Repository-wide commercial redistribution remains unqualified** because other components' license/notice coverage and third-party/dependency rights, including SDK-derived bindings, have not been established. The bundle manifest records `commercialRedistribution: UNKNOWN`; it does not grant rights or confer a distribution signature. See the integration [verification status](../../integration/docs/IMPLEMENTATION_STATUS.md) and SEMI [third-party notices](../../integration/macos-provider/NOTICE.md) for their separate evidence.
