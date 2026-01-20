#!/usr/bin/env bash
set -euo pipefail

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

run_plugin() {
  local event_json="$1"
  YATMUX_PLUGIN_EVENT="$event_json" YATMUX_PLUGIN_ROOT="$plugin_dir" bash "$plugin"
}

expect_command() {
  local out="$1"
  local expected="$2"
  local first_line
  first_line="$(printf '%s\n' "$out" | head -n1)"
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
  first_line="$(printf '%s\n' "$out" | head -n1)"
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
out="$(run_plugin '{"event":"startup"}')"
expect_command "$out" "subscribe"

# new -> prompt
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"new","cwd":"'$repo'"}}')"
expect_command "$out" "prompt"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# prompt_response -> new_tab
out="$(run_plugin '{"event":"prompt_response","data":{"id":"'$req_id'","ok":true,"value":"feat-one"}}')"
expect_command "$out" "new_tab"
new_path="$repo/.worktrees/feat-one"
assert_worktree_exists "$new_path"
expect_command_field "$out" cwd "$new_path"

# new with args -> new_tab
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"new","cwd":"'$repo'","args":{"branch":"feat-two"}}}')"
expect_command "$out" "new_tab"
path_two="$repo/.worktrees/feat-two"
assert_worktree_exists "$path_two"
expect_command_field "$out" cwd "$path_two"

# switch -> request_state
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"switch","cwd":"'$repo'"}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# state_response -> pick
tabs_json='[{"id":7,"title":"feat-two","focused_pane":1,"panes":[1],"cwd":"'$path_two'"}]'
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "pick"
pick_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# pick -> request_state -> focus_tab
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

out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "focus_tab"
expect_command_field "$out" tab_id "7"

# close -> request_state
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"close","cwd":"'$repo'"}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# state_response -> pick for close
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "pick"
pick_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# close pick -> request_state
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
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

# close path -> close_tab
out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$tabs_json'}}')"
expect_command "$out" "close_tab"
expect_command_field "$out" tab_id "7"
assert_worktree_missing "$path_two"

# sync with orphan tab
orphan_tabs='[{"id":42,"title":"orphan","focused_pane":1,"panes":[1],"cwd":"'$repo'/ghost"}]'
out="$(run_plugin '{"event":"plugin_command","data":{"plugin":"worktree","command":"sync","cwd":"'$repo'","args":{"close_orphans":true}}}')"
expect_command "$out" "request_state"
req_id="$(json_field id "$(printf '%s\n' "$out" | head -n1)")"

out="$(run_plugin '{"event":"state_response","data":{"id":"'$req_id'","tabs":'$orphan_tabs'}}')"
expect_contains_command "$out" "close_tab"
expect_contains_command "$out" "new_tab"

echo "worktree plugin e2e: ok"
