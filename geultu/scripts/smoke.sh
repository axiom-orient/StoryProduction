#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
BIN="$ROOT/target/release/geultu"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/geultu-smoke.XXXXXX")
trap 'rm -rf "$TMP"' EXIT

cargo build --manifest-path "$ROOT/Cargo.toml" -p geultu-cli --bin geultu --release --offline --target-dir "$ROOT/target"
if [ ! -x "$BIN" ]; then
  printf 'smoke: release artifact missing: %s\n' "$BIN" >&2
  exit 1
fi
"$BIN" --workspace "$TMP/work" init >/dev/null
"$BIN" --workspace "$TMP/work" project-create book "검증 소설" >/dev/null
printf '첫 문장입니다.\n\n두 번째 문단입니다.\n' > "$TMP/scene.txt"
"$BIN" --workspace "$TMP/work" scene-add book scene_1 "첫 장면" "$TMP/scene.txt" >/dev/null
"$BIN" --workspace "$TMP/work" call fact.add '{"project_id":"book","fact_id":"fact-1","subject":"주인공","predicate":"이름","object":"해나","evidence":[{"scene_id":"scene_1"}],"command_id":"smoke-fact"}' >/dev/null
"$BIN" --workspace "$TMP/work" review book --pretty > "$TMP/review.json"
"$BIN" --workspace "$TMP/work" call candidate.create '{"project_id":"book","candidate_id":"cand-1","kind":"scene_rewrite","source_revision":2,"content":"수정된 문장입니다.","provider":"fixture","model":"deterministic","command_id":"smoke-candidate"}' >/dev/null
"$BIN" --workspace "$TMP/work" call changeset.create '{"project_id":"book","candidate_id":"cand-1","changeset_id":"change-1","operations":[{"type":"replace_scene_text","scene_id":"scene_1","expected_scene_revision":0,"text":"수정된 문장입니다."}],"command_id":"smoke-changeset"}' >/dev/null
REV=$("$BIN" --workspace "$TMP/work" call project.get '{"project_id":"book"}' | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["revision"])')
SUBJECT=$("$BIN" --workspace "$TMP/work" call candidate.subject '{"project_id":"book","candidate_id":"cand-1"}')
ARTIFACT_REV=$(printf '%s' "$SUBJECT" | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["revision"])')
ARTIFACT_DIGEST=$(printf '%s' "$SUBJECT" | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["digest"])')
"$BIN" --workspace "$TMP/work" call changeset.approve "{\"project_id\":\"book\",\"changeset_id\":\"change-1\",\"expected_project_revision\":$REV,\"actor_id\":\"smoke-human\",\"expected_artifact_revision\":$ARTIFACT_REV,\"expected_artifact_digest\":\"$ARTIFACT_DIGEST\",\"command_id\":\"smoke-approve\"}" >/dev/null
"$BIN" --workspace "$TMP/work" export book "$TMP/book.md" >/dev/null
grep -q '수정된 문장입니다.' "$TMP/book.md"
"$BIN" --workspace "$TMP/work" doctor --pretty > "$TMP/doctor.json"
printf 'smoke: PASS\n'
