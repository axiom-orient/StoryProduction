#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --offline --target-dir "$ROOT/target" -- -D warnings
cargo test --workspace --all-features --offline --target-dir "$ROOT/target"
cargo build --workspace --release --offline --target-dir "$ROOT/target"
# Canonical package gates (G2-G4, docs/canonical/documentation/20).
( cd "$ROOT/docs/canonical" && shasum -a 256 -c manifests/SHA256SUMS --quiet )
for asset in craft-packs-scriptforge-v4 craft-packs-story-baseline-v1 refinement-packs-language-v1 nvc-upstream-reference; do
  ( cd "$ROOT/assets/$asset" && shasum -a 256 -c SHA256SUMS --quiet )
done
python3 "$ROOT/docs/canonical/tools/verify.py"
node --check adapters/geultu-studio/web/app.js
# Smoke helpers rebuild and bind only to target/release artifacts from this checkout.
./scripts/smoke.sh
./scripts/mcp-smoke.py
# The Studio smoke builds and launches exact release artifacts from this checkout.
python3 scripts/studio-smoke.py
printf 'verify: PASS\n'
