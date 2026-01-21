#!/usr/bin/env bash
set -euo pipefail

verbose="${VERBOSE:-0}"
color="${COLOR:-1}"

is_tty() {
  [ -t 1 ]
}

if [ "$color" -eq 1 ] && is_tty; then
  c_step=$'\033[1;36m'
  c_event=$'\033[0;36m'
  c_out=$'\033[0;33m'
  c_err=$'\033[0;31m'
  c_reset=$'\033[0m'
else
  c_step=""
  c_event=""
  c_out=""
  c_err=""
  c_reset=""
fi

step() {
  if [ "$verbose" -eq 1 ]; then
    echo "${c_step}==>${c_reset} $1" >&2
  fi
}

show_output() {
  if [ "$verbose" -eq 1 ]; then
    echo "${c_out}--- output ---${c_reset}" >&2
    if [ -z "$1" ]; then
      echo "(no output)" >&2
    else
      python3 -c '
import json,sys
for line in sys.stdin.read().splitlines():
    line=line.strip()
    if not line:
        continue
    try:
        obj=json.loads(line)
        print(json.dumps(obj, indent=2, sort_keys=True))
    except Exception:
        print(line)
' <<<"$1" >&2
    fi
    echo "${c_out}-------------${c_reset}" >&2
  fi
}

root_dir="$(pwd)"
plugin_src="$root_dir/examples/plugins/worktree"

if [ ! -d "$plugin_src" ]; then
  echo "worktree plugin not found at $plugin_src" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 required" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "git required" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

plugin_dir="$tmp_dir/worktree"
cp -R "$plugin_src" "$plugin_dir"
plugin="$plugin_dir/plugin.sh"
chmod +x "$plugin"

repo="$tmp_dir/repo"
git -c init.defaultBranch=main init "$repo" >/dev/null
(
  cd "$repo"
  git config user.email "test@example.com"
  git config user.name "Test"
  echo "hello" > README.md
  git add README.md
  git commit -m "init" >/dev/null
)

json_field() {
  python3 - "$1" "$2" <<'PY'
import json,sys
path=sys.argv[1]
raw=sys.argv[2]
obj=json.loads(raw)
cur=obj
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

LAST_STDERR=""

run_plugin() {
  local event_json="$1"
  if [ "$verbose" -eq 1 ]; then
    if [ -z "$event_json" ]; then
      echo "${c_event}event:${c_reset} (empty)" >&2
    else
      echo "${c_event}event:${c_reset}" >&2
      python3 -c '
import json,sys
raw=sys.stdin.read().strip()
try:
    obj=json.loads(raw)
    print(json.dumps(obj, indent=2, sort_keys=True))
except Exception:
    print(raw)
' <<<"$event_json" >&2
    fi
  fi
  local out
  local err_file="$tmp_dir/plugin-stderr.txt"
  : >"$err_file"
  out="$(YATMUX_PLUGIN_EVENT="$event_json" YATMUX_PLUGIN_ROOT="$plugin_dir" bash "$plugin" 2>"$err_file")"
  LAST_STDERR="$(cat "$err_file")"
  if [ "$verbose" -eq 1 ] && [ -n "$LAST_STDERR" ]; then
    echo "${c_err}--- stderr ---${c_reset}" >&2
    printf '%s\n' "$LAST_STDERR" >&2
    echo "${c_err}--------------${c_reset}" >&2
  fi
  show_output "$out"
  printf '%s' "$out"
}

expect_command() {
  local out="$1"
  local expected="$2"
  local first_line
  first_line="$(printf '%s\n' "$out" | head -n1 | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  if [ -z "$first_line" ]; then
    echo "Expected command $expected, got no output" >&2
    if [ -n "$LAST_STDERR" ]; then
      echo "$LAST_STDERR" >&2
    fi
    exit 1
  fi
  local cmd
  cmd="$(json_field command "$first_line")"
  if [ "$cmd" != "$expected" ]; then
    echo "Expected command $expected, got $cmd" >&2
    echo "$out" >&2
    exit 1
  fi
}

expect_contains_command() {
  local out="$1"
  local expected="$2"
  python3 -c '
import json,sys
expected=sys.argv[1]
lines=sys.stdin.read().splitlines()
commands=[]
for line in lines:
    line=line.strip()
    if not line:
        continue
    commands.append(json.loads(line).get("command"))
if expected not in commands:
    print("Expected command not found:", expected, "in", commands)
    sys.exit(1)
' "$expected" <<<"$out"
}

expect_command_field() {
  local out="$1"
  local field="$2"
  local expected="$3"
  local first_line
  first_line="$(printf '%s\n' "$out" | head -n1 | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  if [ -z "$first_line" ]; then
    echo "Expected $field=$expected, got no output" >&2
    if [ -n "$LAST_STDERR" ]; then
      echo "$LAST_STDERR" >&2
    fi
    exit 1
  fi
  local value
  value="$(json_field "$field" "$first_line")"
  if [ "$value" != "$expected" ]; then
    echo "Expected $field=$expected, got $value" >&2
    echo "$out" >&2
    exit 1
  fi
}

assert_worktree_exists() {
  local path="$1"
  if [ ! -d "$path" ]; then
    echo "Expected worktree at $path" >&2
    exit 1
  fi
}

assert_worktree_missing() {
  local path="$1"
  if [ -d "$path" ]; then
    echo "Expected worktree removed at $path" >&2
    exit 1
  fi
}

# startup
step "startup -> subscribe"
out="$(run_plugin '{"event":"startup"}')"
expect_command "$out" "subscribe"

# new -> prompt
step "new -> prompt"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"new","cwd":"'$repo'"}}')"
expect_command "$out" "prompt"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# prompt_response -> new_tab
step "prompt_response(new) -> new_tab"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":true,"value":"feat-one"}}')"
expect_command "$out" "new_tab"
new_path="$repo/.worktrees/feat-one"
assert_worktree_exists "$new_path"
expect_command_field "$out" cwd "$new_path"

