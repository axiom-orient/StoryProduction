# SceneMotionCompiler adapters — responsibility map

## Verdict·owner boundary

`packages/adapters` verifies producer-owned release bytes and projects them into SMC-owned
runtime graphs. It does not own the narrative, approve cinematic intent, or write SceneAssetCompiler
asset approvals. Direction is **MAINTAIN + split pure projection from file custody**.

## Module boundaries

| Module | Responsibility | Boundary retained |
|---|---|---|
| `src/narrative-courier-release.js` | Consumer entrypoint for Completed release ingress, durable projection publication, current projection loading, approved-projection checks, and adapter-owned typed errors | Exports only operations consumed by Application/Production; the pure graph builder remains in its defining projection module |
| `src/narrative-scene-projection.js` | Pure SceneGraph projection, semantic occurrence validation, receipt/graph binding, rebuild-from-capsule approval check, and `deriveSmcNarrativeIdentifier` | Preserves canonical NVC IDs within the SceneGraph limit; derives other internal IDs from the full raw-ID SHA-256 while retaining the exact `sourceNarrativeId` |
| `src/narrative-courier-release-files.js` | Declared-root validation, bounded/hash-checked JSON reads, symlink-safe path resolution, generation directories, atomic file publication | File effects stay inside the declared SMC project root |
| `src/narrative-courier-release-support.js` | Adapter-owned release error type | Raw byte digest primitives come from SMC `core/canonical.js`; no dependency on Application error classes or shared Story-domain authority |
| `src/cinematic-contract.js` | Validation of continuity bible, DirectorPlan, and approval against NVC scene facts | SMC consumes these producer artifacts; it does not generate or adopt them |

`src/index.js` was an unused forwarding barrel: application and production callers import the
`narrative-courier-release.js` entrypoint directly. The barrel is removed; the pure graph builder is
imported from `narrative-scene-projection.js` by its owner and white-box fixtures instead of being
forwarded through the workflow entrypoint.

Adapter failures use `NarrativeCourierReleaseError`, owned by this package and exported from the
public release entrypoint for consumer-boundary mapping. Application maps that typed value into its
command envelope; the adapter does not import the application error class, and consumers do not
reach into the private support module.

## Flow and failure ownership

`Completed NVC release → release-files bounded/contained reads → producer identity/hash checks →
pure SceneGraph projection → release-files staged graph → stable projection receipt`. The read path
follows only that receipt, rechecks raw bytes and hashes, rebuilds the expected graph from its inline
source capsule, and rejects any graph drift. Errors retain the existing `ApplicationCommandError`
codes at the adapter boundary.

The three-module separation was revalidated in the earlier source layout: all 227 Node tests,
the public owner-chain CLI E2E, the root deterministic gate, and the final SMC source manifest pass.
The E2E uses immutable local fixtures. Real NVC authoring quality, human cinematic approval, locked
Chromium, packaged custody publication, and external media generation remain separately qualified.
