# Agent Instructions

This document is for AI coding assistants working on the tmux-bridge codebase.

## Project Overview

`tb` is a Rust CLI allowing AI agents to inject commands into an interactive terminal session controlled by a human user. Built on tmux.

## Commands

| Command | Purpose |
|---------|---------|
| `tb start` | Human starts session, displays ID like `a7x` |
| `tb info` | Agent inspects shell confidence for a target pane |
| `tb run` | Agent runs synchronous command, waits for output |
| `tb run --dry-run` | Agent prints the exact `tmux send-keys` string without running it |
| `tb launch` | Agent starts background task in split pane |
| `tb check` | Agent checks background task status/output, or captures main pane |
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

# Run tests (96 tests defining behavior)
cargo test

# Run specific test file
cargo test --test start
cargo test --test info
cargo test --test run
cargo test --test launch
cargo test --test check
cargo test --test done

# Run with release optimizations
cargo build --release

# Run the test ratchet (CI uses this)
python3 scripts/ratchet.py

# Stress test for flakiness (run N times, report pass rate)
./scripts/stress-test.sh 20
```

### Test Architecture

Tests are real E2E tests using real tmux sessions. The key principle: **never use fixed sleeps — always poll for observable state.** The test helpers in `tests/common/mod.rs` provide polling primitives:

- `wait_until(description, timeout, poll_interval, probe)` — generic polling
- `wait_for_pane_content(session, predicate, timeout)` — poll tmux pane capture
- `wait_for_pane_count(session, expected, timeout)` — poll pane count
- `wait_for_session_exists(prefix, id, timeout)` — poll session existence
- `TestSession::wait_for_check_output(task_id, predicate)` — poll `tb check` output
- `TestSession::wait_for_main_check_output(predicate)` — poll `tb check` (main pane)

When adding new tests: use these helpers instead of `thread::sleep`. If a new wait pattern is needed, add it to `tests/common/mod.rs`.

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

Use `./scripts/release.sh` as the primary release path.

```bash
# Auto-bump the patch version from Cargo.toml
./scripts/release.sh

# Or release an explicit version
./scripts/release.sh 0.1.5
```

The script runs the full release flow in order:
1. Runs `python3 scripts/ratchet.py`
2. Updates `Cargo.toml`
3. Runs `cargo build` to refresh `Cargo.lock`
4. Commits `Bump version to v{version}`
5. Tags `v{version}`
6. Pushes the commit and tags
7. Runs `cargo install --path .`
8. Prints the released version and local install path

Releases are automated via GitHub Actions when the tag is pushed.

**Always bump the version and tag a release** after merging behavioral changes (features, bug fixes, quoting changes). Don't leave unreleased work sitting on main.

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
  info.rs         # E2E tests for tb info
  run.rs          # E2E tests for tb run
  launch.rs       # E2E tests for tb launch
  check.rs        # E2E tests for tb check
  done.rs         # E2E tests for tb done
```

## Implementation Details

### Session naming
Sessions are named `tb-{id}` where id is `{letter}{random}{random}` (e.g., `tb-a7x`).

### Session resolution
Commands use `--target TARGET` / `-t`. Simple names first try a literal tmux session, then fall back to `tb-{name}` for `tb start` compatibility. Targets containing tmux syntax (`:`, `.`, `%`) pass through unchanged.

### Command markers
Format: `___START_$id___` and `___END_${id}_$exit_status___` where `$id` is random.

### Shell-adaptive wrappers
`tb run` uses direct marker wrappers only when the target shell is confidently identified as one of the tested cases for this phase:

- fish: `echo ___START_xxx___; <cmd>; echo ___END_xxx_{$status}___`
- bash / `sh`: `echo ___START_xxx___; <cmd>; echo ___END_xxx_$?___`
- unknown / not confident: fallback to `sh -c '...'`

This keeps the existing POSIX fallback while removing one quoting layer for confident fish, bash, and `sh` targets.

The confidence policy is intentionally split by responsibility:

- `tb run` starts from tmux's `#{pane_current_command}` but also checks that the pane still looks idle / prompt-like before using a direct wrapper. If tmux reports `fish`, `bash`, or `sh` but the pane looks busy (for example a startup command is still occupying the pane), `tb run` falls back to `sh -c`.
- `tb info` starts from the same foreground-process signal but also sends a small live probe before reporting `confident`, because its job is to give the agent richer assessment context.

This difference is intentional, not a bug.

### Quoting principles
The human sees every command typed into their terminal. Quoting must be **correct** and **minimal** — only add quotes/escapes that are strictly necessary.

- **Single-arg mode (shell script):** A single argument after `--` is treated as shell code for the active shell when `tb` knows the shell confidently; otherwise it falls back to `sh -c '...'`. **Never** add your own `bash -c` wrapper just to get a shell script mode — that creates a redundant quoting layer. If you specifically want POSIX semantics in a fish pane, send `sh -c '...'` explicitly.
  - ✅ `tb run -- 'echo "hello"; ls -la'`
  - ❌ `tb run -- bash -c 'echo "hello"; ls -la'`
- **Multi-arg mode:** Multiple arguments after `--` are each quoted individually with smart per-arg quoting — bare for shell-safe text, double quotes for whitespace/metacharacters, single quotes for literal shell symbols (`\ $ \` " !`). This also goes direct for confident fish, bash, and `sh` targets, and falls back to `sh -c` otherwise.
- Markers (`___START_xxx___`) are alphanumeric + underscores — never quote them

### Timeout behavior
1. No-output timeout (default 10s) - no new output for N seconds
2. Overall timeout (default 120s) - total elapsed time
3. Two-phase kill: SIGINT, wait 3s, SIGQUIT

### Background task layout
- `tb launch` splits the targeted pane directly
- Task accounting uses `@tb_task`-tagged panes only
- Maximum 6 concurrent background tasks per target scope

## Error Messages

Error messages should be self-documenting and guide the agent to the next action:

```
Error: No target specified.

Use --target TARGET.
Ask the user which tmux target to use.
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
