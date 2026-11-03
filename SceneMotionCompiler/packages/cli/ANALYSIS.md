# SceneMotionCompiler CLI — module ownership

## Boundary

`packages/cli/src/main.js` is the `smc` executable entrypoint declared by the private monorepo root
package. It owns command-line framing, option validation, CLI-to-Application input projection, and
console output only. Application command policy and the authoritative command/option registry remain
in `packages/application`; project state, custody, and approval remain with their respective owners.

## Flow

`argv → CLI_COMMAND_REGISTRY-derived option spec → exact CLI-to-Application command mapping → typed
Application input object → ApplicationCommandService → result/error JSON`.

The CLI derives option names and requiredness from `CLI_COMMAND_REGISTRY`, then projects each accepted
command to its Application input shape. A registered CLI name without an Application mapping is now
an explicit `cli-command-unmapped` failure; it is never forwarded under its raw transport name.
`doctor`, help, and version are CLI-local commands and do not construct an Application service.

The CLI owns no persistent state and does not decide Human approval. Release approval remains absent
from its registry, while Studio Human approval is a separate app entrypoint.

## Evidence boundary

This package contains one source module and has no library barrel. The command-mapping fallback
cleanup and this analysis are source-only. Tests, build, lint, CLI runtime, aggregate gates, and
manifest verification were not run.
