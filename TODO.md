# TODO

## Current State

The Rust CLI (`tb`) is **complete**. All 78 E2E tests pass.

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
- [x] **`tb run`** - Synchronous command execution (40 tests)
- [x] **`tb launch`** - Background task in split pane (9 tests)
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

### Future

- [ ] **Auto-update** — `tb` should be able to update itself (e.g. `tb update` or automatic check on startup). Currently the release script installs locally, but remote machines with `tb` installed via curl still need manual updates.
