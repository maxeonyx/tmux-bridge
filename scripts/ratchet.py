#!/usr/bin/env python3
"""
Test Ratchet - Enforces the "Not Rocket Science" rule for tests.

Rules:
1. New tests must fail first (prevents trivial/wrong tests)
2. Once a test passes, it must keep passing (prevents regressions)
3. Tests in status file must exist (prevents silent removal)

Usage:
    python scripts/ratchet.py          # Run with ratchet enforcement
    python scripts/ratchet.py --init   # Initialize status file from current state
"""

import json
import re
import subprocess
import sys
from pathlib import Path

STATUS_FILE = Path(".test-status.json")


def run_cargo_test() -> dict[str, bool]:
    """Run cargo test and parse results. Returns {test_name: passed}."""
    result = subprocess.run(
        ["cargo", "test", "--no-fail-fast", "--", "--test-threads=1"],
        capture_output=True,
        text=True,
    )

    # Combine stdout and stderr
    output = result.stdout + result.stderr

    # Parse test results
    # Format: "test module::name ... ok" or "test module::name ... FAILED"
    pattern = re.compile(r"^test\s+(\S+)\s+\.\.\.\s+(ok|FAILED|ignored)", re.MULTILINE)

    tests = {}
    for match in pattern.finditer(output):
        name, status = match.groups()
        if status != "ignored":
            tests[name] = status == "ok"

    return tests


def run_checks() -> dict[str, bool]:
    """Run lint/format checks. Returns {check_name: passed}."""
    checks = {}

    # cargo fmt
    result = subprocess.run(["cargo", "fmt", "--check"], capture_output=True)
    checks["cargo_fmt"] = result.returncode == 0

    # cargo clippy
    result = subprocess.run(
        ["cargo", "clippy", "--", "-D", "warnings"],
        capture_output=True,
    )
    checks["cargo_clippy"] = result.returncode == 0

    return checks


def load_status() -> dict:
    """Load the status file."""
    if STATUS_FILE.exists():
        return json.loads(STATUS_FILE.read_text())
    return {"tests": {}, "checks": {}}


def save_status(status: dict):
    """Save the status file."""
    STATUS_FILE.write_text(json.dumps(status, indent=2, sort_keys=True) + "\n")


def apply_ratchet(
    current: dict[str, bool], saved: dict[str, str], category: str
) -> tuple[dict[str, str], list[str]]:
    """
    Apply ratchet rules and return (new_status, errors).

    Rules:
    - New test that fails -> pending (ok)
    - New test that passes -> error (must fail first)
    - Pending test that fails -> pending (ok)
    - Pending test that passes -> passing (ok, promoted)
    - Passing test that passes -> passing (ok)
    - Passing test that fails -> error (regression)
    - Test in saved but not in current -> error (missing)
    """
    new_status = {}
    errors = []

    # Check all current tests
    for name, passed in current.items():
        prev = saved.get(name)

        if prev is None:
            # New test
            if passed:
                errors.append(f"[{category}] NEW TEST PASSED (must fail first): {name}")
                new_status[name] = "passing"  # Still track it
            else:
                print(f"[{category}] New pending test: {name}")
                new_status[name] = "pending"
        elif prev == "pending":
            if passed:
                print(f"[{category}] PROMOTED to passing: {name}")
                new_status[name] = "passing"
            else:
                new_status[name] = "pending"
        elif prev == "passing":
            if passed:
                new_status[name] = "passing"
            else:
                errors.append(f"[{category}] REGRESSION: {name}")
                new_status[name] = "passing"  # Keep as passing to show it should pass

    # Check for missing tests
    for name in saved:
        if name not in current:
            errors.append(
                f"[{category}] MISSING TEST (remove from status file if intentional): {name}"
            )

    return new_status, errors


def main():
    init_mode = "--init" in sys.argv

    print("=" * 60)
    print("Running tests...")
    print("=" * 60)

    # Run all tests and checks
    test_results = run_cargo_test()
    check_results = run_checks()

    # Load saved status
    saved = load_status()

    if init_mode:
        # Initialize mode: set current state as baseline
        new_status = {
            "tests": {
                name: "passing" if passed else "pending"
                for name, passed in test_results.items()
            },
            "checks": {
                name: "passing" if passed else "pending"
                for name, passed in check_results.items()
            },
        }
        save_status(new_status)

        passing_tests = sum(1 for p in test_results.values() if p)
        pending_tests = sum(1 for p in test_results.values() if not p)
        passing_checks = sum(1 for p in check_results.values() if p)
        pending_checks = sum(1 for p in check_results.values() if not p)

        print()
        print("=" * 60)
        print("Initialized .test-status.json:")
        print(f"  Tests:  {passing_tests} passing, {pending_tests} pending")
        print(f"  Checks: {passing_checks} passing, {pending_checks} pending")
        print("=" * 60)
        return 0

    # Apply ratchet rules
    all_errors = []

    new_tests, test_errors = apply_ratchet(
        test_results, saved.get("tests", {}), "tests"
    )
    all_errors.extend(test_errors)

    new_checks, check_errors = apply_ratchet(
        check_results, saved.get("checks", {}), "checks"
    )
    all_errors.extend(check_errors)

    # Save updated status
    new_status = {"tests": new_tests, "checks": new_checks}
    save_status(new_status)

    # Summary
    print()
    print("=" * 60)

    passing_tests = sum(1 for s in new_tests.values() if s == "passing")
    pending_tests = sum(1 for s in new_tests.values() if s == "pending")
    passing_checks = sum(1 for s in new_checks.values() if s == "passing")
    pending_checks = sum(1 for s in new_checks.values() if s == "pending")

    print(f"Tests:  {passing_tests} passing, {pending_tests} pending")
    print(f"Checks: {passing_checks} passing, {pending_checks} pending")

    if all_errors:
        print()
        print("RATCHET FAILED:")
        for error in all_errors:
            print(f"  ❌ {error}")
        print("=" * 60)
        return 1
    else:
        print()
        print("✅ Ratchet passed!")
        print("=" * 60)
        return 0


if __name__ == "__main__":
    sys.exit(main())
