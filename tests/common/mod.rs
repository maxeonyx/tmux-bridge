//! Shared test utilities for E2E tests
//!
//! Provides session management with automatic cleanup via Drop.
//!
//! Tests use `TB_TEST_MODE=1` which makes tb use "tbtest-" prefix
//! instead of "tb-", avoiding interference with real sessions.

use assert_cmd::Command;
use std::process::Command as StdCommand;

/// The prefix used for test sessions (matches TB_TEST_MODE behavior)
const TEST_SESSION_PREFIX: &str = "tbtest-";

/// Create a tb command with TB_TEST_MODE=1 set
pub fn tb_cmd() -> Command {
    let mut cmd = Command::cargo_bin("tb").unwrap();
    cmd.env("TB_TEST_MODE", "1");
    cmd
}

/// A test session that automatically cleans up when dropped.
///
/// Use this instead of manually managing session cleanup:
/// ```
/// let session = TestSession::new();  // Creates session directly
/// // ... run tests using session.id ...
/// // Session is automatically killed when `session` goes out of scope
/// ```
pub struct TestSession {
    pub id: String,
}

impl TestSession {
    /// Start a new session directly via tmux and return a handle.
    /// Cleans up any existing test sessions first.
    ///
    /// Note: We create sessions directly with tmux rather than `tb start`
    /// because `tb start` requires an interactive terminal.
    pub fn new() -> Self {
        cleanup_all_test_sessions();

        // Generate a simple test session ID
        let id = format!("test{}", std::process::id() % 1000);
        let tmux_name = format!("{}{}", TEST_SESSION_PREFIX, id);

        // Create session directly with tmux
        let status = StdCommand::new("tmux")
            .args(["new-session", "-d", "-s", &tmux_name])
            .status()
            .expect("Failed to create tmux session");

        if !status.success() {
            panic!("Failed to create tmux session {}", tmux_name);
        }

        TestSession { id }
    }

    /// Get the full tmux session name (tbtest-{id})
    pub fn tmux_name(&self) -> String {
        format!("{}{}", TEST_SESSION_PREFIX, self.id)
    }

    /// Launch a task and return its task ID
    pub fn launch_task(&self, command: &[&str]) -> String {
        let output = Command::cargo_bin("tb")
            .unwrap()
            .env("TB_TEST_MODE", "1")
            .env("TB_SESSION", &self.id)
            .args(["launch", "--"])
            .args(command)
            .output()
            .expect("Failed to run tb launch");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract task ID from "Task t1 started"
        extract_task_id(&stdout).expect(&format!("Could not extract task ID from: {}", stdout))
    }

    /// Run a tb command with this session
    pub fn tb_command(&self) -> Command {
        let mut cmd = Command::cargo_bin("tb").unwrap();
        cmd.env("TB_TEST_MODE", "1").env("TB_SESSION", &self.id);
        cmd
    }

    /// Count panes in this session
    pub fn count_panes(&self) -> usize {
        let output = StdCommand::new("tmux")
            .args(["list-panes", "-t", &self.tmux_name(), "-F", "#{pane_id}"])
            .output()
            .expect("Failed to list panes");

        String::from_utf8_lossy(&output.stdout).lines().count()
    }

    /// Check if the session still exists
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
        // Kill this specific session
        let _ = StdCommand::new("tmux")
            .args(["kill-session", "-t", &self.tmux_name()])
            .output();
    }
}

/// Extract task ID from tb launch output
fn extract_task_id(output: &str) -> Option<String> {
    // Format: "Task t1 started"
    let start = output.find("Task ")?;
    let rest = &output[start + 5..];
    let end = rest.find(" ")?;
    Some(rest[..end].to_string())
}

/// Kill all test sessions (tbtest-*) and test runner sessions (for test isolation)
pub fn cleanup_all_test_sessions() {
    // Kill the test runner session if it exists
    let _ = StdCommand::new("tmux")
        .args(["kill-session", "-t", "tb-test-runner"])
        .output();

    if let Ok(output) = StdCommand::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
    {
        let sessions = String::from_utf8_lossy(&output.stdout);
        for session in sessions.lines() {
            // Only clean up test sessions, not real tb-* sessions
            if session.starts_with(TEST_SESSION_PREFIX) {
                let _ = StdCommand::new("tmux")
                    .args(["kill-session", "-t", session])
                    .output();
            }
        }
    }
}

/// Legacy alias for cleanup_all_test_sessions (used in start.rs tests)
pub fn cleanup_all_tb_sessions() {
    cleanup_all_test_sessions();
}

/// Check if a specific test session exists
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

/// Check if a specific session exists (checks tb- prefix for start.rs tests)
pub fn session_exists(session_id: &str) -> bool {
    // First check tbtest- (test mode)
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

    // Fall back to tb- (for testing non-test-mode behavior)
    StdCommand::new("tmux")
        .args(["has-session", "-t", &format!("tb-{}", session_id)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Cleanup a specific session (tries both prefixes)
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
