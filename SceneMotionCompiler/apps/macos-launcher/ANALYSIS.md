# SceneMotionCompiler macOS launcher — module ownership

## Boundary

The launcher is a local macOS UI adapter for selecting an already-published SceneAssetCompiler
project and starting the SMC Studio host. It does not create assets, approve a release, or own
SceneGraph/DeliverySet state. `Package.swift` declares the executable and the small `LauncherCore`
protocol library for macOS 14+.

## Modules and ownership

| Module | Responsibility | State/effect boundary |
|---|---|---|
| `Sources/SceneMotionCompilerLauncher/main.swift` | AppKit window, project selection, Node/Studio `Process` lifecycle, stdout protocol dispatch, and opening the local Studio URL | The UI acts only on a `StudioReady` response whose canonical project root matches the selected folder and whose origin is a plain `http://127.0.0.1:<port>` URL |
| `Sources/LauncherCore/StudioProtocol.swift` | Decode `SMC_STUDIO_READY`/`SMC_STUDIO_ERROR` lines | Pure value parsing; no AppKit, process, or project filesystem effects |

The ready message carries the canonical project root because the Studio host resolves the selected
path before starting. The launcher now checks that identity before opening the endpoint instead of
discarding the field. Expected versus unexpected process termination is determined directly from the
AppDelegate-owned `expectedTermination` state; a bool-to-enum classifier duplicated that policy and
has been removed. Ready/not-ready presentation text remains AppDelegate-owned.

`macosOperatorId` still owns the local username normalization and opaque ID format; only the raw
SHA-256 calculation comes from the SMC canonical-byte owner.

## Evidence boundary

This launcher source is not signed or app-bundled in the repository checkout. macOS build, signing,
cross-process host lifecycle, and real Studio interaction remain unqualified. The current source-only
refactor has **NOT_RUN** Swift tests, build, or runtime checks.
