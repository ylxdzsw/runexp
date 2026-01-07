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
    
    // If keywords are specified, check that all were found
    if !options.keywords.is_empty() {
        let mut missing_keywords = Vec::new();
        for keyword in &options.keywords {
            // Check if any metric label contains this keyword
            let found = parsed.keys().any(|label| 
                label.to_lowercase().contains(&keyword.to_lowercase())
            );
            if !found {
                missing_keywords.push(keyword.clone());
            }
        }
        
        if !missing_keywords.is_empty() {
            // Write the collected stdout and stderr to runexp's output so user can inspect
            eprintln!("=== stdout ===");
            eprint!("{}", stdout);
            eprintln!("=== stderr ===");
            eprint!("{}", stderr);
            return Err(format!("Missing keywords in output: {}", missing_keywords.join(", ")));
        }
    }
    
    Ok((parsed, stdout, stderr))
}

fn parse_output(text: &str, results: &mut HashMap<String, String>, keywords: &[String]) {
    // Split by both \n and \r to handle carriage returns (e.g., progress bars)
    // This ensures we process each line refresh separately and keep only the last value
    let lines: Vec<&str> = text.split(|c| c == '\n' || c == '\r').collect();
    
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Try to extract numbers from the line in multiple ways:
        // 1. Split by whitespace and check for pure numbers
        // 2. Find numbers embedded in tokens (e.g., "2.3us", "100ms", "label:2.3ms")
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        for i in 0..parts.len() {
            let part = parts[i];
            
            // Try to parse the entire part as a number first
            if let Ok(num) = part.parse::<f64>() {
                // Found a standalone number, use preceding text as label
                let label = if i > 0 {
                    parts[..i].join(" ")
                } else {
                    "value".to_string()
                };
                
                // Remove trailing colons or other punctuation from label
                let label = label.trim_end_matches(':').trim().to_string();
                
                // Check if label matches keywords (if specified)
                if should_keep_label(&label, keywords) {
                    // Keep the last value if keyword appears multiple times
                    results.insert(label, num.to_string());
                }
            } else {
                // Try to extract a number from within the part (e.g., "2.3us" or "label:2.3ms")
                if let Some((num, label_str)) = extract_number_from_token(part, &parts[..i]) {
                    let label = label_str.trim_end_matches(':').trim().to_string();
                    
                    // Check if label matches keywords (if specified)
                    if should_keep_label(&label, keywords) {
                        // Keep the last value if keyword appears multiple times
                        results.insert(label, num);
                    }
                }
            }
        }
    }
}

// Helper function to check if a label matches the keywords filter
fn should_keep_label(label: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    
    keywords.iter().any(|kw| 
        label.to_lowercase().contains(&kw.to_lowercase())
    )
}

// Extract a number from a token that may contain non-numeric characters
// Returns (number_as_string, label) if a number is found
// Handles cases like:
//   - "2.3us" -> ("2.3", "") [number at start]
//   - "label:2.3ms" -> ("2.3", "label") [label:number]
fn extract_number_from_token(token: &str, preceding_parts: &[&str]) -> Option<(String, String)> {
    // First, check if token contains a colon (e.g., "latency:4.5ms")
    if let Some(colon_pos) = token.find(':') {
        let before_colon = &token[..colon_pos];
        let after_colon = &token[colon_pos + 1..];
        
        // Try to extract a number from the part after the colon
        if let Some((num, _)) = extract_number_from_string(after_colon) {
            // The label is the part before the colon
            return Some((num, before_colon.to_string()));
        }
    }
    
    // Otherwise, try to find a number at the start of the token
    if let Some((num, _)) = extract_number_from_string(token) {
        // Build label from preceding parts
        let label = if preceding_parts.is_empty() {
            "value".to_string()
        } else {
            preceding_parts.join(" ")
        };
        
        return Some((num, label));
    }
    
    None
}

