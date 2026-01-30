#!/usr/bin/env bash
#
# Creates a demo tmux session showing tmux-bridge in action.
# After running this script, attach to the session and take screenshots.
#
# Usage:
#   ./demo-setup.sh          # Set up demo
#   ./demo-setup.sh clean    # Clean up demo session
#
# For screenshots:
#   1. Run this script
#   2. Attach: tmux attach -t tb-demo
#   3. Take screenshot (light terminal theme)
#   4. Switch terminal to dark theme
#   5. Take screenshot
#   6. Save as docs/screenshot-light.png and docs/screenshot-dark.png

set -e

SESSION="tb-demo"
DEMO_ID="x7k"

if [[ "$1" == "clean" ]]; then
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    tmux kill-session -t "tb-$DEMO_ID" 2>/dev/null || true
    echo "Cleaned up demo sessions"
    exit 0
fi

# Clean up any existing demo sessions
tmux kill-session -t "$SESSION" 2>/dev/null || true
tmux kill-session -t "tb-$DEMO_ID" 2>/dev/null || true

# Create the actual tb session that tb commands will target
tmux new-session -d -s "tb-$DEMO_ID" -x 120 -y 30

# Create the demo display session with bash explicitly
tmux new-session -d -s "$SESSION" -x 100 -y 28 "bash --norc --noprofile"

# Wait for bash to start
sleep 0.3

# Create a horizontal split at top for background task (8 lines)
tmux split-window -t "$SESSION" -v -l 8 -b "bash --norc --noprofile"
sleep 0.2

# --- Set up the background pane (top) to look like a running build ---
# First clear and set empty prompt, then output the build log
tmux send-keys -t "$SESSION":0.0 $'PS1=""; clear; echo "   Compiling libc v0.2.155
   Compiling cfg-if v1.0.0
   Compiling proc-macro2 v1.0.86
   Compiling quote v1.0.36
   Compiling syn v2.0.72"; printf "\\e[1;32m   Compiling\\e[0m tb v0.1.0\\n"' Enter
sleep 0.3

# --- Set up the main pane (bottom) ---
tmux select-pane -t "$SESSION":0.1
sleep 0.1

# Set a clean prompt
tmux send-keys -t "$SESSION":0.1 'PS1="\[\e[1;32m\]user\[\e[0m\] \[\e[1;34m\]~/project\[\e[0m\] $ "' Enter
sleep 0.1
tmux send-keys -t "$SESSION":0.1 "clear" Enter
sleep 0.2

# Show the session start message (fake it for demo)
tmux send-keys -t "$SESSION":0.1 "tb start --session $DEMO_ID" Enter
sleep 0.5

# Show agent commands
tmux send-keys -t "$SESSION":0.1 "tb launch --session $DEMO_ID -- cargo build --release" Enter
sleep 0.5
tmux send-keys -t "$SESSION":0.1 "tb run --session $DEMO_ID -- echo 'Ready to deploy!'" Enter
sleep 0.5

echo ""
echo "Demo session created: $SESSION"
echo ""
echo "To take screenshots:"
echo "  1. tmux attach -t $SESSION"
echo "  2. Take screenshot with light terminal theme"
echo "  3. Switch to dark theme, take another screenshot"
echo "  4. Save as docs/screenshot-light.png and docs/screenshot-dark.png"
echo ""
echo "To clean up:"
echo "  ./demo-setup.sh clean"
