# Copilot Instructions for runexp

## Project Purpose

**runexp** is a zero-dependency, language-agnostic CLI tool for running experiments with different parameter combinations.

**Key Flow:**
1. `parser.rs` - Parses CLI args, converts params to uppercase env vars (`--batch-size` → `BATCH_SIZE`)
2. `evaluator.rs` - Evaluates param combinations (lists, ranges, expressions with dependency resolution)
3. `executor.rs` - Runs commands with params as env vars, parses numeric output, saves to CSV
4. `main.rs` - Main entry point

## Important Design Decisions

### Zero Dependencies
No external crates in `Cargo.toml`. This is a core design principle.

### Parameter Order Preservation
Parameters are evaluated in dependency order but CSV output preserves input order via `param_order` field.

### Loop Nesting
First parameter changes least frequently (outer loop). Example: `--gpu 1,2 --batch 32,64` produces: (1,32), (1,64), (2,32), (2,64).

### Output Parsing (Hacky but Intentional)
`extract_numbers_from_line()` has known limitations:
- Skips numbers after alphanumeric chars (e.g., "F1" won't parse "1")
- May incorrectly parse version strings like "v2.3"
- Uses preceding text as labels
- Keeps last value for duplicate metrics
- Simple but works well for typical experiment output

### CSV Compatibility
When resuming, validates existing CSV has same params, metrics, and output settings. Errors if incompatible.

### Expression Support
- Implicit multiplication: `32n` means `32 * n`
- Case-insensitive variable lookup (normalized to uppercase)

## Code Quality

Before finalizing changes:
1. **Clippy**: `cargo clippy -- -D warnings` (fix all warnings)
2. **Format**: `cargo fmt`
3. **Tests**: `./examples/run_tests.sh` (ensure all pass)
4. **Update docs**: If changing CLI behavior, update `readme.md`