# prompt_response cancel -> no output
step "prompt_response(new cancel) -> no output"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":false,"value":"ignored"}}')"
if [ -n "$out" ]; then
  echo "Expected no output on canceled prompt" >&2
  echo "$out" >&2
  exit 1
fi

# new with args -> new_tab
step "new(args) -> new_tab"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"new","cwd":"'$repo'","args":{"branch":"feat-two"}}}')"
expect_command "$out" "new_tab"
path_two="$repo/.worktrees/feat-two"
assert_worktree_exists "$path_two"
expect_command_field "$out" cwd "$path_two"

# new with args path/base_dir -> new_tab
step "new(args path/base_dir) -> new_tab"
base_dir="$repo/custom-worktrees"
explicit_path="$repo/custom-worktrees/feat-explicit"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"new","cwd":"'$repo'","args":{"branch":"feat-explicit","path":"'$explicit_path'","base_dir":"'$base_dir'"}}}')"
expect_command "$out" "new_tab"
assert_worktree_exists "$explicit_path"
expect_command_field "$out" cwd "$explicit_path"

# switch -> request_state
step "switch -> request_state"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"switch","cwd":"'$repo'"}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# state_response -> pick
step "state_response(switch) -> pick"
tabs_json='[{"id":7,"title":"feat-two","focused_pane":1,"panes":[1],"cwd":"'$path_two'","pane_cwds":{"1":"'$path_two'"}}]'
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "pick"
pick_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# pick -> request_state -> focus_tab
step "prompt_response(pick) -> request_state"
pick_index="$(python3 - <<PY
import json,subprocess,sys
out=subprocess.check_output(['git','-C','$repo','worktree','list','--porcelain']).decode()
items=[]
cur=None
for line in out.splitlines():
    if line.startswith('worktree '):
        if cur:
            items.append(cur)
        cur={'path':line.split(' ',1)[1]}
    elif cur is not None and line.startswith('branch '):
        ref=line.split(' ',1)[1]
        if ref.startswith('refs/heads/'):
            cur['branch']=ref[len('refs/heads/'):]
        else:
            cur['branch']=ref
    elif cur is not None and line.startswith('detached'):
        cur['branch']='(detached)'
if cur:
    items.append(cur)
for i,item in enumerate(items):
    if item.get('branch')=='feat-two':
        print(i)
        break
PY
)"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$pick_id'","ok":true,"index":'$pick_index'}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

