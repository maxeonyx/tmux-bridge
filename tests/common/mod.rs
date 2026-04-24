//! Shared test utilities for E2E tests.
//!
//! Each test gets its own tmux prefix and session names so parallel test runs do
//! not interfere with each other.

use assert_cmd::Command;
use rand::Rng;
use std::process::Command as StdCommand;
use std::process::Output;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const TEST_SESSION_PREFIX: &str = "tbtest-";
const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);
static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

// Generic polling primitives.

struct WaitStatus<T> {
    value: Option<T>,
    observed: String,
}

impl<T> WaitStatus<T> {
    fn ready(value: T, observed: impl Into<String>) -> Self {
        Self {
            value: Some(value),
            observed: observed.into(),
        }
    }

    fn pending(observed: impl Into<String>) -> Self {
        Self {
            value: None,
            observed: observed.into(),
        }
    }
}

pub fn wait_until<T, F>(
    description: &str,
    timeout: Duration,
    poll_interval: Duration,
    mut probe: F,
) -> T
where
    F: FnMut() -> WaitStatus<T>,
{
    let deadline = Instant::now() + timeout;
    let mut last_observed = String::from("<nothing observed>");

    loop {
        let status = probe();
        last_observed = status.observed;

        if let Some(value) = status.value {
            return value;
        }

        if Instant::now() >= deadline {
            panic!(
                "Timed out waiting for {} after {:?}\nlast observed:\n{}",
                description, timeout, last_observed
            );
        }

        thread::sleep(poll_interval);
    }
}

// Low-level tmux helpers.

fn run_tmux_output(args: &[&str], description: &str) -> Output {
    StdCommand::new("tmux")
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("Failed to {}: {}", description, error))
}

