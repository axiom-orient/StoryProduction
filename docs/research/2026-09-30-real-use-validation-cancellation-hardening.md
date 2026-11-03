# 2026-09-30 current-source real-use validation

## Verdict

The verification run is **DONE**. Current local code, integration gates, and the public text-authoring
path were exercised. Product qualification remains **PARTIAL**: the synthetic screenplay lost two
source facts, external providers and real human approvals were not used, the NarrativeCourier image
renderer was unavailable, and ImagePromptCompiler's official gate stops at a missing historical
image-evidence input.

The repository has no Git metadata, so there is no commit ID. Commands ran against
`/Users/ax/repository/StoryProduction` on Python 3.12.14, `jsonschema` 4.26.0, Node 24.19.0, and
Rust/Cargo 1.97.1. Evidence logs and QA workspaces are under
`/private/tmp/storyproduction-realuse-current-after-cancel/`.

## Verification results

| Boundary | Result | Evidence |
|---|---|---|
| NarrativeCourier | **PASS** | `bash NarrativeCourier/verify-all.sh --require-rust`: 68 Rust tests, 60 contract checks, 112 Delivery/source checks, 14 STS audit tests, `cargo check`, fmt, and Clippy. Its 505-entry source manifest passed freshness verification with SHA-256 `8cafe051f54781662664a3d7188a6c4bb44e58ca87d37c43cfcc4ba22623e842`. [Gate log](</private/tmp/storyproduction-realuse-current-after-cancel/logs/nvc-verify-final2.log>) · [Manifest verification](</private/tmp/storyproduction-realuse-current-after-cancel/logs/nvc-manifest-verify-final.log>) |
| Root integration | **PASS** | Final `bash integration/scripts/verify-local.sh`: 29 Python integration tests, NVC contracts and SceneMap checks, 27 SMC↔SAC Node boundary tests, SAC source/skill manifests, and IPC source-snapshot regression. [Final gate log](</private/tmp/storyproduction-realuse-current-after-cancel/logs/root-integration-gate-final.log>) |
| Geultu | **PASS** | `bash geultu/scripts/verify.sh`: 96 Rust tests, fmt/Clippy, release build, 146 canonical checks, CLI smoke, MCP stdio smoke, and local Studio lifecycle smoke. [Gate log](</private/tmp/storyproduction-realuse-current-after-cancel/logs/geultu-verify.log>) |
| SceneMotionCompiler | **PASS · LOCAL SUITE** | The package script body `node --test tests/*.test.js` passed 227/227 tests. The post-report source manifest passed at 242 files / 2,609,455 bytes with tree hash `sha256:9a9497125bafc1b8afec1d13db2ca7eb5543f20ed89bcb43d0160cce4850b409`. Native `npm` was unavailable, so the declared Node test command was run directly. [Suite log](</private/tmp/storyproduction-realuse-current-after-cancel/logs/smc-node-tests-final.log>) · [Manifest verification](</private/tmp/storyproduction-realuse-current-after-cancel/logs/smc-manifest-verify-final.log>) |
| SceneAssetCompiler | **PASS · BOUNDARY SUITE** | `python3 -m unittest discover -s tests -p "test_*.py" -v` passed 14/14. The root gate also passed its skill/docs/release manifest checks, and the public owner-chain exercised Scene Release. [Test log](</private/tmp/storyproduction-realuse-current-after-cancel/logs/sac-python-tests.log>) |
| ImagePromptCompiler | **PARTIAL** | Full workspace check/tests/doc tests/Clippy/docs/release build passed; 273 Rust tests passed. Its inspected source snapshot is revision `promptgen-1.5.0-20260810.1`, 271 files / 17,377,142 bytes, tree hash `5258d309f8cc17cbfb080c27d58a65d8adf145e6888ae271f164954de0e36455` (fingerprint only; no receipt). The official `scripts/verify.sh` stopped in `audit-image-evidence.py` because `evals/dogfood/historical/1.4.0/image/image-evidence.json` is absent. No evidence or source-identity receipt was fabricated. Current CLI and loopback Web smokes plus documentation/catalog/structure/source-snapshot audits passed separately. [Official gate log](</private/tmp/storyproduction-realuse-current-after-cancel/logs/imageprompt-verify-final2.log>) · [Source fingerprint](</private/tmp/storyproduction-realuse-current-after-cancel/logs/imageprompt-source-fingerprint.json>) · [CLI smoke](</private/tmp/storyproduction-realuse-current-after-cancel/logs/imageprompt-cli-smoke.log>) · [Web smoke](</private/tmp/storyproduction-realuse-current-after-cancel/logs/imageprompt-web-smoke.log>) · [Other local audits](</private/tmp/storyproduction-realuse-current-after-cancel/logs/imageprompt-non-evidence-audits-final.log>) |

