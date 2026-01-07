use std::collections::{HashMap, HashSet};
use std::process::{Command, Stdio};
use std::fs::{self, File};
use std::io::Write;
use crate::evaluator::Combination;
use crate::parser::Options;

#[derive(Debug, Clone)]
struct ExperimentResult {
    params: HashMap<String, String>,
    metrics: HashMap<String, String>,
    stdout: String,
    stderr: String,
}

pub fn execute_experiments(
    combinations: &[Combination],
    command: &[String],
    options: &Options,
) -> Result<(), String> {
    let mut results = Vec::new();
    
    // Load existing results if output file exists
    let existing_results = if std::path::Path::new(&options.output_file).exists() {
        match load_existing_results(&options.output_file) {
            Ok(res) => res,
            Err(_) => Vec::new(), // If failed to load, start fresh
        }
    } else {
        Vec::new()
    };
    
    for (idx, combo) in combinations.iter().enumerate() {
        // Skip if already exists in the result file
        if result_exists(&existing_results, combo) {
            println!("Skipping combination {}/{} (already exists)", idx + 1, combinations.len());
            // Find and copy the existing result
            if let Some(existing) = existing_results.iter().find(|r| r.params == combo.params) {
                results.push(existing.clone());
            }
            continue;
        }
        
        println!("Running combination {}/{}", idx + 1, combinations.len());
        
        match execute_single(combo, command, options) {
            Ok((metrics, stdout, stderr)) => {
                let result = ExperimentResult {
                    params: combo.params.clone(),
                    metrics,
                    stdout,
                    stderr,
                };
                results.push(result);
                // Store results immediately after each successful run
                save_results(&results, &options.output_file, options)?;
            }
            Err(e) => {
                eprintln!("Failed to run combination: {}", e);
                // Continue with other combinations
            }
        }
    }
    
    println!("Completed {} out of {} combinations", results.len(), combinations.len());
    
    Ok(())
}

