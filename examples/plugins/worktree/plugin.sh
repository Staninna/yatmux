#!/usr/bin/env bash
set -euo pipefail

# Configuration
# Number of days after which to clean up old state files (0 to disable cleanup)
STATE_CLEANUP_DAYS=${WORKTREE_STATE_CLEANUP_DAYS:-1}
# Require an extra confirmation after selecting a worktree to close (0 to disable)
CLOSE_CONFIRM_AFTER_PICK=${WORKTREE_CLOSE_CONFIRM_AFTER_PICK:-1}

event_json="${YATMUX_PLUGIN_EVENT:-}"
if [ -z "$event_json" ]; then
  event_json="$(cat)"
fi

if [ -z "$event_json" ]; then
  exit 0
fi

plugin_root="${YATMUX_PLUGIN_ROOT:-.}"
state_dir="$plugin_root/.state"
mkdir -p "$state_dir"

json_get() {
  python3 - "$1" "$2" <<'PY'
import json,sys
path=sys.argv[1]
raw=sys.argv[2] if len(sys.argv) > 2 else ""
if not raw:
    sys.exit(1)
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

json_escape() {
  python3 - "$1" <<'PY'
import json,sys
s=sys.argv[1] if len(sys.argv) > 1 else ""
print(json.dumps(s)[1:-1])
PY
}

emit_toast() {
  local msg esc
  msg="$1"
  esc="$(json_escape "$msg")"
  printf '{"command":"toast","message":"%s"}\n' "$esc"
}

emit_new_tab() {
  local cwd title cwd_esc title_esc
  cwd="$1"
  title="${2:-}"
  cwd_esc="$(json_escape "$cwd")"
  if [ -n "$title" ]; then
    title_esc="$(json_escape "$title")"
    printf '{"command":"new_tab","cwd":"%s","title":"%s"}\n' "$cwd_esc" "$title_esc"
  else
    printf '{"command":"new_tab","cwd":"%s"}\n' "$cwd_esc"
  fi
}

emit_pick() {
  local id title items_json id_esc title_esc
  id="$1"
  title="$2"
  items_json="$3"
  id_esc="$(json_escape "$id")"
  title_esc="$(json_escape "$title")"
  printf '{"command":"pick","id":"%s","title":"%s","items":%s}\n' "$id_esc" "$title_esc" "$items_json"
}

emit_prompt() {
  local id title message id_esc title_esc msg_esc
  id="$1"
  title="$2"
  message="$3"
  id_esc="$(json_escape "$id")"
  title_esc="$(json_escape "$title")"
  msg_esc="$(json_escape "$message")"
  printf '{"command":"prompt","id":"%s","title":"%s","message":"%s"}\n' "$id_esc" "$title_esc" "$msg_esc"
}

emit_confirm() {
  local id title message ok_label cancel_label id_esc title_esc msg_esc ok_esc cancel_esc
  id="$1"
  title="$2"
  message="$3"
  ok_label="${4:-Close}"
  cancel_label="${5:-Cancel}"
  id_esc="$(json_escape "$id")"
  title_esc="$(json_escape "$title")"
  msg_esc="$(json_escape "$message")"
  ok_esc="$(json_escape "$ok_label")"
  cancel_esc="$(json_escape "$cancel_label")"
  printf '{"command":"confirm","id":"%s","title":"%s","message":"%s","ok_label":"%s","cancel_label":"%s"}\n' \
    "$id_esc" "$title_esc" "$msg_esc" "$ok_esc" "$cancel_esc"
}

emit_request_state() {
  local id id_esc
  id="$1"
  id_esc="$(json_escape "$id")"
  printf '{"command":"request_state","id":"%s"}\n' "$id_esc"
}

emit_focus_tab() {
  printf '{"command":"focus_tab","tab_id":%s}\n' "$1"
}

emit_close_tab() {
  printf '{"command":"close_tab","tab_id":%s}\n' "$1"
}

emit_close_pane() {
  printf '{"command":"close_pane","tab_id":%s,"pane_id":%s}\n' "$1" "$2"
}

slugify() {
  echo "$1" | \
    sed 's#[^A-Za-z0-9._-]#-#g' | \
    sed 's#-\+#-#g' | \
    sed 's#^-##' | \
    sed 's#-$##'
}

repo_root_from() {
  local root
  root="$({ git -C "$1" worktree list --porcelain 2>/dev/null || true; } | awk '/^worktree /{print $2; exit}')"
  if [ -n "$root" ]; then
    echo "$root"
    return 0
  fi
  git -C "$1" rev-parse --show-toplevel 2>/dev/null || true
}

worktree_list_json() {
  { git -C "$1" worktree list --porcelain 2>/dev/null || true; } | python3 -c '
import sys,json
text=sys.stdin.read().splitlines()
items=[]
cur=None
for line in text:
    if line.startswith("worktree "):
        if cur:
            items.append(cur)
        cur={"path":line.split(" ",1)[1]}
    elif cur is not None and line.startswith("branch "):
        ref=line.split(" ",1)[1]
        if ref.startswith("refs/heads/"):
            cur["branch"]=ref[len("refs/heads/"):]
        else:
            cur["branch"]=ref
    elif cur is not None and line.startswith("detached"):
        cur["branch"]="(detached)"
if cur:
    items.append(cur)
print(json.dumps(items))
'
}

worktree_items_json() {
  python3 -c '
import json,sys
wt=json.loads(sys.stdin.read())
out=[f"{item.get("branch","(detached)")} — {item.get("path","")}" for item in wt]
print(json.dumps(out))
'
}

worktree_path_for_branch() {
  python3 -c '
import json,sys
wt=json.loads(sys.stdin.read())
branch=sys.argv[1]
for item in wt:
    if item.get("branch")==branch:
        print(item.get("path",""))
        break
' "$1"
}

tab_id_for_path() {
  python3 -c '
import json,sys
_tabs=json.loads(sys.stdin.read())
target=sys.argv[1]
for tab in _tabs:
    if tab.get("cwd")==target:
        print(tab.get("id"))
        break
' "$1"
}

panes_for_path() {
  python3 -c '
import json,sys
tabs=json.loads(sys.stdin.read())
target=sys.argv[1]
out=[]
for tab in tabs:
    tab_id=tab.get("id")
    pane_cwds=tab.get("pane_cwds") or {}
    if isinstance(pane_cwds, dict):
        items=pane_cwds.items()
    else:
        items=[]
    for pane_id, cwd in items:
        if cwd == target:
            try:
                pane_id_int=int(pane_id)
            except Exception:
                continue
            out.append((tab_id, pane_id_int))
for tab_id, pane_id in out:
    print(f"{tab_id} {pane_id}")
' "$1"
}

select_item_by_index() {
  python3 -c '
import json,sys
items=json.loads(sys.stdin.read())
idx=int(sys.argv[1]) if sys.argv[1] else -1
if 0 <= idx < len(items):
    print(json.dumps(items[idx]))
' "$1"
}

emit_sync_commands() {
  local close_orphans="$1"
  python3 -c '
import json,sys
_tabs=json.loads(sys.stdin.readline())
_wt=json.loads(sys.stdin.readline())
close_orphans=sys.stdin.readline().strip().lower()=="true"

tab_map={}
for tab in _tabs:
    cwd=tab.get("cwd")
    if cwd:
        tab_map[cwd]=tab

for item in _wt:
    path=item.get("path")
    branch=item.get("branch","")
    if not path:
        continue
    tab=tab_map.get(path)
    if tab:
        print(json.dumps({"command":"set_tab_title","tab_id":tab["id"],"title":branch}))
    else:
        print(json.dumps({"command":"new_tab","cwd":path,"title":branch}))

if close_orphans:
    wt_paths=set([item.get("path") for item in _wt if item.get("path")])
    for tab in _tabs:
        cwd=tab.get("cwd")
        if cwd and cwd not in wt_paths:
            print(json.dumps({"command":"close_tab","tab_id":tab["id"]}))
'
}

create_worktree() {
  local repo branch path base_dir safe_branch
  repo="$1"
  branch="$2"
  path="$3"
  base_dir="$4"
  if [ -z "$path" ]; then
    if [ -z "$base_dir" ]; then
      base_dir="$repo/.worktrees"
    fi
    mkdir -p "$base_dir"
    safe_branch="$(slugify "$branch")"
    path="$base_dir/$safe_branch"
  fi
  if git -C "$repo" show-ref --verify --quiet "refs/heads/$branch"; then
    git -C "$repo" worktree add "$path" "$branch" >/dev/null 2>&1 || true
  else
    git -C "$repo" worktree add -b "$branch" "$path" >/dev/null 2>&1 || true
  fi
  emit_new_tab "$path" "$branch"
}

save_request() {
  local id payload
  id="$1"
  payload="$2"
  printf '%s' "$payload" >"$state_dir/$id.json"
}

load_request() {
  local id
  id="$1"
  [ -f "$state_dir/$id.json" ] && cat "$state_dir/$id.json"
}

rm_request() {
  rm -f "$state_dir/$1.json"
}

event="$(json_get event "$event_json" 2>/dev/null || true)"

if [ "$event" = "startup" ]; then
  # Check for python3 dependency
  if ! command -v python3 >/dev/null 2>&1; then
    printf '{"command":"toast","message":"⚠️  Worktree plugin disabled: python3 required"}\n'
    exit 0
  fi

  # Cleanup old state files (configurable age, 0 to disable)
  if [ "$STATE_CLEANUP_DAYS" -gt 0 ] 2>/dev/null; then
    find "$state_dir" -name "*.json" -type f -mtime +"$STATE_CLEANUP_DAYS" -delete 2>/dev/null || true
  fi

  printf '{"command":"subscribe","events":["plugin_command","prompt_response","state_response"]}\n'
  exit 0
fi

if [ "$event" = "plugin_command" ]; then
  plugin="$(json_get data.plugin "$event_json" 2>/dev/null || true)"
  if [ "$plugin" != "worktree" ]; then
    exit 0
  fi
  cmd="$(json_get data.command "$event_json" 2>/dev/null || true)"
  cwd="$(json_get data.cwd "$event_json" 2>/dev/null || true)"
  args_json="$(json_get data.args "$event_json" 2>/dev/null || true)"

  repo="$(repo_root_from "$cwd")"
  if [ -z "$repo" ]; then
    emit_toast "worktree: not in a git repo"
    exit 0
  fi

  case "$cmd" in
    new)
      branch=""
      path=""
      base_dir=""
      if [ -n "$args_json" ]; then
        branch="$(json_get branch "$args_json" 2>/dev/null || true)"
        path="$(json_get path "$args_json" 2>/dev/null || true)"
        base_dir="$(json_get base_dir "$args_json" 2>/dev/null || true)"
      fi
      if [ -z "$branch" ]; then
        req_id="wt-new-$(date +%s%N)"
        save_request "$req_id" "{\"action\":\"new\",\"repo\":\"$repo\",\"path\":\"$path\",\"base_dir\":\"$base_dir\"}"
        emit_prompt "$req_id" "New worktree" "Branch name?"
        exit 0
      fi
      create_worktree "$repo" "$branch" "$path" "$base_dir"
      exit 0
      ;;
    switch)
      req_id="wt-switch-$(date +%s%N)"
      save_request "$req_id" "{\"action\":\"switch\",\"repo\":\"$repo\",\"args\":${args_json:-null}}"
      emit_request_state "$req_id"
      exit 0
      ;;
    close)
      req_id="wt-close-confirm-$(date +%s%N)"
      save_request "$req_id" "{\"action\":\"close_confirm\",\"repo\":\"$repo\"}"
      emit_confirm "$req_id" "Close Worktree" "Select a worktree to remove. This cannot be undone." "Close" "Cancel"
      exit 0
      ;;
    sync)
      req_id="wt-sync-$(date +%s%N)"
      save_request "$req_id" "{\"action\":\"sync\",\"repo\":\"$repo\",\"args\":${args_json:-null}}"
      emit_request_state "$req_id"
      exit 0
      ;;
    *)
      emit_toast "worktree: unknown command $cmd"
      exit 0
      ;;
  esac
