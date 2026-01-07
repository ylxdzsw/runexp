use std::io::{self, Read};

#[derive(Debug, Clone)]
pub struct Options {
    pub stdout_only: bool,
    pub stderr_only: bool,
    pub keywords: Vec<String>,
    pub output_file: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            stdout_only: false,
            stderr_only: false,
            keywords: Vec::new(),
            output_file: "results.csv".to_string(),
        }
    }
}

pub fn parse_args(args: &[String]) -> Result<(Vec<(String, String)>, Vec<String>, Options), String> {
    let mut params = Vec::new();
    let mut options = Options::default();
    let mut i = 0;
    
    while i < args.len() {
        let arg = &args[i];
        
        if arg == "--stdout" {
            options.stdout_only = true;
            i += 1;
        } else if arg == "--stderr" {
            options.stderr_only = true;
            i += 1;
        } else if arg == "--keywords" {
            i += 1;
            if i >= args.len() {
                return Err("--keywords requires an argument".to_string());
            }
            options.keywords = args[i].split(',').map(|s| s.trim().to_string()).collect();
            i += 1;
        } else if arg == "--output" {
            i += 1;
            if i >= args.len() {
                return Err("--output requires an argument".to_string());
            }
            options.output_file = args[i].clone();
            i += 1;
        } else if arg.starts_with("--") {
            let name = arg[2..].to_uppercase().replace('-', "_");
            i += 1;
            if i >= args.len() {
                return Err(format!("Parameter {} requires a value", arg));
            }
            let value = args[i].clone();
            params.push((name, value));
            i += 1;
        } else {
            break;
        }
    }
    
    if options.stdout_only && options.stderr_only {
        return Err("Cannot specify both --stdout and --stderr".to_string());
    }
    
    let mut command = args[i..].to_vec();
    
    // If no command provided, read from stdin (for heredoc usage)
    if command.is_empty() {
        let mut stdin_content = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut stdin_content) {
            return Err(format!("Failed to read from stdin: {}", e));
        }
        
        if !stdin_content.trim().is_empty() {
            command = vec!["bash".to_string(), "-c".to_string(), stdin_content];
        } else {
            return Err("No command specified and no input from stdin".to_string());
        }
    }
    
    Ok((params, command, options))
}
