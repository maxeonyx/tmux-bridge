//! End-to-end black-box tests for `tb launch`
//!
//! These tests verify background task launching in split panes.

mod common;

use common::{TestSession, cleanup_all_tb_sessions, tb_cmd};
use predicates::prelude::*;

mod launch_basic {
    use super::*;

    #[test]
    fn returns_task_id() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .success()
            .stdout(predicate::str::is_match(r"Task t\d+ started").unwrap());
    }

    #[test]
    fn output_includes_check_instruction() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .success()
            .stdout(predicate::str::contains("tb check t"));
    }

    #[test]
    fn creates_new_pane() {
        let session = TestSession::new();

        // Should start with 1 pane (main session)
        assert_eq!(session.count_panes(), 1, "Should start with 1 pane");

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .success();

        // Should now have 2 panes
        assert_eq!(session.count_panes(), 2, "Should have 2 panes after launch");
    }

    #[test]
    fn task_ids_are_sequential() {
        let session = TestSession::new();

        let output1 = tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
            .output()
            .unwrap();

        let output2 = tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
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
}

mod launch_pane_layout {
    use super::*;

    #[test]
    fn first_three_tasks_create_horizontal_splits() {
        let session = TestSession::new();

        for i in 1..=3 {
            tb_cmd()
                .env("TB_SESSION", &session.id)
                .args(["launch", "--", "sleep", "60"])
                .assert()
                .success();

            assert_eq!(
                session.count_panes(),
                i + 1,
                "Should have {} panes after {} launches",
                i + 1,
                i
            );
        }
    }

    #[test]
    fn tasks_four_through_six_split_vertically() {
        let session = TestSession::new();

        // Launch 6 tasks
        for _ in 1..=6 {
            tb_cmd()
                .env("TB_SESSION", &session.id)
                .args(["launch", "--", "sleep", "60"])
                .assert()
                .success();
        }

        // Should have 7 panes (1 main + 6 tasks)
        assert_eq!(
            session.count_panes(),
            7,
            "Should have 7 panes after 6 launches"
        );
    }

    #[test]
    fn rejects_seventh_task() {
        let session = TestSession::new();

        // Launch 6 tasks
        for _ in 1..=6 {
            tb_cmd()
                .env("TB_SESSION", &session.id)
                .args(["launch", "--", "sleep", "60"])
                .assert()
                .success();
        }

        // Seventh should fail
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("too many"))
            .stderr(predicate::str::contains("tb done"));
    }
}

mod launch_session_resolution {
    use super::*;

    #[test]
    fn fails_without_session() {
        cleanup_all_tb_sessions();

        tb_cmd()
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No session specified"));
    }

    #[test]
    fn uses_tb_session_env_var() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "echo", "test"])
            .assert()
            .success();
    }
}
