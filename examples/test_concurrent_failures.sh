#!/usr/bin/env bash

# Test script for concurrent execution in faulty scenarios
# This validates that CSV file is never corrupted even when experiments fail

set -e  # Exit on first error

RUNEXP="./target/release/runexp"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Helper function to count CSV rows properly (handles multiline quoted fields)
count_csv_rows() {
    python3 -c "import csv; print(len(list(csv.reader(open('$1')))))"
}

echo "=== Testing concurrent execution in faulty scenarios ==="
echo

# Clean up any previous test files
rm -f test_concurrent_*.csv test_concurrent_*.py

# Build the project
cargo build --release

# Test 1: Some experiments fail (simulate command failures)
echo "Test 1: Concurrent execution with some experiment failures"
echo "-----------------------------------------------------------"

# Create a test script that fails for specific parameter values
cat > test_concurrent_fail.py << 'EOF'
import os
import sys
import random
import time

gpu = int(os.environ.get("GPU", "0"))
batch = int(os.environ.get("BATCH", "0"))

# Simulate some processing time
time.sleep(random.uniform(0.05, 0.2))

# Fail for GPU=2 to test failure handling
if gpu == 2:
    print("Intentional failure for GPU=2", file=sys.stderr)
    sys.exit(1)

# Output metrics
accuracy = 0.8 + (gpu * 0.01) + (batch * 0.001)
loss = 1.0 - accuracy
print(f"accuracy: {accuracy:.4f}")
print(f"loss: {loss:.4f}")
EOF

# Run with concurrent execution (some will fail)
set +e  # Don't exit on failure
$RUNEXP --concurrency 3 --preserve-output --gpu 1,2,3,4 --batch 32,64 --output test_concurrent_fail.csv python3 test_concurrent_fail.py
exit_code=$?
set -e

# Validate CSV file integrity
if [ ! -f test_concurrent_fail.csv ]; then
    echo -e "${RED}✗ CSV file was not created${NC}"
    exit 1
fi

# Check that the CSV is valid (has header and some data rows)
if ! head -1 test_concurrent_fail.csv | grep -q "GPU,BATCH"; then
    echo -e "${RED}✗ CSV header is corrupted${NC}"
    cat test_concurrent_fail.csv
    exit 1
fi

# Count rows (should have header + successful experiments, which is 1 + 6 = 7)
# GPU=1,2,3,4 x BATCH=32,64 = 8 total, minus 2 failures for GPU=2 = 6 successful
row_count=$(count_csv_rows test_concurrent_fail.csv)
expected_rows=7  # 1 header + 6 successful
if [ "$row_count" -ne "$expected_rows" ]; then
    echo -e "${RED}✗ Expected $expected_rows rows (including header), got $row_count${NC}"
    exit 1
fi

# Validate no GPU=2 entries exist
if grep -q "^2," test_concurrent_fail.csv; then
    echo -e "${RED}✗ Failed experiments should not be in CSV${NC}"
    exit 1
fi

echo -e "${GREEN}✓ CSV file remains intact despite some experiment failures${NC}"
echo

# Test 2: Thread panic scenario simulation
echo "Test 2: Concurrent execution with panic recovery"
echo "-------------------------------------------------"

# Create a test script that occasionally panics (via Python crash)
cat > test_concurrent_panic.py << 'EOF'
import os
import sys
import random
import time

gpu = int(os.environ.get("GPU", "0"))
batch = int(os.environ.get("BATCH", "0"))

# Simulate some processing time
time.sleep(random.uniform(0.05, 0.2))

# Crash ungracefully for GPU=3 to simulate panic
if gpu == 3:
    # This will cause Python to exit with a signal (ungraceful termination)
    os._exit(137)  # Simulate SIGKILL-like behavior

# Output metrics
accuracy = 0.8 + (gpu * 0.01) + (batch * 0.001)
loss = 1.0 - accuracy
print(f"accuracy: {accuracy:.4f}")
print(f"loss: {loss:.4f}")
EOF

# Run with concurrent execution
set +e
$RUNEXP --concurrency 4 --preserve-output --gpu 1,2,3,4,5 --batch 32,64,128 --output test_concurrent_panic.csv python3 test_concurrent_panic.py
exit_code=$?
set -e

# Validate CSV file integrity
if [ ! -f test_concurrent_panic.csv ]; then
    echo -e "${RED}✗ CSV file was not created${NC}"
    exit 1
fi

# Check CSV validity
if ! head -1 test_concurrent_panic.csv | grep -q "GPU,BATCH"; then
    echo -e "${RED}✗ CSV header is corrupted${NC}"
    exit 1
fi

