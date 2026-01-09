# Copilot Instructions for runexp

## Project Purpose and Structure

**runexp** is a zero-dependency, language-agnostic command-line tool for running experiments with different parameter combinations. It works by:

1. **Parameter Processing** (`src/parser.rs`): Parses CLI arguments, converts parameter names to uppercase environment variables (e.g., `--batch-size` → `BATCH_SIZE`)
2. **Parameter Evaluation** (`src/evaluator.rs`): Evaluates parameter combinations with support for:
   - Lists: `1,2,4`
   - Ranges: `1:4` (expands to 1,2,3), `1:10:2` (step support)
   - Expressions: `32n`, `n+1`, `n^2` (with topological sorting to handle forward/backward references)
   - Circular dependency detection
3. **Experiment Execution** (`src/executor.rs`): Runs commands with parameter combinations as environment variables, parses numeric output, saves to CSV
4. **Main** (`src/main.rs`): Orchestrates the flow and handles CLI interface

**Key Files:**
- `src/parser.rs` - CLI argument parsing
- `src/evaluator.rs` - Parameter expression evaluation and dependency resolution
- `src/executor.rs` - Experiment execution and output parsing
- `src/main.rs` - Main entry point
- `examples/` - Test scripts and comprehensive test suite
- `readme.md` - User-facing documentation

## Important Design Decisions and Behaviors

### 1. Zero Dependencies Philosophy
The project uses **zero external dependencies** (check `Cargo.toml` - the `[dependencies]` section is empty). This is a core design principle. Do not add crates unless absolutely necessary.

### 2. Parameter Order Preservation
Parameters are evaluated in **dependency order** (topologically sorted) but the **original input order is preserved** in the CSV output. This is tracked via the `param_order` field in the `Combination` struct.

### 3. Loop Nesting Order
Parameter combinations follow a specific nesting: the **first parameter changes least frequently** (outer loop), later parameters change more frequently (inner loop). For example, `--gpu 1,2 --batch 32,64` produces: (1,32), (1,64), (2,32), (2,64).

### 4. Output Parsing Behavior (Hacky but Intentional)
The number extraction in `executor.rs::extract_numbers_from_line()` has known limitations:
- **Skips numbers following alphanumeric chars** (e.g., "F1" won't parse "1" as a metric)
- **May incorrectly parse version strings** like "v2.3" 
- **Uses preceding text as labels** - whatever comes before a number becomes its label
- **Handles carriage returns** (`\r`) for progress bars - keeps only the last value
- This simple approach works well for typical experiment output but isn't a full parser

### 5. CSV Compatibility Enforcement
When resuming experiments, the tool validates that the existing CSV file has:
- Same parameters in the same order
- Same metrics
- Same `--preserve-output`, `--stdout-only`, `--stderr-only` settings
If incompatible, it errors and asks the user to use a different output file.

### 6. Implicit Multiplication in Expressions
The evaluator supports implicit multiplication: `32n` means `32 * n`, `2gpu` means `2 * gpu`. This is parsed in `evaluator.rs::parse_atom_expr()`.

### 7. Case-Insensitive Variable Lookup
Variable references in expressions are normalized to uppercase for case-insensitive matching (see `evaluator.rs`).

### 8. Rust Edition 2024
The project uses `edition = "2024"` in Cargo.toml, which requires a recent Rust compiler.

### 9. Aggressive Release Optimization
The release profile in `Cargo.toml` uses:
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for better optimization
- `opt-level = 3` - Maximum optimization
- `strip = true` - Strip symbols for smaller binary

These settings prioritize binary size and performance over compile time.

## Updating This File

**When you add new hacky behaviors or important design choices:**
1. Add them to the "Important Design Decisions and Behaviors" section above
2. Mark them as "Hacky but Intentional" if they're workarounds
3. Explain why the decision was made and any known limitations
4. Update the relevant section (numbered for clarity)

## Code Quality Checks

Before finalizing any code changes:

1. **Run clippy**: `cargo clippy -- -D warnings`
   - Fix all clippy warnings - the codebase should have zero warnings
   
2. **Run rustfmt**: `cargo fmt`
   - Ensure code follows standard Rust formatting

3. **Update examples**: If you change CLI behavior or add features:
   - Update `examples/test_experiment.py` if needed
   - Update `examples/run_tests.sh` to test new features
   - Ensure all tests pass: `./examples/run_tests.sh`

4. **Update readme.md**: If you change:
   - CLI flags or options
   - Parameter syntax or expression support
   - Output format or behavior
   - Add corresponding updates to `readme.md`

5. **Run the example**: Manually test with:
   ```bash
   cargo build --release
   ./target/release/runexp --preserve-output --gpu 1,2 --batchsize 32,64 python3 examples/test_experiment.py
   ```

## Testing Philosophy

The project has extensive unit tests in each module (see `#[cfg(test)]` sections) and integration tests in `examples/run_tests.sh`. When adding features:
- Add unit tests for new parsing/evaluation logic
- Add integration tests to `run_tests.sh` for end-to-end features
- Keep test coverage high
