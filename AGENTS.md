# Agent Instructions

This document is for AI coding assistants working on the tmux-bridge codebase.

## Project Overview

`tb` is a Rust CLI allowing AI agents to inject commands into an interactive terminal session controlled by a human user. Built on tmux.

## Commands

| Command | Purpose |
|---------|---------|
| `tb start` | Human starts session, displays ID like `a7x` |
| `tb run` | Agent runs synchronous command, waits for output |
| `tb launch` | Agent starts background task in split pane |
| `tb check` | Agent checks background task status/output |
| `tb done` | Agent closes background task pane |

## Key Design Decisions

1. **tmux is the foundation** - don't reinvent tmux
2. **Human terminal is primary** - agent is a guest, not the owner
3. **`tb run` behaves like a command wrapper** - stdout/exit status work normally
4. **Progressive disclosure** - each command only hints at the next logical step
5. **Multi-session support** - session IDs like `a7x` allow multiple bridges
6. **Background tasks** - up to 6 concurrent tasks in split panes

## Building and Testing

```bash
# Build
cargo build

# Run tests (51 E2E tests defining behavior)
cargo test

# Run specific test file
cargo test --test start
cargo test --test run
cargo test --test launch
cargo test --test check
cargo test --test done

# Run with release optimizations
cargo build --release

# Run the test ratchet (CI uses this)
python3 scripts/ratchet.py
```

### Test Ratchet

The project uses a test ratchet system (`scripts/ratchet.py`) that enforces:

1. **TDD workflow**: New tests must be added as "pending" (failing) first, then promoted to "passing" in a separate commit
2. **No regressions**: Once a test passes, it must keep passing
3. **No silent removal**: Tests in `.test-status.json` must exist

When adding a new test:
1. Add the test code
2. Add entry to `.test-status.json` as `"pending"`
3. Commit: "Add failing test for X"
4. Implement the fix
5. Change status to `"passing"` in `.test-status.json`
6. Commit: "Fix X"

## Releasing

Releases are automated via GitHub Actions when a version tag is pushed.

```bash
# After merging changes to main:
git tag v0.1.1
git push --tags
```

This triggers the CI workflow which:
1. Runs the test ratchet
2. Builds binaries for linux-amd64, linux-amd64-musl, macos-amd64, macos-arm64
3. Creates a GitHub release with the binaries

### After Release

Update the skill file in your opencode config if needed:
```bash
curl -sLo ~/.config/opencode/skills/tmux-bridge/SKILL.md \
     https://maxeonyx.github.io/tmux-bridge/SKILL.md
```

## Project Structure

```
src/
  main.rs         # CLI setup with clap, dispatch to commands
tests/
  start.rs        # E2E tests for tb start
  run.rs          # E2E tests for tb run
  launch.rs       # E2E tests for tb launch
  check.rs        # E2E tests for tb check
  done.rs         # E2E tests for tb done
bin/              # Old fish scripts (to be removed)
```

## Implementation Details

### Session naming
Sessions are named `tb-{id}` where id is `{letter}{random}{random}` (e.g., `tb-a7x`).

### Session resolution
Commands use `--session ID` flag or `$TB_SESSION` environment variable.

### Command markers
Format: `___START_$id___` and `___END_${id}_$exit_status___` where `$id` is random.

### Timeout behavior
1. No-output timeout (default 10s) - no new output for N seconds
2. Overall timeout (default 120s) - total elapsed time
3. Two-phase kill: SIGINT, wait 3s, SIGQUIT

### Background task layout
- Tasks 1-3: horizontal splits at top (10 lines each)
- Tasks 4-6: two columns of horizontal splits
- Maximum 6 concurrent background tasks

## Error Messages

Error messages should be self-documenting and guide the agent to the next action:

```
Error: No session specified.

Set TB_SESSION environment variable, or use --session ID.
Ask the user which tmux-bridge session to use.
```

## Code Style

- Rust, idiomatic
- Use `clap` derive macros for CLI parsing
- Shell out to `tmux` via `std::process::Command`
- Minimal dependencies - this is a simple tool
- Comments explain "why", not "what"

## What Not To Do

- Don't hide markers from the human terminal yet (future possibility)
- Don't add features without updating VISION.md
- Don't implement REPL support - it was removed in favor of simpler design
