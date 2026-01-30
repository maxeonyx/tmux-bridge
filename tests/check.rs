//! End-to-end black-box tests for `tb check`
//!
//! These tests verify checking on background task status and output.

mod common;

use assert_cmd::Command;
use common::{TestSession, cleanup_all_tb_sessions};
use predicates::prelude::*;
use std::thread;
use std::time::Duration;

mod check_output {
    use super::*;

    #[test]
    fn shows_task_output() {
        let session = TestSession::new();

        // Launch a task that outputs something
        let task_id = session.launch_task(&["sh", "-c", "echo 'task output here'; sleep 60"]);

        // Give it a moment to produce output
        thread::sleep(Duration::from_millis(500));

        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &session.id)
            .args(["check", &task_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("task output here"));
    }

    #[test]
    fn shows_running_status_for_active_task() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &session.id)
            .args(["check", &task_id])
            .assert()
            .success()
            // Should indicate task is still running (no "complete" message)
            .stdout(predicate::str::contains("complete").not());
    }

    #[test]
    fn shows_complete_status_and_done_hint_for_finished_task() {
        let session = TestSession::new();

        // Launch a task that finishes quickly
        let task_id = session.launch_task(&["echo", "done"]);

        // Wait for it to complete
        thread::sleep(Duration::from_secs(1));

        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &session.id)
            .args(["check", &task_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete").or(predicate::str::contains("finished")))
            .stdout(predicate::str::contains("tb done"));
    }

    #[test]
    fn shows_exit_code_for_finished_task() {
        let session = TestSession::new();

        // Launch a task that exits with specific code
        let task_id = session.launch_task(&["sh", "-c", "exit 42"]);

        thread::sleep(Duration::from_secs(1));

        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &session.id)
            .args(["check", &task_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("42").or(predicate::str::contains("exit")));
    }
}

mod check_truncation {
    use super::*;

    #[test]
    fn respects_first_and_last_flags() {
        let session = TestSession::new();

        // Launch task that outputs many lines
        let task_id = session.launch_task(&["sh", "-c", "seq 1 200; sleep 60"]);

        thread::sleep(Duration::from_secs(1));

        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &session.id)
            .args(["check", &task_id, "--first", "5", "--last", "5"])
            .assert()
            .success()
            .stdout(predicate::str::contains("truncated"));
    }
}

mod check_errors {
    use super::*;

    #[test]
    fn fails_for_nonexistent_task() {
        let session = TestSession::new();

        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &session.id)
            .args(["check", "t999"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("No task")));
    }

    #[test]
    fn fails_without_session() {
        cleanup_all_tb_sessions();

        Command::cargo_bin("tb")
            .unwrap()
            .args(["check", "t1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No session specified"));
    }
}
