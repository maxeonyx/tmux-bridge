//! tb - tmux bridge for AI agents
//!
//! A CLI tool that allows AI agents to inject commands into interactive
//! terminal sessions controlled by humans.

use clap::{Parser, Subcommand};
use rand::Rng;
use std::collections::HashSet;
use std::process::Command;

/// Returns the tmux session prefix for this process.
///
/// TB_SESSION_PREFIX is primarily for test isolation. TB_TEST_MODE keeps tests
/// away from real tb-* sessions when no explicit prefix override is set.
fn session_prefix() -> String {
    if let Ok(prefix) = std::env::var("TB_SESSION_PREFIX") {
        return prefix;
    }

    if std::env::var("TB_TEST_MODE").is_ok() {
        "tbtest-".to_string()
    } else {
        "tb-".to_string()
    }
}

/// Format a full tmux session name from a session ID.
/// Idempotent: if session_id already has the prefix, returns it unchanged.
fn tmux_session_name(session_id: &str) -> String {
    let prefix = session_prefix();
    if session_id.starts_with(&prefix) {
        session_id.to_string()
    } else {
        format!("{}{}", prefix, session_id)
    }
}

#[derive(Parser)]
#[command(name = "tb")]
#[command(about = "A tmux bridge for AI agents to run commands in interactive terminals")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new tmux-bridge session (human runs this)
    Start {
        /// Use a specific session ID instead of auto-generating
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Run a command synchronously and wait for output (agent uses this)
    Run {
        /// Use specific session (default: $TB_SESSION)
        #[arg(short, long)]
        session: Option<String>,

        /// Print the exact command sent to tmux and exit
        #[arg(long)]
        dry_run: bool,

        /// No-output timeout in seconds
        #[arg(long, default_value = "10")]
        timeout: u64,

        /// Overall timeout in seconds
        #[arg(long, default_value = "120")]
        max_time: u64,

        /// Lines to show from start of output
        #[arg(long, default_value = "50")]
        first: usize,

        /// Lines to show from end of output
        #[arg(long, default_value = "50")]
        last: usize,

        /// The command to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Launch a background task in a split pane (agent uses this)
    Launch {
        /// Use specific session (default: $TB_SESSION)
        #[arg(short, long)]
        session: Option<String>,

        /// The command to run
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Check a task's status or capture the main pane
    Check {
        /// Optional task ID (e.g., t1); omit to capture the main pane
        task: Option<String>,

        /// Use specific session (default: $TB_SESSION)
        #[arg(short, long)]
        session: Option<String>,

        /// Lines to show from start of output
        #[arg(long, default_value = "50")]
        first: usize,

        /// Lines to show from end of output
        #[arg(long, default_value = "50")]
        last: usize,
    },

    /// Close a background task's pane
    Done {
        /// The task ID (e.g., t1, t2)
        task: String,

        /// Use specific session (default: $TB_SESSION)
        #[arg(short, long)]
        session: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Start { session } => cmd_start(session),
        Commands::Run {
            session,
            dry_run,
            timeout,
            max_time,
            first,
            last,
            command,
        } => cmd_run(session, dry_run, timeout, max_time, first, last, command),
        Commands::Launch { session, command } => cmd_launch(session, command),
        Commands::Check {
            task,
            session,
            first,
            last,
        } => cmd_check(task, session, first, last),
        Commands::Done { task, session } => cmd_done(task, session),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn cmd_start(session: Option<String>) -> Result<(), String> {
    // tb start is for humans only - must be run interactively
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return Err("tb start must be run in an interactive terminal.\n\n\
             Ask the user to run: tb start"
            .to_string());
    }

    let session_id = match session {
        Some(explicit_id) => {
            // Check if session already exists
            if session_exists(&explicit_id) {
                return Err(format!("Session '{}' already exists.", explicit_id));
            }
            explicit_id
        }
        None => generate_session_id()?,
    };

    // Create the tmux session
    let tmux_name = tmux_session_name(&session_id);
    let status = Command::new("tmux")
        .args(["new-session", "-d", "-s", &tmux_name])
        .status()
        .map_err(|e| format!("Failed to run tmux: {}", e))?;

    if !status.success() {
        return Err("Failed to create tmux session.".to_string());
    }

    println!("Started session '{}'", session_id);
    println!();
    println!("Tell your agent: export TB_SESSION={}", session_id);
    println!();

    use std::io::Write;
    let _ = std::io::stdout().flush();

    #[cfg(unix)]
    {
        // exec replaces this process with tmux attach
        use std::os::unix::process::CommandExt;
        let err = Command::new("tmux")
            .args(["attach-session", "-t", &tmux_name])
            .exec();
        Err(format!("Failed to attach to session: {}", err))
    }

    #[cfg(not(unix))]
    {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", &tmux_name])
            .status()
            .map_err(|e| format!("Failed to run tmux: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to attach to session.".to_string())
        }
    }
}

/// Check if a session with the given ID already exists
fn session_exists(session_id: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", &tmux_session_name(session_id)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Generate a session ID with format: {first-free-letter}{random}{random}
fn generate_session_id() -> Result<String, String> {
    let prefix = session_prefix();

    // Get list of existing sessions with our prefix
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
        .map_err(|e| format!("Failed to list tmux sessions: {}", e))?;

    // Extract used first letters from {prefix}{letter}** sessions
    let stdout = String::from_utf8_lossy(&output.stdout);
    let used_letters: HashSet<char> = stdout
        .lines()
        .filter_map(|line| {
            if line.starts_with(&prefix) && line.len() > prefix.len() {
                line.chars().nth(prefix.len())
            } else {
                None
            }
        })
        .collect();

    // Find first free letter
    let first_letter = ('a'..='z')
        .find(|c| !used_letters.contains(c))
        .ok_or_else(|| "All 26 session letters are in use.".to_string())?;

    // Generate 2 random alphanumeric chars
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let r1 = chars[rng.gen_range(0..chars.len())];
    let r2 = chars[rng.gen_range(0..chars.len())];

    Ok(format!("{}{}{}", first_letter, r1, r2))
}

fn cmd_run(
    session: Option<String>,
    dry_run: bool,
    timeout: u64,
    max_time: u64,
    first: usize,
    last: usize,
    command: Vec<String>,
) -> Result<(), String> {
    if dry_run {
        println!("{}", build_shell_command(&command, "dryrunid"));
        return Ok(());
    }

    let tmux_name = resolve_existing_session_name(session)?;

    // Generate unique marker ID
    let marker_id: String = {
        let mut rng = rand::thread_rng();
        (0..8)
            .map(|_| {
                let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
                chars[rng.gen_range(0..chars.len())] as char
            })
            .collect()
    };

    let start_marker = format!("___START_{}___", marker_id);
    let end_marker_prefix = format!("___END_{}_", marker_id);

    // Build the shell command to inject
    // We wrap the command with markers and capture exit status
    let shell_command = build_shell_command(&command, &marker_id);

    // Send the command to tmux
    let status = Command::new("tmux")
        .args(["send-keys", "-t", &tmux_name, &shell_command, "Enter"])
        .status()
        .map_err(|e| format!("Failed to send command to tmux: {}", e))?;

    if !status.success() {
        return Err("Failed to send command to tmux.".to_string());
    }

    // Poll for output with timeouts
    let start_time = std::time::Instant::now();
    let mut last_output_time = start_time;
    let mut last_output_len = 0;
    let poll_interval = std::time::Duration::from_millis(100);

    loop {
        std::thread::sleep(poll_interval);

        // Check max-time timeout
        if start_time.elapsed().as_secs() >= max_time {
            kill_running_command(&tmux_name);
            eprintln!("Timeout: max-time of {} seconds exceeded.", max_time);
            std::process::exit(124);
        }

        // Capture pane content
        let output = capture_pane_scrollback(&tmux_name)?;

        let pane_content = String::from_utf8_lossy(&output.stdout);

        // Check for end marker
        if let Some(exit_code) = find_exit_code(&pane_content, &end_marker_prefix) {
            // Extract output between markers
            let cmd_output = extract_output(&pane_content, &start_marker, &end_marker_prefix);

            // Truncate and print output
            print_output(&cmd_output, first, last);

            if exit_code != 0 {
                std::process::exit(exit_code);
            }
            return Ok(());
        }

        // Check for new output (for no-output timeout)
        if pane_content.len() != last_output_len {
            last_output_len = pane_content.len();
            last_output_time = std::time::Instant::now();
        }

        // Check no-output timeout
        if last_output_time.elapsed().as_secs() >= timeout {
            kill_running_command(&tmux_name);
            eprintln!("Timeout: no output for {} seconds.", timeout);
            std::process::exit(124);
        }
    }
}

/// Resolve session ID from --session flag or TB_SESSION env var
fn resolve_session(session: Option<String>) -> Result<String, String> {
    if let Some(s) = session {
        return Ok(s);
    }

    if let Ok(s) = std::env::var("TB_SESSION")
        && !s.is_empty()
    {
        return Ok(s);
    }

    Err("No session specified.\n\nSet TB_SESSION environment variable, or use --session ID.\nAsk the user which tmux-bridge session to use.".to_string())
}

fn resolve_existing_session_name(session: Option<String>) -> Result<String, String> {
    let session_id = resolve_session(session)?;

    if !session_exists(&session_id) {
        return Err(format!(
            "Session '{}' not found.\n\nStart a new session with: tb start",
            session_id
        ));
    }

    Ok(tmux_session_name(&session_id))
}

fn capture_pane_scrollback(pane_target: &str) -> Result<std::process::Output, String> {
    Command::new("tmux")
        .args([
            "capture-pane",
            "-t",
            pane_target,
            "-p",
            "-S",
            "-32768", // Capture full scrollback
        ])
        .output()
        .map_err(|e| format!("Failed to capture pane: {}", e))
}

/// Build the shell command with markers
fn build_shell_command(command: &[String], marker_id: &str) -> String {
    let cmd_str = shell_command_text(command);

    // Build the inner script that will run inside sh -c
    // This script: echoes start marker, runs command, echoes end marker with exit status.
    // Markers use only alphanumeric characters and underscores, so we keep them bare.
    let inner_script = format!(
        "echo ___START_{}___; {}; echo ___END_{}_$?___",
        marker_id, cmd_str, marker_id
    );

    // Wrap in single quotes for outer shell - prevents variable expansion
    // Single quotes in inner_script need escaping as '\''
    let escaped_script = inner_script.replace('\'', "'\\''");

    format!("sh -c '{}'", escaped_script)
}

fn shell_command_text(command: &[String]) -> String {
    match command {
        [script] => script.clone(),
        _ => command
            .iter()
            .map(|arg| quote_shell_arg(arg))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Quote one argv element for the inner `sh -c` script.
///
/// We prefer the least noisy form that still preserves the exact argument:
/// bare for shell-safe text, double quotes for whitespace/metacharacters,
/// single quotes for literal shell symbols, and double quotes with escaping
/// only when the argument itself contains a single quote.
fn quote_shell_arg(s: &str) -> String {
    if is_bare_shell_arg(s) {
        return s.to_string();
    }

    if s.contains('\'') {
        return format!("\"{}\"", escape_for_double_quotes(s));
    }

    if s.chars().any(is_single_quote_symbol) {
        return format!("'{}'", s);
    }

    format!("\"{}\"", s)
}

/// Bare arguments need no quoting in the inner shell script.
fn is_bare_shell_arg(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '-' | '_' | '.' | '/' | ',' | ':' | '@' | '=' | '+' | '%')
        })
}

/// These characters are easiest to preserve literally with single quotes,
/// as long as the argument does not itself contain a single quote.
fn is_single_quote_symbol(c: char) -> bool {
    matches!(c, '\\' | '$' | '`' | '"' | '!')
}

/// Escape the characters that still have meaning inside double quotes.
fn escape_for_double_quotes(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace('"', "\\\"")
        .replace('!', "\\!")
}

/// Find exit code from end marker in output
fn find_exit_code(content: &str, end_marker_prefix: &str) -> Option<i32> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(end_marker_prefix) {
            // Format: ___END_{id}_{exit_code}___
            if let Some(end) = rest.find("___")
                && let Ok(code) = rest[..end].parse::<i32>()
            {
                return Some(code);
            }
        }
    }
    None
}

/// Extract output between start and end markers
fn extract_output(content: &str, start_marker: &str, end_marker_prefix: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut start_idx = None;
    let mut end_idx = None;

    for (i, line) in lines.iter().enumerate() {
        // Match lines that start with the marker (not just contain it)
        if start_idx.is_none() && line.starts_with(start_marker) {
            start_idx = Some(i + 1); // Start after the marker line
        } else if start_idx.is_some() && line.starts_with(end_marker_prefix) {
            end_idx = Some(i);
            break;
        }
    }

    match (start_idx, end_idx) {
        (Some(s), Some(e)) if s < e => lines[s..e].join("\n"),
        _ => String::new(),
    }
}

/// Print output with truncation if needed
fn print_output(output: &str, first: usize, last: usize) {
    let lines: Vec<&str> = output.lines().collect();
    let total = lines.len();

    if total <= first + last {
        // No truncation needed
        println!("{}", output);
    } else {
        // Print first N lines
        for line in lines.iter().take(first) {
            println!("{}", line);
        }

        let truncated = total - first - last;
        println!("\n... ({} lines truncated) ...\n", truncated);

        // Print last N lines
        for line in lines.iter().skip(total - last) {
            println!("{}", line);
        }
    }
}

/// Kill running command in pane with SIGINT, then SIGQUIT
fn kill_running_command(tmux_name: &str) {
    // Send Ctrl+C (SIGINT)
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", tmux_name, "C-c"])
        .status();

    std::thread::sleep(std::time::Duration::from_secs(3));

    // Send Ctrl+\ (SIGQUIT) as backup
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", tmux_name, "C-\\"])
        .status();
}

fn cmd_launch(session: Option<String>, command: Vec<String>) -> Result<(), String> {
    let tmux_name = resolve_existing_session_name(session)?;

    // Count existing task panes to get next task ID
    let task_count = count_task_panes(&tmux_name);

    if task_count >= 6 {
        return Err(
            "Error: too many background tasks (max 6).\n\nClose a task with: tb done <task>"
                .to_string(),
        );
    }

    let task_id = format!("t{}", task_count + 1);

    // Create split pane for the task
    // First 3 tasks: horizontal split at top (above main pane)
    // Tasks 4-6: split existing task panes vertically
    let pane_target = if task_count < 3 {
        // Get the main pane ID (the last pane, which is always the original main pane)
        let pane_count_total = task_count + 1;
        let main_pane_index = pane_count_total - 1; // Last pane is main

        // Split main pane horizontally, creating new pane above
        // -b = before (above), -l 5 = 5 lines (small to fit more panes), -d = don't switch focus
        let status = Command::new("tmux")
            .args([
                "split-window",
                "-t",
                &format!("{}:0.{}", tmux_name, main_pane_index),
                "-b", // Before (above)
                "-l",
                "5",  // 5 lines
                "-d", // Don't focus
                "-P", // Print pane info
                "-F",
                "#{pane_id}",
            ])
            .output()
            .map_err(|e| format!("Failed to create task pane: {}", e))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(format!("Failed to create task pane: {}", stderr));
        }

        String::from_utf8_lossy(&status.stdout).trim().to_string()
    } else {
        // Split an existing task pane vertically
        // Task panes are at indices 0, 1, 2 for t1, t2, t3
        // t4 splits t1 (index 0), t5 splits t2 (index 1), t6 splits t3 (index 2)
        let split_pane_index = task_count - 3;

        let status = Command::new("tmux")
            .args([
                "split-window",
                "-t",
                &format!("{}:0.{}", tmux_name, split_pane_index),
                "-h", // Horizontal split (left-right)
                "-d", // Don't focus
                "-P", // Print pane info
                "-F",
                "#{pane_id}",
            ])
            .output()
            .map_err(|e| format!("Failed to create task pane: {}", e))?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(format!("Failed to create task pane: {}", stderr));
        }

        String::from_utf8_lossy(&status.stdout).trim().to_string()
    };

    // Build the command to run in the task pane
    let cmd_str = shell_command_text(&command);

    // Send the command to the new pane
    let status = Command::new("tmux")
        .args(["send-keys", "-t", &pane_target, &cmd_str, "Enter"])
        .status()
        .map_err(|e| format!("Failed to send command to task pane: {}", e))?;

    if !status.success() {
        return Err("Failed to send command to task pane.".to_string());
    }

    // Set pane option to track task ID for later identification
    // Using @tb_task as a custom pane option
    let _ = Command::new("tmux")
        .args(["set-option", "-p", "-t", &pane_target, "@tb_task", &task_id])
        .status();

    println!("Task {} started.", task_id);
    println!("Check status with: tb check {}", task_id);

    Ok(())
}