An initial ImagePromptCompiler gate attempt exposed missing owner imports and unused imports after the
module split. Those imports were corrected at their defining owners, Rust formatting was applied, and
the full check/test/quality/docs/release path passed before the missing evidence gate. An initial
NVC test compile also caught one validator omitted from a narrowed test import; it was restored and
the final full NVC gate passed. These were code/fixture integration defects, not waived checks.

## Public runtime flows

The public component chain passed through `geultu → NarrativeCourier → SMC → SceneAssetCompiler →
SMC`. It produced 19 STS agent nodes, 10 simulated Human gates, a hash-bound two-scene projection,
10 asset requirements, five shots, and a 15,442-byte HTML output. Image and PSD inputs were
synthetic 1×1 fixtures. [Owner-chain output](</private/tmp/storyproduction-realuse-current-after-cancel/logs/storyctl-e2e.log>)

A fresh public `storyctl text` workspace completed intake, story and synopsis candidate reviews,
hash-bound synopsis handoff, 19 token-bound STS tasks, 10 simulated Human gates, current-source
storyboard review, and package publication. The 569-byte `source.md` is byte-equal to the intake
source; all nine hash-listed package artifacts match their manifest, with `story-package.json` as the
tenth file. The final program status is `complete`. [Run summary](</private/tmp/storyproduction-realuse-current-after-cancel/text-authoring-current-r2/summary.json>) · [CLI transcript](</private/tmp/storyproduction-realuse-current-after-cancel/text-authoring-current-r2/storyctl-text-transcript.jsonl>) · [package manifest](</private/tmp/storyproduction-realuse-current-after-cancel/text-authoring-current-r2/package/story-package.json>)

STS Codex execution was exercised with a local fake executable, without an account or external API.
Accepted cancellation while the child was running returned an explicit cancellation error, committed
no step_00 artifact, preserved the pending state, and cleaned both markers. A separate successful
fake run completed step_00 through `run_one_codex → submit_with_lock`; the published artifact hash
matched state. [Cancellation result](</private/tmp/storyproduction-realuse-current-after-cancel/codex-cancellation-r3/summary.json>) · [Successful commit result](</private/tmp/storyproduction-realuse-current-after-cancel/codex-cancellation-r3/success-summary.json>)

The runtime exercise covered active-child cancellation and the successful commit path. It did not
force-schedule the narrow interleaving where the child has exited but submission has not yet acquired
`.workflow.lock`; that ordering is enforced structurally by the shared lock and marker check.

## Content fidelity and limits

The independent review of the synthetic text package is **FAIL**. The source says the mother believes
Minjun caused the fire and that both the record editor and fire cause remain unknown. The screenplay
preserves the 9:10 departure, 9:40 report time, correction, and departure, but omits the mother's
belief and the two explicit unknowns. The fixture-generated Source IR and STEP 8D table did not carry
those facts, so their structural check could not catch the omission. A complete package hash chain
does not certify narrative fidelity.

All story reviewers and STS Human gates in these QA workspaces were fixture actors. No real user
manuscript, human creative/adoption decision, external writer/model, image/video provider, or generated
pixel quality was verified. `magick` is absent, so the renderer smoke was **NOT_RUN_ENVIRONMENT**.
ImagePromptCompiler's source-snapshot regression test passed, but its official source-identity receipt
was not written because the full gate stopped at missing historical evidence.

The first attempt to replay a prior text transcript reused JSON files that had been overwritten later
in that earlier run and supplied a synopsis payload to the story task; the owner correctly rejected
it as `task not found`. That QA harness mistake was isolated and replaced with fresh per-stage inputs;
it did not change repository or workspace source state.
