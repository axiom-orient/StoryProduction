#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN=${1:-${PROMPTGEN_BIN:-$ROOT/target/debug/promptgen}}
[[ -x "$BIN" ]] || { echo "[FAIL] build promptgen before generating schemas" >&2; exit 1; }
TMP=$(mktemp -d "${TMPDIR:-/tmp}/promptgen-schemas.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
targets=(image interview compilation)
for target in "${targets[@]}"; do
  "$BIN" schema "$target" >"$TMP/$target.schema.json"
  python3 - "$TMP/$target.schema.json" <<'PY'
import json, pathlib, sys
value = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert value.get("$schema") == "https://json-schema.org/draft/2020-12/schema"
PY
  install -m 0644 "$TMP/$target.schema.json" "$ROOT/schemas/$target.schema.json"
done
echo "[OK] generated ${#targets[@]} image schemas"