step "state_response(switch_path) -> focus_tab"
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "focus_tab"
expect_command_field "$out" tab_id "7"

# switch args branch -> new_tab
step "switch(args branch) -> request_state -> new_tab"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"switch","cwd":"'$repo'","args":{"branch":"feat-one"}}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "new_tab"
expect_command_field "$out" cwd "$new_path"

# switch args path -> request_state -> new_tab
step "switch(args path) -> request_state -> new_tab"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"switch","cwd":"'$repo'","args":{"path":"'$new_path'"}}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "new_tab"
expect_command_field "$out" cwd "$new_path"

# close -> confirm (from inside a worktree path)
step "close -> confirm (from worktree cwd)"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"close","cwd":"'$path_two'"}}')"
expect_command "$out" "confirm"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# confirm -> request_state
step "prompt_response(confirm) -> request_state"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":true}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# state_response -> pick for close
step "state_response(close) -> pick"
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "pick"
pick_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# close pick -> request_state
step "prompt_response(close pick) -> request_state"
close_index="$(python3 - <<PY
import json,subprocess,sys
out=subprocess.check_output(['git','-C','$repo','worktree','list','--porcelain']).decode()
items=[]
cur=None
for line in out.splitlines():
    if line.startswith('worktree '):
        if cur:
            items.append(cur)
        cur={'path':line.split(' ',1)[1]}
    elif cur is not None and line.startswith('branch '):
        ref=line.split(' ',1)[1]
        if ref.startswith('refs/heads/'):
            cur['branch']=ref[len('refs/heads/'):]
        else:
            cur['branch']=ref
    elif cur is not None and line.startswith('detached'):
        cur['branch']='(detached)'
if cur:
    items.append(cur)
for i,item in enumerate(items):
    if item.get('branch')=='feat-two':
        print(i)
        break
PY
)"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$pick_id'","ok":true,"index":'$close_index'}}')"
expect_command "$out" "confirm"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# confirm removal -> request_state
step "prompt_response(close confirm) -> request_state"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":true}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# close path -> close_pane(s)
step "state_response(close_path) -> close_pane"
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_contains_command "$out" "close_pane"
assert_worktree_missing "$path_two"

# close cancel -> no output
step "close cancel -> no output"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"close","cwd":"'$repo'"}}')"
expect_command "$out" "confirm"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":false}}')"
if [ -n "$out" ]; then
  echo "Expected no output on canceled confirm" >&2
  echo "$out" >&2
  exit 1
fi

# close pick cancel -> no output
step "close pick cancel -> no output"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"close","cwd":"'$repo'"}}')"
expect_command "$out" "confirm"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":true}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "pick"
pick_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$pick_id'","ok":false}}')"
if [ -n "$out" ]; then
  echo "Expected no output on canceled pick" >&2
  echo "$out" >&2
  exit 1
fi

# sync with orphan tab
step "sync -> request_state"
orphan_tabs='[{"id":42,"title":"orphan","focused_pane":1,"panes":[1],"cwd":"'$repo'/ghost","pane_cwds":{"1":"'$repo'/ghost"}}]'
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"sync","cwd":"'$repo'","args":{"close_orphans":true}}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

step "state_response(sync) -> close_pane + new_tab"
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$orphan_tabs'}}')"
expect_contains_command "$out" "close_pane"
expect_contains_command "$out" "new_tab"

# sync without close_orphans -> new_tab only
step "sync(close_orphans=false) -> request_state -> new_tab only"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"sync","cwd":"'$repo'","args":{"close_orphans":false}}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$orphan_tabs'}}')"
expect_contains_command "$out" "new_tab"
if python3 - <<'PY' <<<"$out"
import json,sys
lines=sys.stdin.read().splitlines()
for line in lines:
    if not line.strip():
        continue
    if json.loads(line).get("command")=="close_pane":
        sys.exit(1)
sys.exit(0)
PY
then
  :
else
  echo "Expected no close_tab when close_orphans=false" >&2
  echo "$out" >&2
  exit 1
fi

# not in repo -> toast
step "not in repo -> toast"
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"new","cwd":"'$tmp_dir'"}}')"
expect_command "$out" "toast"

echo "worktree plugin e2e: ok"
