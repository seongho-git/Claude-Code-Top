#!/bin/bash

# Set background session name
SESSION="claude_auto_usage"

echo "⏳ Fetching usage information from Claude Code..."

# 1. Run Claude Code in a background tmux session
tmux new-session -d -s $SESSION "claude"

# Wait for loading (adjustable between 2~3 seconds depending on system speed)
sleep 2

# 2. Automatically type '/usage' and send Enter (simulates human typing)
tmux send-keys -t $SESSION "/usage" Enter

# Wait for API response
sleep 2

# 3. Capture screen text and parse
# tmux capture-pane removes all terminal UI color codes and retrieves pure text.
echo "==================================="
tmux capture-pane -p -t $SESSION | awk '
  # Start output when the word "Usage" appears
  /Usage/ { flag=1 }
  
  # End output if the prompt (>) symbol appears again
  flag && /^> / { flag=0; exit }
  
  # Print text only when flag is 1 (ignore empty lines)
  flag && NF > 0 { print $0 }
'
echo "==================================="

# 4. Exit session securely (type /exit then kill session)
tmux send-keys -t $SESSION "/exit" Enter
sleep 1
tmux kill-session -t $SESSION 2>/dev/null