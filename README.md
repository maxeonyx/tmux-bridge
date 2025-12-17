# pty-bridge

A PTY bridge system that allows AI agents to inject commands into an interactive terminal session controlled by a human user.

## Why?

AI coding agents need to run commands, but some commands require human interaction:

- `sudo` requiring a password
- Commands that need credential caching (`sudo -v`)
- Interactive setup wizards
- Anything where the human needs to see and respond

`pty-bridge` solves this by letting the human maintain an interactive terminal while the agent sends commands into it.

## Installation

Requires: `tmux`, `fish`

```fish
# Clone the repo
git clone git@github.com:maxeonyx/pty-bridge.git
cd pty-bridge

# Install to ~/.local/bin (must be on PATH)
cp bin/pty-bridge bin/pty-send ~/.local/bin/
chmod +x ~/.local/bin/pty-bridge ~/.local/bin/pty-send
```

## Usage

### Human: Start the bridge

```fish
$ pty-bridge
# You're now in an interactive shell
# Run sudo -v to cache credentials if needed
# Keep this terminal open
```

You can open multiple terminals and run `pty-bridge` in each - they all attach to the same session. The session stays alive as long as at least one terminal is attached.

### Agent: Send commands

```fish
$ pty-send -- ls -la
drwxr-xr-x  5 mclarke mclarke 4096 Dec 17 10:00 .
-rw-r--r--  1 mclarke mclarke  123 Dec 17 09:00 foo.txt

$ pty-send -- sudo -n apt update
# Works if user has cached credentials

$ echo $status
0
```

Stdout and stderr are separated - you can redirect them independently:

```fish
$ pty-send -- ls /nonexistent 2>err.txt
$ cat err.txt
ls: cannot access '/nonexistent': No such file or directory
```

### Timeouts

```fish
# Default: 10s no-output timeout, 120s overall timeout
$ pty-send -- make build

# Increase no-output timeout for slow commands
$ pty-send --timeout 60 -- make build

# Increase overall timeout for long-running commands
$ pty-send --max-time 600 -- ./slow-test.sh

# Both
$ pty-send --timeout 60 --max-time 300 -- make
```

If a timeout triggers, `pty-send` sends SIGINT, waits 3 seconds, then SIGQUIT if needed.

## How It Works

- `pty-bridge` creates/attaches to a tmux session named `pty-bridge-$USER`
- `pty-send` injects commands via `tmux send-keys` with unique markers
- Output is captured via `tmux capture-pane` and parsed between markers
- Stderr is redirected to a temp file and read back separately

## Limitations

- Markers are visible in the human's terminal (v1)
- Very long output may hit tmux scrollback limits
- Don't type in the bridge terminal while agent commands are running
- Binary output not supported
- REPL support is planned for v2

## License

MIT
