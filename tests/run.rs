//! End-to-end black-box tests for `tb run`
//!
//! These tests verify synchronous command execution through the bridge.

mod common;

use common::{TestSession, tb_cmd};
use predicates::prelude::*;
use std::time::Duration;

mod run_session_resolution {
    use super::*;

    #[test]
    fn fails_without_target() {
        tb_cmd()
            .args(["run", "--", "echo", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("No target specified"))
            .stderr(predicate::str::contains("--target"));
    }

    #[test]
    fn uses_target_flag_with_tb_session_id_fallback() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["run", "--target", session.target(), "--", "echo", "hello"])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello"));
    }

    #[test]
    fn target_flag_works_with_prefixed_tmux_session_name() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                &session.tmux_name(),
                "--",
                "echo",
                "prefix works",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("prefix works"));
    }

    #[test]
    fn fails_with_nonexistent_target() {
        tb_cmd()
            .args(["run", "--target", "nonexistent99", "--", "echo", "hello"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("not found"))
            .stderr(predicate::str::contains("tb start"));
    }

    #[test]
    fn accepts_short_target_flag() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["run", "-t", session.target(), "--", "echo", "short flag"])
            .assert()
            .success()
            .stdout(predicate::str::contains("short flag"));
    }

    #[test]
    fn uses_raw_tmux_session_name_target() {
        let session = TestSession::new();

        tb_cmd()
            .args([
                "run",
                "--target",
                &session.tmux_name(),
                "--",
                "echo",
                "raw target",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("raw target"));
    }

    #[test]
    fn accepts_pane_syntax_target() {
        let session = TestSession::new();
        let pane_target = session.pane_target();

        tb_cmd()
            .args(["run", "-t", &pane_target, "--", "echo", "pane target"])
            .assert()
            .success()
            .stdout(predicate::str::contains("pane target"));
    }
}

mod run_command_execution {
    use super::*;

    #[test]
    fn simple_echo_returns_output() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--",
                "echo",
                "hello world",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello world"));
    }

    #[test]
    fn captures_multiline_output() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--",
                "printf",
                "line1\\nline2\\nline3\\n",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("line1"))
            .stdout(predicate::str::contains("line2"))
            .stdout(predicate::str::contains("line3"));
    }

    #[test]
    fn preserves_exit_status_zero() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["run", "--target", session.target(), "--", "true"])
            .assert()
            .success();
    }

    #[test]
    fn preserves_exit_status_nonzero() {
        let session = TestSession::new();

        session
            .tb_command()
            .args(["run", "--target", session.target(), "--", "false"])
            .assert()
            .failure()
            .code(1);
    }

    #[test]
    fn preserves_specific_exit_code() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--",
                "sh",
                "-c",
                "exit 42",
            ])
            .assert()
            .failure()
            .code(42);
    }

    #[test]
    fn handles_command_with_special_characters() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--",
                "echo",
                "hello; world && test | pipe",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello; world && test | pipe"));
    }

    #[test]
    fn handles_command_with_quotes() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--",
                "echo",
                "it's a \"quoted\" string",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("it's a \"quoted\" string"));
    }

    #[test]
    fn single_arg_multi_statement_script_runs_as_shell_code() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--timeout",
                "5",
                "--",
                "echo hello; echo world",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("hello"))
            .stdout(predicate::str::contains("world"));
    }
}

mod run_single_arg_shell_quoting {
    use super::*;

    fn assert_single_arg_script_exact(script: &str, expected_stdout: &str) {
        let session = TestSession::new();

        session
            .tb_command()
            .arg("run")
            .arg("--target")
            .arg(session.target())
            .arg("--timeout")
            .arg("5")
            .arg("--")
            .arg(script)
            .assert()
            .success()
            .stdout(predicate::eq(expected_stdout.as_bytes().to_vec()));
    }

    #[test]
    fn simple_script_outputs_exactly() {
        assert_single_arg_script_exact("echo hello", "hello\n");
    }

    #[test]
    fn script_with_double_quotes_outputs_exactly() {
        assert_single_arg_script_exact("echo \"hello world\"", "hello world\n");
    }