/// Count number of task panes (total panes minus 1 for main pane)
fn count_task_panes(tmux_name: &str) -> usize {
    let output = Command::new("tmux")
        .args(["list-panes", "-t", tmux_name, "-F", "#{pane_id}"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let count = String::from_utf8_lossy(&o.stdout).lines().count();
            count.saturating_sub(1) // Subtract 1 for main pane
        }
        _ => 0,
    }
}

fn cmd_check(
    task: Option<String>,
    session: Option<String>,
    first: usize,
    last: usize,
) -> Result<(), String> {
    let tmux_name = resolve_existing_session_name(session)?;

    let (pane_id, task) = match task {
        Some(task) => (find_task_pane(&tmux_name, &task)?, Some(task)),
        None => (find_main_pane(&tmux_name)?, None),
    };

    // Capture pane content
    let output = capture_pane_scrollback(&pane_id)?;

    if !output.status.success() {
        return match task {
            Some(task) => Err(format!("Task {} not found or pane inaccessible.", task)),
            None => Err("Main pane not found or pane inaccessible.".to_string()),
        };
    }

    let pane_content = String::from_utf8_lossy(&output.stdout);

    // Print the pane content (with truncation)
    print_output(&pane_content, first, last);

    if let Some(task) = task.as_deref() {
        report_task_check_status(task, &pane_content);
    }

    Ok(())
}

