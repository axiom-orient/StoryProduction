#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
echo '== stage ids =='
grep -RhoE 'S(0[0-9]|1[0-2])_[A-Z0-9_]+' "$ROOT" --exclude-dir=.git | sort | uniq -c
echo '== schema refs =='
grep -RhoE 'schemas/[A-Za-z0-9._-]+\.schema\.json' "$ROOT" --exclude-dir=.git | sort | uniq -c
echo '== failure codes =='
grep -RhoE '[A-Z]{3}_[0-9]{3}_[A-Z0-9_]+' "$ROOT" --exclude-dir=.git | sort | uniq -c