fi

if [ "$event" = "prompt_response" ]; then
  req_id="$(json_get data.id "$event_json" 2>/dev/null || true)"
  req_json="$(load_request "$req_id")"
  if [ -z "$req_json" ]; then
    exit 0
  fi
  action="$(json_get action "$req_json" 2>/dev/null || true)"
  repo="$(json_get repo "$req_json" 2>/dev/null || true)"
  ok="$(json_get data.ok "$event_json" 2>/dev/null || true)"
  if [ "$ok" != "True" ] && [ "$ok" != "true" ]; then
    rm_request "$req_id"
    exit 0
  fi
  case "$action" in
    new)
      branch="$(json_get data.value "$event_json" 2>/dev/null || true)"
      path="$(json_get path "$req_json" 2>/dev/null || true)"
      base_dir="$(json_get base_dir "$req_json" 2>/dev/null || true)"
      if [ -z "$branch" ]; then
        emit_toast "worktree: branch required"
        rm_request "$req_id"
        exit 0
      fi
      create_worktree "$repo" "$branch" "$path" "$base_dir"
      ;;
    close_confirm)
      # User confirmed they want to close a worktree, now show picker
      new_req_id="wt-close-$(date +%s%N)"
      save_request "$new_req_id" "{\"action\":\"close\",\"repo\":\"$repo\"}"
      emit_request_state "$new_req_id"
      ;;
    switch_pick)
      index="$(json_get data.index "$event_json" 2>/dev/null || true)"
      item_json="$(select_item_by_index "$index" <<<"$(json_get items "$req_json" 2>/dev/null || echo '[]')")"
      if [ -z "$item_json" ]; then
        rm_request "$req_id"
        exit 0
      fi
      target_path="$(json_get path "$item_json" 2>/dev/null || true)"
      if [ -n "$target_path" ]; then
        new_id="wt-switch-path-$(date +%s%N)"
        save_request "$new_id" "{\"action\":\"switch_path\",\"repo\":\"$repo\",\"path\":\"$target_path\"}"
        emit_request_state "$new_id"
      fi
      ;;
    close_pick)
      index="$(json_get data.index "$event_json" 2>/dev/null || true)"
      item_json="$(select_item_by_index "$index" <<<"$(json_get items "$req_json" 2>/dev/null || echo '[]')")"
      if [ -z "$item_json" ]; then
        rm_request "$req_id"
        exit 0
      fi
      target_path="$(json_get path "$item_json" 2>/dev/null || true)"
      if [ -n "$target_path" ]; then
        if [ "${CLOSE_CONFIRM_AFTER_PICK}" -gt 0 ] 2>/dev/null; then
          branch="$(json_get branch "$item_json" 2>/dev/null || true)"
          new_id="wt-close-final-$(date +%s%N)"
          save_request "$new_id" "{\"action\":\"close_confirm_final\",\"repo\":\"$repo\",\"path\":\"$target_path\",\"branch\":\"$branch\"}"
          if [ -n "$branch" ]; then
            emit_confirm "$new_id" "Confirm removal" "Remove worktree '$branch'? This cannot be undone." "Remove" "Cancel"
          else
            emit_confirm "$new_id" "Confirm removal" "Remove worktree at '$target_path'? This cannot be undone." "Remove" "Cancel"
          fi
        else
          git -C "$repo" worktree remove -f "$target_path" >/dev/null 2>&1 || true
          new_id="wt-close-path-$(date +%s%N)"
          save_request "$new_id" "{\"action\":\"close_path\",\"repo\":\"$repo\",\"path\":\"$target_path\"}"
          emit_request_state "$new_id"
        fi
      fi
      ;;
    close_confirm_final)
      target_path="$(json_get path "$req_json" 2>/dev/null || true)"
      if [ -z "$target_path" ]; then
        rm_request "$req_id"
        exit 0
      fi
      git -C "$repo" worktree remove -f "$target_path" >/dev/null 2>&1 || true
      new_id="wt-close-path-$(date +%s%N)"
      save_request "$new_id" "{\"action\":\"close_path\",\"repo\":\"$repo\",\"path\":\"$target_path\"}"
      emit_request_state "$new_id"
      ;;
  esac
  rm_request "$req_id"
  exit 0
