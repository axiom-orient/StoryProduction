#!/bin/sh
# Geultu writer provider bridge: Codex CLI via subscription auth (no API key).
# stdin: GenerationPlan JSON -> stdout: GeneratedDraft JSON {"content","model"}
set -eu
PLAN=$(cat)
PROMPT=$(printf '%s' "$PLAN" | /usr/bin/python3 -c 'import json,sys;print(json.load(sys.stdin)["prompt"])')
OUT=$(mktemp)
trap 'rm -f "$OUT"' EXIT
codex exec --skip-git-repo-check --sandbox read-only --output-last-message "$OUT" "$PROMPT" >/dev/null 2>&1
/usr/bin/python3 - "$OUT" <<'PY'
import json, pathlib, sys
content = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
if not content.strip():
    sys.exit("codex returned an empty last message")
print(json.dumps({"content": content, "model": "codex-cli"}, ensure_ascii=False))
PY
