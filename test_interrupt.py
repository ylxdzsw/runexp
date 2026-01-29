#!/usr/bin/env python3
"""
Test script for interrupt signal handling.
This script sleeps for 30 seconds and handles SIGINT gracefully.
"""
import os
import sys
import time
import signal

def signal_handler(signum, frame):
    print(f"\nReceived signal {signum} in child process!")
    print(f"GPU={os.environ.get('GPU', 'N/A')}, BATCH={os.environ.get('BATCH', 'N/A')}")
    sys.exit(130)  # Standard exit code for SIGINT

# Register signal handler
signal.signal(signal.SIGINT, signal_handler)

gpu = os.environ.get('GPU', 'unknown')
batch = os.environ.get('BATCH', 'unknown')

print(f"Starting experiment with GPU={gpu}, BATCH={batch}")
print("Sleeping for 30 seconds (press Ctrl-C to interrupt)...")

try:
    for i in range(30):
        time.sleep(1)
        if i % 5 == 0:
            print(f"Still running... {i}s elapsed")
    print("Completed successfully!")
    print("accuracy: 0.95")
except KeyboardInterrupt:
    print("\nKeyboardInterrupt caught in Python!")
    sys.exit(130)
