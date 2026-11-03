# SceneMotionCompiler core — responsibility map

## Verdict and boundary

`packages/core` owns SMC-local canonical value operations and schema-registry access. It has no
independent filesystem authority, workflow state, provider effect, or approval role. The monorepo
root package is private and in-tree consumers import the specific owner modules directly.

## Module ownership

| Module | Responsibility | Consumers |
|---|---|---|
| `src/canonical.js` | Canonical JSON projection, canonical string comparison, buffered and async chunk-stream SHA-256 primitives, and canonical hash domains | Application, contracts, adapters, composer, source-integrity, browser-renderer, production |
| `src/schema-registry.js` | Resolve the closed SMC or Track B schema set and validate a value against its producer-owned schema identity | Adapters and Track B application paths |
| `src/errors.js` | Core `ValidationError` used when schema validation normalizes a schema failure | `schema-registry.js` |

## Removed unowned surfaces — source-only

`src/index.js` was an unused wildcard barrel; no SMC source caller consumed it. The current callers
already import `canonical.js` or `schema-registry.js` directly.

`src/fs.js` exposed generic JSON read/write effects with no SMC source caller. Filesystem policy
belongs to bounded owner-specific modules such as `application/scene-asset-compiler-handoff-files.js`
and `adapters/narrative-courier-release-files.js`.

`src/implementation-bundle.js` had no SMC source caller and duplicated inventory/hash concepts
outside the registered source-package owner. Source archive inventory and verification remain in
`packages/source-integrity`; Track B implementation bundles are not inferred from this helper.

These removals change no serialized Story contract or registered command. Node tests, build, lint,
runtime, aggregate release gates, and manifest verification were not run under the active instruction.
