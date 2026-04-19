//! End-to-end black-box tests for `tb start`
//!
//! These tests verify the behavior of the `tb start` command.
//! Since `tb start` requires an interactive terminal, most tests run
//! the command inside a temporary tmux session to provide a TTY.

mod common;

use assert_cmd::Command;
use common::{cleanup_session, session_exists, wait_for_pane_content, wait_for_session_exists};
use predicates::prelude::*;
use std::process::Command as StdCommand;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static RUNNER_COUNTER: AtomicU64 = AtomicU64::new(1);
static PREFIX_COUNTER: AtomicU64 = AtomicU64::new(1);

fn unique_runner_session_name() -> String {
    format!(
        "tb-test-runner-{}-{}",
        std::process::id(),
        RUNNER_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn unique_test_prefix() -> String {
    format!(
        "tbtest-start-{}-{}-",
        std::process::id(),
        PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn extract_session_id(content: &str) -> Option<String> {
    let start = content.find("Started session '")?;
    let rest = &content[start + 17..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

fn session_exists_with_prefix(prefix: &str, session_id: &str) -> bool {
    StdCommand::new("tmux")
        .args(["has-session", "-t", &format!("{}{}", prefix, session_id)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn cleanup_session_with_prefix(prefix: &str, session_id: &str) {
    let _ = StdCommand::new("tmux")
        .args(["kill-session", "-t", &format!("{}{}", prefix, session_id)])
        .output();
}

fn env_value<'a>(env: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    env.iter()
        .find_map(|(name, value)| (*name == key).then_some(*value))
}

fn expected_session_name(env: &[(&str, &str)], session_id: &str) -> String {
    if let Some(prefix) = env_value(env, "TB_SESSION_PREFIX") {
        format!("{}{}", prefix, session_id)
    } else if env_value(env, "TB_TEST_MODE") == Some("1") {
        format!("tbtest-{}", session_id)
    } else {
        format!("tb-{}", session_id)
    }
}

struct RunnerSession {
    name: String,
}

impl RunnerSession {
    fn new() -> Self {
        let name = unique_runner_session_name();
        let status = StdCommand::new("tmux")
            .args(["new-session", "-d", "-s", &name])
            .status()
            .expect("Failed to create test tmux session");

        assert!(status.success(), "Failed to create test tmux session");

        Self { name }
    }

    fn send_keys(&self, command: &str) {
        let status = StdCommand::new("tmux")
            .args(["send-keys", "-t", &self.name, command, "Enter"])
            .status()
            .expect("Failed to send keys to tmux");

        assert!(status.success(), "Failed to send keys to tmux");
    }

    fn wait_for_start_output(&self) -> String {
        wait_for_pane_content(
            &self.name,
            &format!("tb start output to appear in runner session {}", self.name),
            Duration::from_secs(15),
            |content| {
                content.contains("Started session '")
                    || content.contains("already exists")
                    || content.contains("interactive")
            },
        )
    }
}

impl Drop for RunnerSession {
    fn drop(&mut self) {
        let _ = StdCommand::new("tmux")
            .args(["kill-session", "-t", &self.name])
            .output();
    }
}

/// Run `tb start` inside a temporary tmux session with custom environment variables.
fn run_tb_start_in_tmux_with_env(args: &[&str], env: &[(&str, &str)]) -> (bool, String) {
    let runner = RunnerSession::new();

    // Build the tb command with optional env vars
    let tb_path = assert_cmd::cargo::cargo_bin("tb");
    let env_prefix = env
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(" ");

    let tb_cmd = if env.is_empty() {
        if args.is_empty() {
            format!("{} start", tb_path.display())
        } else {
            format!("{} start {}", tb_path.display(), args.join(" "))
        }
    } else if args.is_empty() {
        format!("{} {} start", env_prefix, tb_path.display())
    } else {
        format!(
            "{} {} start {}",
            env_prefix,
            tb_path.display(),
            args.join(" ")
        )
    };

    // Send the command to the tmux session
    runner.send_keys(&tb_cmd);

    // Capture the pane content once the command has produced observable output.
    let content = runner.wait_for_start_output();

    if let Some(session_id) = extract_session_id(&content) {
        let session_name = expected_session_name(env, &session_id);
        wait_for_session_exists(&session_name, Duration::from_secs(15));
    }

    // Check if the tb-test-runner session still exists (it might have been replaced)
    let success = StdCommand::new("tmux")
        .args(["has-session", "-t", &runner.name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    (success, content)
}

mod start {
    use super::*;

    #[test]
    fn creates_tmux_session_with_auto_id() {
        let prefix = unique_test_prefix();
        let (_, content) = run_tb_start_in_tmux_with_env(
            &[],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );

        // Extract session ID from "Started session 'xyz'"
        if let Some(session_id) = extract_session_id(&content) {
            assert!(
                session_exists_with_prefix(&prefix, &session_id),
                "Session '{}' should exist after tb start",
                session_id
            );
            cleanup_session_with_prefix(&prefix, &session_id);
            return;
        }
        panic!("Could not extract session ID from output: {}", content);
    }

    #[test]
    fn session_id_format_is_letter_plus_two_alphanumeric() {
        let prefix = unique_test_prefix();
        let (_, content) = run_tb_start_in_tmux_with_env(
            &[],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );

        // Should contain a session ID matching pattern [a-z][a-z0-9][a-z0-9]
        assert!(
            predicate::str::is_match(r"'[a-z][a-z0-9]{2}'")
                .unwrap()
                .eval(&content),
            "Output should contain session ID in format 'X##' (e.g., 'a7x'): {}",
            content
        );

        if let Some(session_id) = extract_session_id(&content) {
            cleanup_session_with_prefix(&prefix, &session_id);
        }
    }

    #[test]
    fn output_includes_target_instruction() {
        let prefix = unique_test_prefix();
        let (_, content) = run_tb_start_in_tmux_with_env(
            &[],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );

        assert!(
            content.contains("tb run --target"),
            "Output should contain target instruction: {}",
            content
        );

        if let Some(session_id) = extract_session_id(&content) {
            cleanup_session_with_prefix(&prefix, &session_id);
        }
    }

    #[test]
    fn output_includes_tell_your_agent_message() {
        let prefix = unique_test_prefix();
        let (_, content) = run_tb_start_in_tmux_with_env(
            &[],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );

        assert!(
            content.contains("Tell your agent:"),
            "Output should contain 'Tell your agent:': {}",
            content
        );

        if let Some(session_id) = extract_session_id(&content) {
            cleanup_session_with_prefix(&prefix, &session_id);
        }
    }

    #[test]
    fn sequential_sessions_get_different_first_letters() {
        let prefix = unique_test_prefix();

        // Start first session
        let (_, content1) = run_tb_start_in_tmux_with_env(
            &[],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );

        // Extract first letter
        let extract_first_letter = |s: &str| -> Option<char> {
            s.find("Started session '")
                .and_then(|i| s.chars().nth(i + 17))
        };

        let letter1 = extract_first_letter(&content1);
        assert_eq!(
            letter1,
            Some('a'),
            "First session should start with 'a': {}",
            content1
        );

        // Start second session
        let (_, content2) = run_tb_start_in_tmux_with_env(
            &[],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );
        let letter2 = extract_first_letter(&content2);
        assert_eq!(
            letter2,
            Some('b'),
            "Second session should start with 'b': {}",
            content2
        );

        if let Some(session_id) = extract_session_id(&content1) {
            cleanup_session_with_prefix(&prefix, &session_id);
        }
        if let Some(session_id) = extract_session_id(&content2) {
            cleanup_session_with_prefix(&prefix, &session_id);
        }
    }

    #[test]
    fn explicit_session_id_is_used() {
        let prefix = unique_test_prefix();
        let explicit_id = format!("test123-{}", PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed));
        let (_, content) = run_tb_start_in_tmux_with_env(
            &["--session", &explicit_id],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );

        assert!(
            content.contains(&explicit_id),
            "Output should contain explicit session ID: {}",
            content
        );
        assert!(
            session_exists_with_prefix(&prefix, &explicit_id),
            "Session '{}' should exist",
            explicit_id
        );

        cleanup_session_with_prefix(&prefix, &explicit_id);
    }

    #[test]
    fn rejects_duplicate_explicit_session_id() {
        let prefix = unique_test_prefix();
        let explicit_id = format!("dupe-{}", PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed));

        // Start first session with explicit ID
        let (_, _) = run_tb_start_in_tmux_with_env(
            &["--session", &explicit_id],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );
        assert!(
            session_exists_with_prefix(&prefix, &explicit_id),
            "First session should exist"
        );

        // Try to start second session with same ID - should fail
        // Run directly (no TTY) since we want to check the error
        let output = Command::cargo_bin("tb")
            .unwrap()
            .env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &prefix)
            .args(["start", "--session", &explicit_id])
            .output()
            .unwrap();

        assert!(
            !output.status.success(),
            "Non-interactive duplicate check should still fail"
        );

        // It will fail due to no TTY, but if it got past the TTY check,
        // let's verify the duplicate check works by running in tmux
        let (_, content) = run_tb_start_in_tmux_with_env(
            &["--session", &explicit_id],
            &[("TB_TEST_MODE", "1"), ("TB_SESSION_PREFIX", &prefix)],
        );
        assert!(
            content.contains("already exists"),
            "Should reject duplicate session ID: {}",
            content
        );

        cleanup_session_with_prefix(&prefix, &explicit_id);
    }

    #[test]
    fn fails_when_not_interactive() {
        // tb start is for humans only - it must be run interactively.
        // When run without a TTY (like by an agent), it should fail with
        // a helpful error message.

        let output = Command::cargo_bin("tb")
            .unwrap()
            .args(["start", "--session", "attach-test"])
            .output()
            .unwrap();

        // Should fail
        assert!(
            !output.status.success(),
            "tb start should fail when not interactive"
        );

        // Session should NOT be created
        assert!(
            !session_exists("attach-test"),
            "Session should not be created when not interactive"
        );

        // Error should explain what to do
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("interactive"),
            "Error should mention 'interactive': {}",
            stderr
        );
    }

    #[test]
    fn uses_tbtest_prefix_when_test_mode_set() {
        // When TB_TEST_MODE is set, sessions should use "tbtest-" prefix
        // instead of "tb-" to avoid interfering with real sessions.

        let (_, content) = run_tb_start_in_tmux_with_env(&[], &[("TB_TEST_MODE", "1")]);

        // Extract session ID
        if let Some(start) = content.find("Started session '") {
            let rest = &content[start + 17..];
            if let Some(end) = rest.find("'") {
                let session_id = &rest[..end];

                // The tmux session should be named "tbtest-{id}" not "tb-{id}"
                let tbtest_exists = StdCommand::new("tmux")
                    .args(["has-session", "-t", &format!("tbtest-{}", session_id)])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                let tb_exists = StdCommand::new("tmux")
                    .args(["has-session", "-t", &format!("tb-{}", session_id)])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                assert!(tbtest_exists, "Session should exist with tbtest- prefix");
                assert!(!tb_exists, "Session should NOT exist with tb- prefix");

                // Cleanup
                let _ = StdCommand::new("tmux")
                    .args(["kill-session", "-t", &format!("tbtest-{}", session_id)])
                    .output();
                return;
            }
        }
        panic!("Could not extract session ID from output: {}", content);
    }

    #[test]
    fn uses_tb_prefix_when_test_mode_not_set() {
        // Without TB_TEST_MODE, sessions should use normal "tb-" prefix.
        // Explicitly pass empty env to avoid the default TB_TEST_MODE=1.

        let (_, content) = run_tb_start_in_tmux_with_env(&[], &[]);

        // Extract session ID
        if let Some(start) = content.find("Started session '") {
            let rest = &content[start + 17..];
            if let Some(end) = rest.find("'") {
                let session_id = &rest[..end];

                // The tmux session should be named "tb-{id}"
                let tb_exists = StdCommand::new("tmux")
                    .args(["has-session", "-t", &format!("tb-{}", session_id)])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);

                assert!(tb_exists, "Session should exist with tb- prefix");

                cleanup_session(session_id);
                return;
            }
        }
        panic!("Could not extract session ID from output: {}", content);
    }
}
