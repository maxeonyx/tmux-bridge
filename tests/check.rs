//! End-to-end black-box tests for `tb check`
//!
//! These tests verify checking task panes and the main session pane.

mod common;

use common::{TestSession, tb_cmd};
use predicates::prelude::*;
use std::process::Command as StdCommand;

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
            .args(["check", "--target", session.target(), &task_id])
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
            .args(["check", "--target", session.target(), &task_id])
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
            .args(["check", "--target", session.target(), &task_id])
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
            .args(["check", "--target", session.target(), &task_id])
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

    #[test]
    fn captures_exact_target_pane_without_task_id() {
        let session = TestSession::new();

        let split = StdCommand::new("tmux")
            .args([
                "split-window",
                "-t",
                &session.tmux_name(),
                "-d",
                "-P",
                "-F",
                "#{window_index}.#{pane_index}:#{pane_id}",
            ])
            .output()
            .expect("Failed to split tmux pane");
        assert!(split.status.success(), "Failed to split tmux pane");

        let split_stdout = String::from_utf8_lossy(&split.stdout).trim().to_string();
        let (window_and_pane, pane_id) = split_stdout
            .split_once(':')
            .expect("tmux split output should include pane target and id");
        let pane_target = format!("{}:{}", session.tmux_name(), window_and_pane);

        session.send_main_pane_command("echo original pane marker");

        let send_split = StdCommand::new("tmux")
            .args([
                "send-keys",
                "-t",
                &pane_id,
                "echo split pane marker",
                "Enter",
            ])
            .status()
            .expect("Failed to send keys to split pane");
        assert!(send_split.success(), "Failed to send keys to split pane");

        common::wait_for_pane_content(
            &pane_target,
            "split pane content",
            std::time::Duration::from_secs(10),
            |content| content.contains("split pane marker"),
        );

        common::wait_for_pane_content(
            &session.tmux_name(),
            "original pane content",
            std::time::Duration::from_secs(10),
            |content| content.contains("original pane marker"),
        );

        tb_cmd()
            .args(["check", "-t", &pane_target])
            .assert()
            .success()
            .stdout(predicate::str::contains("split pane marker"))
            .stdout(predicate::str::contains("original pane marker").not());
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
            .args([
                "check",
                "--target",
                session.target(),
                "--first",
                "5",
                "--last",
                "5",
            ])
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
            .args(["check", "--target", session.target(), "t999"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found").or(predicate::str::contains("No task")));
    }

    #[test]
    fn fails_without_target() {
        tb_cmd()
            .args(["check", "t1"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No target specified"));
    }
}
