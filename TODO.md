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

### Cleanup

- [ ] Remove old fish scripts (`bin/`)
- [ ] Remove old fish tests (`test/`)