# Count rows - should have successful experiments only (excluding GPU=3)
# GPU=1,2,4,5 x BATCH=32,64,128 = 12 successful (minus GPU=3 which has 3 combos)
row_count=$(count_csv_rows test_concurrent_panic.csv)
expected_rows=13  # 1 header + 12 successful
if [ "$row_count" -ne "$expected_rows" ]; then
    echo -e "${RED}✗ Expected $expected_rows rows, got $row_count${NC}"
    exit 1
fi

# Validate no GPU=3 entries
if grep -q "^3," test_concurrent_panic.csv; then
    echo -e "${RED}✗ Panicked experiments should not be in CSV${NC}"
    exit 1
fi

echo -e "${GREEN}✓ CSV file remains intact despite ungraceful terminations${NC}"
echo

# Test 3: High concurrency with many experiments
echo "Test 3: High concurrency stress test"
echo "--------------------------------------"

cat > test_concurrent_stress.py << 'EOF'
import os
import random
import time

n = int(os.environ.get("N", "0"))

# Variable processing time to create more thread interleavings
time.sleep(random.uniform(0.01, 0.1))

# Output metrics
result = n * 2
print(f"result: {result}")
EOF

# Run with high concurrency (more workers than experiments to test edge cases)
$RUNEXP --concurrency 10 --preserve-output --n 1,2,3,4,5,6,7,8,9,10,11,12 --output test_concurrent_stress.csv python3 test_concurrent_stress.py

# Validate CSV file integrity
if [ ! -f test_concurrent_stress.csv ]; then
    echo -e "${RED}✗ CSV file was not created${NC}"
    exit 1
fi

# Check CSV validity
if ! head -1 test_concurrent_stress.csv | grep -q "N"; then
    echo -e "${RED}✗ CSV header is corrupted${NC}"
    exit 1
fi

# Count rows - should have exactly 13 (1 header + 12 experiments)
row_count=$(count_csv_rows test_concurrent_stress.csv)
expected_rows=13
if [ "$row_count" -ne "$expected_rows" ]; then
    echo -e "${RED}✗ Expected $expected_rows rows, got $row_count${NC}"
    exit 1
fi

# Validate no duplicate entries
duplicates=$(python3 -c "import csv; rows = list(csv.reader(open('test_concurrent_stress.csv')))[1:]; ns = [r[0] for r in rows]; dups = [n for n in set(ns) if ns.count(n) > 1]; print(','.join(dups))")
if [ -n "$duplicates" ]; then
    echo -e "${RED}✗ Found duplicate entries: $duplicates${NC}"
    exit 1
fi

echo -e "${GREEN}✓ High concurrency stress test passed - no corruption or duplicates${NC}"
echo

# Test 4: Resume interrupted concurrent execution
echo "Test 4: Resume interrupted concurrent execution"
echo "------------------------------------------------"

cat > test_concurrent_resume.py << 'EOF'
import os
import random
import time

gpu = int(os.environ.get("GPU", "0"))

# Variable processing time
time.sleep(random.uniform(0.05, 0.15))

# Output metrics
accuracy = 0.7 + (gpu * 0.05)
print(f"accuracy: {accuracy:.4f}")
EOF

# First run: complete some experiments
$RUNEXP --concurrency 2 --preserve-output --gpu 1,2,3 --output test_concurrent_resume.csv python3 test_concurrent_resume.py

# Verify first run
first_run_rows=$(count_csv_rows test_concurrent_resume.csv)
expected_first=4  # 1 header + 3 experiments
if [ "$first_run_rows" -ne "$expected_first" ]; then
    echo -e "${RED}✗ First run: expected $expected_first rows, got $first_run_rows${NC}"
    exit 1
fi

# Second run: add more experiments (should append without corrupting existing data)
$RUNEXP --concurrency 3 --preserve-output --gpu 1,2,3,4,5,6 --output test_concurrent_resume.csv python3 test_concurrent_resume.py

# Verify second run
second_run_rows=$(count_csv_rows test_concurrent_resume.csv)
expected_second=7  # 1 header + 6 total experiments (3 skipped, 3 new)
if [ "$second_run_rows" -ne "$expected_second" ]; then
    echo -e "${RED}✗ Second run: expected $expected_second rows, got $second_run_rows${NC}"
    exit 1
fi

# Validate no duplicates
duplicates=$(python3 -c "import csv; rows = list(csv.reader(open('test_concurrent_resume.csv')))[1:]; gpus = [r[0] for r in rows]; dups = [g for g in set(gpus) if gpus.count(g) > 1]; print(','.join(dups))")
if [ -n "$duplicates" ]; then
    echo -e "${RED}✗ Found duplicate entries after resume: $duplicates${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Concurrent resume works correctly - no corruption or duplicates${NC}"
echo

# Clean up
rm -f test_concurrent_*.csv test_concurrent_*.py

echo "=== All concurrent failure scenario tests passed! ==="
