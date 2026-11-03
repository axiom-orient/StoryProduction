# Verification scripts — module ownership

## Aggregate verifier boundary

aggregate-verifier.mjs remains the CLI entrypoint and public JavaScript API. It registers process
signals, orders aggregate gates, reconciles source identity, and publishes the final summary. Product
command contracts stay in their existing gates.

| Module | Responsibility |
|---|---|
| aggregate-verifier.mjs | Stable CLI/public facade, gate ordering/fail-fast state, summary construction/publication, and signal registration |
| aggregate-verifier/configuration.mjs | Closed gate registry, default timings/log caps, environment override validation, and caller-supplied gate normalization |
| aggregate-verifier/gate-runner.mjs | One gate's child launch, output/timeout/cancel arbitration, exit classification, and cleanup/log result composition |
| aggregate-verifier/source-identity.mjs | Bounded lstat-guarded SOURCE_MANIFEST read, before/after file identity comparison, manifest verification, bounded mismatch projection, and PASS/FAIL source identity result |
| aggregate-verifier/workspace.mjs | Root/output path containment, symlink refusal, output directory ownership, lock acquire/release, and atomic summary write |
| aggregate-verifier/process-group.mjs | Child process-group probe, TERM/KILL escalation, bounded absence wait, and structured cleanup evidence |
| aggregate-verifier/log-capture.mjs | Bounded stream capture, truncation signaling, readable close handling, and closed log file identity/size verification |
| aggregate-verifier/errors.mjs | One shared typed VerifyError class used by configuration, workspace, source identity, and aggregate orchestration |

The facade continues to export SUMMARY_SCHEMA_VERSION, default timing/log constants, SOURCE_MANIFEST_RELATIVE_PATH,
DEFAULT_GATE_NAMES, VerifyError, createDefaultGates, resolveGateTimings, and runAggregateVerification.
validateGates remains internal to configuration and is consumed by the facade.

## Source-only refactor state

The policy, source-identity, workspace/lock, process-group, log-capture, and per-gate runner
responsibilities were extracted from the large entrypoint while preserving the named facade exports and
CLI path. The change is intentionally not claimed as behavior-preserving proof: Node tests, build, lint,
runtime, aggregate gates, and SOURCE_MANIFEST regeneration/verification are NOT_RUN for this graph.
The checked-in source manifest no longer covers the new internal modules until that requested
verification window is reopened.

The remaining aggregate owner is runAggregateVerification: summary state, fail-fast ordering, source
identity timing, and final publication. These responsibilities remain together because gate status,
identity proof, lock release, and durable summary outcome are causally ordered.
