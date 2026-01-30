//! End-to-end black-box tests for `tb done`
//!
//! These tests verify closing background task panes.

mod common;

use common::{TestSession, cleanup_all_tb_sessions, tb_cmd};
use predicates::prelude::*;
use std::thread;
use std::time::Duration;

mod done_basic {
    use super::*;

    #[test]
    fn closes_task_pane() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        // Should have 2 panes now
        assert_eq!(session.count_panes(), 2);

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &task_id])
            .assert()
            .success();

        // Should be back to 1 pane
        assert_eq!(session.count_panes(), 1);
    }

    #[test]
    fn outputs_confirmation() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &task_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("Closed").or(predicate::str::contains("closed")));
    }

    #[test]
    fn can_close_multiple_tasks() {
        let session = TestSession::new();

        let t1 = session.launch_task(&["sleep", "60"]);
        let t2 = session.launch_task(&["sleep", "60"]);
        let t3 = session.launch_task(&["sleep", "60"]);

        assert_eq!(session.count_panes(), 4);

        // Close middle one
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &t2])
            .assert()
            .success();

        assert_eq!(session.count_panes(), 3);

        // Close first
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &t1])
            .assert()
            .success();

        assert_eq!(session.count_panes(), 2);

        // Close last
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &t3])
            .assert()
            .success();

        assert_eq!(session.count_panes(), 1);
    }
}

mod done_with_finished_tasks {
    use super::*;

    #[test]
    fn can_close_already_finished_task() {
        let session = TestSession::new();

        // Launch a task that finishes immediately
        let task_id = session.launch_task(&["echo", "done"]);

        // Wait for it to complete
        thread::sleep(Duration::from_secs(1));

        // Should still be able to close the pane
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &task_id])
            .assert()
            .success();
    }
}

mod done_errors {
    use super::*;

    #[test]
    fn fails_for_nonexistent_task() {
        let session = TestSession::new();

        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", "t999"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("No task")));
    }

    #[test]
    fn fails_without_session() {
        cleanup_all_tb_sessions();

        tb_cmd()
            .args(["done", "t1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No session specified"));
    }

    #[test]
    fn fails_for_already_closed_task() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        // Close it
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &task_id])
            .assert()
            .success();

        // Try to close again
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &task_id])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("already")));
    }
}

mod done_allows_new_launches {
    use super::*;

    #[test]
    fn can_launch_after_closing_task() {
        let session = TestSession::new();

        // Fill up with 6 tasks
        let mut tasks = Vec::new();
        for _ in 0..6 {
            tasks.push(session.launch_task(&["sleep", "60"]));
        }

        assert_eq!(session.count_panes(), 7);

        // Close one
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["done", &tasks[0]])
            .assert()
            .success();

        assert_eq!(session.count_panes(), 6);

        // Should now be able to launch a new one
        tb_cmd()
            .env("TB_SESSION", &session.id)
            .args(["launch", "--", "sleep", "60"])
            .assert()
            .success();

        assert_eq!(session.count_panes(), 7);
    }
}
