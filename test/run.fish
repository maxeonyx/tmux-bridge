#!/usr/bin/env fish
#
# Test harness for tmux-bridge
#
# Runs the full flow: starts tmux-bridge, sends commands via tmux-send,
# verifies results, and cleans up.
#
# Usage: ./test/run.fish

set -g script_dir (dirname (status filename))
set -g repo_dir (dirname $script_dir)
set -g bin_dir $repo_dir/bin

# Colors for output
set -l RED '\033[0;31m'
set -l GREEN '\033[0;32m'
set -l YELLOW '\033[0;33m'
set -l NC '\033[0m' # No Color

set -g tests_passed 0
set -g tests_failed 0

function log_pass
    set tests_passed (math $tests_passed + 1)
    printf "$GREEN✓$NC %s\n" $argv[1]
end

function log_fail
    set tests_failed (math $tests_failed + 1)
    printf "$RED✗$NC %s\n" $argv[1]
    if test (count $argv) -gt 1
        printf "  %s\n" $argv[2..-1]
    end
end

function log_info
    printf "$YELLOW→$NC %s\n" $argv[1]
end

# Clean up any existing session before starting
function cleanup
    tmux kill-session -t "tmux-bridge-$USER" 2>/dev/null
    rm -rf /tmp/tmux-bridge-$USER
end

# Start the tmux session directly for testing
# (tmux-bridge would attach interactively, which we can't do in tests)
function start_bridge
    log_info "Starting tmux-bridge session..."
    
    set -l session_name "tmux-bridge-$USER"
    set -l runtime_dir /tmp/tmux-bridge-$USER
    
    mkdir -p $runtime_dir
    chmod 700 $runtime_dir
    
    # Create session with destroy-unattached off for testing
    # (normally it's on, but we have no attached client in tests)
    tmux new-session -d -s $session_name fish
    
    # Verify session exists
    if tmux has-session -t $session_name 2>/dev/null
        log_pass "Bridge session started"
        return 0
    else
        log_fail "Bridge session failed to start"
        return 1
    end
end

function stop_bridge
    log_info "Stopping tmux-bridge session..."
    tmux kill-session -t "tmux-bridge-$USER" 2>/dev/null
end

# Test helper: run tmux-send and capture results
function run_tmux_send
    set -l stdout_file (mktemp)
    set -l stderr_file (mktemp)
    
    command $bin_dir/tmux-send $argv >$stdout_file 2>$stderr_file
    set -l exit_status $status
    
    set -g last_stdout (cat $stdout_file)
    set -g last_stderr (cat $stderr_file)
    set -g last_status $exit_status
    
    rm -f $stdout_file $stderr_file
    return $exit_status
end

# --- Tests ---

function test_no_bridge_error
    log_info "Test: tmux-send without bridge shows error"
    
    cleanup  # Ensure no session
    
    run_tmux_send -- echo hello
    
    if test $last_status -ne 0
        and string match -q "*No tmux-bridge session*" $last_stderr
        log_pass "tmux-send shows helpful error when no bridge"
    else
        log_fail "tmux-send should error when no bridge" \
            "status: $last_status" \
            "stderr: $last_stderr"
    end
end

function test_simple_command
    log_info "Test: simple command returns stdout"
    
    run_tmux_send -- echo "hello world"
    
    # Output should contain "hello world" - just the command output
    if test $last_status -eq 0
        and string match -q "*hello world*" -- $last_stdout
        log_pass "Simple echo command works"
    else
        log_fail "Simple echo failed" \
            "status: $last_status" \
            "stdout: '$last_stdout'"
    end
end

function test_exit_status
    log_info "Test: exit status is captured"
    
    run_tmux_send -- false
    
    if test $last_status -ne 0
        log_pass "Exit status captured for failing command"
    else
        log_fail "Exit status should be non-zero for 'false'" \
            "status: $last_status"
    end
end

function test_stderr_separation
    log_info "Test: stderr appears in output (pane captures both)"
    
    run_tmux_send -- fish -c "echo stdout; echo stderr >&2"
    
    # Current design: both stdout and stderr go to the pane
    # So both should appear in stdout capture
    if string match -q "*stdout*" $last_stdout
        and string match -q "*stderr*" $last_stdout
        log_pass "Both stdout and stderr captured from pane"
    else
        log_fail "Output not captured correctly" \
            "stdout: '$last_stdout'" \
            "stderr: '$last_stderr'"
    end
end

function test_multiline_output
    log_info "Test: multiline output captured"
    
    run_tmux_send -- fish -c "echo line1; echo line2; echo line3"
    
    if string match -q "*line1*" $last_stdout
        and string match -q "*line2*" $last_stdout
        and string match -q "*line3*" $last_stdout
        log_pass "Multiline output captured"
    else
        log_fail "Multiline output not captured correctly" \
            "stdout: '$last_stdout'"
    end
end

function test_multiline_input
    log_info "Test: multiline input preserved"
    
    run_tmux_send -- 'echo "line1
line2
line3"'
    
    if test $last_status -eq 0
        and string match -q "*line1*" $last_stdout
        and string match -q "*line2*" $last_stdout
        and string match -q "*line3*" $last_stdout
        # Verify newlines preserved (not collapsed to spaces)
        and not string match -q "*line1 line2*" $last_stdout
        log_pass "Multiline input preserved with newlines"
    else
        log_fail "Multiline input not preserved correctly" \
            "stdout: '$last_stdout'"
    end
end

