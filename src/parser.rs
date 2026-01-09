use std::io::{self, Read};

#[derive(Debug, Clone)]
pub struct Options {
    pub stdout_only: bool,
    pub stderr_only: bool,
    pub metrics: Vec<String>,
    pub output_file: String,
    pub preserve_output: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            stdout_only: false,
            stderr_only: false,
            metrics: Vec::new(),
            output_file: "results.csv".to_string(),
            preserve_output: false,
        }
    }
}

pub type ParseResult = Result<(Vec<(String, String)>, Vec<String>, Options), String>;

pub fn parse_args(args: &[String]) -> ParseResult {
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
        } else if arg == "--metrics" {
            i += 1;
            if i >= args.len() {
                return Err("--metrics requires an argument".to_string());
            }
            options.metrics = args[i].split(',').map(|s| s.trim().to_string()).collect();
            i += 1;
        } else if arg == "--output" {
            i += 1;
            if i >= args.len() {
                return Err("--output requires an argument".to_string());
            }
            options.output_file = args[i].clone();
            i += 1;
        } else if arg == "--preserve-output" {
            options.preserve_output = true;
            i += 1;
        } else if let Some(stripped) = arg.strip_prefix("--") {
            let name = stripped.to_uppercase().replace('-', "_");
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
