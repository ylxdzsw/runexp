# Runexp

A command-line tool for running experiments with different parameter combinations.

## Features

- **Zero dependencies**: Single binary, no installation required
- **Language agnostic**: Works via environment variables - supports any language
- **Plain text output**: Results saved as CSV for easy processing

## Quick Start

```bash
# Run all combinations of parameters
runexp --metrics accuracy --gpu 1,2,4 --batchsize 32,64 python exp.py

# Use expressions (dependent parameters)
runexp --metrics accuracy --n 1,2,4 --gpu n --batchsize 32n python exp.py

# Use heredoc for complex commands
runexp --preserve-output --gpu 1,2 --batchsize 32,64 <<"EOF"
python train.py --gpu $GPU --batchsize $BATCHSIZE
python evaluate.py
EOF
```

Your script reads parameters from environment variables (converted to uppercase):

```python
import os
gpu = int(os.environ["GPU"])
batchsize = int(os.environ["BATCHSIZE"])
print(f"accuracy: 0.9")
```

## Parameter Syntax

**Naming**: Parameters are converted to uppercase environment variables. Dashes and underscores both become underscores. Parameters can be specified using either space or equal sign syntax.
- `--batch-size 32` or `--batch-size=32` → `BATCH_SIZE`
- `--gpu 1,2` or `--gpu=1,2` → `GPU`

**Values** support:
- **Lists**: `clos,fullmesh` (creates combinations)
- **Ranges**: `start:end` or `start:end:step` (end is exclusive)
  - `1:4` = `1,2,3`
  - `1:10:2` = `1,3,5,7,9`
  - Multiple ranges can be concatenated: `1:4,10:13` = `1,2,3,10,11,12`
  - Duplicates are automatically filtered: `1:5,3:7` = `1,2,3,4,5,6`
- **Expressions**: Reference other parameters with `+`, `*`, `^`
  - `32n` (multiplication)
  - `n+1` (addition)
  - `n^2` (exponentiation)

## Output

**Parsing**: `runexp` extracts numbers from stdout/stderr. Text before a number becomes its label. Use `--metrics` to specify metrics to collect. Numbers whose label contains a metric becomes its value.

**Format**: Results saved to `results.csv` (or use `--output FILE`):
- Parameter columns (in input order)
- Metric columns (if `--metrics` specified)
- stdout/stderr columns (if `--preserve-output` specified)

**Resuming**: Completed experiments are skipped when re-running the same command. Failed experiments are retried.

## Options

```
--stdout               Parse only stdout
--stderr               Parse only stderr  
-m, --metrics m1,m2    Filter and validate specific metrics
-p, --preserve-output  Include stdout/stderr columns in the result CSV
-o, --output FILE      Output file (default: results.csv)
-h, --help            Show help
```

## Examples

See the `examples/` directory:
- `test_experiment.py` - Sample experiment script
- `run_tests.sh` - Comprehensive test showing all features

```bash
./target/release/runexp --metrics accuracy,loss --gpu 1,2 --batchsize 32,64 python examples/test_experiment.py
```
