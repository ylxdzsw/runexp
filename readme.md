# Runexp

A command-line tool for running experiments with different parameter combinations.

## Features

- **Zero dependencies**: Single binary, no installation required
- **Language agnostic**: Works via environment variables - supports any language
- **Plain text output**: Results saved as CSV for easy processing

## Installation

Download the latest release from the [Releases](https://github.com/ylxdzsw/runexp/releases) page, or build from source:

```bash
cargo build --release
./target/release/runexp --version
```

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
# ... run experiment, print results to stdout
```

## Parameter Syntax

**Naming**: Parameters are converted to uppercase environment variables. Dashes and underscores both become underscores. Parameters can be specified using either space or equal sign syntax.
- `--batch-size 32` or `--batch-size=32` → `BATCH_SIZE`
- `--gpu 1,2` or `--gpu=1,2` → `GPU`

**Values** support:
- **Lists**: `1,2,4` (creates combinations)
- **Ranges**: `1:4` = `1,2,3`, `1:10:2` = `1,3,5,7,9`
- **Expressions**: Reference other parameters with `+`, `*`, `^`
  - `32n` (multiplication)
  - `n+1` (addition)
  - `n^2` (exponentiation)

Parameters can reference each other in any order (forward/backward). Circular dependencies are detected.

## Output

**Parsing**: `runexp` extracts numbers from stdout/stderr. Text before a number becomes its label. Use `--metrics` to filter specific metrics.

**Format**: Results saved to `results.csv` (or use `--output FILE`):
- Parameter columns (in input order)
- Metric columns (if `--metrics` specified)
- stdout/stderr columns (if `--preserve-output` specified)

**Resuming**: Failed experiments are skipped and can be resumed by re-running the same command.

## Options

```
--stdout               Parse only stdout
--stderr               Parse only stderr  
-m, --metrics m1,m2    Filter and validate specific metrics
-p, --preserve-output  Include stdout/stderr columns in the result CSV
--output FILE          Output file (default: results.csv)
-v, --version          Show version information
-h, --help             Show help
```

**Note**: At least one of `-m`/`--metrics` or `-p`/`--preserve-output` must be specified.

**Short options**: Single-letter parameters can also use short form with a single dash (e.g., `-n 1,2` instead of `--n 1,2`).

## Examples

See the `examples/` directory:
- `test_experiment.py` - Sample experiment script
- `run_tests.sh` - Comprehensive test showing all features

```bash
./target/release/runexp --metrics accuracy,loss --gpu 1,2 --batchsize 32,64 python3 examples/test_experiment.py
```
