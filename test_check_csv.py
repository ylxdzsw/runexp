import csv
import sys

with open('test_concurrent_fail.csv', 'r') as f:
    reader = csv.reader(f)
    rows = list(reader)
    print(f"Total CSV rows: {len(rows)}")
    print(f"Header: {rows[0]}")
    print(f"Data rows: {len(rows) - 1}")
    for i, row in enumerate(rows[1:], 1):
        if row:
            print(f"Row {i}: GPU={row[0]}, BATCH={row[1]}")
