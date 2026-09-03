#!/usr/bin/env bash
# Run a Codex CLI task non-interactively against a directory.
#
# Usage: scripts/codex-task.sh <prompt-file> [workdir] [output-file]
#   prompt-file  : markdown/plain text task spec (read via stdin so no shell quoting issues)
#   workdir      : directory Codex may edit (default: repo root)
#   output-file  : where Codex's final message is written (default: <prompt-file>.result.md)
#
# Env overrides: CODEX_MODEL (default gpt-5.6-sol), CODEX_EFFORT (default xhigh),
#                CODEX_TIMEOUT (seconds, default 5400), CODEX_EXTRA (extra args)
set -uo pipefail
prompt="${1:?prompt file required}"
workdir="${2:-$(git -C "$(dirname "$0")/.." rev-parse --show-toplevel 2>/dev/null || pwd)}"
out="${3:-${prompt%.md}.result.md}"
model="${CODEX_MODEL:-gpt-5.6-sol}"
effort="${CODEX_EFFORT:-xhigh}"
timeout_s="${CODEX_TIMEOUT:-5400}"
log="${out%.md}.log"

echo "[codex-task] model=$model effort=$effort workdir=$workdir prompt=$prompt out=$out"
timeout "$timeout_s" codex exec \
  --dangerously-bypass-approvals-and-sandbox \
  --skip-git-repo-check \
  -m "$model" \
  -c model_reasoning_effort="\"$effort\"" \
  -C "$workdir" \
  -o "$out" \
  ${CODEX_EXTRA:-} \
  - < "$prompt" > "$log" 2>&1
code=$?
echo "[codex-task] exit=$code log=$log"
exit $code
