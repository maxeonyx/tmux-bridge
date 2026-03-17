//! End-to-end black-box tests for `tb check`
//!
//! These tests verify checking task panes and the main session pane.

mod common;

use common::{TestSession, tb_cmd};
use predicates::prelude::*;

mod check_output {
    use super::*;

    #[test]
    fn shows_task_output() {
        let session = TestSession::new();

        // Launch a task that outputs something
        let task_id = session.launch_task(&["sh", "-c", "echo 'task output here'; sleep 60"]);

        session.wait_for_check_output(&task_id, |stdout| stdout.contains("task output here"));

        session
            .tb_command()
            .args(["check", &task_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("task output here"));
    }

    #[test]
    fn shows_running_status_for_active_task() {
        let session = TestSession::new();

        let task_id = session.launch_task(&["sleep", "60"]);

        session.wait_for_check_output(&task_id, |stdout| !stdout.contains("complete"));

        session
            .tb_command()
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

        session.wait_for_check_output(&task_id, |stdout| {
            (stdout.contains("complete") || stdout.contains("finished"))
                && stdout.contains("tb done")
        });

        session
            .tb_command()
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

        session.wait_for_check_output(&task_id, |stdout| {
            (stdout.contains("complete") || stdout.contains("finished"))
                && (stdout.contains("42") || stdout.contains("exit"))
        });

        session
            .tb_command()
            .args(["check", &task_id])
            .assert()
            .success()
            .stdout(predicate::str::contains("42").or(predicate::str::contains("exit")));
    }
}

mod check_main_output {
    use super::*;

    #[test]
    fn shows_main_pane_output_without_task_id() {
        let session = TestSession::new();

        session.send_main_pane_command("echo main pane output here");

        session.wait_for_main_check_output(|stdout| stdout.contains("main pane output here"));

        let output = session.check_main_output();
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "tb check failed: {}", stdout);
        assert!(stdout.contains("main pane output here"));
    }

    #[test]
    fn captures_main_pane_even_when_task_panes_exist() {
        let session = TestSession::new();

        session.launch_task(&["sh", "-c", "echo task pane output; sleep 60"]);
        session.send_main_pane_command("echo main pane stays visible");

        session.wait_for_main_check_output(|stdout| stdout.contains("main pane stays visible"));

        let output = session.check_main_output();
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "tb check failed: {}", stdout);
        assert!(stdout.contains("main pane stays visible"));
        assert!(!stdout.contains("task pane output"));
    }

    #[test]
    fn does_not_show_task_lifecycle_messages_for_main_pane() {
        let session = TestSession::new();

        session.send_main_pane_command("echo plain main pane snapshot");

        session.wait_for_main_check_output(|stdout| stdout.contains("plain main pane snapshot"));

        let output = session.check_main_output();
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "tb check failed: {}", stdout);
        assert!(!stdout.contains("appears complete"));
        assert!(!stdout.contains("finished with exit code"));
        assert!(!stdout.contains("Close pane with: tb done"));
    }
}

mod check_truncation {
    use super::*;

    #[test]
    fn respects_first_and_last_flags() {
        let session = TestSession::new();

        // Launch task that outputs many lines
        let task_id = session.launch_task(&["sh", "-c", "seq 1 200; echo READY; sleep 60"]);

        session.wait_for_check_output(&task_id, |stdout| stdout.contains("READY"));

        let stdout = session.wait_for_check_command_output(
            &task_id,
            &["--first", "5", "--last", "5"],
            |stdout| stdout.contains("truncated"),
        );

        assert!(
            stdout.contains("truncated"),
            "Expected truncation output: {}",
            stdout
        );
    }
}

mod check_main_truncation {
    use super::*;

    #[test]
    fn respects_first_and_last_flags_for_main_pane() {
        let session = TestSession::new();

        session.send_main_pane_command("seq 1 200");

        session.wait_for_main_check_output(|stdout| stdout.contains("200"));

        session
            .tb_command()
            .args(["check", "--first", "5", "--last", "5"])
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

        session
            .tb_command()
            .args(["check", "t999"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("No task")));
    }

    #[test]
    fn fails_without_session() {
        tb_cmd()
            .args(["check", "t1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No session specified"));
    }
}
