#!/usr/bin/env bash
# Snapshot the gitignored SDD workspace (ledger, briefs, dispatches, reports, reviews)
# into docs/superpowers/ledgers/<plan>/ and commit, so orchestration state survives
# session loss, quota pauses and `git clean`. Large review diff packages are skipped.
set -euo pipefail
cd "$(dirname "$0")/.."
for ws in .superpowers/sdd/*/; do
  [ -d "$ws" ] || continue
  name=$(basename "$ws")
  dest="docs/superpowers/ledgers/$name"
  mkdir -p "$dest"
  rsync -a --delete --exclude 'review-*.diff' --exclude '*.log' "$ws" "$dest/"
done
git add docs/superpowers/ledgers
if ! git diff --cached --quiet; then
  git -c commit.gpgsign=false commit -q -m "docs(ledger): snapshot SDD workspace $(date -u +%Y-%m-%dT%H:%MZ)"
  echo "snapshot committed: $(git rev-parse --short HEAD)"
else
  echo "snapshot: no changes"
fi
