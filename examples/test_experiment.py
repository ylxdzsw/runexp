#!/usr/bin/env python3
"""
Simple experiment script for testing runexp.
This script reads parameters from environment variables and prints results.
"""
import os
import random

# Read parameters from environment variables
gpu = int(os.environ.get("GPU", 1))
batch_size = int(os.environ.get("BATCHSIZE", 32))

# Simulate some computation
random.seed(gpu * batch_size)
accuracy = round(random.random() * 100, 2)
loss = round(random.random() * 2, 4)
training_time = round(batch_size / gpu + random.random() * 10, 2)

# Report the results
print(f"accuracy: {accuracy}")
print(f"loss: {loss}")
print(f"training time: {training_time}")
print(f"GPU count: {gpu}")
print(f"Batch size: {batch_size}")
