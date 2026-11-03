#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)

if [ "$#" -eq 0 ]; then
  MODE=clean
elif [ "$#" -eq 1 ] && [ "$1" = "--dry-run" ]; then
  MODE=dry-run
else
  printf 'usage: %s [--dry-run]\n' "$0" >&2
  exit 2
fi

print_local_candidates() {
  printf 'clean: local candidate paths:\n'
  printf '  %s\n' "$ROOT/target"
  find "$ROOT/scripts" \
    \( -type d -name '__pycache__' -o -type f \( -name '*.pyc' -o -name '*.pyo' \) \) \
    -print
  printf '  %s\n' "$ROOT/.DS_Store"
}

remove_python_cache() {
  find "$ROOT/scripts" -type d -name '__pycache__' -prune -exec rm -rf {} +
  find "$ROOT/scripts" -type f \( -name '*.pyc' -o -name '*.pyo' \) -exec rm -f {} +
}

if [ "$MODE" = dry-run ]; then
  cargo clean --manifest-path "$ROOT/Cargo.toml" --target-dir "$ROOT/target" --dry-run
  print_local_candidates
  exit 0
fi

cargo clean --manifest-path "$ROOT/Cargo.toml" --target-dir "$ROOT/target"
remove_python_cache
if [ -f "$ROOT/.DS_Store" ] || [ -L "$ROOT/.DS_Store" ]; then
  rm -f "$ROOT/.DS_Store"
fi

if [ -e "$ROOT/target" ]; then
  printf 'clean: Cargo target residue remains: %s\n' "$ROOT/target" >&2
  exit 1
fi
remaining_cache=$(find "$ROOT/scripts" \
  \( -type d -name '__pycache__' -o -type f \( -name '*.pyc' -o -name '*.pyo' \) \) \
  -print -quit)
if [ -n "$remaining_cache" ]; then
  printf 'clean: Python cache residue remains: %s\n' "$remaining_cache" >&2
  exit 1
fi
if [ -e "$ROOT/.DS_Store" ] || [ -L "$ROOT/.DS_Store" ]; then
  printf 'clean: .DS_Store residue remains: %s\n' "$ROOT/.DS_Store" >&2
  exit 1
fi

printf 'clean: PASS (generated repository residue removed)\n'
