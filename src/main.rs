use std::env;

mod parser;
mod evaluator;
mod executor;
mod daemon;

use parser::parse_args;
use evaluator::evaluate_params;
use executor::execute_experiments;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    
    if args.is_empty() {
        print_usage();
        return;
    }
    
    // Parse command line arguments
    let (params, command, options) = match parse_args(&args) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            std::process::exit(1);
        }
    };
    
    // Evaluate parameter combinations
    let combinations = match evaluate_params(&params) {
        Ok(combos) => combos,
        Err(e) => {
            eprintln!("Error evaluating parameters: {}", e);
            std::process::exit(1);
        }
    };
    
    // Execute experiments
    if let Err(e) = execute_experiments(&combinations, &command, &options) {
        eprintln!("Error executing experiments: {}", e);
        std::process::exit(1);
    }
}

fn print_usage() {
    println!("Usage: runexp [OPTIONS] --param1 value1 --param2 value2 ... COMMAND [ARGS...]");
    println!();
    println!("Options:");
    println!("  --stdout          Parse output only from stdout");
    println!("  --stderr          Parse output only from stderr");
    println!("  --keywords k1,k2  Filter results by keywords");
    println!("  --continue_from FILE  Continue from incomplete results");
    println!();
    println!("Parameters are specified as --name value");
    println!("Values can contain:");
    println!("  - Comma-separated lists: 1,2,4");
    println!("  - Ranges: 1..4");
    println!("  - Expressions: n, 2n, n+1, n^2");
}
