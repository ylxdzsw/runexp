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

# Helper function to count CSV rows (excluding header)
# Uses python to properly handle quoted multiline fields
count_csv_rows() {
    local file=$1
    python3 -c "import csv; print(sum(1 for _ in csv.DictReader(open('$file'))))"
}

# Helper function to get CSV header
get_csv_header() {
    local file=$1
    head -n 1 "$file"
}

# Helper function to check if a CSV contains specific values
# Uses python to properly handle quoted multiline fields
check_csv_contains() {
    local file=$1
    local col1=$2
    local col2=$3
    python3 -c "
import csv
with open('$file') as f:
    reader = csv.DictReader(f)
    for row in reader:
        if row.get('GPU') == '$col1' and row.get('BATCHSIZE') == '$col2':
            exit(0)
    exit(1)
"
}

# Helper function to check if CSV contains a parameter combination (N,GPU,BATCHSIZE)
check_csv_contains_n() {
    local file=$1
    local n=$2
    local gpu=$3
    local batchsize=$4
    python3 -c "
import csv
with open('$file') as f:
    reader = csv.DictReader(f)
    for row in reader:
        if row.get('N') == '$n' and row.get('GPU') == '$gpu' and row.get('BATCHSIZE') == '$batchsize':
            exit(0)
    exit(1)
"
}

# Helper function to extract column value from CSV
# Uses python to properly handle quoted multiline fields
get_csv_value() {
    local file=$1
    local row=$2
    local col=$3
    python3 -c "
import csv
with open('$file') as f:
    reader = csv.DictReader(f)
    rows = list(reader)
    if $row <= len(rows) and $row > 0:
        print(rows[$row-1].get('$col', ''))
"
}

echo "Test 1: Basic parameter combinations"
echo "-------------------------------------"
$RUNEXP --preserve-output --gpu 1,2 --batchsize 32,64 --output test_results1.csv python3 examples/test_experiment.py
# Validate: Should have 4 combinations (2 gpu x 2 batchsize)
row_count=$(count_csv_rows test_results1.csv)
if [ "$row_count" -ne 4 ]; then
    echo "✗ Expected 4 rows, got $row_count"
    exit 1
fi
# Validate: Header should contain GPU and BATCHSIZE columns
header=$(get_csv_header test_results1.csv)
if ! echo "$header" | grep -q "GPU"; then
    echo "✗ Header missing GPU column"
    exit 1
fi
if ! echo "$header" | grep -q "BATCHSIZE"; then
    echo "✗ Header missing BATCHSIZE column"
    exit 1
fi
# Validate: All combinations exist
if ! check_csv_contains test_results1.csv 1 32; then
    echo "✗ Missing combination: GPU=1, BATCHSIZE=32"
    exit 1
fi
if ! check_csv_contains test_results1.csv 1 64; then
    echo "✗ Missing combination: GPU=1, BATCHSIZE=64"
    exit 1
fi
if ! check_csv_contains test_results1.csv 2 32; then
    echo "✗ Missing combination: GPU=2, BATCHSIZE=32"
    exit 1
fi
if ! check_csv_contains test_results1.csv 2 64; then
    echo "✗ Missing combination: GPU=2, BATCHSIZE=64"
    exit 1
fi
echo "✓ Created test_results1.csv with correct 4 combinations"
echo

echo "Test 2: Using expressions"
echo "-------------------------"
$RUNEXP --preserve-output --n 1,2 --gpu n --batchsize 32n --output test_results2.csv python3 examples/test_experiment.py
# Validate: Should have 2 combinations (n=1,2)
row_count=$(count_csv_rows test_results2.csv)
if [ "$row_count" -ne 2 ]; then
    echo "✗ Expected 2 rows, got $row_count"
    exit 1
fi
# Validate: Expressions evaluated correctly
# When n=1: gpu=1, batchsize=32
if ! check_csv_contains_n test_results2.csv 1 1 32; then
    echo "✗ Expression evaluation failed: n=1 should give GPU=1, BATCHSIZE=32"
    exit 1
fi
# When n=2: gpu=2, batchsize=64
if ! check_csv_contains_n test_results2.csv 2 2 64; then
    echo "✗ Expression evaluation failed: n=2 should give GPU=2, BATCHSIZE=64"
    exit 1
