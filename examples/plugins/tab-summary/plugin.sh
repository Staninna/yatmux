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

emit_request_state() {
  python3 - "$1" <<'PY'
import json,sys
print(json.dumps({"command":"request_state","id":sys.argv[1]}))
PY
}

emit_window_title() {
  python3 - "$1" <<'PY'
import json,sys
print(json.dumps({"command":"set_window_title","title":sys.argv[1]}))
PY
}

event="$(json_get event "$event_json" 2>/dev/null || true)"

if [ "$event" = "startup" ]; then
  printf '%s\n' '{"command":"subscribe","events":["plugin_command","state_response"]}'
  exit 0
fi

if [ "$event" = "plugin_command" ]; then
  plugin="$(json_get data.plugin "$event_json" 2>/dev/null || true)"
  command="$(json_get data.command "$event_json" 2>/dev/null || true)"
  name="$(json_get data.name "$event_json" 2>/dev/null || true)"
  if [ "$name" != "tab-summary" ] && { [ "$plugin" != "tab-summary" ] || [ "$command" != "summary" ]; }; then
    exit 0
  fi
  emit_request_state "tab-summary"
  exit 0
fi

if [ "$event" = "state_response" ]; then
  req_id="$(json_get data.id "$event_json" 2>/dev/null || true)"
  if [ "$req_id" != "tab-summary" ]; then
    exit 0
  fi
  summary="$(python3 - "$event_json" <<'PY'
import json,sys
data=json.loads(sys.argv[1]).get("data",{})
tabs=data.get("tabs") or []
tab_count=len(tabs)
pane_count=0
for tab in tabs:
    pane_count += len(tab.get("panes") or [])
print(f"tabs: {tab_count}, panes: {pane_count}")
PY
)"
  emit_window_title "$summary"
  exit 0
fi
