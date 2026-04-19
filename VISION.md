# Vision

## The Problem

AI coding agents run in isolated environments. They can execute commands, but they can't:

1. **See what the user sees** - no shared terminal context
2. **Interact with prompts** - password prompts, confirmations, Y/n questions
3. **Benefit from user's credentials** - sudo, SSH agents, GPG keys
4. **Run background tasks** - long-running builds that outlast agent timeouts
5. **Share an interactive session** - the user and agent work in separate worlds

This creates friction. The agent asks the user to run commands manually. The user copy-pastes output back. Context is lost.

## The Solution

A shared tmux session where:

- **Human maintains control** - full interactive terminal, can type anything
- **Agent injects commands** - sends commands, receives clean output
- **Inputs merge** - both human and agent can send to the same shell
- **Output forks** - human sees everything, agent extracts its command's output
- **Background tasks** - agent can launch long-running tasks and check on them later

The human runs `tb start` and keeps the terminal open. The agent uses `tb run` for synchronous commands, or `tb launch` for background tasks.

## Design Principles

### Human-first

The human's terminal must feel completely normal. No special modes, no restrictions. The agent is a guest in the human's session.

### Simple mental model

- `tb start` = "start a shared terminal session"
- `tb run` = "run a command and wait for output"
- `tb launch` = "start a background task"
- `tb check` = "what's on screen?" / "how's that task going?"
- `tb done` = "close that task's pane"

### Clean agent interface

`tb run` behaves like a normal command wrapper:
- Stdout → stdout
- Exit status → exit status
- Blocks until complete

`tb launch` returns immediately with a task ID for later checking.

### Progressive disclosure

Commands only reveal the next logical step - no overwhelming the agent with options:

- `tb start` → "Tell your agent: `tb run --target a7x -- <command>`"
- `tb run` (no target) → "Use --target"
- `tb launch` → "Check status with: `tb check --target TARGET t1`"
- `tb check` (finished) → "Close pane with: `tb done --target TARGET t1`"

### Fail informatively

When things go wrong, error messages explain exactly what the agent should ask the human to do.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                 tmux session: tb-a7x                            │
│  ┌─────────────────────────────┬─────────────────────────────┐  │
│  │ [task t1 - 10 lines]        │ [task t4 - 10 lines]        │  │
│  ├─────────────────────────────┼─────────────────────────────┤  │
│  │ [task t2 - 10 lines]        │ [task t5 - 10 lines]        │  │
│  ├─────────────────────────────┼─────────────────────────────┤  │
│  │ [task t3 - 10 lines]        │ [task t6 - 10 lines]        │  │
│  ├─────────────────────────────┴─────────────────────────────┤  │
│  │ [main session - interactive shell]                        │  │
│  │ $                                                          │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
        ▲                                   ▲
        │ attach (human)                    │ commands (agent)
        │                                   │
┌───────┴───────┐                   ┌───────┴───────┐
│   tb start    │                   │  tb run/      │
│   (human)     │                   │  launch/check │
└───────────────┘                   └───────────────┘
```

### Session IDs

Format: `{sequential-letter}{random1}{random2}` (e.g., `a7x`, `b3m`, `c9k`)

- First char: `a`, `b`, `c`... based on what's not in use
- Chars 2-3: random from `[a-z0-9]` for uniqueness
- Prevents accidental reuse after session close

Sessions are named `tb-{id}` in tmux (e.g., `tb-a7x`).

### Task panes

Background tasks split the targeted pane directly.

- `tb launch -t my-session:1.2 -- ...` splits that pane
- panes tagged with `@tb_task` are the only panes counted as tasks
- Maximum 6 concurrent background tasks per target scope

### Why tmux?

Battle-tested. Handles:
- Multiple clients attaching to same session
- Window resizing
- Signal handling
- Session lifecycle
- Pane splitting and management

We write a thin wrapper with good UX; tmux does the hard work.

### Command injection protocol

`tb run` wraps commands with unique markers:

```sh
echo "___START_$id___"
$command
echo "___END_${id}_$?___"
```

Then parses the output between markers via `tmux capture-pane`.

### Timeout handling

Two timeouts protect against hung commands:
- **No-output timeout (default 10s)**: Nothing printed for N seconds
- **Overall timeout (default 120s)**: Total elapsed time

When triggered:
1. SIGINT (Ctrl+C)
2. Wait 3 seconds
3. SIGQUIT (Ctrl+\)

## CLI Reference

### tb start

Human runs this to create a session.

```
$ tb start
Started session 'a7x'

Tell your agent:
  tb run --target a7x -- <command>
```

Options:
- `--session ID` - Use specific ID instead of auto-generating

### tb run

Agent runs synchronous commands.

```
# Simple command (each arg passed separately)
$ tb run --target a7x -- ls -la
$ tb run --target my-session:1.2 --timeout 60 -- make build

# Shell script (single arg = treated as shell code)
$ tb run --target a7x -- 'echo "Starting..."; make build && echo "Done"'
$ tb run --target %42 -- 'grep -r "TODO" src/ | wc -l'
```

A single argument after `--` is treated as a shell script — `tb` wraps it in `sh -c` automatically. **Do not** wrap in `bash -c` yourself; that adds an unnecessary quoting layer.

Options:
- `--target TARGET` / `-t` - Tmux target to use
- `--timeout N` - No-output timeout in seconds (default: 10)
- `--max-time N` - Overall timeout in seconds (default: 120)
- `--first N` - Lines from start to show (default: 50)
- `--last N` - Lines from end to show (default: 50)

### tb launch

Agent starts a background task.

```
$ tb launch --target my-session:1.2 -- npm run build
Task t1 started.
Check status with: tb check --target my-session:1.2 t1
```

Options:
- `--target TARGET` / `-t` - Tmux target to use

### tb check

Agent checks on a background task, or captures what the human sees in the main pane.

```
# Check a background task
$ tb check --target my-session:1.2 t1
[output from the task pane]

# If task has finished:
Task t1 complete (exit 0).
Close pane with: tb done --target my-session:1.2 t1

# Capture the targeted pane
$ tb check --target my-session:1.2
[visible output from the targeted pane]
```

Options:
- `--target TARGET` / `-t` - Tmux target to use
- `--first N` - Lines from start to show (default: 50)
- `--last N` - Lines from end to show (default: 50)

### tb done

Agent closes a background task's pane.

```
$ tb done --target my-session:1.2 t1
Closed task t1.
```

## Implementation

Single Rust binary using:
- `clap` for CLI parsing
- Direct `tmux` command invocation via `std::process::Command`
- No async runtime needed (simple blocking I/O)

Distribution:
- GitHub releases with prebuilt binaries (Linux, macOS)
- Single binary, no dependencies beyond tmux

## Future Possibilities

- **Cleaner human display**: Hide markers, show `[agent] $ command` prefix
- **Concurrent command queue**: Multiple `tb run` calls handled sequentially
- **Session persistence**: Keep session alive briefly after last terminal exits
- **Windows support**: WSL detection and guidance
