//! Shared test utilities for E2E tests
//!
//! Provides session management with automatic cleanup via Drop.

use assert_cmd::Command;
use std::process::Command as StdCommand;

/// A test session that automatically cleans up when dropped.
///
/// Use this instead of manually managing session cleanup:
/// ```
/// let session = TestSession::new();  // Creates session via tb start
/// // ... run tests using session.id ...
/// // Session is automatically killed when `session` goes out of scope
/// ```
pub struct TestSession {
    pub id: String,
}

impl TestSession {
    /// Start a new session via `tb start` and return a handle.
    /// Cleans up any existing tb-* sessions first.
    pub fn new() -> Self {
        cleanup_all_tb_sessions();

        let output = Command::cargo_bin("tb")
            .unwrap()
            .arg("start")
            .output()
            .expect("Failed to run tb start");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract session ID from "Started session 'xyz'"
        let id = extract_session_id(&stdout)
            .expect(&format!("Could not extract session ID from: {}", stdout));

        TestSession { id }
    }

    /// Get the full tmux session name (tb-{id})
    pub fn tmux_name(&self) -> String {
        format!("tb-{}", self.id)
    }

    /// Launch a task and return its task ID
    pub fn launch_task(&self, command: &[&str]) -> String {
        let output = Command::cargo_bin("tb")
            .unwrap()
            .env("TB_SESSION", &self.id)
            .args(["launch", "--"])
            .args(command)
            .output()
            .expect("Failed to run tb launch");

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract task ID from "Task t1 started"
        extract_task_id(&stdout).expect(&format!("Could not extract task ID from: {}", stdout))
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

/// Extract session ID from tb start output
fn extract_session_id(output: &str) -> Option<String> {
    // Format: "Started session 'xyz'"
    let start = output.find("'")?;
    let end = output[start + 1..].find("'")?;
    Some(output[start + 1..start + 1 + end].to_string())
}

/// Extract task ID from tb launch output
fn extract_task_id(output: &str) -> Option<String> {
    // Format: "Task t1 started"
    let start = output.find("Task ")?;
    let rest = &output[start + 5..];
    let end = rest.find(" ")?;
    Some(rest[..end].to_string())
}

/// Kill all tb-* sessions (for test isolation)
pub fn cleanup_all_tb_sessions() {
    if let Ok(output) = StdCommand::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
    {
        let sessions = String::from_utf8_lossy(&output.stdout);
        for session in sessions.lines() {
            if session.starts_with("tb-") {
                let _ = StdCommand::new("tmux")
                    .args(["kill-session", "-t", session])
                    .output();
            }
        }
    }
}

/// Check if a specific session exists
pub fn session_exists(session_id: &str) -> bool {
    StdCommand::new("tmux")
        .args(["has-session", "-t", &format!("tb-{}", session_id)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Cleanup a specific session
pub fn cleanup_session(session_id: &str) {
    let _ = StdCommand::new("tmux")
        .args(["kill-session", "-t", &format!("tb-{}", session_id)])
        .output();
}