fn report_task_check_status(task: &str, pane_content: &str) {
    if is_process_running(pane_content) {
        return;
    }

    let exit_code = find_task_exit_code(pane_content);

    println!();
    match exit_code {
        Some(code) => println!("Task {} finished with exit code {}.", task, code),
        None => println!("Task {} appears complete.", task),
    }
    println!("Close pane with: tb done {}", task);
}

/// Check if a process is still running in a pane
/// Returns true if likely running, false if likely finished
fn is_process_running(pane_content: &str) -> bool {
    let lines: Vec<&str> = pane_content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    // If no content, assume running (just started)
    if lines.is_empty() {
        return true;
    }

    // Check last line for common shell prompt patterns
    // This is a heuristic and won't be perfect
    let last_line = lines.last().unwrap_or(&"");

    // Common prompt endings: $, #, >, %
    // Also check for user@host patterns
    let prompt_patterns = ["$ ", "> ", "% ", "# "];
    let has_prompt_ending = prompt_patterns.iter().any(|p| last_line.ends_with(p))
        || last_line.contains("@")
            && (last_line.ends_with("$")
                || last_line.ends_with(">")
                || last_line.ends_with("%")
                || last_line.ends_with("#"));

    !has_prompt_ending
}

/// Try to find exit code from pane content
fn find_task_exit_code(pane_content: &str) -> Option<i32> {
    // Look for patterns like "exit 42" or shell-specific exit indicators
    // This is very heuristic and may not work for all shells/commands
    for line in pane_content.lines().rev() {
        // Look for "exit" followed by a number
        if let Some(idx) = line.find("exit") {
            let after = &line[idx + 4..];
            let trimmed = after.trim_start();
            if let Some(code) = trimmed
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<i32>().ok())
            {
                return Some(code);
            }
        }
    }
    None
}