fi
echo "✓ Created test_results2.csv with correct expression evaluation"
echo

echo "Test 3: With metric filtering"
echo "-------------------------------"
$RUNEXP --metrics accuracy,loss --gpu 1,2 --batchsize 32 --output test_results3.csv python3 examples/test_experiment.py
# Validate: Should have 2 combinations (2 gpu x 1 batchsize)
row_count=$(count_csv_rows test_results3.csv)
if [ "$row_count" -ne 2 ]; then
    echo "✗ Expected 2 rows, got $row_count"
    exit 1
fi
# Validate: Header should contain accuracy and loss columns
header=$(get_csv_header test_results3.csv)
if ! echo "$header" | grep -q "accuracy"; then
    echo "✗ Header missing accuracy column"
    exit 1
fi
if ! echo "$header" | grep -q "loss"; then
    echo "✗ Header missing loss column"
    exit 1
fi
# Validate: Metrics are extracted (check for numeric values in accuracy and loss columns)
accuracy_val=$(get_csv_value test_results3.csv 1 accuracy)
loss_val=$(get_csv_value test_results3.csv 1 loss)
if ! echo "$accuracy_val" | grep -qE '^-?[0-9]*\.?[0-9]+$'; then
    echo "✗ Accuracy not properly extracted: '$accuracy_val'"
    exit 1
fi
if ! echo "$loss_val" | grep -qE '^-?[0-9]*\.?[0-9]+$'; then
    echo "✗ Loss not properly extracted: '$loss_val'"
    exit 1
fi
echo "✓ Created test_results3.csv with correctly filtered and extracted metrics"
echo

echo "Test 4: Auto-skip finished experiments"
echo "---------------------------------------"
echo "Running experiments..."
$RUNEXP --preserve-output --gpu 1,2,4 --batchsize 32 --output test_results4.csv python3 examples/test_experiment.py
# Validate: Should have 3 combinations
row_count=$(count_csv_rows test_results4.csv)
if [ "$row_count" -ne 3 ]; then
    echo "✗ Expected 3 rows, got $row_count"
    exit 1
fi
echo "Re-running same command (should skip existing experiments)..."
output=$($RUNEXP --preserve-output --gpu 1,2,4 --batchsize 32 --output test_results4.csv python3 examples/test_experiment.py 2>&1)
# Validate: Should report skipping
if ! echo "$output" | grep -q "Skipping"; then
    echo "✗ Did not skip existing experiments"
    exit 1
fi
# Validate: Still have 3 combinations (no duplicates)
row_count=$(count_csv_rows test_results4.csv)
if [ "$row_count" -ne 3 ]; then
    echo "✗ After re-run, expected 3 rows, got $row_count"
    exit 1
fi
echo "✓ Correctly skipped existing experiments and maintained 3 combinations"
echo

echo "Test 5: Using heredoc"
echo "---------------------"
$RUNEXP --preserve-output --gpu 1,2 --batchsize 32 --output test_results5.csv <<'EOF'
GPU=$GPU
BATCHSIZE=$BATCHSIZE
python3 examples/test_experiment.py
EOF
# Validate: Should have 2 combinations
row_count=$(count_csv_rows test_results5.csv)
if [ "$row_count" -ne 2 ]; then
    echo "✗ Expected 2 rows, got $row_count"
    exit 1
fi
# Validate: Correct parameters in output
if ! check_csv_contains test_results5.csv 1 32; then
    echo "✗ Missing combination: GPU=1, BATCHSIZE=32"
    exit 1
fi
if ! check_csv_contains test_results5.csv 2 32; then
    echo "✗ Missing combination: GPU=2, BATCHSIZE=32"
    exit 1
fi
echo "✓ Created test_results5.csv using heredoc with correct 2 combinations"
echo

echo "Test 6: Missing metric error"
echo "------------------------------"
echo "Testing with a non-existent metric (should fail)..."
if $RUNEXP --metrics "nonexistent" --gpu 1 --batchsize 32 --output test_results6.csv python3 examples/test_experiment.py 2>&1 | grep -q "Missing metrics"; then
    echo "✓ Correctly failed when metric not found"
else
    echo "✗ Failed to detect missing metric"
    exit 1
fi
echo

