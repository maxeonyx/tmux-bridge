---
name: tmux-bridge
description: If a task requires interactive-only steps such as authentication, or background commands
---

# tmux-bridge

Use `tb` for interactive commands (sudo, auth) or background tasks.
If "No session specified", ask user to run `tb start`.

## Install

```bash
curl -Lo ~/.local/bin/tb https://tmux-bridge.maxeonyx.com/releases/tb-x86_64-linux
chmod +x ~/.local/bin/tb
```

## Synchronous

```bash
tb run -s <id> -- sudo apt install foo
```

## Background tasks

```bash
tb launch -s <id> -- cargo build   # Start background task
tb check -s <id> t1                 # Check status
tb done -s <id> t1                  # Close pane
```
