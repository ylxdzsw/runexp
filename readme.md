Runexp
======

`runexp` is a commandline tool to run a script with different parameters and collects results, designed for running experiments for research projects.

## Motivation / Features

- **No installation**: `runexp` is a statically-linked single-file dependency-free binary. Just wget, chmod, then run. No, you don't need Python.
- **No integration**: read environment variable, write to stdout/stderr, that's it. All languages supported. The experiment scripts are regular scripts that can run without `runexp`. No need to import anything.
- **No learning**: `runexp` comes with a online script builder. All options are listed and you just click the checkboxes. Documents? No, we don't need that thing.
- **Text-in, text-out**. All related files are plain text that can seamlessly work with `sed`, `awk`, `grep`, `vim`, `vscode™`, `Excel™`. Who wants MySQL for 10kb of experiment data?
- **Auto daemonize**. `runexp` run in the background by default and continues when `ssh` connection is dropped. Stopping it is as easy as `rm ~/.runexp.pid`. `runexp` gracefully stops when it detects that the pid file no longer exists.

## Usage (Commandline)

### Example 1

Suppose our experiment script is as follows:

```python
# read parameters from environment variables
import os
ngpu = int(os.environ["GPU"]) # by default the names are capitalized.
batch_size = int(os.environ["BATCHSIZE"])

# do the experiments
import random
accuracy = random.random()
time = batch_size / ngpu + random.random() 

# report the results
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

As illustrated above, a parameter can refer to parameters defined earlier and simple calculation is supported. Parameters that have multiple values (expressed using `,`) instruct `runexp` to run any combinations of the values.

### Parameter expressions

Currently, supported expressions include:

- variables that are defined earlier
- literal numbers
- addition: `2+n`
- multiplication: `2n`, `n*n`. Be reminded about bash substitution when using `*`.
- exponentiation: `n^2`
- comma-separated list: `1,2,4,n,2n+1,4n^3`
- integer ranges: `1..4` means `1,2,3`
- literal strings that do not contain any of the above symbols (`+`, `*`, `^`, `,`, `..`)

`runexp` does not intend to embed a scripting language. These expressions should fit most use cases.

### Example 2

We demonstrate another usage:

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

In this example the experiment command is long and the programs do not directly read environment variables. Therefore, we use heredoc to send the ad-hoc experiment script to `runexp` through stdin. Note that we need to quote `EOF` in the heredoc to prevent the variables from being expanded too early.

### How the output is parsed

Without options, `runexp` concatenates both stdout and stderr, splitting the outputs by line breaks `\n` and numbers. Each number is labeled by the text before it and included in the results.

The `--stdout` and `--stderr` options can be used to specify the output stream. If `--keywords keyword1,keyword2` is specified, only the numbers whose label contain any of the keyword1 or keyword2 are kept and others are discarded.

### Dealing with Failures

If any of the experiment failed, it will not be included in the result file. To continue an experiment, run the original command with an additional option `--continue_from incomplete_result`. This option copy the result of any combinations that already exist in the incomplete_result, therefore only run the combinations that previously failed.
