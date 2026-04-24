# TODO

## Current State

The Rust CLI (`tb`) now includes the core bridge commands plus a minimal `tb info` shell-assessment command. All 95 E2E tests pass.

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

- [x] **`tb start`** - Create tmux session with auto-generated ID (10 tests)
- [x] **`tb info`** - Minimal shell assessment for Stage 4 (4 tests)
- [x] **`tb run`** - Synchronous command execution (51 tests)
- [x] **`tb launch`** - Background task in split pane (10 tests)
- [x] **`tb check`** - Check background task status + main pane capture (11 tests)
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

### Cleanup

- [x] Remove old fish scripts (`bin/`) — already removed during Rust rewrite
- [x] Remove old fish tests (`test/`) — already removed during Rust rewrite
- [x] Fix flaky E2E tests — fixed by giving each test unique tmux session IDs and removing global cleanup
- [x] **Eliminate test flakiness** — replaced all fixed sleeps with polling helpers. Added `wait_until`, `wait_for_pane_content`, `wait_for_pane_count`, `wait_for_session_exists` to common helpers. Refactored `start.rs` with `RunnerSession` RAII cleanup and pane-content polling. 70/70 consecutive passes (from 66.7% baseline). Stress test script: `./scripts/stress-test.sh N`

### Documentation

- [x] **Document single-arg script mode** — added clear ✅/❌ examples to AGENTS.md, VISION.md, and the opencode skill showing that `tb run -- 'script; here'` is correct and `bash -c` wrappers are never needed.

### Complete

- [x] **Arbitrary session/pane targeting** — replaced `--session` flag and `TB_SESSION` env var with single `--target` (`-t`) flag that accepts any tmux target syntax. Simple names try a literal tmux session first, then `tb-{id}` fallback. `tb start` is now optional convenience. Task accounting uses `@tb_task`-tagged panes only.

### In Progress — New Commands

- [ ] **`tb info` broader probe follow-up** — Stage 4 shipped the minimum shell assessment that `tb run` needs right now: plain-text confidence-aware reporting for fish / bash / `sh` / unknown, with unknown falling back to the existing `sh -c` path. The broader probe remains deferred:
  - richer environment details beyond shell assessment
  - stronger REPL / non-shell detection
  - tmux copy/scroll mode handling
  - broader shell families and non-Unix shells
  - any structured output format

- [ ] **`tb send`** — raw keystroke injection into target pane. No wrapping, no markers, no output capture. For `exit`, `cd`, shell builtins, control codes.
  - Does NOT send Enter by default — sends exactly what you give it, nothing more
  - `--key` flag for named keys: `tb send -t foo --key C-d`, `tb send -t foo --key Enter`
  - Text mode for literal text: `tb send -t foo -- "exit"` (agent adds `--key Enter` explicitly if needed)
  - Most common use case is single keystrokes (`--key C-d`, `--key C-c`)
  - Uses same `--target` resolution as other commands

- [ ] **`tb launch` / `tb done` safety for non-tb sessions** — if the target session was NOT started with `tb start` (doesn't have the `tb-` prefix), refuse by default. These commands create/destroy panes on the user's session — they might not want that. Require `--force` or similar override flag. Error message should explain why.

### Future

- [ ] **Auto-update** — `tb` should be able to update itself (e.g. `tb update` or automatic check on startup). Currently the release script installs locally, but remote machines with `tb` installed via curl still need manual updates.