echo "Test 6b: Missing --metrics and --preserve-output (should fail)"
echo "----------------------------------------------------------------"
echo "Testing without --metrics or --preserve-output (should fail)..."
if $RUNEXP --gpu 1 --batchsize 32 --output test_results6b.csv python3 examples/test_experiment.py 2>&1 | grep -q "At least one of --metrics or --preserve-output must be specified"; then
    echo "✓ Correctly failed when neither option specified"
else
    echo "✗ Failed to detect missing options"
    exit 1
fi
echo

echo "Test 7: Metrics with spaces"
echo "-----------------------------"
$RUNEXP --metrics "training time,GPU count" --gpu 1 --batchsize 32 --output test_results7.csv python3 examples/test_experiment.py
# Validate: Should have 1 combination
row_count=$(count_csv_rows test_results7.csv)
if [ "$row_count" -ne 1 ]; then
    echo "✗ Expected 1 row, got $row_count"
    exit 1
fi
# Validate: Header contains metrics with spaces
header=$(get_csv_header test_results7.csv)
if ! echo "$header" | grep -q "training time"; then
    echo "✗ Header missing 'training time' column"
    exit 1
fi
if ! echo "$header" | grep -q "GPU count"; then
    echo "✗ Header missing 'GPU count' column"
    exit 1
fi
echo "✓ Metrics with spaces work correctly and are properly extracted"
echo

echo "Test 8: Equal sign parameter syntax"
echo "-------------------------------------"
$RUNEXP --preserve-output --gpu=1,2 --batchsize=32,64 --output=test_results8.csv python3 examples/test_experiment.py
# Validate: Should have 4 combinations (2 gpu x 2 batchsize)
row_count=$(count_csv_rows test_results8.csv)
if [ "$row_count" -ne 4 ]; then
    echo "✗ Expected 4 rows, got $row_count"
    exit 1
fi
# Validate: All combinations exist
if ! check_csv_contains test_results8.csv 1 32; then
    echo "✗ Missing combination: GPU=1, BATCHSIZE=32"
    exit 1
fi
if ! check_csv_contains test_results8.csv 1 64; then
    echo "✗ Missing combination: GPU=1, BATCHSIZE=64"
    exit 1
fi
if ! check_csv_contains test_results8.csv 2 32; then
    echo "✗ Missing combination: GPU=2, BATCHSIZE=32"
    exit 1
fi
if ! check_csv_contains test_results8.csv 2 64; then
    echo "✗ Missing combination: GPU=2, BATCHSIZE=64"
    exit 1
fi
echo "✓ Equal sign parameter syntax works correctly"
echo

echo "Test 9: Mixed equal sign and space syntax"
echo "-------------------------------------------"
$RUNEXP --metrics=accuracy --gpu 1,2 --batchsize=32 --output test_results9.csv python3 examples/test_experiment.py
# Validate: Should have 2 combinations
row_count=$(count_csv_rows test_results9.csv)
if [ "$row_count" -ne 2 ]; then
    echo "✗ Expected 2 rows, got $row_count"
    exit 1
fi
# Validate: Combinations exist
if ! check_csv_contains test_results9.csv 1 32; then
    echo "✗ Missing combination: GPU=1, BATCHSIZE=32"
    exit 1
fi
if ! check_csv_contains test_results9.csv 2 32; then
    echo "✗ Missing combination: GPU=2, BATCHSIZE=32"
    exit 1
fi
echo "✓ Mixed equal sign and space syntax works correctly"
echo

echo "Test 10: Version flag"
echo "----------------------"
version_output=$($RUNEXP --version 2>&1)
if ! echo "$version_output" | grep -qE '^runexp [0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "✗ Version output format incorrect: '$version_output'"
    exit 1
fi
# Test short form too
version_output_short=$($RUNEXP -v 2>&1)
if [ "$version_output" != "$version_output_short" ]; then
    echo "✗ Short version flag output differs from long form"
    exit 1
fi
echo "✓ Version flag works correctly ($version_output)"
echo

echo "=== Showing sample output ==="
echo "First 3 lines of test_results1.csv:"
head -3 test_results1.csv
echo
echo "All CSV files created:"
ls -lh test_results[1-9].csv 2>/dev/null || true
echo

echo "=== All tests passed! ==="
echo "Results can be opened in Excel or any CSV viewer."

# Clean up test files
rm -f test_results*.csv

echo "✓ Cleaned up test files"
