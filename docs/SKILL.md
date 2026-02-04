---
name: tmux-bridge
description: If a task requires interactive-only steps such as authentication, or background commands
---

# tmux-bridge

Use `tb` for interactive commands (sudo, auth) or background tasks.
If "No session specified", ask user to run `tb start`.

## Install

```bash
curl -Lo ~/.local/bin/tb https://github.com/maxeonyx/tmux-bridge/releases/latest/download/tb-linux-amd64
chmod +x ~/.local/bin/tb
```

## Synchronous

```bash
tb run -- sudo apt install foo
```

## Background tasks

```bash
tb launch -- cargo build   # Start background task
tb check t1                 # Check status
tb done t1                  # Close pane
```
