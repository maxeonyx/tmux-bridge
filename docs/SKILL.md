---
name: tmux-bridge
description: If a task requires interactive-only steps such as authentication, or an interactive terminal workflow that should stay attachable; for non-interactive background jobs, use the systemd-user-jobs skill instead
---

# tmux-bridge

Use `tb` for interactive commands (sudo, auth) or interactive terminal workflows that you may want to reattach to later. If "No target specified", ask user which tmux session/pane to use, or ask them to run `tb start`.

`tmux-bridge` is mostly for local use, not the default answer for remote durable jobs.

For ad hoc non-interactive background processes that should survive logout, especially on a remote Linux machine, prefer `systemd-run --user` with linger enabled. See the `systemd-user-jobs` skill.

## Install

```bash
curl -Lo ~/.local/bin/tb https://tmux-bridge.maxeonyx.com/releases/tb-x86_64-linux
chmod +x ~/.local/bin/tb
```

## Targeting

All commands (except `tb start`) use `--target` (`-t`) to specify the tmux target:

```bash
# tb-started session (resolves a7x → tb-a7x)
tb run -t a7x -- ls -la

# Any existing tmux session by name
tb run -t my-session -- ls -la

# Specific pane in a session
tb run -t my-session:0.1 -- make build

# tmux pane ID
tb run -t %42 -- make build
```

Simple names try a literal tmux session first, then fall back to `tb-{name}`. Targets containing tmux syntax (`:`, `.`, `%`) pass through unchanged.

## Inspect the target pane first

Run `tb info` before your first interaction with any target pane:

```bash
tb info -t <target>
# Shell assessment: fish (confident)
# Shell assessment: bash (confident)
# Shell assessment: unknown (direct shell-specific execution unsafe)
```

This tells you the shell type and whether `tb run` can send commands directly. Use it to decide whether fish-native syntax is safe, or whether you need POSIX fallbacks.

## Synchronous

```bash
# Simple command
tb run -t <target> -- sudo apt install foo

# Shell script — single arg is treated as shell code automatically
tb run -t <target> --timeout 60 -- 'echo "Starting..."; sudo systemctl restart nginx; echo "Done"'
```

**Never wrap in `bash -c`** — `tb run` adapts its marker wrapper to the detected shell automatically:

- **Fish (confident):** sends commands directly in fish syntax — fish-native code works: `tb run -t <target> -- 'math 1 + 2'`
- **Bash/sh (confident):** sends commands directly in POSIX syntax
- **Unknown:** falls back to `sh -c` wrapper

If you specifically need POSIX semantics in a fish pane, send `sh -c '...'` explicitly as your command.

- ✅ `tb run -t <target> -- 'cmd1; cmd2'`
- ❌ `tb run -t <target> -- bash -c 'cmd1; cmd2'`

Multiple arguments after `--` are treated as argv (each quoted individually).

## Authentication prompts

**Ask immediately** when a command triggers an authentication step (AWS SSO, sudo password, SSH key passphrase, browser OAuth, etc.). Don't silently wait or poll — use the question tool to ask the user to complete it. Wasted minutes waiting in silence are wasted context.

If a `tb run` or `tb launch` command gets stuck (no output, timeout) because of a sudo prompt or other authentication step, **do not retry**. The user needs to complete the authentication interactively. Ask the user to authenticate, then retry.

## Pagers

Always use `--no-pager` with `systemctl`, `journalctl`, and similar commands. The tmux pane is an interactive terminal, so pagers (like `less`) will wait for you to press `q` — which you can't do.

## Background tasks

Use this for terminal-centric jobs where an interactive pane is still the right abstraction.

```bash
tb launch -t <target> -- cargo build   # Start background task
tb check -t <target> t1                 # Check status
tb done -t <target> t1                  # Close pane
```

## Checking a pane

`tb check -t <target>` without a task ID captures the targeted pane's visible output — useful for seeing what the human sees after an interactive prompt, auth flow, or manual command.

```bash
tb check -t <target>              # Capture targeted pane
tb check -t <target> t1           # Check background task status
```
