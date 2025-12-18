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
    
    if test $last_status -eq 0
        and test "$last_stdout" = "hello world"
        log_pass "Simple echo command works"
    else
        log_fail "Simple echo failed" \
            "status: $last_status" \
            "stdout: '$last_stdout'" \
            "expected: 'hello world'"
    end
end

function test_exit_status
    log_info "Test: exit status is captured"
    
    run_tmux_send -- false
    
    if test $last_status -ne 0
        log_pass "Exit status captured for failing command"
    else
        log_fail "Exit status should be non-zero for 'false'"
    end
end

function test_stderr_separation
    log_info "Test: stderr is separated from stdout"
    
    run_tmux_send -- fish -c "echo stdout; echo stderr >&2"
    
    if string match -q "*stdout*" $last_stdout
        and string match -q "*stderr*" $last_stderr
        and not string match -q "*stderr*" $last_stdout
        log_pass "Stderr separated from stdout"
    else
        log_fail "Stderr not properly separated" \
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
    
    if test $last_status -eq 0
        and test "$last_stdout" = "with timeout"
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
    
    if test $last_status -ne 0
        and string match -q "*timed out*" $last_stderr
        log_pass "No-output timeout works"
    else
        log_fail "No-output timeout should have triggered" \
            "status: $last_status" \
            "stderr: '$last_stderr'"
    end
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
    test_command_with_args
    test_timeout_flag
    test_no_output_timeout
    
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