fn cmd_done(task: String, session: Option<String>) -> Result<(), String> {
    let tmux_name = resolve_existing_session_name(session)?;

    // Find the pane with the matching task title
    let pane_id = find_task_pane(&tmux_name, &task)?;

    // Kill the pane
    let status = Command::new("tmux")
        .args(["kill-pane", "-t", &pane_id])
        .status()
        .map_err(|e| format!("Failed to close task pane: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to close task {}.", task));
    }

    println!("Closed task {}.", task);

    Ok(())
}

/// Find pane ID for a task by its @tb_task option
fn list_panes_with_task_ids(tmux_name: &str) -> Result<Vec<(String, String)>, String> {
    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-t",
            tmux_name,
            "-F",
            "#{pane_id}:#{@tb_task}",
        ])
        .output()
        .map_err(|e| format!("Failed to list panes: {}", e))?;

    if !output.status.success() {
        return Err("Failed to list panes.".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            line.split_once(':')
                .map(|(pane_id, task_id)| (pane_id.to_string(), task_id.to_string()))
        })
        .collect())
}

fn find_task_pane(tmux_name: &str, task: &str) -> Result<String, String> {
    for (pane_id, task_id) in list_panes_with_task_ids(tmux_name)? {
        if task_id == task {
            return Ok(pane_id);
        }
    }

    Err(format!(
        "Task {} not found.\n\nLaunch a task with: tb launch -- <command>",
        task
    ))
}

fn find_main_pane(tmux_name: &str) -> Result<String, String> {
    for (pane_id, task_id) in list_panes_with_task_ids(tmux_name)? {
        if task_id.is_empty() {
            return Ok(pane_id);
        }
    }

    Err(
        "Main pane not found.\n\nAsk the user to restart the tmux-bridge session with: tb start"
            .to_string(),
    )
}
