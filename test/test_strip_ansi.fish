#!/usr/bin/env fish
#
# Tests for ANSI stripping function
#
# Run: ./test/test_strip_ansi.fish

set -g tests_passed 0
set -g tests_failed 0

function strip_ansi
    # The implementation we're testing - will be copied from tmux-send
    perl -pe 's/\e\[[<>=?]?[0-9;]*[a-zA-Z]//g; s/\e\][^\007]*\007//g; s/\e[^[\]]//g; s/\r//g; s/[\x00-\x08\x0b-\x0c\x0e-\x1f]//g'
end

function run_test_raw --argument-names description input_printf expected_printf
    # Use bash printf for reliable binary handling, write directly to files
    set -l input_file (mktemp)
    set -l expected_file (mktemp)
    set -l actual_file (mktemp)
    
    bash -c "printf '$input_printf'" > $input_file
    bash -c "printf '$expected_printf'" > $expected_file
    cat $input_file | strip_ansi > $actual_file
    
    if diff -q $expected_file $actual_file > /dev/null 2>&1
        set tests_passed (math $tests_passed + 1)
        printf "✓ %s\n" $description
    else
        set tests_failed (math $tests_failed + 1)
        printf "✗ %s\n" $description
        printf "  expected: %s\n" (cat $expected_file | cat -v)
        printf "  actual:   %s\n" (cat $actual_file | cat -v)
    end
    
    rm -f $input_file $expected_file $actual_file
end

echo "=== ANSI Strip Tests ==="
echo

# Basic text - should pass through unchanged
run_test_raw "plain text unchanged" 'hello world' 'hello world'
run_test_raw "newlines preserved" 'line1\nline2' 'line1\nline2'
run_test_raw "tabs preserved" 'col1\tcol2' 'col1\tcol2'

# CSI sequences (ESC [ ... letter)
run_test_raw "basic color codes" '\e[32mgreen\e[0m' 'green'
run_test_raw "multi-param color" '\e[1;32mbold green\e[0m' 'bold green'
run_test_raw "256-color code" '\e[38;5;196mred\e[0m' 'red'
run_test_raw "cursor show (question mark param)" '\e[?25h' ''
run_test_raw "bracketed paste off" '\e[?2004l' ''
run_test_raw "CSI with > prefix" '\e[>4;1m' ''
run_test_raw "CSI with = prefix" '\e[=0u' ''

# OSC sequences (ESC ] ... BEL)
run_test_raw "window title OSC" '\e]0;window title\007' ''
run_test_raw "shell integration OSC" '\e]133;A\007' ''
run_test_raw "OSC in middle of text" 'before\e]0;title\007after' 'beforeafter'

# Simple escape sequences (ESC + single char)
run_test_raw "reverse index" '\eM' ''
run_test_raw "keypad mode" '\e>' ''
run_test_raw "application keypad" '\e=' ''

# Control characters
run_test_raw "SI (shift in) stripped" 'hello\017world' 'helloworld'
run_test_raw "SO (shift out) stripped" 'hello\016world' 'helloworld'
run_test_raw "BEL stripped (outside OSC)" 'a\007b' 'ab'

# Carriage return
run_test_raw "CRLF becomes LF" 'line1\r\nline2' 'line1\nline2'
run_test_raw "CR stripped" 'hello\rworld' 'helloworld'

# Combined/realistic examples
run_test_raw "multiple color sequences" '\e[32mhello\e[0m \e[1mworld\e[0m' 'hello world'
run_test_raw "OSC followed by CSI" '\e]0;title\007\e[32mtext\e[0m' 'text'

echo
printf "Passed: %d  Failed: %d\n" $tests_passed $tests_failed

if test $tests_failed -gt 0
    exit 1
end
