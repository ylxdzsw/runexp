#!/bin/bash
# Test script to verify interrupt signal propagation to spawned processes
# This demonstrates that Ctrl-C now properly terminates child processes

set -e

cd "$(dirname "$0")/.."
cargo build --release 2>&1 | tail -1

RUNEXP="./target/release/runexp"

echo "=== Testing Interrupt Signal Propagation ==="
echo
echo "This test will run concurrent experiments that take time."
echo "Each experiment will print messages every second."
echo "Press Ctrl-C to interrupt. You should see that child processes"
echo "receive the signal and can handle it (they will print a message)."
echo
echo "Starting test in 3 seconds..."
sleep 3

# Clean up any existing test file
rm -f test_interrupt_results.csv

# Run with concurrency to test signal propagation
# The test_interrupt.py script will handle SIGINT and print a message
echo
echo "Running: $RUNEXP --preserve-output --concurrency 2 --gpu 1,2 --batch 32,64 python3 test_interrupt.py"
echo
echo "The processes will run for 30 seconds. Press Ctrl-C to test interrupt handling."
echo "You should see messages like 'Received signal 2 in child process!' when you press Ctrl-C."
echo

$RUNEXP --preserve-output --concurrency 2 --gpu 1,2 --batch 32,64 python3 test_interrupt.py || {
    exit_code=$?
    if [ $exit_code -eq 130 ]; then
        echo
        echo "✓ Correctly interrupted by Ctrl-C (exit code 130)"
    else
        echo
        echo "Process exited with code: $exit_code"
    fi
    exit $exit_code
}

echo
echo "If you let it complete, check test_interrupt_results.csv"
