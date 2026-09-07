#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cargo run -p fc-devserver -- --port 8787 &
devserver_pid=$!

pnpm --dir app dev &
vite_pid=$!

cleanup() {
  kill "$devserver_pid" "$vite_pid" 2>/dev/null || true
  wait "$devserver_pid" "$vite_pid" 2>/dev/null || true
}

trap cleanup EXIT INT TERM
wait -n "$devserver_pid" "$vite_pid"
