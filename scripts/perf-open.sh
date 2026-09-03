#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

readonly dejavu_font="fixtures/fonts/DejaVuSans.ttf"
readonly source_han_font="/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf"
readonly fw_binary="target/release/fw"
readonly open_budget_ns=3000000000
readonly save_load_budget_ns=6000000000

if [[ ! -f "$source_han_font" ]]; then
  echo "missing performance fixture: $source_han_font" >&2
  exit 2
fi

fwf_path="$(mktemp --suffix=.fwf)"
trap 'rm -f "$fwf_path"' EXIT

echo "Building release fw..."
cargo build --release -p fw-cli --bin fw

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

time_step "inspect DejaVuSans.ttf" "$fw_binary" inspect "$dejavu_font"
dejavu_open_ns=$step_ns
time_step "inspect SourceHanSansSC-Regular.otf" "$fw_binary" inspect "$source_han_font"
source_han_open_ns=$step_ns
time_step "convert Source Han Sans to FWF" "$fw_binary" convert "$source_han_font" "$fwf_path"
save_ns=$step_ns
time_step "inspect converted FWF" "$fw_binary" inspect "$fwf_path"
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
  echo "FAIL: Source Han Sans FWF save+load exceeded 6 s" >&2
  failed=1
fi
if ((failed != 0)); then
  exit 1
fi

echo "PASS: open <= 3 s; save+load <= 6 s"