fi

if [ "$event" = "state_response" ]; then
  req_id="$(json_get data.id "$event_json" 2>/dev/null || true)"
  req_json="$(load_request "$req_id")"
  if [ -z "$req_json" ]; then
    exit 0
  fi
  action="$(json_get action "$req_json" 2>/dev/null || true)"
  repo="$(json_get repo "$req_json" 2>/dev/null || true)"
  tabs_json="$(json_get data.tabs "$event_json" 2>/dev/null || true)"
  wt_json="$(worktree_list_json "$repo")"

  case "$action" in
    switch)
      args_json="$(json_get args "$req_json" 2>/dev/null || true)"
      target_branch=""
      target_path=""
      if [ -n "$args_json" ]; then
        target_branch="$(json_get branch "$args_json" 2>/dev/null || true)"
        target_path="$(json_get path "$args_json" 2>/dev/null || true)"
      fi
      if [ -z "$target_path" ] && [ -n "$target_branch" ]; then
        target_path="$(worktree_path_for_branch "$target_branch" <<<"$wt_json")"
      fi
      if [ -z "$target_path" ]; then
        items_json="$(worktree_items_json <<<"$wt_json")"
        pick_id="wt-pick-$(date +%s%N)"
        save_request "$pick_id" "{\"action\":\"switch_pick\",\"repo\":\"$repo\",\"items\":$wt_json}"
        emit_pick "$pick_id" "Switch worktree" "$items_json"
        rm_request "$req_id"
        exit 0
      fi
      ;;
    close)
      items_json="$(worktree_items_json <<<"$wt_json")"
      pick_id="wt-close-pick-$(date +%s%N)"
      save_request "$pick_id" "{\"action\":\"close_pick\",\"repo\":\"$repo\",\"items\":$wt_json}"
      emit_pick "$pick_id" "Close worktree" "$items_json"
      rm_request "$req_id"
      exit 0
      ;;
    sync)
      close_orphans="$(json_get args.close_orphans "$req_json" 2>/dev/null || true)"
      emit_sync_commands "$close_orphans" <<<"$tabs_json
