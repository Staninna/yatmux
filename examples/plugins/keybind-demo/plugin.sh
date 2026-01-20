#!/usr/bin/env bash
set -euo pipefail

if ! command -v python3 >/dev/null 2>&1; then
  exit 0
fi

event_json="${YATMUX_PLUGIN_EVENT:-}"
if [ -z "$event_json" ]; then
  event_json="$(cat)"
fi

if [ -z "$event_json" ]; then
  exit 0
fi

json_get() {
  python3 - "$1" "$2" <<'PY'
import json,sys
path=sys.argv[1]
raw=sys.argv[2]
data=json.loads(raw)
cur=data
for part in path.split('.'):
    if not part:
        continue
    if isinstance(cur, dict):
        cur=cur.get(part)
    else:
        cur=None
        break
if cur is None:
    sys.exit(1)
if isinstance(cur,(dict,list)):
    print(json.dumps(cur))
else:
    print(cur)
PY
}

emit_toast() {
  python3 - "$1" <<'PY'
import json,sys
print(json.dumps({"command":"toast","message":sys.argv[1]}))
PY
}

event="$(json_get event "$event_json" 2>/dev/null || true)"

if [ "$event" = "startup" ]; then
  printf '%s\n' '{"command":"register_keybind","key":"ctrl+shift+k","action":{"plugin":"keybind-demo","command":"ping"}}'
  printf '%s\n' '{"command":"subscribe","events":["plugin_command"]}'
  exit 0
fi

if [ "$event" = "plugin_command" ]; then
  plugin="$(json_get data.plugin "$event_json" 2>/dev/null || true)"
  command="$(json_get data.command "$event_json" 2>/dev/null || true)"
  name="$(json_get data.name "$event_json" 2>/dev/null || true)"
  if [ "$name" != "keybind-demo" ] && { [ "$plugin" != "keybind-demo" ] || [ "$command" != "ping" ]; }; then
    exit 0
  fi
  emit_toast "keybind-demo: pong"
  exit 0
fi
