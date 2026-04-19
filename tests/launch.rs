//! End-to-end black-box tests for `tb launch`
//!
//! These tests verify background task launching in split panes.

mod common;

use common::{TestSession, tb_cmd};
use predicates::prelude::*;
use std::process::Command as StdCommand;

mod launch_basic {
    use super::*;

    #[test]
    fn returns_task_id() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .assert()
            .success()
            .stdout(predicate::str::is_match(r"Task t\d+ started").unwrap());
    }

    #[test]
    fn output_includes_check_instruction() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .assert()
            .success()
            .stdout(predicate::str::contains("tb check --target"));
    }

    #[test]
    fn creates_new_pane() {
        let session = TestSession::new();

        // Should start with 1 pane (main session)
        assert_eq!(
            session.wait_for_pane_count(1),
            1,
            "Should start with 1 pane"
        );

        session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .assert()
            .success();

        // Should now have 2 panes
        assert_eq!(
            session.wait_for_pane_count(2),
            2,
            "Should have 2 panes after launch"
        );
    }

    #[test]
    fn task_ids_are_sequential() {
        let session = TestSession::new();

        let output1 = session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .output()
            .unwrap();

        let output2 = session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .output()
            .unwrap();

        let stdout1 = String::from_utf8_lossy(&output1.stdout);
        let stdout2 = String::from_utf8_lossy(&output2.stdout);

        assert!(
            stdout1.contains("t1"),
            "First task should be t1: {}",
            stdout1
        );
        assert!(
            stdout2.contains("t2"),
            "Second task should be t2: {}",
            stdout2
        );
    }

    #[test]
    fn ignores_untagged_panes_when_assigning_task_ids() {
        let session = TestSession::new();

        let split = StdCommand::new("tmux")
            .args(["split-window", "-t", &session.tmux_name(), "-d", "-l", "5"])
            .output()
            .expect("Failed to create extra untagged pane");
        assert!(split.status.success(), "Failed to create extra pane");

        tb_cmd()
            .args(["launch", "-t", &session.pane_target(), "--", "sleep", "60"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Task t1 started."));
    }
}

mod launch_pane_splitting {
    use super::*;

    #[test]
    fn repeated_launches_keep_creating_panes() {
        let session = TestSession::new();

        for i in 1..=3 {
            session
                .tb_command()
                .args(["launch", "--target", session.target(), "--", "sleep", "60"])
                .assert()
                .success();

            assert_eq!(
                session.wait_for_pane_count(i + 1),
                i + 1,
                "Should have {} panes after {} launches",
                i + 1,
                i
            );
        }
    }

    #[test]
    fn can_launch_multiple_tasks() {
        let session = TestSession::new();

        for _ in 1..=4 {
            session
                .tb_command()
                .args(["launch", "--target", session.target(), "--", "sleep", "60"])
                .assert()
                .success();
        }

        assert_eq!(
            session.wait_for_pane_count(5),
            5,
            "Should have 5 panes after 4 launches"
        );
    }

    #[test]
    fn rejects_seventh_tagged_task_even_with_untagged_panes() {
        let session = TestSession::new();

        for _ in 1..=6 {
            session
                .tb_command()
                .args(["launch", "--target", session.target(), "--", "sleep", "60"])
                .assert()
                .success();
        }

        let split = StdCommand::new("tmux")
            .args(["split-window", "-t", &session.tmux_name(), "-d", "-l", "5"])
            .output()
            .expect("Failed to create extra untagged pane");
        assert!(split.status.success(), "Failed to create extra pane");

        session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("too many"))
            .stderr(predicate::str::contains("tb done"));
    }
}

mod launch_session_resolution {
    use super::*;

    #[test]
    fn fails_without_target() {
        tb_cmd()
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No target specified"));
    }

    #[test]
    fn uses_target_flag_with_tb_session_id_fallback() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "echo", "test"])
            .assert()
            .success();
    }
}
