# Agent Instructions

This document is for AI coding assistants working on the tmux-bridge codebase.

## Project Overview

tmux-bridge is a two-script system allowing AI agents to inject commands into an interactive terminal session controlled by a human user. Built on tmux.

## File Structure

```
bin/
  tmux-bridge    # Human runs this - creates/attaches to tmux session
  tmux-send      # Agent uses this - injects command, returns output
```

Both scripts are fish shell.

## Key Design Decisions

1. **tmux is the foundation** - don't reinvent tmux
2. **Human terminal is primary** - agent is a guest, not the owner
3. **`tmux-send` behaves like a command wrapper** - stdout/stderr/exit status all work normally
4. **Markers are visible in v1** - this is acceptable, cleaner display is v2
5. **REPL support is v2** - don't try to implement it now

## Building and Testing

```fish
# Run the scripts directly from the repo
./bin/tmux-bridge
./bin/tmux-send -- ls -la

# Install to ~/.local/bin for PATH access
cp bin/tmux-bridge bin/tmux-send ~/.local/bin/
```

No build step. No dependencies beyond tmux and fish.

### Automated Tests

Run the automated test suite before asking the user for manual testing:

```fish
./test/run.fish
```

This creates its own tmux session, runs all tests, and cleans up. All tests should pass before proceeding to manual testing with the user.

## Implementation Details

### Session naming
tmux session is named `tmux-bridge-$USER` to avoid conflicts between users.

### Runtime directory
`/tmp/tmux-bridge-$USER/` stores temporary stderr files. Created with mode 700.

### Command markers
Use format `___TMUXSEND_START_$id___` and `___TMUXSEND_END_$id $exit_status___` where `$id` is random.

### Timeout behavior
1. No-output timeout (default 10s) - no new output for N seconds
2. Overall timeout (default 120s) - total elapsed time
3. Two-phase kill: SIGINT, wait 3s, SIGQUIT

## Error Messages

When no bridge is running, `tmux-send` must output:

```
Error: No tmux-bridge session is running.

You have used `tmux-send`, but it currently has nowhere to send to.

The user needs to start the interactive backing terminal using `tmux-bridge`.
Do not try this yourself - instead, ask the user to do it for you.

This is necessary for *interactive* commands - for example, sudo (requiring
a password prompt). If you can avoid interactive tools, then prefer that.
Otherwise, please ask the user now.
```

## Code Style

- Fish shell, not bash
- Minimal code - do the simplest thing that works
- Comments explain "why", not "what"
- Error messages should be helpful and actionable

## What Not To Do

- Don't hide markers from the human terminal yet (future possibility)
- Don't add features without updating README and VISION
- Don't use bash - this project uses fish

## Manual Walkthrough Testing

When making significant changes, run through this manual test matrix with the user.
The user must have `tmux-bridge` running in a terminal they control.

### Test Matrix

For each shell type (bash, Python, Nix, Node), test these scenarios:

| Scenario | What to test |
|----------|--------------|
| Normal | Command/eval completes successfully |
| Multiline | Commands with newlines are sent correctly |
| User types during | User types while command is running - their input gets captured |
| User kills/closes | User interrupts (Ctrl+C), kills the REPL, or closes the bridge |

### Bash (command mode)

```bash
# 1. Normal
tmux-send -- echo "hello"

# 2. Multiline
tmux-send -- 'echo "line1
line2
line3"'

# 3. User types during (use sleep so they have time)
tmux-send --timeout 20 -- 'sleep 10 && echo "done"'

# 4. User interrupts - expect timeout since no end marker
tmux-send --timeout 20 -- sleep 60
```

### Python REPL

```bash
# 1. Normal
tmux-send --repl python -- python3
tmux-send --repl python -- "1 + 1"
tmux-send --close

# 2. Multiline (note: Python needs extra blank line for indented blocks)
tmux-send --repl python -- python3
tmux-send --repl python -- 'print("hello"); print("world")'
tmux-send --close

# 3. User types during
tmux-send --repl python -- python3
tmux-send --repl python --timeout 20 -- 'import time; time.sleep(10); "done"'
tmux-send --close

# 4. User kills REPL (killall -9 python3) - expect timeout
tmux-send --repl python -- python3
tmux-send --repl python --timeout 20 -- 'import time; time.sleep(60); "done"'
tmux-send --close
```

### psql REPL

```bash
# 1. Normal
tmux-send --repl psql -- psql -d mydb
tmux-send --repl psql -- "SELECT 1 + 1;"
tmux-send --close

# 2. Multiline SQL
tmux-send --repl psql -- psql -d mydb
tmux-send --repl psql -- "
SELECT count(*), sum(amount)
FROM (
  SELECT sum(balance) AS amount
  FROM accounts
  GROUP BY user_id
) x;
"
tmux-send --close
```

### Nix REPL

```bash
# 1. Normal
tmux-send --repl nix -- nix repl
tmux-send --repl nix -- "1 + 1"
tmux-send --close

# 2. User types during (slow fold operation)
tmux-send --repl nix -- nix repl
tmux-send --repl nix --timeout 30 -- 'builtins.foldl (a: b: a + b) 0 (builtins.genList (x: x) 1000000)'
tmux-send --close
```

### Node REPL

```bash
# 1. Normal
tmux-send --repl node -- node
tmux-send --repl node -- "1 + 1"
tmux-send --close

# 2. User types during (sync busy loop)
tmux-send --repl node -- node
tmux-send --repl node --timeout 20 -- 'const start = Date.now(); while (Date.now() - start < 10000) {}; "done"'
tmux-send --close
```

### Expected behaviors

- **Normal**: Output shows result and prompt
- **Multiline**: Newlines preserved, semicolons not escaped
- **User types during**: Their keystrokes appear in captured output
- **User Ctrl+C**: Command/REPL handles interrupt, may return to prompt with error
- **User kills process**: Timeout occurs, last 20 lines of pane shown
- **User closes bridge**: Clean error "tmux-bridge session closed"
