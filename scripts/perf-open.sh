#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

readonly dejavu_font="fixtures/fonts/DejaVuSans.ttf"
primary_repo_root="$PWD"
if [[ -f .git ]]; then
  git_dir="$(sed -n 's/^gitdir:[[:space:]]*//p' .git)"
  if [[ "$git_dir" != /* ]]; then
    git_dir="$PWD/$git_dir"
  fi
  primary_repo_root="$(cd "$(dirname "$git_dir")/../.." && pwd)"
fi
readonly primary_repo_root
readonly source_han_font="${FC_SOURCE_HAN_SANS:-$(dirname "$primary_repo_root")/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf}"
readonly fc_binary="target/release/fontcarver"
readonly open_budget_ns=3000000000
readonly save_load_budget_ns=6000000000

if [[ ! -f "$source_han_font" ]]; then
  echo "missing performance fixture: $source_han_font" >&2
  exit 2
fi

fcf_path="$(mktemp --suffix=.fcf)"
trap 'rm -f "$fcf_path"' EXIT

echo "Building release fontcarver..."
cargo build --release -p fc-cli --bin fontcarver

step_ns=0
time_step() {
  local label="$1"
  shift
  local start_ns end_ns
  start_ns="$(date +%s%N)"
  "$@" >/dev/null
  end_ns="$(date +%s%N)"
  step_ns=$((end_ns - start_ns))
  printf '%s: %d.%03d s\n' "$label" "$((step_ns / 1000000000))" \
    "$(((step_ns / 1000000) % 1000))"
}

time_step "inspect DejaVuSans.ttf" "$fc_binary" inspect "$dejavu_font"
dejavu_open_ns=$step_ns
time_step "inspect SourceHanSansSC-Regular.otf" "$fc_binary" inspect "$source_han_font"
source_han_open_ns=$step_ns
time_step "convert Source Han Sans to FCF" "$fc_binary" convert "$source_han_font" "$fcf_path"
save_ns=$step_ns
time_step "inspect converted FCF" "$fc_binary" inspect "$fcf_path"
load_ns=$step_ns
save_load_ns=$((save_ns + load_ns))
printf 'save+load total: %d.%03d s\n' "$((save_load_ns / 1000000000))" \
  "$(((save_load_ns / 1000000) % 1000))"

failed=0
if ((dejavu_open_ns > open_budget_ns)); then
  echo "FAIL: DejaVu open exceeded 3 s" >&2
  failed=1
fi
if ((source_han_open_ns > open_budget_ns)); then
  echo "FAIL: Source Han Sans open exceeded 3 s" >&2
  failed=1
fi
if ((save_load_ns > save_load_budget_ns)); then
  echo "FAIL: Source Han Sans FCF save+load exceeded 6 s" >&2
  failed=1
fi
if ((failed != 0)); then
  exit 1
fi

echo "PASS: open <= 3 s; save+load <= 6 s"