    #[test]
    fn script_with_single_quotes_outputs_exactly() {
        assert_single_arg_script_exact("echo 'hello world'", "hello world\n");
    }

    #[test]
    fn script_with_both_quote_types_outputs_exactly() {
        assert_single_arg_script_exact("printf '%s\\n' \"it's here\"", "it's here\n");
    }

    #[test]
    fn script_with_backslashes_outputs_exactly() {
        assert_single_arg_script_exact("printf '%s\\n' 'back\\slash'", "back\\slash\n");
    }

    #[test]
    fn script_with_literal_dollar_sign_outputs_exactly() {
        assert_single_arg_script_exact("printf '%s\\n' '$HOME'", "$HOME\n");
    }

    #[test]
    fn script_with_literal_backticks_outputs_exactly() {
        assert_single_arg_script_exact("printf '%s\\n' '`date`'", "`date`\n");
    }

    #[test]
    fn nested_sh_c_outputs_exactly() {
        assert_single_arg_script_exact("sh -c 'echo inner'", "inner\n");
    }

    #[test]
    fn nested_sh_c_with_escaped_quotes_outputs_exactly() {
        assert_single_arg_script_exact("sh -c \"echo \\\"inner quotes\\\"\"", "inner quotes\n");
    }

    #[test]
    fn real_world_style_nested_grep_script_outputs_exactly() {
        assert_single_arg_script_exact(
            "printf '%s\n' 'before'; sh -c \"printf '%s\n' \\\"CPU(s): 8\\\" \\\"MemFree: 1234 kB\\\" | grep -E \\\"CPU\\\\(s\\\\)|MemFree:\\\"\"; printf '%s\n' 'after'",
            "before\nCPU(s): 8\nMemFree: 1234 kB\nafter\n",
        );
    }
}

mod run_dry_run_shell_quoting {
    use super::*;

    fn assert_dry_run_exact(args: &[&str], expected_stdout: &str) {
        tb_cmd()
            .arg("run")
            .arg("--dry-run")
            .arg("--")
            .args(args)
            .assert()
            .success()
            .stdout(predicate::eq(expected_stdout.as_bytes().to_vec()));
    }

    fn expected_single_arg_dry_run(script: &str) -> String {
        format!(
            "sh -c 'echo ___START_dryrunid___; {}; echo ___END_dryrunid_$?___'\n",
            script.replace('\'', "'\\''")
        )
    }

    fn expected_direct_posix_dry_run(script: &str) -> String {
        format!(
            "echo ___START_dryrunid___; {}; echo ___END_dryrunid_$?___\n",
            script
        )
    }

    fn expected_direct_fish_dry_run(script: &str) -> String {
        format!(
            "echo ___START_dryrunid___; {}; echo ___END_dryrunid_{{$status}}___\n",
            script
        )
    }

    #[test]
    fn single_arg_simple_script_is_preserved_exactly() {
        let script = "echo hi";
        assert_dry_run_exact(&[script], &expected_single_arg_dry_run(script));
    }

    #[test]
    fn single_arg_only_escapes_single_quotes_for_outer_wrapper() {
        let script = r#"printf '%s\n' "$HOME""#;
        assert_dry_run_exact(&[script], &expected_single_arg_dry_run(script));
    }

    #[test]
    fn multi_arg_uses_bare_form_for_safe_characters() {
        assert_dry_run_exact(
            &["echo", "path/to:file@host=1+50%"],
            "sh -c 'echo ___START_dryrunid___; echo path/to:file@host=1+50%; echo ___END_dryrunid_$?___'\n",
        );
    }

    #[test]
    fn multi_arg_uses_double_quotes_for_spaces() {
        assert_dry_run_exact(
            &["echo", "two words"],
            "sh -c 'echo ___START_dryrunid___; echo \"two words\"; echo ___END_dryrunid_$?___'\n",
        );
    }

    #[test]
    fn multi_arg_uses_double_quotes_for_metacharacters_without_symbols() {
        assert_dry_run_exact(
            &["echo", "*.rs"],
            "sh -c 'echo ___START_dryrunid___; echo \"*.rs\"; echo ___END_dryrunid_$?___'\n",
        );
    }

