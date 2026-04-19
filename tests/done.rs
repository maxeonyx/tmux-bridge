//! End-to-end black-box tests for `tb done`
//!
//! These tests verify closing background task panes.

mod common;

use common::{TestSession, tb_cmd};
use predicates::prelude::*;

mod done_basic {
    use super::*;

    #[test]
    fn closes_task_pane() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        // Should have 2 panes now
        assert_eq!(session.wait_for_pane_count(2), 2);

        session
            .tb_command()
            .args(["done", "--target", session.target(), &task_id])
            .assert()
            .success();

        // Should be back to 1 pane
        assert_eq!(session.wait_for_pane_count(1), 1);
    }

    #[test]
    fn outputs_confirmation() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        session
            .tb_command()
            .args(["done", "--target", session.target(), &task_id])
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

        assert_eq!(session.wait_for_pane_count(4), 4);

        // Close middle one
        session
            .tb_command()
            .args(["done", "--target", session.target(), &t2])
            .assert()
            .success();

        assert_eq!(session.wait_for_pane_count(3), 3);

        // Close first
        session
            .tb_command()
            .args(["done", "--target", session.target(), &t1])
            .assert()
            .success();

        assert_eq!(session.wait_for_pane_count(2), 2);

        // Close last
        session
            .tb_command()
            .args(["done", "--target", session.target(), &t3])
            .assert()
            .success();

        assert_eq!(session.wait_for_pane_count(1), 1);
    }
}

mod done_with_finished_tasks {
    use super::*;

    #[test]
    fn can_close_already_finished_task() {
        let session = TestSession::new();

        // Launch a task that finishes immediately
        let task_id = session.launch_task(&["echo", "done"]);

        session.wait_for_check_output(&task_id, |stdout| {
            stdout.contains("complete") || stdout.contains("finished")
        });

        // Should still be able to close the pane
        session
            .tb_command()
            .args(["done", "--target", session.target(), &task_id])
            .assert()
            .success();
    }
}

mod done_errors {
    use super::*;

    #[test]
    fn fails_for_nonexistent_task() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["done", "--target", session.target(), "t999"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("No task")));
    }

    #[test]
    fn fails_without_target() {
        tb_cmd()
            .args(["done", "t1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No target specified"));
    }

    #[test]
    fn fails_for_already_closed_task() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        // Close it
        session
            .tb_command()
            .args(["done", "--target", session.target(), &task_id])
            .assert()
            .success();

        // Try to close again
        session
            .tb_command()
            .args(["done", "--target", session.target(), &task_id])
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

        let first = session.launch_task(&["sleep", "60"]);
        session.launch_task(&["sleep", "60"]);

        assert_eq!(session.wait_for_pane_count(3), 3);

        // Close one
        session
            .tb_command()
            .args(["done", "--target", session.target(), &first])
            .assert()
            .success();

        assert_eq!(session.wait_for_pane_count(2), 2);

        // Should now be able to launch a new one
        session
            .tb_command()
            .args(["launch", "--target", session.target(), "--", "sleep", "60"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Task t1 started."));

        assert_eq!(session.wait_for_pane_count(3), 3);
    }
}
