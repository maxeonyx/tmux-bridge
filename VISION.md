# Vision

## The Problem

AI coding agents run in isolated environments. They can execute commands, but they can't:

1. **See what the user sees** - no shared terminal context
2. **Interact with prompts** - password prompts, confirmations, Y/n questions
3. **Benefit from user's credentials** - sudo, SSH agents, GPG keys
4. **Share an interactive session** - the user and agent work in separate worlds

This creates friction. The agent asks the user to run commands manually. The user copy-pastes output back. Context is lost.

## The Solution

A shared tmux session where:

- **Human maintains control** - full interactive terminal, can type anything
- **Agent injects commands** - sends commands, receives clean output
- **Inputs merge** - both human and agent can send to the same shell
- **Output forks** - human sees everything, agent extracts its command's output

The human runs `tmux-bridge` and keeps the terminal open. The agent uses `tmux-send` to run commands as if it were typing them.

## Design Principles

### Human-first

The human's terminal must feel completely normal. No special modes, no restrictions. The agent is a guest in the human's session.

### Simple mental model

- `tmux-bridge` = "start/attach to the shared terminal"
- `tmux-send` = "run a command in the shared terminal"

### Clean agent interface

`tmux-send` behaves like a normal command wrapper:
- Stdout → stdout
- Stderr → stderr
- Exit status → exit status
- Blocks until complete

### Fail informatively

When things go wrong, error messages explain exactly what the agent should ask the human to do.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│           tmux session: tmux-bridge-$USER                │
│   - One shell process (fish)                            │
│   - Stays alive while ≥1 tmux-bridge attached            │
│   - destroy-unattached on                               │
└─────────────────────────────────────────────────────────┘
        ▲                           ▲
        │ attach (keeps alive)      │ send/receive (passive)
        │                           │
┌───────┴───────┐           ┌───────┴───────┐
│  tmux-bridge   │           │   tmux-send    │
│  (human)      │           │  (agent)      │
└───────────────┘           └───────────────┘
```

### Why tmux?

Battle-tested tmux. Handles:
- Multiple clients attaching to same session
- Window resizing
- Signal handling
- Session lifecycle

We write thin wrappers with good UX; tmux does the hard work.

### Command injection protocol

`tmux-send` wraps commands with unique markers:

```fish
echo "___TMUXSEND_START_$id___"
eval $command 2>/tmp/tmux-bridge-$USER/stderr.$id
set __tmux_exit $status
echo "___TMUXSEND_END_$id $__tmux_exit___"
```

Then parses the output between markers and reads stderr from the temp file.

### Timeout handling

Two timeouts protect against hung commands:
- **No-output timeout (10s)**: Nothing printed for N seconds
- **Overall timeout (120s)**: Total elapsed time

When triggered, two-phase kill:
1. SIGINT (Ctrl+C)
2. Wait 3 seconds
3. SIGQUIT (Ctrl+\)

## v1 Scope

**In scope:**
- Basic `tmux-bridge` and `tmux-send` scripts
- Stdout/stderr separation
- Timeout handling with two-phase kill
- Helpful error messages
- Multiple `tmux-bridge` terminals attaching to same session

**Known limitations (acceptable for v1):**
- Markers visible in human's terminal
- No REPL support
- Don't type while agent command runs
- Scrollback buffer limits

## v2: REPL Support

REPLs (python, node, nix repl) are long-running interactive processes. The agent needs to:

1. Start the REPL
2. Send input lines
3. Get output for each input
4. Eventually exit

Planned approach:
- `tmux-send --repl "python3"` - starts REPL, returns when prompt detected
- `tmux-send --repl-input "print('hello')"` - sends line, returns output
- Agent specifies prompt pattern for detection
- Timeout returns partial output with explanation

## Future Possibilities

- **Cleaner human display**: Hide markers, show `[agent] $ command` prefix
- **Concurrent command queue**: Multiple `tmux-send` calls handled sequentially
- **Session persistence**: Keep session alive briefly after last terminal exits
- **Alternative backends**: Direct terminal management without tmux dependency
- **Partial output on timeout**: Show captured output up to the timeout point (if practicable)
