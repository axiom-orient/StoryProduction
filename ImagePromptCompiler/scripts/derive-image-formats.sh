#!/usr/bin/env bash
# promptGen -> perfectpixel, composed at the shell.
#
# The two tools stay independent binaries: promptGen compiles a typed request into a prompt
# and publishes a validated PNG; perfectpixel derives whatever other formats you need from
# that PNG. Neither depends on the other at build time.
#
#   ./pipeline.sh <request.json> <out-dir> [--image <existing.png>]
#
# Without --image the script stops after compiling the prompt, because image generation needs
# an authenticated Codex CLI. With --image it treats the supplied file as the generated
# artifact and runs the derivation stage, which is what makes the pipeline verifiable without
# spending generation quota.

set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PROMPTGEN=${PROMPTGEN:-$ROOT/target/release/promptgen}
if [ -z "${PERFECTPIXEL:-}" ]; then
  PERFECTPIXEL=$(command -v perfectpixel || true)
fi

die() { printf '%s\n' "$*" >&2; exit 1; }

[ $# -ge 2 ] || die "usage: $0 <request.json> <out-dir> [--image <existing.png>]"
REQUEST=$1
OUT=$2
shift 2

SUPPLIED_IMAGE=""
while [ $# -gt 0 ]; do
  case $1 in
    --image) [ $# -ge 2 ] || die "--image needs a path"; SUPPLIED_IMAGE=$2; shift 2 ;;
    *) die "unknown argument: $1" ;;
  esac
done

[ -f "$REQUEST" ]      || die "request not found: $REQUEST"
[ -x "$PROMPTGEN" ]    || die "promptgen binary not found: $PROMPTGEN (set PROMPTGEN)"
[ -x "$PERFECTPIXEL" ] || die "perfectpixel binary not found: $PERFECTPIXEL (set PERFECTPIXEL)"
mkdir -p "$OUT"

echo "[1/3] compile prompt"
# --format text is the human-readable compilation; the same compiler backs --format json.
# On a rejected request promptGen prints diagnostics on stdout and exits non-zero. Writing
# straight to prompt.txt would leave that diagnostic text sitting at the path a later stage
# reads as if it were a prompt, so the file is only published once the compile succeeded.
PROMPT_TMP="$OUT/.prompt.txt.partial"
# `$?` inside `if ! cmd` is the status of the negation, not of `cmd`, so the exit code is
# captured directly with `set -e` lifted for exactly one command.
set +e
"$PROMPTGEN" image --input "$REQUEST" --mode prompt-only --format text > "$PROMPT_TMP"
compile_status=$?
set -e
if [ "$compile_status" -ne 0 ]; then
  echo "      request rejected; diagnostics:" >&2
  sed 's/^/      /' "$PROMPT_TMP" >&2
  rm -f "$PROMPT_TMP"
  exit "$compile_status"
fi
mv "$PROMPT_TMP" "$OUT/prompt.txt"
echo "      $OUT/prompt.txt ($(wc -c < "$OUT/prompt.txt" | tr -d ' ') bytes)"

echo "[2/3] obtain the image artifact"
if [ -n "$SUPPLIED_IMAGE" ]; then
  [ -f "$SUPPLIED_IMAGE" ] || die "supplied image not found: $SUPPLIED_IMAGE"
  # promptGen publishes PNG, so normalise whatever was supplied to the same contract before
  # the derivation stage. This keeps the measured sizes comparable to a real run.
  "$PERFECTPIXEL" convert "$SUPPLIED_IMAGE" --out "$OUT/artifact.png" > /dev/null
  echo "      $OUT/artifact.png (from $SUPPLIED_IMAGE)"
else
  echo "      skipped: no --image supplied and Codex generation was not requested"
  echo "      to generate: $PROMPTGEN image --input $REQUEST --mode codex-imagegen \\"
  echo "                     --image-output $OUT/artifact.png --result $OUT/receipt.json"
  exit 0
fi

echo "[3/3] derive delivery formats"
"$PERFECTPIXEL" inspect "$OUT/artifact.png" > "$OUT/inspect.json"
"$PERFECTPIXEL" convert "$OUT/artifact.png" --out "$OUT/artifact.webp" > /dev/null
"$PERFECTPIXEL" convert "$OUT/artifact.png" --out "$OUT/artifact-512.webp" --width 512 > /dev/null
# JPEG has no alpha channel, so it is only produced when the artifact is fully opaque.
if [ "$(python3 -c "import json;print(json.load(open('$OUT/inspect.json'))['alphaRatio'])")" = "1.0" ]; then
  "$PERFECTPIXEL" convert "$OUT/artifact.png" --out "$OUT/artifact.jpg" --jpeg-quality 90 > /dev/null
fi

printf '\n%-24s %10s\n' "artifact" "size"
for f in "$OUT"/artifact*.png "$OUT"/artifact*.webp "$OUT"/artifact*.jpg; do
  [ -f "$f" ] && printf '%-24s %9dKB\n' "$(basename "$f")" $(( $(stat -f%z "$f") / 1024 ))
done
echo
echo "pipeline complete: $OUT"
