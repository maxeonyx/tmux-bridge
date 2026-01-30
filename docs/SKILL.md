---
name: tmux-bridge
description: If a task requires interactive-only steps such as authentication, or background commands
---

# tmux-bridge

Use `tb` for interactive commands (sudo, auth) or background tasks.
If "No session specified", ask user to run `tb start`.

## Install

```bash
curl -Lo ~/.local/bin/tb https://github.com/maxeonyx/tmux-bridge/releases/latest/download/tb-linux-x86_64
chmod +x ~/.local/bin/tb
```

## Usage

```bash
tb run -- sudo apt install foo   # Synchronous
tb launch -- cargo build          # Background task
tb check t1                        # Check status
tb done t1                         # Close pane
```
