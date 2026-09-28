#!/usr/bin/env bash
# Deterministic part of the audit: snapshot, diff, scanners and facts. No model involved.
# Everything runs in Docker with pinned images; nothing is installed on the machine.
# Usage: prepare.sh <repo> <work-dir> [base-commit]
set -u
REPO=$(cd "$1" && pwd)
WORK=$2
BASE=${3:-}
BIN=$(cd "$(dirname "$0")" && pwd)
COMMIT=$(git -C "$REPO" rev-parse main)
SNAP="$WORK/src"; T="$WORK/tools"; F="$WORK/facts"
rm -rf "$SNAP" "$T" "$F" "$WORK/packets"
mkdir -p "$SNAP" "$T" "$F"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$SNAP"
echo "$COMMIT" > "$T/commit.txt"

RANGE=()
if [ -n "$BASE" ]; then
  git -C "$REPO" diff --name-only "$BASE" "$COMMIT" > "$T/changed.txt"
  git -C "$REPO" diff -U0 "$BASE" "$COMMIT" > "$T/diff.patch"
  git -C "$REPO" log --oneline "$BASE..$COMMIT" > "$T/commits.txt"
  RANGE=("--log-opts=$BASE..$COMMIT")
fi

U="$(id -u):$(id -g)"
OSV=ghcr.io/google/osv-scanner:v2.6.0
ALLOWED="MIT,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,0BSD,Zlib,Unicode-3.0,Unicode-DFS-2016,CC0-1.0,BSL-1.0,Unlicense"
# osv-scanner and gitleaks exit with 1 when they find something: that is not a failure.
run() { local name=$1; shift; "$@" > "$T/$name.log" 2>&1; echo "$name: exit $?"; }

run osv docker run --rm --user "$U" -v "$SNAP:/src:ro" -v "$T:/out" "$OSV" \
  scan source --recursive --format json --output-file /out/osv.json /src &
run licenses docker run --rm --user "$U" -v "$SNAP:/src:ro" -v "$T:/out" "$OSV" \
  scan source --recursive --licenses="$ALLOWED" --format json --output-file /out/osv-licenses.json /src &
run gitleaks docker run --rm --user "$U" -v "$REPO:/repo:ro" -v "$T:/out" \
  -e GIT_CONFIG_COUNT=1 -e GIT_CONFIG_KEY_0=safe.directory -e GIT_CONFIG_VALUE_0='*' \
  ghcr.io/gitleaks/gitleaks:v8.30.1 git /repo --no-banner --redact "${RANGE[@]}" \
  --report-format json --report-path /out/gitleaks.json &
run semgrep docker run --rm --user "$U" -e HOME=/tmp -v "$SNAP:/src:ro" -v "$T:/out" semgrep/semgrep:1.177.0 \
  semgrep scan --metrics=off --disable-version-check --quiet \
  --config p/default --config p/owasp-top-ten --config p/security-audit --config p/secrets \
  --config p/php --config p/typescript --config p/react --config p/javascript --config p/rust \
  --config p/dockerfile --config p/docker-compose --config p/github-actions --config p/nginx \
  --exclude docs/historico --exclude docs/research --exclude docs/design \
  --json --output /out/semgrep.json /src &
wait

run facts docker run --rm --user "$U" -v "$SNAP:/src:ro" -v "$T:/tools:ro" -v "$F:/out" -v "$BIN:/audit-bin:ro" \
  python:3.13.15-slim python /audit-bin/facts.py /src /tools /out
echo "COMMIT=$COMMIT"
echo "SUMMARY=$F/SUMMARY.md"
