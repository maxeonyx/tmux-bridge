# tmux-bridge

A CLI that lets AI agents run commands in your interactive terminal session.

**Site:** https://maxeonyx.github.io/tmux-bridge/

## Why?

AI agents need to run commands, but some require human interaction:

- `sudo` requiring a password
- Browser-based authentication flows
- Interactive setup wizards
- Long-running builds that outlast agent timeouts

`tb` solves this by letting the human maintain an interactive terminal while the agent sends commands into it.

## Installation

Requires: `tmux`

```bash
# Download binary (Linux x86_64)
curl -L https://github.com/maxeonyx/tmux-bridge/releases/latest/download/tb-linux-x86_64 \
  -o ~/.local/bin/tb && chmod +x ~/.local/bin/tb

# Or build from source
cargo install --git https://github.com/maxeonyx/tmux-bridge
```

## Quick Start

### Human: Start a session

```bash
$ tb start
Started session 'a7x'

Tell your agent: export TB_SESSION=a7x
```

### Agent: Run commands

```bash
export TB_SESSION=a7x

# Synchronous command
tb run -- cargo build

# Background task
tb launch -- npm run dev
tb check t1
tb done t1
```

## Commands

| Command | Purpose |
|---------|---------|
| `tb start` | Human starts session, displays ID |
| `tb run` | Run command synchronously, wait for output |
| `tb launch` | Start background task in split pane |
| `tb check` | Check background task status/output |
| `tb done` | Close background task pane |

## Agent Skill

For [OpenCode](https://opencode.ai) and compatible agents, install the skill:

```bash
mkdir -p ~/.config/opencode/skills/tmux-bridge
curl -sL https://maxeonyx.github.io/tmux-bridge/SKILL.md \
  -o ~/.config/opencode/skills/tmux-bridge/SKILL.md
```

## License

MIT
