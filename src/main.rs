use std::env;

mod evaluator;
mod executor;
mod parser;

use evaluator::evaluate_params;
use executor::execute_experiments;
use parser::parse_args;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return;
    }

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage();
        return;
    }

    // Parse command line arguments
    let (params, command, options) = match parse_args(&args) {
        Ok(result) => result,
        Err(e) => {
            if e == "HELP_REQUESTED" {
                print_usage();
                return;
            }
            eprintln!("Error: {}", e);
            eprintln!("Use --help or -h for usage information");
            std::process::exit(1);
        }
    };

    // Validate that at least one of --metrics or --preserve-output is specified
    if options.metrics.is_empty() && !options.preserve_output {
        eprintln!("Error: At least one of --metrics or --preserve-output must be specified");
        eprintln!("       (Otherwise no meaningful output would be generated)");
        eprintln!("Use --help or -h for usage information");
        std::process::exit(1);
    }

    if params.is_empty() {
        eprintln!("Error: No parameters specified");
        eprintln!("Use --help or -h for usage information");
        std::process::exit(1);
    }

    // Evaluate parameter combinations
    let combinations = match evaluate_params(&params) {
        Ok(combos) => combos,
        Err(e) => {
            eprintln!("Error evaluating parameters: {}", e);
            std::process::exit(1);
        }
    };

    println!("Generated {} parameter combinations", combinations.len());

    // Execute experiments
    if let Err(e) = execute_experiments(&combinations, &command, &options) {
        eprintln!("Error executing experiments: {}", e);
        std::process::exit(1);
    }
}

fn print_usage() {
    println!("runexp - Run experiments with different parameter combinations");
    println!();
    println!("Usage: runexp [OPTIONS] --param1 value1 --param2 value2 ... COMMAND [ARGS...]");
    println!("       runexp [OPTIONS] --param1 value1 --param2 value2 ... < script.sh");
    println!();
    println!("Options:");
    println!("  --stdout               Parse output only from stdout");
    println!("  --stderr               Parse output only from stderr");
    println!("  -m, --metrics m1,m2    Filter results by metrics (comma-separated)");
    println!("  -p, --preserve-output  Include stdout/stderr columns in the result CSV");
    println!("  -o, --output FILE      Output file (default: results.csv)");
    println!("  -c, --concurrency N    Run up to N experiments in parallel (default: 1)");
    println!("  -h, --help             Show this help message");
    println!();
    println!("Parameters:");
    println!("  Parameters are specified as --name value or --name=value");
    println!("  Single-letter parameters can use short form: -n value or -n=value");
    println!("  Parameter names are converted to uppercase environment variables");
    println!("  Dashes and underscores in names are converted to underscores");
    println!("  Example: --batch-size becomes BATCH_SIZE, --gpu becomes GPU, -n becomes N");
    println!();
    println!("Values can contain:");
    println!("  - Comma-separated lists: 1,2,4");
    println!("  - Ranges: 1:4 (expands to 1,2,3)");
    println!("  - Expressions referencing other parameters:");
    println!("    - Variables: n");
    println!("    - Addition: n+1, 2+n");
    println!("    - Multiplication: 2n, n*n");
    println!("    - Exponentiation: n^2");
    println!("  - Literal strings");
    println!();
    println!("Examples:");
    println!("  # Filter results by metrics");
    println!("  runexp --metrics accuracy --gpu 1,2,4 --batchsize 32,64 python train.py");
    println!();
    println!("  # Use expressions for dependent parameters");
    println!("  runexp --metrics accuracy --n 1,2,4 --gpu n --batchsize 32n python train.py");
    println!();
    println!("  # Use heredoc for complex scripts (quote EOF for lazy expansion)");
    println!("  runexp --preserve-output --gpu 1,2,4 --batchsize 32,64 <<\"EOF\"");
    println!("  python tune.py --gpu $GPU --batchsize $BATCHSIZE");
    println!("  python evaluate.py");
    println!("  EOF");
    println!();
    println!("  # Preserve stdout/stderr in the output CSV");
    println!("  runexp --preserve-output --gpu 1,2 --batchsize 32 python train.py");
    println!();
    println!("  # Specify output file");
    println!(
        "  runexp --output my_results.csv --metrics accuracy --gpu 1,2 --batchsize 32 python train.py"
    );
}