function test_semicolon_preserved
    log_info "Test: semicolons preserved in commands"
    
    run_tmux_send -- 'echo "before;after"'
    
    if test $last_status -eq 0
        and string match -q "*before;after*" $last_stdout
        log_pass "Semicolons preserved in command"
    else
        log_fail "Semicolons not preserved" \
            "stdout: '$last_stdout'"
    end
end

function test_command_with_args
    log_info "Test: command with multiple arguments"
    
    run_tmux_send -- ls -la /tmp
    
    if test $last_status -eq 0
        and string match -q "*total*" -- $last_stdout
        log_pass "Command with args works"
    else
        log_fail "Command with args failed" \
            "status: $last_status" \
            "stdout: '$last_stdout'"
    end
end

function test_timeout_flag
    log_info "Test: --timeout flag accepted"
    
    run_tmux_send --timeout 5 -- echo "with timeout"
    
    # Output should contain "with timeout" - just the command output
    if test $last_status -eq 0
        and string match -q "*with timeout*" -- $last_stdout
        log_pass "--timeout flag works"
    else
        log_fail "--timeout flag failed" \
            "status: $last_status" \
            "stdout: '$last_stdout'"
    end
end

function test_no_output_timeout
    log_info "Test: no-output timeout triggers"
    
    # sleep produces no output, should timeout after 2s
    run_tmux_send --timeout 2 -- sleep 10
    
    if test $last_status -eq 124
        and string match -q "*Timeout*" -- $last_stderr
        log_pass "No-output timeout works"
    else
        log_fail "No-output timeout should have triggered" \
            "status: $last_status" \
            "stderr: '$last_stderr'"
    end
end

function test_output_truncation
    log_info "Test: long output is truncated (first + last)"
    
    # Generate 200 lines of output, with --first 10 --last 10
    # Should see lines 1-10, truncation message, lines 191-200
    run_tmux_send --first 10 --last 10 -- 'seq 1 200'
    
    if test $last_status -ne 0
        log_fail "Command failed" \
            "status: $last_status"
        return
    end
    
    # Check for first lines (1-10) - may be on separate lines or space-separated
    set -l has_first true
    for i in (seq 1 10)
        if not printf "%s" "$last_stdout" | grep -qE "(^|[^0-9])$i([^0-9]|\$)"
            set has_first false
            break
        end
    end
    
    # Check for last lines (191-200)
    set -l has_last true
    for i in (seq 191 200)
        if not printf "%s" "$last_stdout" | grep -qE "(^|[^0-9])$i([^0-9]|\$)"
            set has_last false
            break
        end
    end
    
    # Check for truncation indicator
    set -l has_truncation (printf "%s" "$last_stdout" | grep -c "truncated")
    
    # Should NOT have middle lines (like 100, 101)
    set -l has_middle (printf "%s" "$last_stdout" | grep -cE "(^|[^0-9])100([^0-9]|\$)")
    
    if test "$has_first" = true
        and test "$has_last" = true
        and test "$has_truncation" -ge 1
        and test "$has_middle" -eq 0
        log_pass "Output correctly truncated"
    else
        log_fail "Truncation not working correctly" \
            "has_first: $has_first" \
            "has_last: $has_last" \
            "has_truncation: $has_truncation" \
            "has_middle: $has_middle"
    end
end

function test_repl_semicolon
    log_info "Test: REPL mode preserves trailing semicolons"
    
    # Use fish's prompt pattern - just need to verify semicolon is sent
    # The fish prompt in the test session matches the pattern we use
    run_tmux_send --prompt "^>" -- 'echo "test;"'
    
    # Check if semicolon was preserved in the output (echo should print test;)
    # Use printf to handle the variable safely
    if printf "%s" "$last_stdout" | grep -q "test;"
        log_pass "REPL mode preserves trailing semicolons"
    else
        log_fail "Semicolon not preserved in REPL mode" \
            "stdout: '$last_stdout'"
    end
end

function test_python_repl
    log_info "Test: Python REPL start and eval"
    
    # Start Python REPL
    run_tmux_send --repl python --timeout 5 -- python3
    
    if test $last_status -ne 0
        log_fail "Failed to start Python REPL" \
            "status: $last_status" \
            "stderr: '$last_stderr'"
        return
    end
    
    if not string match -q "*>>>*" -- $last_stdout
        log_fail "Python REPL did not show prompt" \
            "stdout: '$last_stdout'"
        # Clean up
        run_tmux_send --close
        return
    end
    
    # Evaluate expression
    run_tmux_send --repl python --timeout 5 -- "1 + 1"
    
    if not string match -q "*2*" -- $last_stdout
        log_fail "Python eval did not return expected result" \
            "stdout: '$last_stdout'"
        run_tmux_send --close
        return
    end
    
    # Close REPL
    run_tmux_send --close
    
    log_pass "Python REPL works"
end

# --- Main ---

function main
    log_info "tmux-bridge test harness"
    log_info "======================"
    echo
    
    # Test without bridge first
    test_no_bridge_error
    echo
    
    # Start bridge for remaining tests
    cleanup
    if not start_bridge
        log_fail "Cannot continue without bridge"
        exit 1
    end
    echo
    
    # Run tests that need the bridge
    test_simple_command
    test_exit_status
    test_stderr_separation
    test_multiline_output
    test_multiline_input
    test_semicolon_preserved
    test_command_with_args
    test_timeout_flag
    test_no_output_timeout
    test_output_truncation
    test_repl_semicolon
    test_python_repl
    
    echo
    stop_bridge
    cleanup
    
    # Summary
    echo
    log_info "======================"
    printf "Passed: $GREEN%d$NC  Failed: $RED%d$NC\n" $tests_passed $tests_failed
    
    if test $tests_failed -gt 0
        exit 1
    end
end

main
