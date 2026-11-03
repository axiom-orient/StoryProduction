#!/usr/bin/env bash
set -euo pipefail

# Cross-repository regression: the producer must be the real geultu binary and
# the consumer must be the real NarrativeCourier ingress.  The temporary
# roots are deliberately separate so the test cannot pass through a shared
# in-process object or an implicit fallback.
ROOT=$(cd "$(dirname "$0")/.." && pwd)
GEULTU_ROOT="$ROOT/../geultu"
GEULTU_BIN="$GEULTU_ROOT/target/debug/geultu"
NVC_BIN="$ROOT/target/debug/narrative-courier-cli"

cargo build --manifest-path "$GEULTU_ROOT/Cargo.toml" -p geultu-cli --bin geultu \
  --offline --target-dir "$GEULTU_ROOT/target" >/dev/null
cargo build --manifest-path "$ROOT/Cargo.toml" -p narrative-courier \
  --bin narrative-courier-cli --offline --target-dir "$ROOT/target" >/dev/null

TMP_ROOT=$(mktemp -d "/tmp/nvc-geultu-handoff.XXXXXX")
trap 'rm -rf "$TMP_ROOT"' EXIT

GEULTU_WORK="$TMP_ROOT/geultu-work"
NVC_PROJECTS="$TMP_ROOT/nvc-projects"
SCENE_TEXT="$TMP_ROOT/scene.md"
mkdir -p "$NVC_PROJECTS"
printf '지우가 문을 연다.  \n\n' >"$SCENE_TEXT"

"$GEULTU_BIN" --workspace "$GEULTU_WORK" init >/dev/null
"$GEULTU_BIN" --workspace "$GEULTU_WORK" project-create book "교차 경계 회귀" >/dev/null
"$GEULTU_BIN" --workspace "$GEULTU_WORK" scene-add book scene_1 "첫 장면" "$SCENE_TEXT" >/dev/null
"$GEULTU_BIN" --workspace "$GEULTU_WORK" export-reviewed-synopsis \
  book human-author ko "cross-repository regression" >"$TMP_ROOT/geultu.json"

python3 - "$GEULTU_WORK" "$TMP_ROOT/geultu.json" "$ROOT/tools" <<'PY'
import json
import pathlib
import sys

sys.path.insert(0, sys.argv[3])
from digest import sha256_bytes

work = pathlib.Path(sys.argv[1])
result = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
handoff = result["result"]
handoff_dir = work / ".geultu" / "projects" / "book" / "handoff"
synopsis = (handoff_dir / "synopsis.md").read_bytes()
persisted = json.loads((handoff_dir / "reviewed-synopsis-handoff.v1.json").read_text(encoding="utf-8"))
assert persisted == handoff, "geultu CLI output differs from persisted handoff"
assert sha256_bytes(synopsis) == handoff["synopsis_sha256"]
assert handoff["approval"]["synopsis_sha256"] == handoff["synopsis_sha256"]
assert handoff["approval"]["decision"] == "APPROVE"
print("geultu producer: handoff identity and raw synopsis hash verified")
PY

STS_ASSETS_ROOT="$ROOT" STS_PROJECTS_ROOT="$NVC_PROJECTS" \
  "$NVC_BIN" import-reviewed-synopsis nvc-book \
  --handoff-root "$GEULTU_WORK/.geultu/projects/book/handoff" \
  --handoff reviewed-synopsis-handoff.v1.json >"$TMP_ROOT/nvc.json"

python3 - "$NVC_PROJECTS/nvc-book" "$GEULTU_WORK" "$TMP_ROOT/nvc.json" "$ROOT/tools" <<'PY'
import json
import pathlib
import sys

sys.path.insert(0, sys.argv[4])
from digest import sha256_bytes

nvc_project = pathlib.Path(sys.argv[1])
geultu_work = pathlib.Path(sys.argv[2])
json.loads(pathlib.Path(sys.argv[3]).read_text(encoding="utf-8"))
source = (geultu_work / ".geultu" / "projects" / "book" / "handoff" / "synopsis.md").read_bytes()
persisted = (nvc_project / "input" / "synopsis.md").read_bytes()
assert persisted == source, "NVC input/synopsis.md changed the approved raw bytes"
config = (nvc_project / "config.yaml").read_text(encoding="utf-8")
assert "reviewed_synopsis:" in config
assert sha256_bytes(persisted) == sha256_bytes(source)
print("NVC consumer: exact approved bytes persisted and project created")
PY
