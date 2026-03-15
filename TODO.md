# TODO

## Current State

The Rust CLI (`tb`) is **complete**. All 47 E2E tests pass.

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test start
cargo test --test run
cargo test --test launch
cargo test --test check
cargo test --test done
```

## Implementation Tasks

### Core Commands - COMPLETE

- [x] **`tb start`** - Create tmux session with auto-generated ID (7 tests)
- [x] **`tb run`** - Synchronous command execution (16 tests)
- [x] **`tb launch`** - Background task in split pane (9 tests)
- [x] **`tb check`** - Check background task status (7 tests)
- [x] **`tb done`** - Close background task pane (8 tests)

### Infrastructure - COMPLETE

- [x] Set up Cargo project
- [x] Set up clap CLI structure with subcommands
- [x] Write E2E black-box tests
- [x] Set up test ratchet
- [x] Set up GitHub Actions CI with ratchet
- [x] Set up release workflow for binaries
- [x] Update README.md
- [x] Update AGENTS.md
- [x] Update opencode skill

### Bug: `tb run` breaks multi-statement shell scripts

**User story:** As an AI agent, I want to pass a multi-statement shell script to `tb run` (with `echo`, `read`, `ssh`, semicolons, single quotes, etc.) and have it execute correctly in the user's shell. Currently, `build_shell_command` wraps the command args in `sh -c` with `double_quote_escape` on each arg — this treats each arg as a single token to be double-quoted, so a script like `echo "hello"; read; ssh host "cmd"` gets mangled into `"echo \"hello\"; read; ssh host \"cmd\""` which `sh` tries to execute as a single command name.

**What the agent wants to do:** Run interactive multi-step scripts through `tb run` that include:
- Multiple statements separated by `;`
- `echo` messages explaining what's about to happen
- `read` calls that pause for the user to press Enter (for auth prompts)
- `ssh` commands with their own quoting
- A final `read` so the user can type feedback if something went wrong

**Example that should work:**
```
tb run -s tb-a45 --timeout 180 --max-time 900 -- 'echo "About to SSH to server. Press Enter when ready."; read; ssh myhost "sudo -u postgres psql -c \"SHOW work_mem\""; echo "Press Enter (type feedback if wrong):"; read feedback; echo "Feedback: $feedback"'
```

**Root cause:** `build_shell_command` in `main.rs` double-quote-escapes each positional arg and joins them, then wraps in `sh -c '...'`. When the agent passes a single-quoted script as one arg, it gets double-quoted, turning the entire script into a string literal that `sh` interprets as a command name rather than shell code.

### Cleanup

- [ ] Remove old fish scripts (`bin/`)
- [ ] Remove old fish tests (`test/`)
