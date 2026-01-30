# tmux-bridge

A Rust CLI that allows AI agents to inject commands into an interactive terminal session controlled by a human user.

## Why?

AI coding agents need to run commands, but some commands require human interaction:

- `sudo` requiring a password
- Commands that need credential caching (`sudo -v`)
- Interactive setup wizards
- Long-running builds that outlast agent timeouts

`tb` solves this by letting the human maintain an interactive terminal while the agent sends commands into it.

## Installation

Requires: `tmux`

```bash
# From source
cargo install --path .

# Or download a release binary
# (coming soon)
```

## Quick Start

### Human: Start a session

```bash
$ tb start
Started session 'a7x'

Tell your agent:
  export TB_SESSION=a7x
```

### Agent: Run commands

```bash
# Set the session (or use --session flag)
export TB_SESSION=a7x

# Run a command synchronously
$ tb run -- ls -la
drwxr-xr-x  5 user user 4096 Jan 30 10:00 .
-rw-r--r--  1 user user  123 Jan 30 09:00 foo.txt

# Run with custom timeout
$ tb run --timeout 60 -- make build
```

### Agent: Background tasks

```bash
# Start a long-running task
$ tb launch -- npm run build
Task t1 started.
Check status with: tb check t1

# Check on it later
$ tb check t1
[build output...]

# When done
Task t1 complete (exit 0).
Close pane with: tb done t1

# Clean up
$ tb done t1
Closed task t1.
```

## Commands

| Command | Purpose |
|---------|---------|
| `tb start` | Human starts session, displays ID |
| `tb run` | Run command synchronously |
| `tb launch` | Start background task in split pane |
| `tb check` | Check background task status |
| `tb done` | Close background task pane |

## Session Management

Sessions get short unique IDs like `a7x`:
- First letter: sequential (`a`, `b`, `c`...)
- Next two chars: random for uniqueness

Multiple sessions can run simultaneously. The agent specifies which to use via:
- `TB_SESSION` environment variable
- `--session` flag on any command

## Background Task Layout

Up to 6 concurrent background tasks, displayed as split panes:

```
┌─────────────────────────────┬─────────────────────────────┐
│ [task t1]                   │ [task t4]                   │
├─────────────────────────────┼─────────────────────────────┤
│ [task t2]                   │ [task t5]                   │
├─────────────────────────────┼─────────────────────────────┤
│ [task t3]                   │ [task t6]                   │
├─────────────────────────────┴─────────────────────────────┤
│ [main session]                                            │
└───────────────────────────────────────────────────────────┘
```

## Timeouts

```bash
# Default: 10s no-output timeout, 120s overall timeout
$ tb run -- make build

# Increase no-output timeout for slow commands
$ tb run --timeout 60 -- make build

# Increase overall timeout for long-running commands
$ tb run --max-time 600 -- ./slow-test.sh
```

If a timeout triggers, `tb run` sends SIGINT, waits 3 seconds, then SIGQUIT.

## Status

⚠️ **Work in progress** - all 47 E2E tests are failing. See [TODO.md](TODO.md) for implementation status.

## License

MIT
