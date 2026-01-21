# Bash shell integration for yatmux.
#
# Provides:
# - OSC 7 (current working directory)
# - OSC 133 (semantic prompt markers)
#
# Usage:
#   source /path/to/yatmux/scripts/shell/yatmux.bash
#
# You can also gate this on TERM_PROGRAM:
#   [[ ${TERM_PROGRAM-} == yatmux ]] && source .../yatmux.bash

__yatmux_integration_is_interactive() {
  [[ $- == *i* ]]
}

__yatmux_osc() {
  # Send an OSC sequence terminated with BEL.
  # Usage: __yatmux_osc "7;file://..." (no leading ESC]).
  printf '\e]%s\a' "$1"
}

__yatmux_emit_cwd_osc7() {
  # OSC 7 uses a file:// URL.
  # Prefer host from $HOSTNAME if present.
  local host=${HOSTNAME-}
  local path=${PWD-}
  # shellcheck disable=SC2059
  __yatmux_osc "7;file://${host}${path}"
}

__yatmux_emit_semantic_prompt_start() {
  __yatmux_osc '133;A'
}

__yatmux_emit_semantic_prompt_end_start_input() {
  __yatmux_osc '133;B'
}

__yatmux_emit_semantic_input_end_start_output() {
  __yatmux_osc '133;C'
}

__yatmux_emit_semantic_command_end() {
  # Include exit status.
  local status=$1
  __yatmux_osc "133;D;${status}"
}

__yatmux_install_ps_markers() {
  # Idempotently wrap PS1 and PS0.
  # We keep the user's original PS1 so they can change it later.
  if [[ -z ${__YATMUX_ORIG_PS1+x} ]]; then
    __YATMUX_ORIG_PS1=${PS1-}
  fi

  # Wrap the prompt with prompt boundary markers.
  # \[ \] tell readline these are zero-width (non-printing).
  # Use BEL (\a) to terminate OSC sequences for broader compatibility.
  local osc133_a=$'\e]133;A\a'
  local osc133_b=$'\e]133;B\a'
  PS1="\[${osc133_a}\]${__YATMUX_ORIG_PS1}\[${osc133_b}\]"

  # PS0 runs just before executing each interactive command.
  # It is not interpreted by readline, so no \[ \] needed.
  PS0=$'\e]133;C\a'
}

__yatmux_prompt_command() {
  __yatmux_integration_is_interactive || return 0

  local last_status=$?

  # Mark the prior command completion and refresh cwd.
  __yatmux_emit_semantic_command_end "${last_status}"
  __yatmux_emit_cwd_osc7

  __yatmux_install_ps_markers

  return 0
}

__yatmux_add_prompt_command() {
  # Respect existing PROMPT_COMMAND (string or array).
  if [[ ${__YATMUX_PROMPT_COMMAND_INSTALLED-} == 1 ]]; then
    return 0
  fi
  __YATMUX_PROMPT_COMMAND_INSTALLED=1

  if declare -p PROMPT_COMMAND &>/dev/null; then
    # If it's an array, append.
    if declare -p PROMPT_COMMAND 2>/dev/null | grep -q 'declare \-a'; then
      PROMPT_COMMAND+=(__yatmux_prompt_command)
      return 0
    fi
  fi

  # Append (not prepend) so we run AFTER other prompt commands that may set PS1.
  if [[ -n ${PROMPT_COMMAND-} ]]; then
    # Trim trailing semicolons/whitespace to avoid ";;" syntax errors.
    local trimmed
    trimmed="$(printf '%s' "$PROMPT_COMMAND" | sed -e 's/[[:space:]]*$//' -e 's/;*$//')"
    if [[ "$trimmed" == *"__yatmux_prompt_command"* ]]; then
      PROMPT_COMMAND="$trimmed"
    else
      PROMPT_COMMAND="${trimmed}; __yatmux_prompt_command"
    fi
  else
    PROMPT_COMMAND="__yatmux_prompt_command"
  fi
}

__yatmux_add_prompt_command