    #[test]
    fn multi_arg_uses_single_quotes_for_literal_symbols_without_single_quotes() {
        assert_dry_run_exact(
            &["echo", "$HOME"],
            concat!(
                r#"sh -c 'echo ___START_dryrunid___; echo '\''$HOME'\''; echo ___END_dryrunid_$?___'"#,
                "\n"
            ),
        );
    }

    #[test]
    fn multi_arg_falls_back_to_double_quotes_for_single_quote_and_space() {
        assert_dry_run_exact(
            &["echo", "it's ok"],
            concat!(
                r#"sh -c 'echo ___START_dryrunid___; echo "it'\''s ok"; echo ___END_dryrunid_$?___'"#,
                "\n"
            ),
        );
    }

    #[test]
    fn multi_arg_falls_back_to_double_quotes_with_symbol_escaping() {
        assert_dry_run_exact(
            &["echo", "it's $HOME"],
            concat!(
                r#"sh -c 'echo ___START_dryrunid___; echo "it'\''s \$HOME"; echo ___END_dryrunid_$?___'"#,
                "\n"
            ),
        );
    }

    #[test]
    fn single_arg_real_world_ssh_grep_script_is_preserved_exactly() {
        let script = r#"ssh host "lscpu | grep -E \"Model name|CPU\\(s\\)|Thread|Core\"""#;
        assert_dry_run_exact(&[script], &expected_single_arg_dry_run(script));
    }

    #[test]
    fn single_arg_nested_sh_c_is_preserved_exactly() {
        let script = r#"sh -c "echo inner""#;
        assert_dry_run_exact(&[script], &expected_single_arg_dry_run(script));
    }

    #[test]
    fn single_arg_triply_nested_sh_c_stress_test_is_preserved_exactly() {
        // Verified by running the emitted command via /bin/sh with THREE=3 -> "L3 it's 3".
        let script = r#"sh -c "sh -c \"sh -c \\\"printf '%s\\\\n' \\\\\\\"L3 it's \\\\\\\\$THREE\\\\\\\"\\\"\"""#;
        assert_dry_run_exact(&[script], &expected_single_arg_dry_run(script));
    }

    #[test]
    fn shell_flag_fish_uses_direct_fish_markers_in_dry_run() {
        tb_cmd()
            .args(["run", "--shell", "fish", "--dry-run", "--", "echo hi"])
            .assert()
            .success()
            .stdout(predicate::eq(
                expected_direct_fish_dry_run("echo hi").into_bytes(),
            ));
    }

    #[test]
    fn shell_flag_bash_uses_direct_posix_markers_in_dry_run() {
        tb_cmd()
            .args(["run", "--shell", "bash", "--dry-run", "--", "echo hi"])
            .assert()
            .success()
            .stdout(predicate::eq(
                expected_direct_posix_dry_run("echo hi").into_bytes(),
            ));
    }

    #[test]
    fn shell_flag_sh_uses_direct_posix_markers_in_dry_run() {
        tb_cmd()
            .args(["run", "--shell", "sh", "--dry-run", "--", "echo hi"])
            .assert()
            .success()
            .stdout(predicate::eq(
                expected_direct_posix_dry_run("echo hi").into_bytes(),
            ));
    }

    #[test]
    fn no_shell_uses_sh_c_wrapper_in_dry_run_even_for_fish_target() {
        let session = TestSession::new();
        session.enter_shell("fish");

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--dry-run",
                "--",
                "echo hi",
            ])
            .assert()
            .success()
            .stdout(predicate::eq(
                expected_single_arg_dry_run("echo hi").into_bytes(),
            ));
    }
}

mod run_shell_adaptive_execution {
    use super::*;

