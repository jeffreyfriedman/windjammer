#!/bin/sh
# Agent adapter: one driver iteration with Claude Code in non-interactive mode.
#
#   python3 -m agentdx run --agent-cmd 'sh {harness}/agents/claude_code.sh {workspace} {prompt_file} {feedback_file} {iteration}' ...
#
# Iteration 1 gets the task prompt. Later iterations get the compiler/test feedback of the
# previous attempt (visible examples only, never hidden results) and edit the file in place.
# Token usage is taken from Claude Code's JSON result and written to $AGENTDX_USAGE_FILE.
#
# Environment (all optional):
#   AGENTDX_CLAUDE_MODEL   model alias or id passed as --model
#   AGENTDX_CLAUDE_TURNS   --max-turns per iteration (default 30)
#   AGENTDX_CLAUDE_ARGS    extra flags, e.g. "--append-system-prompt-file ./wj_agent_notes.md"
set -eu
ws="$1"; prompt_file="$2"; feedback_file="$3"; iteration="$4"
cd "$ws"

if [ "$iteration" = "1" ]; then
  msg="$(cat "$prompt_file")"
else
  msg="Your previous attempt did not yet pass the hidden tests. Build/test feedback on the visible examples from the last attempt:

$(cat "$feedback_file" 2>/dev/null || echo '(none)')

Re-read PROMPT.md, fix the program in place (run ./check as you go), and stop when you believe it is correct for ALL inputs, not just the examples."
fi

model_flag=""
[ -n "${AGENTDX_CLAUDE_MODEL:-}" ] && model_flag="--model ${AGENTDX_CLAUDE_MODEL}"

# shellcheck disable=SC2086
out="$(claude -p "$msg" --output-format json --permission-mode acceptEdits \
  --allowedTools "Read,Edit,Write,Bash" --max-turns "${AGENTDX_CLAUDE_TURNS:-30}" \
  $model_flag ${AGENTDX_CLAUDE_ARGS:-})" || true

printf '%s' "$out" | python3 -c '
import json, os, sys
try:
    d = json.load(sys.stdin)
except ValueError:
    sys.exit(0)
u = d.get("usage") or {}
tin = u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0) + u.get("cache_read_input_tokens", 0)
json.dump({"input_tokens": tin, "output_tokens": u.get("output_tokens", 0),
           "cost_usd": d.get("total_cost_usd")}, open(os.environ["AGENTDX_USAGE_FILE"], "w"))
'
