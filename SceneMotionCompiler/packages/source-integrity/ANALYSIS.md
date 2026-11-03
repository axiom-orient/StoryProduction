# Source Integrity — module ownership

## Purpose and public boundary

This package inventories and hashes the SMC source tree, creates deterministic source ZIPs, verifies
their embedded manifests, and can run an extracted package through a clean npm install and the SMC
aggregate verifier.

packages/source-integrity/src/index.js is the package barrel and names its exports directly from
the defining owner modules. It exposes `runDoctor` plus the stable source-package API:
`DEFAULT_SOURCE_PACKAGE_LIMITS`, `collectSourceFiles`, `buildSourceManifest`, `verifySourceManifest`,
`packageSourceTree`, `verifySourcePackage`, and `verifyCleanSourcePackage`. Callers stay on the
existing barrel; the removed `source-package.js` intermediate re-export had no direct caller. Child
modules own implementation details, and future helper exports do not leak through wildcard
forwarding.

## Owner split

| Module | Responsibility |
|---|---|
| source-package/limits.js | Excluded-directory and generated-metadata sets, default bounds, package-name validation, and limit normalization |
| source-package/manifest.js | Symlink-safe source traversal, bounded file reads, deterministic file records, canonical tree hash, and manifest verification |
| source-package/filesystem.js | Bounded no-follow reads, output type checks, empty extraction checks, and file/directory sync |
| source-package/archive.js | Deterministic ZIP creation, ZIP structural/path validation, extraction, and embedded manifest verification |
| source-package/clean-verification.js | OS-temporary extraction, separation from the source tree, fresh npm cache/install, aggregate verification, and workspace cleanup |

The archive owner consumes the deterministic ZIP writer from packages/export. It does not own npm
policy, application/source authority, or Story-domain state. Clean install and aggregate verification
remain separate from source identity verification.

## Current callers and verification

scripts/source-manifest.mjs builds the SMC source identity; scripts/verify-source-tree.mjs checks it.
scripts/package-source.mjs and scripts/verify-source-package.mjs expose archive operations.
scripts/run-source-integrity.mjs is the deterministic package/extract smoke. The explicit
scripts/verify-clean-source-package.mjs command owns clean npm install plus npm run verify.

Current-source evidence on 2026-09-24:

- The current SOURCE_MANIFEST.json verifies 222 source files.
- node --test --test-concurrency=1 tests/*.test.js passes 227/227.
- run-source-integrity.mjs created two byte-identical ZIPs, extracted one into a temporary directory,
  and verified the embedded manifest on the current source snapshot.
- Clean npm install plus aggregate verification is NOT_RUN. The deterministic extraction smoke does not
  establish that clean-install gate.

The root StoryProduction owner-chain E2E separately passed through SMC HTML compilation with fixture
inputs. Locked Chromium evidence measurement, packaged native custody-helper lifecycle, and Studio
human approval/release remain outside that result.
