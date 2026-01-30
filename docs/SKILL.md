---
name: tmux-bridge
description: If a task requires interactive-only steps such as authentication, or background commands
---

# tmux-bridge

Use `tb` for commands requiring user interaction (sudo, password prompts, confirmations) or long-running background tasks.

If you see "No session specified", ask the user to start one with `tb start`.

## Quick Reference

```bash
# Synchronous command (waits for completion)
tb run -- sudo apt install foo

# Background task (returns immediately)
tb launch -- cargo build --release
tb check t1    # Check status
tb done t1     # Close when finished
```

The commands themselves provide detailed help and next-step hints.
