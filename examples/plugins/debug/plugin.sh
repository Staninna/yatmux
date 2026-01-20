#!/usr/bin/env bash
set -euo pipefail

log_dir="${XDG_CACHE_HOME:-$HOME/.cache}/yatmux"
log_file="$log_dir/plugin-debug.log"
mkdir -p "$log_dir"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

if [ -n "${YATMUX_PLUGIN_EVENT:-}" ]; then
  printf '%s %s\n' "$timestamp" "$YATMUX_PLUGIN_EVENT" >>"$log_file"
  if printf '%s' "$YATMUX_PLUGIN_EVENT" | grep -q '"event":"startup"'; then
    printf '%s\n' '{"command":"subscribe","events":["all"]}'
  fi
else
  payload="$(cat)"
  if [ -n "$payload" ]; then
    printf '%s %s\n' "$timestamp" "$payload" >>"$log_file"
  fi
fi
