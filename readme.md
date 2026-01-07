# Runexp

> **Note**: This project is AI-generated (vibed) and the code has not been reviewed by the author.

`runexp` is a command-line tool for running scripts with different parameters and collecting results, designed for running experiments in research projects.

## Motivation / Features

- **No installation**: `runexp` is a statically-linked, single-file, dependency-free binary. Just download with wget, chmod, then run. No Python required.
- **No integration**: Read environment variables, write to stdout/stderr—that's it. All languages are supported. Experiment scripts are regular scripts that can run without `runexp`. No need to import anything.
- **Text-in, text-out**: All related files are plain text that work seamlessly with `sed`, `awk`, `grep`, `vim`, `vscode™`, `Excel™`. Who wants MySQL for 10KB of experiment data?

## Quick Start

### Example 1

Suppose our experiment script is as follows:

```python
import os
ngpu = int(os.environ["GPU"])  # Parameter names are converted to uppercase
batch_size = int(os.environ["BATCH_SIZE"])  # Dashes and underscores become underscores

import random
accuracy = random.random()
time = batch_size / ngpu + random.random() 

print("accuracy: ", accuracy)
print("time: ", time)
```

To run the experiment with different *combinations* of gpus and batch_size, use

```bash
runexp --gpu 1,2,4 --batchsize 32,64 python exp.py --options passed to script
```

This will run the following commands one by one:

```bash
GPU=1 BATCHSIZE=32 python exp.py --options passed to script
GPU=1 BATCHSIZE=64 python exp.py --options passed to script
GPU=2 BATCHSIZE=32 python exp.py --options passed to script
GPU=2 BATCHSIZE=64 python exp.py --options passed to script
GPU=4 BATCHSIZE=32 python exp.py --options passed to script
GPU=4 BATCHSIZE=64 python exp.py --options passed to script
```

To run the experiment with different *pairs* of gpus and batch_size, use

```bash
runexp --n 1,2,4 --gpu n --batchsize 32n python exp.py
```

This runs

```
N=1 GPU=1 BATCHSIZE=32 python exp.py
N=2 GPU=2 BATCHSIZE=64 python exp.py
N=4 GPU=4 BATCHSIZE=128 python exp.py
```

As illustrated above, a parameter can refer to parameters defined earlier, and simple calculations are supported. Parameters that have multiple values (expressed using `,`) instruct `runexp` to run all combinations of the values.

### Example 2

When the experiment command is long or programs don't directly read environment variables, use a heredoc:

```bash
runexp --gpu 1,2,4 --batchsize 32gpu <<"EOF"
python tune.py --gpu $GPU --batchsize $BATCHSIZE
for ((i=0;i<$GPU;i++)); do
    CUDA_VISIBLE_DEVICES=$i python train.py --batchsize $BATCHSIZE &
done
wait
python report_result.py
EOF
```

Note: Quote `EOF` in the heredoc to prevent shell from expanding variables too early.

## Reference

### Parameter Naming Convention

Parameter names are converted to environment variable names:
1. All letters are converted to **uppercase**
2. Both **dashes (`-`)** and **underscores (`_`)** are converted to **underscores (`_`)**

Examples: `--batch-size` → `BATCH_SIZE`, `--learning_rate` → `LEARNING_RATE`, `--gpu` → `GPU`

### Parameter Expressions

Currently supported expressions include:

- Variables defined earlier
- Literal numbers
- Addition: `2+n`
- Multiplication: `2n`, `n*n` (be aware of bash substitution when using `*`)
- Exponentiation: `n^2`
- Comma-separated list: `1,2,4,n,2n+1,4n^3`
- Integer ranges: `1:4` means `1,2,3` (start:end, end exclusive)
- Integer ranges with step: `1:10:2` means `1,3,5,7,9` (start:end:step)
- Literal strings that do not contain any of the above symbols (`+`, `*`, `^`, `,`, `:`)

`runexp` does not intend to embed a scripting language. These expressions should fit most use cases.

### Output Parsing

`runexp` collects stdout and stderr (both by default, or only one with `--stdout` or `--stderr`). The output is split by line breaks and numbers. The text before a number is considered its label. If a label appears multiple times, the last value is kept.

The `--keywords keyword1,keyword2` option filters results - only numbers whose labels contain any keyword are kept. Keywords can contain spaces (e.g., `--keywords "training time,test-accuracy"`). **Important**: If any specified keyword is not found, the experiment is treated as failed.

### Output Format

Results are saved to a CSV file (default: `results.csv`, or specify with `--output FILE`). Columns are:

1. Parameter values
2. Extracted metrics
3. Complete stdout (unless `--stderr` only)
4. Complete stderr (unless `--stdout` only)

### Dealing with Failures and Resuming

Failed experiments are not included in results, but their stdout/stderr are printed for debugging.

`runexp` automatically skips completed experiments. Re-running the same command resumes from where it left off.

## Examples

The `examples/` directory contains:

- `test_experiment.py` - A simple Python script that reads parameters and outputs results
- `run_tests.sh` - A comprehensive test script showing all runexp features

```bash
bash examples/run_tests.sh
./target/release/runexp --gpu 1,2 --batchsize 32,64 python3 examples/test_experiment.py
```
