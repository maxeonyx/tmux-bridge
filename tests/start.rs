//! End-to-end black-box tests for `tb start`
//!
//! These tests verify the behavior of the `tb start` command by spawning
//! the binary and checking stdout/stderr/exit codes.

mod common;

use assert_cmd::Command;
use common::{cleanup_all_tb_sessions, cleanup_session, session_exists};
use predicates::prelude::*;

mod start {
    use super::*;

    #[test]
    fn creates_tmux_session_with_auto_id() {
        cleanup_all_tb_sessions();

        let output = Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .output()
            .unwrap();

        assert!(output.status.success(), "tb start should succeed");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract session ID and verify session exists
        if let Some(start) = stdout.find("'") {
            if let Some(end) = stdout[start + 1..].find("'") {
                let session_id = &stdout[start + 1..start + 1 + end];
                assert!(
                    session_exists(session_id),
                    "Session '{}' should exist after tb start",
                    session_id
                );
                cleanup_session(session_id);
                return;
            }
        }
        panic!("Could not extract session ID from output: {}", stdout);
    }

    #[test]
    fn session_id_format_is_letter_plus_two_alphanumeric() {
        cleanup_all_tb_sessions();

        let output = Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should contain a session ID matching pattern [a-z][a-z0-9][a-z0-9]
        assert!(
            predicate::str::is_match(r"'[a-z][a-z0-9]{2}'")
                .unwrap()
                .eval(&stdout),
            "Output should contain session ID in format 'X##' (e.g., 'a7x'): {}",
            stdout
        );

        cleanup_all_tb_sessions();
    }

    #[test]
    fn output_includes_export_instruction() {
        cleanup_all_tb_sessions();

        Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .assert()
            .success()
            .stdout(predicate::str::contains("export TB_SESSION="));

        cleanup_all_tb_sessions();
    }

    #[test]
    fn output_includes_tell_your_agent_message() {
        cleanup_all_tb_sessions();

        Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .assert()
            .success()
            .stdout(predicate::str::contains("Tell your agent:"));

        cleanup_all_tb_sessions();
    }

    #[test]
    fn sequential_sessions_get_different_first_letters() {
        cleanup_all_tb_sessions();

        // Start first session
        let output1 = Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .output()
            .unwrap();
        let stdout1 = String::from_utf8_lossy(&output1.stdout);

        // Start second session
        let output2 = Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .output()
            .unwrap();
        let stdout2 = String::from_utf8_lossy(&output2.stdout);

        // Extract first letter of each session ID
        let extract_first_letter =
            |s: &str| -> Option<char> { s.find("'").and_then(|i| s.chars().nth(i + 1)) };

        let letter1 = extract_first_letter(&stdout1);
        let letter2 = extract_first_letter(&stdout2);

        assert!(letter1.is_some(), "First session should have an ID");
        assert!(letter2.is_some(), "Second session should have an ID");
        assert_eq!(letter1, Some('a'), "First session should start with 'a'");
        assert_eq!(letter2, Some('b'), "Second session should start with 'b'");

        cleanup_all_tb_sessions();
    }

    #[test]
    fn explicit_session_id_is_used() {
        cleanup_all_tb_sessions();

        Command::cargo_bin("tb")
            .unwrap()
            .args(["start", "--session", "test123"])
            .assert()
            .success()
            .stdout(predicate::str::contains("test123"));

        assert!(session_exists("test123"), "Session 'test123' should exist");

        cleanup_session("test123");
    }

    #[test]
    fn rejects_duplicate_explicit_session_id() {
        cleanup_all_tb_sessions();

        // Start first session with explicit ID
        Command::cargo_bin("tb")
            .unwrap()
            .args(["start", "--session", "dupe"])
            .assert()
            .success();

        // Try to start second session with same ID - should fail
        Command::cargo_bin("tb")
            .unwrap()
            .args(["start", "--session", "dupe"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));

        cleanup_session("dupe");
    }

    #[test]
    fn prints_attach_hint_when_not_interactive() {
        cleanup_all_tb_sessions();

        // When run without a TTY (like in tests), tb start should create the
        // session but print a message explaining it needs to be run interactively
        // to attach automatically.

        let output = Command::cargo_bin("tb")
            .unwrap()
            .args(["start", "--session", "attach-test"])
            .output()
            .unwrap();

        // Session should still be created
        assert!(
            session_exists("attach-test"),
            "Session should be created even without TTY"
        );

        // Should tell user how to attach manually
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("tmux attach"),
            "Should tell user how to attach when not interactive: {}",
            stdout
        );

        cleanup_session("attach-test");
    }
}