$wt_json
$close_orphans"
      rm_request "$req_id"
      exit 0
      ;;
    switch_path)
      target_path="$(json_get path "$req_json" 2>/dev/null || true)"
      ;;
    close_path)
      target_path="$(json_get path "$req_json" 2>/dev/null || true)"
      ;;
  esac

  if [ "$action" = "switch" ] || [ "$action" = "switch_path" ]; then
    if [ -n "$target_path" ]; then
      tab_id="$(tab_id_for_path "$target_path" <<<"$tabs_json")"
      if [ -n "$tab_id" ]; then
        emit_focus_tab "$tab_id"
      else
        emit_new_tab "$target_path"
      fi
    fi
  fi

  if [ "$action" = "close_path" ]; then
    if [ -n "$target_path" ]; then
      matches="$(panes_for_path "$target_path" <<<"$tabs_json")"
      if [ -n "$matches" ]; then
        while read -r tab_id pane_id; do
          [ -n "$tab_id" ] || continue
          [ -n "$pane_id" ] || continue
          emit_close_pane "$tab_id" "$pane_id"
        done <<<"$matches"
      else
        tab_id="$(tab_id_for_path "$target_path" <<<"$tabs_json")"
        if [ -n "$tab_id" ]; then
          emit_close_tab "$tab_id"
        fi
      fi
    fi
  fi

  rm_request "$req_id"
  exit 0
fi
