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

emit_prompt() {
  python3 - "$1" "$2" "$3" <<'PY'
import json,sys
print(json.dumps({
    "command":"prompt",
    "id":sys.argv[1],
    "title":sys.argv[2],
    "message":sys.argv[3],
}))
PY
}

emit_send_text() {
  python3 - "$1" <<'PY'
import json,sys
print(json.dumps({"command":"send_text","text":sys.argv[1]}))
PY
}

event="$(json_get event "$event_json" 2>/dev/null || true)"

if [ "$event" = "startup" ]; then
  printf '%s\n' '{"command":"subscribe","events":["plugin_command","prompt_response"]}'
  exit 0
fi

if [ "$event" = "plugin_command" ]; then
  plugin="$(json_get data.plugin "$event_json" 2>/dev/null || true)"
  command="$(json_get data.command "$event_json" 2>/dev/null || true)"
  name="$(json_get data.name "$event_json" 2>/dev/null || true)"
  if [ "$name" != "send-snippet" ] && { [ "$plugin" != "send-snippet" ] || [ "$command" != "send" ]; }; then
    exit 0
  fi
  text="$(json_get data.args.text "$event_json" 2>/dev/null || true)"
  if [ -n "$text" ]; then
    emit_send_text "$text"
  else
    emit_prompt "snippet-input" "Send snippet" "Text to send?"
  fi
  exit 0
fi

if [ "$event" = "prompt_response" ]; then
  prompt_id="$(json_get data.id "$event_json" 2>/dev/null || true)"
  if [ "$prompt_id" != "snippet-input" ]; then
    exit 0
  fi
  ok="$(json_get data.ok "$event_json" 2>/dev/null || true)"
  if [ "$ok" != "True" ] && [ "$ok" != "true" ]; then
    exit 0
  fi
  value="$(json_get data.value "$event_json" 2>/dev/null || true)"
  if [ -n "$value" ]; then
    emit_send_text "$value"
  fi
  exit 0
fi
