# StoryProduction SEMI provider

This macOS 26+ executable exposes a workspace-scoped provider through `SEMIProviderSDK`. It calls only StoryProduction's public `storyctl text author` entry point and only these geultu operations:

- `authoring.status`
- `authoring.document`
- `authoring.candidate`
- `authoring.task`
- `authoring.prepare` through SEMI's approved `story.authoring.task.prepare` mutation
- `authoring.brief` through exact brief-revision CAS
- `authoring.submit` to save an untrusted candidate without adopting it
- `authoring.review` to accept/reject the exact candidate shown in SEMI's approval preview

The workspace is fixed by the operator when the provider starts. The helper resolves configured path symlinks once and refuses startup unless the StoryProduction root, workspace, and native `storyctl` executable exist. The selected executable must be a regular file; directories and FIFOs are rejected. `--storyctl-bin` selects an explicit absolute executable; otherwise the provider uses `integration/target/release/storyctl` under the selected root. Invalid explicit paths are refused without fallback. Its workspace path is never accepted from a model tool call. Reads and writes use SEMI's normal access consent and exact mutation approval. Prepared writes bind the `projectId` shown in the preview; each owner call rechecks that identity against the workspace descriptor before calling geultu. A changed project descriptor during a write is reported as `not_applied` before geultu mutation; a mismatch during a prepare-time owner read aborts plan creation. `authoring.brief` also rechecks the current brief revision; `authoring.prepare` binds the approved task packet to a project revision and asks geultu to compare it under the journal lock; `authoring.review` captures the candidate's exact owner revision during prepare and geultu rechecks it with current source/guidance at execution. SEMI binds that revision privately in its prepared plan; the revision hash does not appear in the model tool input or approval preview. Task preparation creates no draft and calls no writer. Candidate submission saves an untrusted draft only; it does not make that draft current. The provider does not expose `text next`, arbitrary CLI operations, source creation, writer execution, handoff, or package publication. In particular, `text next` is not a read: it may persist a new `authoring.prepare` task.

## Local build

SEMI's settings supply the workspace, executable paths, and process instance ID when launching the helper. Build and sign the executable first. The `codesign` example below is for release builds signed by the same Apple developer team as SEMI; the identifier must match `com.axient.storyproduction.semi-provider`. For local development, SEMI and the provider may both use valid ad-hoc signatures. That mode relies on the same effective user and exact signing identifier at the XPC boundary and does not provide release same-team trust.

```sh
cd integration/macos-provider
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift build -c release

codesign --force --sign "<same-team signing identity as SEMI>" \
  --identifier com.axient.storyproduction.semi-provider \
  .build/release/story-semi-provider
```

The provider stays in the foreground, reconnects to a running SEMI instance with bounded backoff, and exits after SIGINT/SIGTERM cleanup. On disconnect or shutdown, `disconnectAndWait()` closes the local provider endpoint and drains active StoryCLI calls before exit; a lost post-dispatch result remains uncertain. It returns `true` only after SEMI confirms exact registration removal and active-call drain; `false` means peer cleanup was not confirmed. SIGINT/SIGTERM cancels the stream waiter, then the provider attempts the graceful deactivation acknowledgment before closing locally. The provider does not install a login item. When SEMI is not running, no StoryProduction operation is performed. A separate launch-at-login choice and native XPC acceptance for same-team release and same-user ad-hoc development remain open.

The SEMI app's **앱 연결** settings start and stop this helper on demand. The launcher validates the exact signed identifier and signing trust mode before launch and immediately before spawning the executable, then passes the selected workspace and a process-specific instance ID as fixed arguments. The provider standardizes configured paths and resolves symlinks once at startup. It holds an open descriptor for the selected workspace and compares its device/inode with the configured path before each owner operation, in `preSpawnProcessConfigurator`, and after an owner process returns. At spawn, a file action sets the child working directory from that descriptor and `storyctl` receives its `/dev/fd/<workspace-fd>` path. Rust reads `story-project.json` with `openat` relative to the held descriptor and inherits that descriptor into owner calls. On Darwin, native storyctl uses `fchdir` and relative writing/narrative paths rather than opening `/dev/fd/N/child`, so replacing the configured workspace path cannot redirect later owner calls to another root. If the file action fails, `story_workspace_spawn_setup_failed` returns `not_applied` before owner dispatch. A mismatch observed before spawn prevents the owner CLI from starting; a mismatch after a call suppresses read output and reports a dispatched mutation as `uncertain`. A process that fails before `geultu` starts reports `not_started` and is treated as not applied. On `story_workspace_changed` or `story_workspace_changed_during_dispatch`, stop this helper and reconnect it with the intended workspace; inspect owner state before approving any new mutation after an uncertain result. `ControlPlaneRegistry` accepts only that ID for this host-managed provider and removes its registration when the process ends. A new XPC registration during reconnect invalidates earlier consent and plans. Direct invocation cannot register with SEMI. The executable identity check is also path-based and is not an atomic inode binding. The helper does not auto-start; access consent and every mutation approval remain in SEMI.

The SwiftPM dependency uses the local sibling `SEMIProviderSDK` path and is intended for this workspace. Third-party license references are in [NOTICE.md](NOTICE.md). The native provider passed `swift build` on 2026-10-03 with Swift 6.4. The local ad-hoc signature and exact signing identifier were verified. Production Team trust, XPC connection, access consent, and real mutation approval remain **NOT_RUN**. Candidate plans over the SDK preview limit fail before dispatch. If a write process is interrupted after dispatch or the helper is stopped, the provider reports `uncertain`; inspect current StoryProduction state before a new operation.

## Native startup contract

```sh
integration/macos-provider/.build/debug/story-semi-provider \
  --story-root /absolute/path/StoryProduction \
  --workspace /absolute/path/my-story \
  --storyctl-bin /absolute/path/storyctl \
  --geultu-bin /absolute/path/geultu --instance-id UUID
```

SEMI supplies the instance ID. `--python` and the Python script preflight have been retired. The authorized SEMI-ControlPlane launcher and its settings now use a native `storyctl` executable and pass `--storyctl-bin`. An explicitly invalid executable is rejected without interpreter fallback. The old Python preference is left unused; user preferences and workspaces are not rewritten. Isolated launcher tests and an ad-hoc signed app build verify this startup contract. They do not certify production XPC registration, user consent, or a real mutation lifecycle.