// Extract a number from the beginning of a string
// Returns (number_as_string, remaining_string) if found
fn extract_number_from_string(s: &str) -> Option<(String, String)> {
    let mut num_end = 0;
    let mut has_dot = false;
    
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_digit() {
            num_end = i + 1;
        } else if c == '.' && !has_dot && i > 0 && num_end == i {
            // Allow one decimal point immediately after digits
            // Note: We require at least one digit before the decimal point (i > 0)
            // This is intentional - we don't support leading decimal points like ".5"
            // since they're uncommon in experiment outputs and could cause false positives
            has_dot = true;
            num_end = i + 1;
        } else if num_end > 0 {
            // We found digits followed by non-digit, stop here
            break;
        } else {
            // No digits found yet and we hit a non-digit, not a valid number at start
            return None;
        }
    }
    
    // Check if we found a valid number
    if num_end > 0 {
        let num_str = &s[..num_end];
        
        // Validate it's a proper number (not ending with a dot)
        if num_str.ends_with('.') {
            // Invalid number ending with dot
            return None;
        }
        
        if let Ok(_parsed_num) = num_str.parse::<f64>() {
            let remaining = &s[num_end..];
            return Some((num_str.to_string(), remaining.to_string()));
        }
    }
    
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_number_from_string() {
        // Test integer extraction
        assert_eq!(
            extract_number_from_string("123abc"),
            Some(("123".to_string(), "abc".to_string()))
        );
        
        // Test float extraction
        assert_eq!(
            extract_number_from_string("2.3us"),
            Some(("2.3".to_string(), "us".to_string()))
        );
        
        // Test number without suffix
        assert_eq!(
            extract_number_from_string("456"),
            Some(("456".to_string(), "".to_string()))
        );
        
        // Test no number at start
        assert_eq!(extract_number_from_string("abc123"), None);
        
        // Test invalid number ending with dot
        assert_eq!(extract_number_from_string("123.abc"), None);
    }

    #[test]
    fn test_extract_number_from_token() {
        // Test token with colon separator (e.g., "latency:4.5ms")
        assert_eq!(
            extract_number_from_token("latency:4.5ms", &[]),
            Some(("4.5".to_string(), "latency".to_string()))
        );
        
        // Test token with number at start and preceding label
        assert_eq!(
            extract_number_from_token("2.3us", &["time:"]),
            Some(("2.3".to_string(), "time:".to_string()))
        );
        
        // Test token with number at start and multiple preceding parts
        assert_eq!(
            extract_number_from_token("100ms", &["execution", "time:"]),
            Some(("100".to_string(), "execution time:".to_string()))
        );
        
        // Test token without number
        assert_eq!(extract_number_from_token("nonum", &["label"]), None);
    }

    #[test]
    fn test_parse_output_basic() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec![];
        
        parse_output("accuracy: 0.95", &mut results, &keywords);
        
        assert_eq!(results.get("accuracy"), Some(&"0.95".to_string()));
    }

    #[test]
    fn test_parse_output_no_space() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec![];
        
        // Test with colon directly attached
        parse_output("time:2.3ms", &mut results, &keywords);
        
        assert_eq!(results.get("time"), Some(&"2.3".to_string()));
    }

    #[test]
    fn test_parse_output_with_units() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec![];
        
        parse_output("latency: 4.5us\nthroughput: 1000req/s", &mut results, &keywords);
        
        assert_eq!(results.get("latency"), Some(&"4.5".to_string()));
        assert_eq!(results.get("throughput"), Some(&"1000".to_string()));
    }

    #[test]
    fn test_parse_output_carriage_return() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec![];
        
        // Simulate progress updates with \r
        parse_output("progress: 10\rprogress: 50\rprogress: 100", &mut results, &keywords);
        
        // Should only keep the last value
        assert_eq!(results.get("progress"), Some(&"100".to_string()));
    }

    #[test]
    fn test_parse_output_keep_label_as_is() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec![];
        
        parse_output(
            "Test-Accuracy: 0.95\ntrain_loss: 1.234\nF1-Score (macro): 0.88",
            &mut results,
            &keywords
        );
        
        assert_eq!(results.get("Test-Accuracy"), Some(&"0.95".to_string()));
        assert_eq!(results.get("train_loss"), Some(&"1.234".to_string()));
        assert_eq!(results.get("F1-Score (macro)"), Some(&"0.88".to_string()));
    }

    #[test]
    fn test_parse_output_with_keywords() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec!["accuracy".to_string()];
        
        parse_output("accuracy: 0.95\nloss: 1.234", &mut results, &keywords);
        
        // Should only include metrics matching keywords
        assert_eq!(results.get("accuracy"), Some(&"0.95".to_string()));
        assert_eq!(results.get("loss"), None);
    }

    #[test]
    fn test_parse_output_multiple_values_same_keyword() {
        let mut results = HashMap::new();
        let keywords: Vec<String> = vec![];
        
        parse_output("score: 10\nscore: 20\nscore: 30", &mut results, &keywords);
        
        // Should keep only the last value
        assert_eq!(results.get("score"), Some(&"30".to_string()));
    }
}
