# Agent Instructions

This document is for AI coding assistants working on the pty-bridge codebase.

## Project Overview

pty-bridge is a two-script system allowing AI agents to inject commands into an interactive terminal session controlled by a human user. Built on tmux.

## File Structure

```
bin/
  pty-bridge    # Human runs this - creates/attaches to tmux session
  pty-send      # Agent uses this - injects command, returns output
```

Both scripts are fish shell.

## Key Design Decisions

1. **tmux is the foundation** - don't reinvent PTY multiplexing
2. **Human terminal is primary** - agent is a guest, not the owner
3. **`pty-send` behaves like a command wrapper** - stdout/stderr/exit status all work normally
4. **Markers are visible in v1** - this is acceptable, cleaner display is v2
5. **REPL support is v2** - don't try to implement it now

## Building and Testing

```fish
# Run the scripts directly from the repo
./bin/pty-bridge
./bin/pty-send -- ls -la

# Install to ~/.local/bin for PATH access
cp bin/pty-bridge bin/pty-send ~/.local/bin/
```

No build step. No dependencies beyond tmux and fish.

## Implementation Details

### Session naming
tmux session is named `pty-bridge-$USER` to avoid conflicts between users.

### Runtime directory
`/tmp/pty-bridge-$USER/` stores temporary stderr files. Created with mode 700.

### Command markers
Use format `___PTYSEND_START_$id___` and `___PTYSEND_END_$id $exit_status___` where `$id` is random.

### Timeout behavior
1. No-output timeout (default 10s) - no new output for N seconds
2. Overall timeout (default 120s) - total elapsed time
3. Two-phase kill: SIGINT, wait 3s, SIGQUIT

## Error Messages

When no bridge is running, `pty-send` must output:

```
Error: No PTY bridge session is running.

You have used `pty-send`, but it currently has nowhere to send to.

The user needs to start the interactive backing terminal using `pty-bridge`.
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
The user must have `pty-bridge` running in a terminal they control.

### Test Matrix

For each shell type (bash, Python, Nix, Node), test three scenarios:

| Scenario | What to test |
|----------|--------------|
| Normal | Command/eval completes successfully |
| User types during | User types while command is running - their input gets captured |
| User kills/closes | User interrupts (Ctrl+C), kills the REPL, or closes the bridge |

### Bash (command mode)

```bash
# 1. Normal
pty-send -- echo "hello"

# 2. User types during (use sleep so they have time)
pty-send --timeout 20 -- sleep 10 && echo "done"

# 3. User interrupts - expect timeout since no end marker
pty-send --timeout 20 -- sleep 60
```

### Python REPL

```bash
# 1. Normal
pty-send --repl-start python3 --prompt "^>>>"
pty-send --repl-eval "1 + 1"
pty-send --repl-close

# 2. User types during
pty-send --repl-start python3 --prompt "^>>>" --timeout 20
pty-send --repl-eval "import time; time.sleep(10); 'done'" --timeout 20
pty-send --repl-close

# 3. User kills REPL (killall -9 python3) - expect timeout, then clean up
pty-send --repl-start python3 --prompt "^>>>" --timeout 20
pty-send --repl-eval "import time; time.sleep(60); 'done'" --timeout 20
rm -f /tmp/pty-bridge-$USER/repl.*  # clean stale state
```

### Nix REPL

```bash
# 1. Normal
pty-send --repl-start "nix repl" --prompt "^nix-repl>"
pty-send --repl-eval "1 + 1"
pty-send --repl-close

# 2. User types during (slow fold operation)
pty-send --repl-start "nix repl" --prompt "^nix-repl>" --timeout 20
pty-send --repl-eval "builtins.seq (builtins.foldl' (a: b: a + b) 0 (builtins.genList (x: x) 10000000)) \"done\"" --timeout 20
pty-send --repl-close

# 3. User kills REPL - get PID first, then kill -9
pty-send --repl-start "nix repl" --prompt "^nix-repl>" --timeout 20
# user runs: ps aux | grep "nix repl" then kill -9 <pid>
pty-send --repl-eval "builtins.seq (builtins.foldl' (a: b: a + b) 0 (builtins.genList (x: x) 50000000)) \"done\"" --timeout 20
rm -f /tmp/pty-bridge-$USER/repl.*
```

### Node REPL

```bash
# 1. Normal
pty-send --repl-start node --prompt "^>"
pty-send --repl-eval "1 + 1"
pty-send --repl-close

# 2. User types during (sync busy loop)
pty-send --repl-start node --prompt "^>" --timeout 20
pty-send --repl-eval "const start = Date.now(); while (Date.now() - start < 10000) {}; 'done'" --timeout 20
pty-send --repl-close

# 3. User kills REPL - get PID first
pty-send --repl-start node --prompt "^>" --timeout 20
pty-send --repl-eval "process.pid"  # note this, then user runs kill -9 <pid>
pty-send --repl-eval "const t = Date.now(); while (Date.now() - t < 60000) {}; 'done'" --timeout 20
rm -f /tmp/pty-bridge-$USER/repl.*
```

### Expected behaviors

- **User types during**: Their keystrokes appear in captured output
- **User Ctrl+C**: Command/REPL handles interrupt, may return to prompt with error
- **User kills process**: Timeout occurs, error message guides agent to ask user
- **User closes bridge**: Clean error "PTY bridge session closed unexpectedly"
