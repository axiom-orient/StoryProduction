# NarrativeCourier app config — analysis

`narrative-courier-app-config` owns only shared NarrativeCourier application roots used by CLI and MCP adapters. It does not own STS state, project contents, or filesystem effects.

`resolve_roots` owns the single `Roots` type and preserves precedence: explicit adapter values, `STS_ASSETS_ROOT` / `STS_PROJECTS_ROOT`, platform application data for projects, then repository defaults. The MCP crate parses its flags and passes explicit overrides; it does not re-export `Roots`. The Tauri CLI passes no explicit values and uses the same environment/platform defaults.

The desktop UI still resolves packaged resources and Tauri application data through its own app lifecycle. That path is intentionally outside this CLI/MCP configuration owner.

## NVC app-config root resolution/entrypoint options — source-only closure

App-config owns fallible resolve_roots returning Roots or ConfigError. Empty selected explicit/env
roots no longer represent the working directory implicitly. Relative overrides are anchored against
one captured working directory; paths that remain drive-relative are rejected. Platform lookup is a
private owner and runs only when no higher-priority project root exists. Only absent platform values
permit fallback; selected HOME/APPDATA/USERPROFILE/XDG_DATA_HOME paths must be absolute and nonempty.
Lower-priority unselected values cannot invalidate an explicit override. OS-native non-UTF-8 paths
remain supported; resolution performs no canonicalization, project read, directory creation, or cleanup.

CLI/MCP consumers propagate ConfigError instead of using an infallible compatibility path. MCP root
flags share one parser branch and reject duplicates, empty/missing values, and another recognized flag
where a path is required. CLI help bypasses configuration and states relative-root behavior. Existing
precedence fixtures consume the fallible API and use platform-absolute paths; they were not run.
The desktop UI retains its Tauri-managed packaged-resource/data lifecycle rather than adopting CLI
repository defaults. Tests/build/fmt/lint/CLI/MCP/platform runtime and manifest qualification are
**NOT_RUN**. Existing user projects, environment settings, and persisted data were not modified.
