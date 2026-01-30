//! End-to-end black-box tests for `tb start`
//!
//! These tests verify the behavior of the `tb start` command.
//! Since `tb start` requires an interactive terminal, most tests run
//! the command inside a temporary tmux session to provide a TTY.

mod common;

use assert_cmd::Command;
use common::{cleanup_all_tb_sessions, cleanup_session, session_exists};
use predicates::prelude::*;
use std::process::Command as StdCommand;
use std::thread::sleep;
use std::time::Duration;

/// Run `tb start` inside a temporary tmux session and return the pane content.
/// This provides `tb start` with a real TTY.
///
/// Note: Does NOT clean up tb-* sessions - caller is responsible for cleanup.
/// The test runner session (tb-test-runner) is always cleaned up.
fn run_tb_start_in_tmux(args: &[&str]) -> (bool, String) {
    run_tb_start_in_tmux_with_env(args, &[])
}

/// Run `tb start` inside a temporary tmux session with custom environment variables.
fn run_tb_start_in_tmux_with_env(args: &[&str], env: &[(&str, &str)]) -> (bool, String) {
    let test_session = "tb-test-runner";

    // Clean up any previous test runner session
    let _ = StdCommand::new("tmux")
        .args(["kill-session", "-t", test_session])
        .output();

    // Create a session to run tb start in
    let status = StdCommand::new("tmux")
        .args(["new-session", "-d", "-s", test_session])
        .status()
        .expect("Failed to create test tmux session");
    assert!(status.success(), "Failed to create test tmux session");

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
    StdCommand::new("tmux")
        .args(["send-keys", "-t", test_session, &tb_cmd, "Enter"])
        .status()
        .expect("Failed to send keys to tmux");

    // Wait for the command to execute and output to appear
    sleep(Duration::from_millis(500));

    // Capture the pane content (this gets the output before tb start execs into attach)
    let output = StdCommand::new("tmux")
        .args(["capture-pane", "-t", test_session, "-p"])
        .output()
        .expect("Failed to capture pane");

    let content = String::from_utf8_lossy(&output.stdout).to_string();

    // Check if the tb-test-runner session still exists (it might have been replaced)
    let success = StdCommand::new("tmux")
        .args(["has-session", "-t", test_session])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // Clean up test runner session only
    let _ = StdCommand::new("tmux")
        .args(["kill-session", "-t", test_session])
        .output();

    (success, content)
}

mod start {
    use super::*;

    #[test]
    fn creates_tmux_session_with_auto_id() {
        cleanup_all_tb_sessions();

        let (_, content) = run_tb_start_in_tmux(&[]);

        // Extract session ID from "Started session 'xyz'"
        if let Some(start) = content.find("Started session '") {
            let rest = &content[start + 17..];
            if let Some(end) = rest.find("'") {
                let session_id = &rest[..end];
                assert!(
                    session_exists(session_id),
                    "Session '{}' should exist after tb start",
                    session_id
                );
                cleanup_session(session_id);
                return;
            }
        }
        panic!("Could not extract session ID from output: {}", content);
    }

    #[test]
    fn session_id_format_is_letter_plus_two_alphanumeric() {
        cleanup_all_tb_sessions();

        let (_, content) = run_tb_start_in_tmux(&[]);

        // Should contain a session ID matching pattern [a-z][a-z0-9][a-z0-9]
        assert!(
            predicate::str::is_match(r"'[a-z][a-z0-9]{2}'")
                .unwrap()
                .eval(&content),
            "Output should contain session ID in format 'X##' (e.g., 'a7x'): {}",
            content
        );

        cleanup_all_tb_sessions();
    }

    #[test]
    fn output_includes_export_instruction() {
        cleanup_all_tb_sessions();

        let (_, content) = run_tb_start_in_tmux(&[]);

        assert!(
            content.contains("export TB_SESSION="),
            "Output should contain export instruction: {}",
            content
        );

        cleanup_all_tb_sessions();
    }

    #[test]
    fn output_includes_tell_your_agent_message() {
        cleanup_all_tb_sessions();

        let (_, content) = run_tb_start_in_tmux(&[]);

        assert!(
            content.contains("Tell your agent:"),
            "Output should contain 'Tell your agent:': {}",
            content
        );

        cleanup_all_tb_sessions();
    }

    #[test]
    fn sequential_sessions_get_different_first_letters() {
        cleanup_all_tb_sessions();

        // Start first session
        let (_, content1) = run_tb_start_in_tmux(&[]);

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
        let (_, content2) = run_tb_start_in_tmux(&[]);
        let letter2 = extract_first_letter(&content2);
        assert_eq!(
            letter2,
            Some('b'),
            "Second session should start with 'b': {}",
            content2
        );

        cleanup_all_tb_sessions();
    }

    #[test]
    fn explicit_session_id_is_used() {
        cleanup_all_tb_sessions();

        let (_, content) = run_tb_start_in_tmux(&["--session", "test123"]);

        assert!(
            content.contains("test123"),
            "Output should contain explicit session ID: {}",
            content
        );
        assert!(session_exists("test123"), "Session 'test123' should exist");

        cleanup_session("test123");
    }

    #[test]
    fn rejects_duplicate_explicit_session_id() {
        cleanup_all_tb_sessions();

        // Start first session with explicit ID
        let (_, _) = run_tb_start_in_tmux(&["--session", "dupe"]);
        assert!(session_exists("dupe"), "First session should exist");

        // Try to start second session with same ID - should fail
        // Run directly (no TTY) since we want to check the error
        let output = Command::cargo_bin("tb")
            .unwrap()
            .args(["start", "--session", "dupe"])
            .output()
            .unwrap();

        // It will fail due to no TTY, but if it got past the TTY check,
        // let's verify the duplicate check works by running in tmux
        let (_, content) = run_tb_start_in_tmux(&["--session", "dupe"]);
        assert!(
            content.contains("already exists"),
            "Should reject duplicate session ID: {}",
            content
        );

        cleanup_session("dupe");
    }

    #[test]
    fn fails_when_not_interactive() {
        cleanup_all_tb_sessions();

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
        cleanup_all_tb_sessions();

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
        cleanup_all_tb_sessions();

        // Without TB_TEST_MODE, sessions should use normal "tb-" prefix.

        let (_, content) = run_tb_start_in_tmux(&[]);

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
