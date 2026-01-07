# Runexp

> **Note**: This project is AI-generated (vibed) and the code has not been reviewed by the author.

`runexp` is a command-line tool for running scripts with different parameters and collecting results, designed for running experiments in research projects.

## Motivation / Features

- **No installation**: `runexp` is a statically-linked, single-file, dependency-free binary. Just download with wget, chmod, then run. No Python required.
- **No integration**: Read environment variables, write to stdout/stderr—that's it. All languages are supported. Experiment scripts are regular scripts that can run without `runexp`. No need to import anything.
- **Text-in, text-out**: All related files are plain text that work seamlessly with `sed`, `awk`, `grep`, `vim`, `vscode™`, `Excel™`. Who wants MySQL for 10KB of experiment data?

## Usage (Command-line)

### Example 1

Suppose our experiment script is as follows:

```python
# Read parameters from environment variables
import os
ngpu = int(os.environ["GPU"])  # By default, parameter names are capitalized
batch_size = int(os.environ["BATCHSIZE"])

# Do the experiments
import random
accuracy = random.random()
time = batch_size / ngpu + random.random() 

# Report the results
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

### Parameter Expressions

Currently supported expressions include:

- Variables that are defined earlier
- Literal numbers
- Addition: `2+n`
- Multiplication: `2n`, `n*n`. Be aware of bash substitution when using `*`.
- Exponentiation: `n^2`
- Comma-separated list: `1,2,4,n,2n+1,4n^3`
- Integer ranges: `1:4` means `1,2,3` (start:end, where end is exclusive)
- Integer ranges with step: `1:10:2` means `1,3,5,7,9` (start:end:step)
- Literal strings that do not contain any of the above symbols (`+`, `*`, `^`, `,`, `:`)

`runexp` does not intend to embed a scripting language. These expressions should fit most use cases.

### Example 2

We demonstrate another usage pattern:

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

In this example, the experiment command is long and the programs do not directly read environment variables. Therefore, we use a heredoc to send the ad-hoc experiment script to `runexp` through stdin. Note that we need to quote `EOF` in the heredoc to prevent the variables from being expanded too early.

### How the Output is Parsed

Without options, `runexp` concatenates both stdout and stderr, splitting the output by line breaks `\n` and numbers. Each number is labeled by the text before it and included in the results.

The `--stdout` and `--stderr` options can be used to specify the output stream. If `--keywords keyword1,keyword2` is specified, only numbers whose labels contain any of the keywords (keyword1 or keyword2) are kept; others are discarded.

### Dealing with Failures

If any experiment fails, it will not be included in the result file. To continue an experiment, run the original command with an additional option `--continue_from incomplete_result`. This option copies the results of any combinations that already exist in the incomplete_result file, therefore only running the combinations that previously failed.