pub fn capture_pane_content(target: &str) -> String {
    let output = run_tmux_output(&["capture-pane", "-t", target, "-p"], "capture tmux pane");

    if !output.status.success() {
        panic!(
            "Failed to capture tmux pane {}\nstdout:\n{}\nstderr:\n{}",
            target,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn current_command(target: &str) -> String {
    let output = run_tmux_output(
        &[
            "display-message",
            "-p",
            "-t",
            target,
            "#{pane_current_command}",
        ],
        "inspect current tmux pane command",
    );

    if !output.status.success() {
        panic!(
            "Failed to inspect current command for {}\nstdout:\n{}\nstderr:\n{}",
            target,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub fn wait_for_current_command(target: &str, expected: &str, timeout: Duration) -> String {
    let description = format!("current command {} for {}", expected, target);

    wait_until(&description, timeout, DEFAULT_POLL_INTERVAL, || {
        let command = current_command(target);
        if command == expected {
            WaitStatus::ready(command.clone(), command)
        } else {
            WaitStatus::pending(command)
        }
    })
}

pub fn wait_for_pane_content<F>(
    target: &str,
    description: &str,
    timeout: Duration,
    mut predicate: F,
) -> String
where
    F: FnMut(&str) -> bool,
{
    wait_until(description, timeout, DEFAULT_POLL_INTERVAL, || {
        let content = capture_pane_content(target);
        if predicate(&content) {
            WaitStatus::ready(content.clone(), content)
        } else {
            WaitStatus::pending(content)
        }
    })
}

fn last_nonempty_line(content: &str) -> Option<&str> {
    content.lines().rev().find(|line| !line.trim().is_empty())
}

fn prompt_char_for_shell(shell: &str) -> Option<char> {
    match shell {
        "fish" => Some('>'),
        "bash" | "sh" => Some('$'),
        _ => None,
    }
}

fn random_test_marker() -> String {
    format!("__TB_TEST_READY_{}__", unique_token())
}

fn pane_snapshot(session_name: &str) -> String {
    let output = run_tmux_output(
        &["list-panes", "-t", session_name, "-F", "#{pane_id}"],
        "list tmux panes",
    );

    if !output.status.success() {
        panic!(
            "Failed to list panes for {}\nstdout:\n{}\nstderr:\n{}",
            session_name,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn has_tmux_session(session_name: &str) -> bool {
    StdCommand::new("tmux")
        .args(["has-session", "-t", session_name])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn wait_for_pane_count(session_name: &str, expected: usize) -> usize {
    let description = format!("pane count {} for tmux session {}", expected, session_name);

    wait_until(
        &description,
        DEFAULT_WAIT_TIMEOUT,
        DEFAULT_POLL_INTERVAL,
        || {
            let snapshot = pane_snapshot(session_name);
            let count = snapshot.lines().count();
            let observed = format!("observed {} panes\n{}", count, snapshot);

            if count == expected {
                WaitStatus::ready(count, observed)
            } else {
                WaitStatus::pending(observed)
            }
        },
    )
}

pub fn wait_for_session_exists(session_name: &str, timeout: Duration) {
    let description = format!("tmux session {} to exist", session_name);

    wait_until(&description, timeout, DEFAULT_POLL_INTERVAL, || {
        let output = run_tmux_output(&["has-session", "-t", session_name], "check tmux session");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let observed = format!(
            "tmux has-session exit status: {}\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        );

        if output.status.success() {
            WaitStatus::ready((), observed)
        } else {
            WaitStatus::pending(observed)
        }
    });
}

// Session naming helpers.

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
        Self::new_with_startup_command(None)
    }

    pub fn new_with_startup_command(command: Option<&str>) -> Self {
        let prefix = unique_prefix();
        let id = unique_session_id();
        let tmux_name = format!("{}{}", prefix, id);

        let mut command_builder = StdCommand::new("tmux");
        command_builder.args([
            "new-session",
            "-d",
            "-x",
            "200",
            "-y",
            "60",
            "-s",
            &tmux_name,
        ]);

        if let Some(command) = command {
            command_builder.arg(command);
        }

        let status = command_builder
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

    pub fn target(&self) -> &str {
        &self.id
    }

    pub fn pane_target(&self) -> String {
        let output = StdCommand::new("tmux")
            .args([
                "display-message",
                "-p",
                "-t",
                &self.tmux_name(),
                "#{session_name}:#{window_index}.#{pane_index}",
            ])
            .output()
            .expect("Failed to inspect tmux pane target");

        assert!(
            output.status.success(),
            "Failed to inspect tmux pane target"
        );

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    pub fn wait_for_current_command(&self, expected: &str, timeout: Duration) -> String {
        wait_for_current_command(&self.tmux_name(), expected, timeout)
    }

    pub fn enter_shell(&self, shell: &str) {
        self.send_main_pane_command(shell);
        self.wait_for_current_command(shell, Duration::from_secs(10));

        if let Some(prompt_char) = prompt_char_for_shell(shell) {
            wait_for_pane_content(
                &self.tmux_name(),
                "shell prompt",
                Duration::from_secs(10),
                |content| {
                    last_nonempty_line(content)
                        .map(|line| line.ends_with(prompt_char))
                        .unwrap_or(false)
                },
            );
        }

        let marker = random_test_marker();
        self.send_main_pane_command(&format!("printf '%s\\n' {}", marker));
        wait_for_pane_content(
            &self.tmux_name(),
            "shell ready marker",
            Duration::from_secs(10),
            |content| content.lines().any(|line| line.trim() == marker),
        );
    }

    pub fn wait_for_shell_ready(&self) {
        let marker = random_test_marker();
        self.send_main_pane_command(&format!("printf '%s\\n' {}", marker));
        wait_for_pane_content(
            &self.tmux_name(),
            "shell ready marker",
            Duration::from_secs(10),
            |content| content.lines().any(|line| line.trim() == marker),
        );
    }

    pub fn wait_for_check_command_output<F>(
        &self,
        task_id: &str,
        extra_args: &[&str],
        mut predicate: F,
    ) -> String
    where
        F: FnMut(&str) -> bool,
    {
        let description = format!(
            "tb check {} output for task {} in session {}",
            extra_args.join(" "),
            task_id,
            self.tmux_name()
        );

        wait_until(
            &description,
            DEFAULT_WAIT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            || {
                let output = self
                    .tb_command()
                    .args(["check", "--target", self.target(), task_id])
                    .args(extra_args)
                    .output()
                    .expect("Failed to run tb check while polling");

                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let observed = format!("stdout:\n{}\nstderr:\n{}", stdout, stderr);

                if !output.status.success() {
                    panic!(
                        "tb check failed while waiting for task {}\n{}",
                        task_id, observed
                    );
                }

                if predicate(&stdout) {
                    WaitStatus::ready(stdout.clone(), observed)
                } else {
                    WaitStatus::pending(observed)
                }
            },
        )
    }

    /// Launch a task and return its task ID.
    pub fn launch_task(&self, command: &[&str]) -> String {
        let output = Command::cargo_bin("tb")
            .unwrap()
            .env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &self.prefix)
            .args(["launch", "--target", &self.id, "--"])
            .args(command)
            .output()
            .expect("Failed to run tb launch");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "tb launch failed\nstdout:\n{}\nstderr:\n{}",
            stdout,
            stderr
        );

        extract_task_id(&stdout).expect(&format!("Could not extract task ID from: {}", stdout))
    }

    /// Run a tb command with this session.
    pub fn tb_command(&self) -> Command {
        let mut cmd = Command::cargo_bin("tb").unwrap();
        cmd.env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &self.prefix);
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
        self.check_output(None)
    }

    fn check_output(&self, task_id: Option<&str>) -> std::process::Output {
        let mut command = Command::cargo_bin("tb").unwrap();
        command
            .env("TB_TEST_MODE", "1")
            .env("TB_SESSION_PREFIX", &self.prefix)
            .arg("check")
            .args(["--target", &self.id]);

        if let Some(task_id) = task_id {
            command.arg(task_id);
        }

        command.output().expect("Failed to run tb check")
    }

    /// Poll `tb check` until its stdout matches the predicate or timeout expires.
    pub fn wait_for_check_output<F>(&self, task_id: &str, mut predicate: F) -> String
    where
        F: FnMut(&str) -> bool,
    {
        let description = format!(
            "tb check output for task {} in session {}",
            task_id,
            self.tmux_name()
        );

        wait_until(
            &description,
            DEFAULT_WAIT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            || {
                let output = self.check_output(Some(task_id));

                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let observed = format!("stdout:\n{}\nstderr:\n{}", stdout, stderr);

                if !output.status.success() {
                    panic!(
                        "tb check failed while waiting for task {}\nstdout:\n{}\nstderr:\n{}",
                        task_id, stdout, stderr
                    );
                }

                if predicate(&stdout) {
                    WaitStatus::ready(stdout.clone(), observed)
                } else {
                    WaitStatus::pending(observed)
                }
            },
        )
    }

    /// Poll `tb check` without a task ID until its stdout matches the predicate.
    pub fn wait_for_main_check_output<F>(&self, mut predicate: F) -> String
    where
        F: FnMut(&str) -> bool,
    {
        let description = format!("tb check main pane output for session {}", self.tmux_name());

        wait_until(
            &description,
            DEFAULT_WAIT_TIMEOUT,
            DEFAULT_POLL_INTERVAL,
            || {
                let output = self.check_main_output();

                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let observed = format!("stdout:\n{}\nstderr:\n{}", stdout, stderr);

                if !output.status.success() {
                    panic!(
                        "tb check failed while waiting for main pane output\nstdout:\n{}\nstderr:\n{}",
                        stdout, stderr
                    );
                }

                if predicate(&stdout) {
                    WaitStatus::ready(stdout.clone(), observed)
                } else {
                    WaitStatus::pending(observed)
                }
            },
        )
    }

    /// Count panes in this session.
    fn count_panes(&self) -> usize {
        pane_snapshot(&self.tmux_name()).lines().count()
    }

    pub fn wait_for_pane_count(&self, expected: usize) -> usize {
        wait_for_pane_count(&self.tmux_name(), expected)
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

// Convenience helpers for tests that work with raw session ids.

/// Check if a specific session exists (checks tbtest- prefix for start.rs tests).
pub fn session_exists(session_id: &str) -> bool {
    let tbtest = has_tmux_session(&format!("{}{}", TEST_SESSION_PREFIX, session_id));

    if tbtest {
        return true;
    }

    has_tmux_session(&format!("tb-{}", session_id))
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
