# Distribution Roadmap

Codex documentation now treats **Skill as the workflow authoring format** and **Plugin as the reusable installable distribution unit**. Keep the current repo-local `.agents/skills/scene-asset-compiler` as the canonical development form because it is directly discoverable in Codex App/CLI/IDE and travels with the project.

After the contracts stabilize:
1. package the unchanged skill directory inside a SceneAssetCompiler plugin;
2. add install/update metadata and optional app surfaces;
3. keep the typed Scene Provider and built-in `$imagegen` executor boundaries unchanged;
4. do not move domain state into plugin configuration.
