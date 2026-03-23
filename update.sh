#!/bin/sh

if [ -n "${SHELL:-}" ] && [ -z "${CCTOP_REEXEC:-}" ] && [ -x "$SHELL" ]; then
  CCTOP_REEXEC=1 exec "$SHELL" "$0" "$@"
fi

if [ -n "${ZSH_VERSION:-}" ]; then
  emulate -L sh
  setopt SH_WORD_SPLIT
fi

# Set background session name
SESSION="claude_auto_usage"

echo "⏳ Fetching usage information from Claude Code..."

confirm_trust_prompt() {
  pane_text="$1"

  if [ "${TRUST_CONFIRMED:-0}" -eq 1 ]; then
    return 1
  fi

  if echo "$pane_text" | grep -q "Yes, I trust this folder"; then
    if echo "$pane_text" | grep -Eq "❯[[:space:]]*1\. Yes, I trust this folder|>[[:space:]]*1\. Yes, I trust this folder"; then
      tmux send-keys -t $SESSION Enter
    else
      tmux send-keys -t $SESSION Up Enter
    fi
    TRUST_CONFIRMED=1
    sleep 1
    return 0
  fi

  return 1
}

WORK_DIR=$(pwd)

echo "📂 Running claude in: $WORK_DIR"

# Kill any existing session to avoid conflicts
tmux kill-session -t $SESSION 2>/dev/null

# 1. Launch Claude in a background tmux session from the current directory
tmux new-session -d -s $SESSION -c "$WORK_DIR" "claude"

# Wait for Claude to fully start (up to 10 s, checking every 0.5 s)
TRUST_CONFIRMED=0
MAX_WAIT=10
elapsed=0
while [ $elapsed -lt $MAX_WAIT ]; do
  pane_text=$(tmux capture-pane -p -t $SESSION 2>/dev/null)
  confirm_trust_prompt "$pane_text" && continue
  # Claude is ready when we see the input prompt ("> " at line start)
  if echo "$pane_text" | grep -q "^> "; then
    break
  fi
  sleep 0.5
  elapsed=$(( elapsed + 1 ))
done

# 2. Send /usage command
tmux send-keys -t $SESSION "/usage" Enter

# 3. Poll until usage text appears or timeout
MAX_WAIT=20
elapsed=0
CAPTURED=""
LAST_PANE=""
while [ $elapsed -lt $MAX_WAIT ]; do
  sleep 1
  pane_text=$(tmux capture-pane -p -t $SESSION 2>/dev/null)
  confirm_trust_prompt "$pane_text" && continue
  [ -n "$pane_text" ] && LAST_PANE="$pane_text"
  if echo "$pane_text" | grep -Eq '%|Usage|Current session|Current week|Extra usage'; then
    CAPTURED="$pane_text"
    break
  fi
  elapsed=$(( elapsed + 1 ))
done

if [ -z "$CAPTURED" ]; then
  CAPTURED="$LAST_PANE"
fi

# 4. Parse and print the Usage section
echo "==================================="
if [ -n "$CAPTURED" ]; then
  PARSED_OUTPUT=$(echo "$CAPTURED" | awk '
    /Usage/        { flag=1 }
    flag && /^> /  { flag=0; exit }
    flag && NF > 0 { print $0 }
  ')
  if [ -n "$PARSED_OUTPUT" ]; then
    echo "$PARSED_OUTPUT"
  else
    echo "$CAPTURED"
  fi
fi
echo "==================================="

# 5. Exit Claude and clean up
tmux send-keys -t $SESSION "/exit" Enter
sleep 1
tmux kill-session -t $SESSION 2>/dev/null
