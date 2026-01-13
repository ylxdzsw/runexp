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
        } else if arg == "--metrics"
            || arg == "-m"
            || arg.starts_with("--metrics=")
            || arg.starts_with("-m=")
        {
            let metrics_value = if let Some(value) = arg.strip_prefix("--metrics=") {
                value.to_string()
            } else if let Some(value) = arg.strip_prefix("-m=") {
                value.to_string()
            } else {
                i += 1;
                if i >= args.len() {
                    return Err("--metrics/-m requires an argument".to_string());
                }
                args[i].clone()
            };
            options.metrics = metrics_value
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            i += 1;
        } else if arg == "--output" || arg.starts_with("--output=") {
            let output_value = if let Some(value) = arg.strip_prefix("--output=") {
                value.to_string()
            } else {
                i += 1;
                if i >= args.len() {
                    return Err("--output requires an argument".to_string());
                }
                args[i].clone()
            };
            options.output_file = output_value;
            i += 1;
        } else if arg == "--preserve-output" || arg == "-p" {
            options.preserve_output = true;
            i += 1;
        } else if arg == "-h" || arg == "--help" {
            // Return a special error that indicates help was requested
            return Err("HELP_REQUESTED".to_string());
        } else if let Some(stripped) = arg.strip_prefix("--") {
            // Handle both "--param value" and "--param=value" syntax
            let (name, value) = if let Some(eq_pos) = stripped.find('=') {
                let param_name = stripped[..eq_pos].to_uppercase().replace('-', "_");
                let param_value = stripped[eq_pos + 1..].to_string();
                (param_name, param_value)
            } else {
                let param_name = stripped.to_uppercase().replace('-', "_");
                i += 1;
                if i >= args.len() {
                    return Err(format!("Parameter --{} requires a value", stripped));
                }
                (param_name, args[i].clone())
            };
            params.push((name, value));
            i += 1;
        } else if let Some(stripped) = arg.strip_prefix("-") {
            // Handle short options with single dash
            if stripped.len() == 1 {
                let short_opt = stripped.chars().next().unwrap();
                // Check if this is a known short option
                if short_opt == 'm' || short_opt == 'p' || short_opt == 'h' {
                    // Already handled above
                    return Err(format!("Unknown option: {}", arg));
                } else {
                    // Treat as a short parameter
                    let param_name = short_opt.to_uppercase().to_string();
                    i += 1;
                    if i >= args.len() {
                        return Err(format!("Parameter {} requires a value", arg));
                    }
                    let param_value = args[i].clone();
                    params.push((param_name, param_value));
                    i += 1;
                }
            } else if let Some(eq_pos) = stripped.find('=') {
                // Handle "-x=value" syntax
                let short_opt = &stripped[..eq_pos];
                if short_opt.len() == 1 {
                    let param_name = short_opt.to_uppercase();
                    let param_value = stripped[eq_pos + 1..].to_string();
                    params.push((param_name, param_value));
                    i += 1;
                } else {
                    return Err(format!("Unknown option: {}", arg));
                }
            } else {
                return Err(format!("Unknown option: {}", arg));
            }
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
