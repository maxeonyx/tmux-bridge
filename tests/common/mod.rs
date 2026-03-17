//! Shared test utilities for E2E tests.
//!
//! Each test gets its own tmux prefix and session names so parallel test runs do
//! not interfere with each other.

use assert_cmd::Command;
use rand::Rng;
use std::process::Command as StdCommand;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const TEST_SESSION_PREFIX: &str = "tbtest-";
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn unique_token() -> String {
    let counter = UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut rng = rand::thread_rng();
    format!(
        "{}-{}-{:08x}",
        std::process::id(),
        counter,
        rng.r#gen::<u32>()
    )
}

fn unique_prefix() -> String {
    format!("{}{}-", TEST_SESSION_PREFIX, unique_token())
}

fn unique_session_id() -> String {
    format!("test-{}", unique_token())
}

/// Create a tb command configured for isolated test mode.
pub fn tb_cmd() -> Command {
    let mut cmd = Command::cargo_bin("tb").unwrap();
    cmd.env("TB_TEST_MODE", "1");
    cmd
}

/// A test session that automatically cleans up when dropped.
pub struct TestSession {
    pub id: String,
    prefix: String,
}

impl TestSession {
    /// Start a new isolated session directly via tmux and return a handle.
    pub fn new() -> Self {
        let prefix = unique_prefix();
        let id = unique_session_id();
        let tmux_name = format!("{}{}", prefix, id);

        let status = StdCommand::new("tmux")
            .args(["new-session", "-d", "-s", &tmux_name])
            .status()
            .expect("Failed to create tmux session");

        if !status.success() {
            panic!("Failed to create tmux session {}", tmux_name);
        }

        TestSession { id, prefix }
    }

    /// Get the full tmux session name for this isolated test session.
    pub fn tmux_name(&self) -> String {
        format!("{}{}", self.prefix, self.id)
    }

    pub fn session_prefix(&self) -> &str {
        &self.prefix
    }

    /// Launch a task and return its task ID.
    pub fn launch_task(&self, command: &[&str]) -> String {
        let output = Command::cargo_bin("tb")
            .unwrap()
            .env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &self.prefix)
            .env("TB_SESSION", &self.id)
            .args(["launch", "--"])
            .args(command)
            .output()
            .expect("Failed to run tb launch");

        let stdout = String::from_utf8_lossy(&output.stdout);

