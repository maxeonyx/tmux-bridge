//! End-to-end black-box tests for `tb run`
//!
//! These tests verify synchronous command execution through the bridge.

mod common;

use common::{TestSession, cleanup_all_tb_sessions, tb_cmd};
use predicates::prelude::*;
use std::time::Duration;

mod run_session_resolution {
    use super::*;

    #[test]
    fn fails_without_session() {
        tb_cmd()
            .args(["run", "--", "echo", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No session specified"))
            .stderr(predicate::str::contains("TB_SESSION"))
            .stderr(predicate::str::contains("--session"));
    }

    #[test]
    fn uses_tb_session_env_var() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "echo", "hello"])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello"));
    }

    #[test]
    fn session_flag_overrides_env_var() {
        let session = TestSession::new();

        // Set env var to nonexistent session, but use --session with real one
        tb_cmd()
            .env("TB_SESSION", "nonexistent")
            .args(["run", "--session", &session.id, "--", "echo", "override"])
            .assert()
            .success()
            .stdout(predicate::str::contains("override"));
    }

    #[test]
    fn fails_with_nonexistent_session() {
        cleanup_all_tb_sessions();

        tb_cmd()
            .args(["run", "--session", "nonexistent99", "--", "echo", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found"))
            .stderr(predicate::str::contains("tb start"));
    }
}

mod run_command_execution {
    use super::*;

    #[test]
    fn simple_echo_returns_output() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "echo", "hello world"])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello world"));
    }

    #[test]
    fn captures_multiline_output() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "printf", "line1\\nline2\\nline3\\n"])
            .assert()
            .success()
            .stdout(predicate::str::contains("line1"))
            .stdout(predicate::str::contains("line2"))
            .stdout(predicate::str::contains("line3"));
    }

    #[test]
    fn preserves_exit_status_zero() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "true"])
            .assert()
            .success();
    }

    #[test]
    fn preserves_exit_status_nonzero() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "false"])
            .assert()
            .failure()
            .code(1);
    }

    #[test]
    fn preserves_specific_exit_code() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "sh", "-c", "exit 42"])
            .assert()
            .failure()
            .code(42);
    }

    #[test]
    fn handles_command_with_special_characters() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "echo", "hello; world && test | pipe"])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello; world && test | pipe"));
    }

    #[test]
    fn handles_command_with_quotes() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--", "echo", "it's a \"quoted\" string"])
            .assert()
            .success()
            .stdout(predicate::str::contains("it's a \"quoted\" string"));
    }
}

mod run_timeouts {
    use super::*;

    #[test]
    fn no_output_timeout_triggers() {
        let session = TestSession::new();

        // sleep produces no output, should timeout
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--timeout", "2", "--", "sleep", "30"])
            .timeout(Duration::from_secs(10))
            .assert()
            .failure()
            .code(124)
            .stderr(predicate::str::contains("Timeout"));
    }

    #[test]
    fn max_time_timeout_triggers() {
        let session = TestSession::new();

        // Command that produces output but runs too long
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args([
                "run",
                "--timeout",
                "60", // long no-output timeout
                "--max-time",
                "2", // short overall timeout
                "--",
                "sh",
                "-c",
                "while true; do echo tick; sleep 1; done",
            ])
            .timeout(Duration::from_secs(10))
            .assert()
            .failure()
            .code(124)
            .stderr(predicate::str::contains("Timeout"));
    }

    #[test]
    fn fast_command_does_not_timeout() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["run", "--timeout", "2", "--", "echo", "quick"])
            .assert()
            .success()
            .stdout(predicate::str::contains("quick"));
    }
}

mod run_output_truncation {
    use super::*;

    #[test]
    fn truncates_long_output() {
        let session = TestSession::new();

        // Generate 200 lines, request first 5 and last 5
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args([
                "run", "--first", "5", "--last", "5", "--", "seq", "1", "200",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("1"))
            .stdout(predicate::str::contains("5"))
            .stdout(predicate::str::contains("truncated"))
            .stdout(predicate::str::contains("196"))
            .stdout(predicate::str::contains("200"));
    }

    #[test]
    fn does_not_truncate_short_output() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args([
                "run", "--first", "50", "--last", "50", "--", "seq", "1", "10",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("1"))
            .stdout(predicate::str::contains("10"))
            // Should NOT contain truncation message
            .stdout(predicate::str::contains("truncated").not());
    }
}
