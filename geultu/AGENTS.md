# Repository governance

- Repository inputs are source, Cargo manifests and lockfiles, `.cargo` offline configuration, documentation, and vendored dependencies. They are never cleanup targets.
- User workspaces passed through `--workspace` are product output outside repository cleanup ownership. Do not remove or rewrite them with repository operations.
- Generated repository residue is limited to Cargo `target/`, Python bytecode/cache below `scripts/`, and the root `.DS_Store`. `./scripts/clean.sh` owns only those paths.
- Canonical operations are `./scripts/verify.sh`, `./scripts/clean.sh --dry-run`, and `./scripts/clean.sh`.
- Do not add compatibility, migration, or fallback paths without an explicitly approved replacement.
- Absolute rule: GitHub Actions, `.github/workflows`, and all CI/CD automation must not be created or modified.

## Project profile

- Project kind: Rust workspace with offline/local Python verification helpers.
- Platform: host-native Rust; vendored dependencies and user workspaces are external inputs, not platform targets.
- Primary gates: `./scripts/verify.sh`, `./scripts/clean.sh --dry-run`, and `./scripts/clean.sh`.

## Shared workspace policy

이 프로젝트에도 parent workspace `AGENTS.md`를 적용한다. 브라우저 작업은 Aside CLI의 `aside`만 사용하고 새 Playwright/Puppeteer 자동화를 추가하지 않는다. GitHub Actions·`.github/workflows`·CI/CD·release automation은 생성·수정·복원하지 않으며 검증은 로컬 명령으로 한다.
