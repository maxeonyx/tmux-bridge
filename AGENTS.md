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

- Don't add REPL support yet (v2)
- Don't hide markers from the human terminal yet (v2)
- Don't add features without updating README and VISION
- Don't use bash - this project uses fish
