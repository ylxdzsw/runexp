use std::env;

mod parser;
mod evaluator;
mod executor;

use parser::parse_args;
use evaluator::evaluate_params;
use executor::execute_experiments;

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
            eprintln!("Error parsing arguments: {}", e);
            eprintln!();
            print_usage();
            std::process::exit(1);
        }
    };
    
    if params.is_empty() {
        eprintln!("Error: No parameters specified");
        eprintln!();
        print_usage();
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
    println!("  --keywords k1,k2       Filter results by keywords (comma-separated)");
    println!("  --output FILE          Output file (default: results.csv)");
    println!("  -h, --help             Show this help message");
    println!();
    println!("Parameters:");
    println!("  Parameters are specified as --name value");
    println!("  Parameter names are converted to uppercase environment variables");
    println!("  Dashes and underscores in names are converted to underscores");
    println!("  Example: --batch-size becomes BATCH_SIZE, --gpu becomes GPU");
    println!();
    println!("Values can contain:");
    println!("  - Comma-separated lists: 1,2,4");
    println!("  - Ranges: 1:4 (expands to 1,2,3)");
    println!("  - Expressions referencing earlier parameters:");
    println!("    - Variables: n");
    println!("    - Addition: n+1, 2+n");
    println!("    - Multiplication: 2n, n*n");
    println!("    - Exponentiation: n^2");
    println!("  - Literal strings");
    println!();
    println!("Examples:");
    println!("  # Run with different GPU and batch size combinations");
    println!("  runexp --gpu 1,2,4 --batchsize 32,64 python train.py");
    println!();
    println!("  # Use expressions for dependent parameters");
    println!("  runexp --n 1,2,4 --gpu n --batchsize 32n python train.py");
    println!();
    println!("  # Use heredoc for complex scripts");
    println!("  runexp --gpu 1,2,4 --batchsize 32,64 <<EOF");
    println!("  python tune.py --gpu $GPU --batchsize $BATCHSIZE");
    println!("  python evaluate.py");
    println!("  EOF");
    println!();
    println!("  # Filter results by keywords");
    println!("  runexp --keywords accuracy --gpu 1,2 --batchsize 32 python train.py");
    println!();
    println!("  # Specify output file");
    println!("  runexp --output my_results.csv --gpu 1,2 --batchsize 32 python train.py");
}