        extract_task_id(&stdout).expect(&format!("Could not extract task ID from: {}", stdout))
    }

    /// Run a tb command with this session.
    pub fn tb_command(&self) -> Command {
        let mut cmd = Command::cargo_bin("tb").unwrap();
        cmd.env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &self.prefix)
            .env("TB_SESSION", &self.id);
        cmd
    }

    /// Send a command to the main pane in this session.
    pub fn send_main_pane_command(&self, command: &str) {
        let status = StdCommand::new("tmux")
            .args(["send-keys", "-t", &self.tmux_name(), command, "Enter"])
            .status()
            .expect("Failed to send command to main pane");

        if !status.success() {
            panic!(
                "Failed to send command to main pane for {}",
                self.tmux_name()
            );
        }
    }

    /// Run `tb check` without a task ID and return the output.
    pub fn check_main_output(&self) -> std::process::Output {
        Command::cargo_bin("tb")
            .unwrap()
            .env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &self.prefix)
            .env("TB_SESSION", &self.id)
            .arg("check")
            .output()
            .expect("Failed to run tb check")
    }

    /// Poll `tb check` until its stdout matches the predicate or timeout expires.
    pub fn wait_for_check_output<F>(&self, task_id: &str, mut predicate: F) -> String
    where
        F: FnMut(&str) -> bool,
    {
        let deadline = Instant::now() + Duration::from_secs(10);
        let poll_interval = Duration::from_millis(100);
        loop {
            let output = Command::cargo_bin("tb")
                .unwrap()
                .env("TB_TEST_MODE", "1")
                .env("TB_SESSION_PREFIX", &self.prefix)
                .env("TB_SESSION", &self.id)
                .args(["check", task_id])
                .output()
                .expect("Failed to run tb check");

            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

            if !output.status.success() {
                panic!(
                    "tb check failed while waiting for task {}\nstdout:\n{}\nstderr:\n{}",
                    task_id, stdout, stderr
                );
            }

            if predicate(&stdout) {
                return stdout;
            }

            if Instant::now() >= deadline {
                panic!(
                    "Timed out waiting for tb check output for task {}\nlast stdout:\n{}\nlast stderr:\n{}",
                    task_id, stdout, stderr
                );
            }

            thread::sleep(poll_interval);
        }
    }

    /// Poll `tb check` without a task ID until its stdout matches the predicate.
    pub fn wait_for_main_check_output<F>(&self, mut predicate: F) -> String
    where
        F: FnMut(&str) -> bool,
    {
        let deadline = Instant::now() + Duration::from_secs(10);
        let poll_interval = Duration::from_millis(100);
        loop {
            let output = self.check_main_output();

            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

            if !output.status.success() {
                panic!(
                    "tb check failed while waiting for main pane output\nstdout:\n{}\nstderr:\n{}",
                    stdout, stderr
                );
            }

            if predicate(&stdout) {
                return stdout;
            }

            if Instant::now() >= deadline {
                panic!(
                    "Timed out waiting for tb check main pane output\nlast stdout:\n{}\nlast stderr:\n{}",
                    stdout, stderr
                );
            }

            thread::sleep(poll_interval);
        }
    }

    /// Count panes in this session.
    pub fn count_panes(&self) -> usize {
        let output = StdCommand::new("tmux")
            .args(["list-panes", "-t", &self.tmux_name(), "-F", "#{pane_id}"])
            .output()
            .expect("Failed to list panes");

        String::from_utf8_lossy(&output.stdout).lines().count()
    }

    /// Check if the session still exists.
    pub fn exists(&self) -> bool {
        StdCommand::new("tmux")
            .args(["has-session", "-t", &self.tmux_name()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        let _ = StdCommand::new("tmux")
            .args(["kill-session", "-t", &self.tmux_name()])
            .output();
    }
}

fn extract_task_id(output: &str) -> Option<String> {
    let start = output.find("Task ")?;
    let rest = &output[start + 5..];
    let end = rest.find(' ')?;
    Some(rest[..end].to_string())
}

/// Legacy helper retained for tests that want a best-effort cleanup boundary.
pub fn cleanup_all_test_sessions() {
    // Intentionally a no-op. Global cleanup races with parallel tests.
}

/// Legacy alias for cleanup_all_test_sessions (used in start.rs tests).
pub fn cleanup_all_tb_sessions() {
    cleanup_all_test_sessions();
}

/// Check if a specific test session exists with the default test prefix.
pub fn test_session_exists(session_id: &str) -> bool {
    StdCommand::new("tmux")
        .args([
            "has-session",
            "-t",
            &format!("{}{}", TEST_SESSION_PREFIX, session_id),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a specific session exists (checks tbtest- prefix for start.rs tests).
pub fn session_exists(session_id: &str) -> bool {
    let tbtest = StdCommand::new("tmux")
        .args([
            "has-session",
            "-t",
            &format!("{}{}", TEST_SESSION_PREFIX, session_id),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if tbtest {
        return true;
    }

    StdCommand::new("tmux")
        .args(["has-session", "-t", &format!("tb-{}", session_id)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Cleanup a specific session (tries both prefixes).
pub fn cleanup_session(session_id: &str) {
    let _ = StdCommand::new("tmux")
        .args([
            "kill-session",
            "-t",
            &format!("{}{}", TEST_SESSION_PREFIX, session_id),
        ])
        .output();
    let _ = StdCommand::new("tmux")
        .args(["kill-session", "-t", &format!("tb-{}", session_id)])
        .output();
}