    #[test]
    fn shell_flag_fish_single_arg_uses_active_shell_semantics() {
        let session = TestSession::new();
        session.enter_shell("fish");

        session
            .tb_command()
            .args([
                "run",
                "--shell",
                "fish",
                "--target",
                session.target(),
                "--",
                "math 1 + 2",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("3"));
    }

    #[test]
    fn shell_flag_fish_single_arg_allows_explicit_sh_escape_hatch() {
        let session = TestSession::new();
        session.enter_shell("fish");

        session
            .tb_command()
            .args([
                "run",
                "--shell",
                "fish",
                "--target",
                session.target(),
                "--",
                "sh -c 'echo $((1 + 2))'",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("3"));
    }

    #[test]
    fn shell_flag_fish_multi_arg_preserves_argument_boundaries_directly() {
        let session = TestSession::new();
        session.enter_shell("fish");

        session
            .tb_command()
            .args([
                "run",
                "--shell",
                "fish",
                "--target",
                session.target(),
                "--",
                "printf",
                "%s\\n",
                "two words",
                "$HOME",
                "*.rs",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("two words"))
            .stdout(predicate::str::contains("$HOME"))
            .stdout(predicate::str::contains("*.rs"));
    }

    #[test]
    fn shell_flag_bash_direct_path_preserves_output_and_exit_status() {
        let session = TestSession::new();
        session.enter_shell("bash");

        session
            .tb_command()
            .args([
                "run",
                "--shell",
                "bash",
                "--target",
                session.target(),
                "--",
                "printf '%s\\n' bash-direct; bash -c 'exit 17'",
            ])
            .assert()
            .failure()
            .code(17)
            .stdout(predicate::str::contains("bash-direct"));
    }

    #[test]
    fn shell_flag_sh_direct_path_preserves_output_and_exit_status() {
        let session = TestSession::new();
        session.enter_shell("sh");

        session
            .tb_command()
            .args([
                "run",
                "--shell",
                "sh",
                "--target",
                session.target(),
                "--",
                "printf '%s\\n' sh-direct; sh -c 'exit 23'",
            ])
            .assert()
            .failure()
            .code(23)
            .stdout(predicate::str::contains("sh-direct"));
    }

    #[test]
    fn no_shell_fallback_executes_via_sh_c_in_fish_pane() {
        let session = TestSession::new();
        session.enter_shell("fish");

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--",
                "test -z \"$version\"",
            ])
            .assert()
            .success();
    }
}

mod run_timeouts {
    use super::*;

    #[test]
    fn no_output_timeout_triggers() {
        let session = TestSession::new();

        // sleep produces no output, should timeout
        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--timeout",
                "2",
                "--",
                "sleep",
                "30",
            ])
            .timeout(Duration::from_secs(10))
            .assert()
            .failure()
            .code(124)
            .stderr(predicate::str::contains("Timeout"));
    }

    #[test]
    fn max_time_timeout_triggers() {
        let session = TestSession::new();

        // Command that produces output but runs too long
        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--timeout",
                "60", // long no-output timeout
                "--max-time",
                "2", // short overall timeout
                "--",
                "sh",
                "-c",
                "while true; do echo tick; sleep 1; done",
            ])
            .timeout(Duration::from_secs(10))
            .assert()
            .failure()
            .code(124)
            .stderr(predicate::str::contains("Timeout"));
    }

    #[test]
    fn fast_command_does_not_timeout() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--timeout",
                "2",
                "--",
                "echo",
                "quick",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("quick"));
    }
}

mod run_output_truncation {
    use super::*;

    #[test]
    fn truncates_long_output() {
        let session = TestSession::new();

        // Generate 200 lines, request first 5 and last 5
        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--first",
                "5",
                "--last",
                "5",
                "--",
                "seq",
                "1",
                "200",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("1"))
            .stdout(predicate::str::contains("5"))
            .stdout(predicate::str::contains("truncated"))
            .stdout(predicate::str::contains("196"))
            .stdout(predicate::str::contains("200"));
    }

    #[test]
    fn does_not_truncate_short_output() {
        let session = TestSession::new();

        session
            .tb_command()
            .args([
                "run",
                "--target",
                session.target(),
                "--first",
                "50",
                "--last",
                "50",
                "--",
                "seq",
                "1",
                "10",
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("1"))
            .stdout(predicate::str::contains("10"))
            // Should NOT contain truncation message
            .stdout(predicate::str::contains("truncated").not());
    }
}
