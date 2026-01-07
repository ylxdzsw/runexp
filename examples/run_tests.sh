#!/bin/bash
# Example test script demonstrating runexp usage
# This script tests the various features of runexp

set -e

echo "=== Testing runexp functionality ==="
echo

cd "$(dirname "$0")/.."
cargo build --release 2>&1 | tail -1

RUNEXP="./target/release/runexp"

# Clean up any existing results
rm -f test_results*.csv

echo "Test 1: Basic parameter combinations"
echo "-------------------------------------"
$RUNEXP --gpu 1,2 --batchsize 32,64 --output test_results1.csv python3 examples/test_experiment.py
echo "✓ Created test_results1.csv"
echo

echo "Test 2: Using expressions"
echo "-------------------------"
$RUNEXP --n 1,2 --gpu n --batchsize 32n --output test_results2.csv python3 examples/test_experiment.py
echo "✓ Created test_results2.csv"
echo

echo "Test 3: With keyword filtering"
echo "-------------------------------"
$RUNEXP --keywords accuracy,loss --gpu 1,2 --batchsize 32 --output test_results3.csv python3 examples/test_experiment.py
echo "✓ Created test_results3.csv (filtered by keywords)"
echo

echo "Test 4: Auto-skip finished experiments"
echo "---------------------------------------"
echo "Running experiments..."
$RUNEXP --gpu 1,2,4 --batchsize 32 --output test_results4.csv python3 examples/test_experiment.py
echo "Re-running same command (should skip existing experiments)..."
$RUNEXP --gpu 1,2,4 --batchsize 32 --output test_results4.csv python3 examples/test_experiment.py
echo "✓ Skipped existing experiments"
echo

echo "Test 5: Using heredoc"
echo "---------------------"
$RUNEXP --gpu 1,2 --batchsize 32 --output test_results5.csv <<'EOF'
GPU=$GPU
BATCHSIZE=$BATCHSIZE
python3 examples/test_experiment.py
EOF
echo "✓ Created test_results5.csv using heredoc"
echo

echo "Test 6: Missing keyword error"
echo "------------------------------"
echo "Testing with a non-existent keyword (should fail)..."
if $RUNEXP --keywords "nonexistent" --gpu 1 --batchsize 32 --output test_results6.csv python3 examples/test_experiment.py 2>&1 | grep -q "Missing keywords"; then
    echo "✓ Correctly failed when keyword not found"
else
    echo "✗ Failed to detect missing keyword"
    exit 1
fi
echo

echo "Test 7: Keywords with spaces"
echo "-----------------------------"
$RUNEXP --keywords "training time,GPU count" --gpu 1 --batchsize 32 --output test_results7.csv python3 examples/test_experiment.py
echo "✓ Keywords with spaces work correctly"
echo

echo "=== Showing sample output ==="
echo "First 3 lines of test_results1.csv:"
head -3 test_results1.csv
echo
echo "All CSV files created:"
ls -lh test_results*.csv 2>/dev/null | grep -v test_results6.csv || true
echo

echo "=== All tests passed! ==="
echo "Results can be opened in Excel or any CSV viewer."

# Clean up test files
rm -f test_results*.csv

echo "✓ Cleaned up test files"
