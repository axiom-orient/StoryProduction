#!/usr/bin/env bash
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
ROOT="$(cd "$(dirname "$0")" && pwd)"
require_rust=0
renderers=0
for arg in "$@"; do
  case "$arg" in
    --require-rust) require_rust=1 ;;
    --renderers) renderers=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 64 ;;
  esac
done

python3 "$ROOT/tools/verify_contract.py"
python3 "$ROOT/tools/verify_delivery_source.py"
python3 "$ROOT/tools/test_sts_audit.py"
python3 "$ROOT/docs/blueprint/scripts/verify_docs.py"
python3 - "$ROOT" <<'PY'
import ast
from pathlib import Path
import sys

root = Path(sys.argv[1])
for path in sorted((root / "tools").glob("*.py")):
    ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
print("PASS Python syntax")
PY
node --check "$ROOT/ui/app.js"
node --check "$ROOT/ui/graph-viewer.js"
node "$ROOT/ui/graph-viewer.test.js"

if [[ "$renderers" -eq 1 ]]; then
  python3 "$ROOT/tools/smoke_renderers.py"
fi

if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  rust_version="$(rustc --version)"
  if [[ "$rust_version" != rustc\ 1.97.1* ]]; then
    echo "RUST FAIL — expected rustc 1.97.1, got: $rust_version" >&2
    exit 2
  fi
  (
    cd "$ROOT"
    cargo check --workspace
    cargo test --workspace
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
  )
  echo "RUST PASS — rustc 1.97.1 / check / test / fmt / clippy"
else
  echo "RUST NOT_RUN — cargo/rustc unavailable"
  if [[ "$require_rust" -eq 1 ]]; then exit 2; fi
fi