fn execute_single(
    combo: &Combination,
    command: &[String],
    options: &Options,
) -> Result<(HashMap<String, String>, String, String), String> {
    // Check if command is stdin (heredoc style) or regular command
    let (cmd, args) = if command.is_empty() {
        return Err("No command specified".to_string());
    } else {
        (&command[0], &command[1..])
    };
    
    // Set up the command
    let mut child = Command::new(cmd);
    child.args(args);
    
    // Set environment variables
    for (name, value) in &combo.params {
        child.env(name, value);
    }
    
    // Capture stdout and stderr
    child.stdout(Stdio::piped());
    child.stderr(Stdio::piped());
    
    // Execute
    let output = child.output().map_err(|e| format!("Failed to execute command: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    // Check exit status
    if !output.status.success() {
        // Write the collected stdout and stderr to runexp's output so user can inspect
        eprintln!("=== stdout ===");
        eprint!("{}", stdout);
        eprintln!("=== stderr ===");
        eprint!("{}", stderr);
        return Err(format!("Command failed with exit code: {:?}", output.status.code()));
    }
    
    // Parse output based on options
    let mut parsed = HashMap::new();
    
    if options.stdout_only {
        parse_output(&stdout, &mut parsed, &options.keywords);
    } else if options.stderr_only {
        parse_output(&stderr, &mut parsed, &options.keywords);
    } else {
        // Parse both stdout and stderr by default
        // Add newline delimiter to prevent joining last line of stdout with first line of stderr
        let combined = format!("{}\n{}", stdout, stderr);
        parse_output(&combined, &mut parsed, &options.keywords);
    }
    
    Ok((parsed, stdout, stderr))
}

fn parse_output(text: &str, results: &mut HashMap<String, String>, keywords: &[String]) {
    // Split by line breaks
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Split by whitespace and check for numbers
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        for i in 0..parts.len() {
            // Try to parse as number (support both integers and floats)
            if let Ok(num) = parts[i].parse::<f64>() {
                // Found a number, use preceding text as label
                let label = if i > 0 {
                    parts[..i].join(" ")
                } else {
                    "value".to_string()
                };
                
                // Remove trailing colons or other punctuation from label
                let label = label.trim_end_matches(':').trim().to_string();
                
                // Check if label matches keywords (if specified)
                if !keywords.is_empty() {
                    let matches = keywords.iter().any(|kw| 
                        label.to_lowercase().contains(&kw.to_lowercase())
                    );
                    if !matches {
                        continue;
                    }
                }
                
                // Keep the last value if keyword appears multiple times
                results.insert(label, num.to_string());
            }
        }
    }
}

fn save_results(results: &[ExperimentResult], filename: &str, options: &Options) -> Result<(), String> {
    let mut file = File::create(filename).map_err(|e| format!("Failed to create results file: {}", e))?;
    
    if results.is_empty() {
        return Ok(());
    }
    
    // Collect all unique parameter names and metric names
    let mut param_names_set = HashSet::new();
    let mut metric_names_set = HashSet::new();
    
    for result in results {
        for name in result.params.keys() {
            param_names_set.insert(name.clone());
        }
        for name in result.metrics.keys() {
            metric_names_set.insert(name.clone());
        }
    }
    
    let mut param_names: Vec<String> = param_names_set.into_iter().collect();
    param_names.sort();
    
    let mut metric_names: Vec<String> = metric_names_set.into_iter().collect();
    metric_names.sort();
    
    // Build header: params, metrics, then stdout and/or stderr
    let mut headers = param_names.clone();
    headers.extend(metric_names.clone());
    
    // Add stdout/stderr columns based on options
    if options.stdout_only {
        headers.push("stdout".to_string());
    } else if options.stderr_only {
        headers.push("stderr".to_string());
    } else {
        headers.push("stdout".to_string());
        headers.push("stderr".to_string());
    }
    
    // Write CSV header
    let header_csv = headers.iter()
        .map(|h| escape_csv_field(h))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(file, "{}", header_csv).map_err(|e| format!("Failed to write to file: {}", e))?;
    
    // Write data rows
    for result in results {
        let mut values: Vec<String> = Vec::new();
        
        // Add parameter values
        for name in &param_names {
            let val = result.params.get(name).map(|s| s.as_str()).unwrap_or("");
            values.push(escape_csv_field(val));
        }
        
        // Add metric values (empty if not found)
        for name in &metric_names {
            let val = result.metrics.get(name).map(|s| s.as_str()).unwrap_or("");
            values.push(escape_csv_field(val));
        }
        
        // Add stdout/stderr based on options
        if options.stdout_only {
            values.push(escape_csv_field(&result.stdout));
        } else if options.stderr_only {
            values.push(escape_csv_field(&result.stderr));
        } else {
            values.push(escape_csv_field(&result.stdout));
            values.push(escape_csv_field(&result.stderr));
        }
        
        writeln!(file, "{}", values.join(",")).map_err(|e| format!("Failed to write to file: {}", e))?;
    }
    
    Ok(())
}

// Escape CSV field according to RFC 4180
fn escape_csv_field(field: &str) -> String {
    // If field contains comma, quote, or newline, it needs to be quoted
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        // Escape quotes by doubling them
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

fn load_existing_results(filename: &str) -> Result<Vec<ExperimentResult>, String> {
    let contents = fs::read_to_string(filename)
        .map_err(|_| format!("Could not read file: {}", filename))?;
    
    // Parse CSV manually to handle multi-line fields properly
    let records = parse_csv(&contents)?;
    
    if records.is_empty() {
        return Err("Empty results file".to_string());
    }
    
    let column_names = &records[0];
    let mut results = Vec::new();
    
    for values in &records[1..] {
        if values.len() != column_names.len() {
            continue;
        }
        
        let mut params = HashMap::new();
        let mut metrics = HashMap::new();
        let mut stdout = String::new();
        let mut stderr = String::new();
        
        for (name, value) in column_names.iter().zip(values.iter()) {
            if name == "stdout" {
                stdout = value.clone();
            } else if name == "stderr" {
                stderr = value.clone();
            } else if name.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
                // Parameter names are uppercase (as set by the parser)
                params.insert(name.to_string(), value.to_string());
            } else {
                // Metric names from output typically have mixed case or lowercase
                metrics.insert(name.to_string(), value.to_string());
            }
        }
        
        results.push(ExperimentResult { params, metrics, stdout, stderr });
    }
    
    Ok(results)
}

// Parse entire CSV content handling multi-line fields
fn parse_csv(content: &str) -> Result<Vec<Vec<String>>, String> {
    let mut records = Vec::new();
    let mut current_record = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();
    
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                // Check if it's an escaped quote (doubled)
                if chars.peek() == Some(&'"') {
                    current_field.push('"');
                    chars.next(); // consume the second quote
                } else {
                    // End of quoted field
                    in_quotes = false;
                }
            } else {
                current_field.push(c);
            }
        } else {
            if c == '"' {
                in_quotes = true;
            } else if c == ',' {
                current_record.push(current_field.clone());
                current_field.clear();
            } else if c == '\n' {
                // End of record
                current_record.push(current_field.clone());
                current_field.clear();
                if !current_record.is_empty() && current_record.iter().any(|s| !s.is_empty()) {
                    records.push(current_record.clone());
                }
                current_record.clear();
            } else if c != '\r' {
                current_field.push(c);
            }
        }
    }
    
    // Add the last field and record if not empty
    if !current_field.is_empty() || !current_record.is_empty() {
        current_record.push(current_field);
        if !current_record.is_empty() && current_record.iter().any(|s| !s.is_empty()) {
            records.push(current_record);
        }
    }
    
    Ok(records)
}

fn result_exists(existing: &[ExperimentResult], combo: &Combination) -> bool {
    existing.iter().any(|r| r.params == combo.params)
}
