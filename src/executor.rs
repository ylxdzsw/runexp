use crate::evaluator::Combination;
use crate::parser::Options;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::process::{Command, Stdio};

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

    // Get expected parameter names from combinations (in input order)
    let expected_params: Vec<String> = if let Some(first_combo) = combinations.first() {
        first_combo.param_order.clone()
    } else {
        Vec::new()
    };

    // Load existing results if output file exists and validate compatibility
    let existing_results = if std::path::Path::new(&options.output_file).exists() {
        match load_existing_results(
            &options.output_file,
            &expected_params,
            &options.metrics,
            options.preserve_output,
            options.stdout_only,
            options.stderr_only,
        ) {
            Ok(res) => res,
            Err(e) => {
                return Err(format!(
                    "Existing result file is incompatible: {}. Please use a different output file or remove the existing one.",
                    e
                ));
            }
        }
    } else {
        Vec::new()
    };

    for (idx, combo) in combinations.iter().enumerate() {
        // Skip if already exists in the result file
        if result_exists(&existing_results, combo) {
            println!(
                "Skipping combination {}/{} (already exists)",
                idx + 1,
                combinations.len()
            );
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
                save_results(&results, &expected_params, &options.output_file, options)?;
            }
            Err(e) => {
                eprintln!("Failed to run combination: {}", e);
                // Continue with other combinations
            }
        }
    }

    println!(
        "Completed {} out of {} combinations",
        results.len(),
        combinations.len()
    );

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
    let output = child
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Check exit status
    if !output.status.success() {
        // Write the collected stdout and stderr to runexp's output so user can inspect
        eprintln!("=== stdout ===");
        eprint!("{}", stdout);
        eprintln!("=== stderr ===");
        eprint!("{}", stderr);
        return Err(format!(
            "Command failed with exit code: {:?}",
            output.status.code()
        ));
    }

    // Parse output based on options
    let mut parsed = HashMap::new();

    if options.stdout_only {
        parse_output(&stdout, &mut parsed, &options.metrics);
    } else if options.stderr_only {
        parse_output(&stderr, &mut parsed, &options.metrics);
    } else {
        // Parse both stdout and stderr by default
        // Add newline delimiter to prevent joining last line of stdout with first line of stderr
        let combined = format!("{}\n{}", stdout, stderr);
        parse_output(&combined, &mut parsed, &options.metrics);
    }

    // If metrics are specified, check that all were found
    if !options.metrics.is_empty() {
        let mut missing_metrics = Vec::new();
        for metric in &options.metrics {
            // Check if any metric label contains this metric
            let found = parsed
                .keys()
                .any(|label| label.to_lowercase().contains(&metric.to_lowercase()));
            if !found {
                missing_metrics.push(metric.clone());
            }
        }

        if !missing_metrics.is_empty() {
            // Write the collected stdout and stderr to runexp's output so user can inspect
            eprintln!("=== stdout ===");
            eprint!("{}", stdout);
            eprintln!("=== stderr ===");
            eprint!("{}", stderr);
            return Err(format!(
                "Missing metrics in output: {}",
                missing_metrics.join(", ")
            ));
        }
    }

    Ok((parsed, stdout, stderr))
}

fn parse_output(text: &str, results: &mut HashMap<String, String>, metrics: &[String]) {
    // Split by both \n and \r to handle carriage returns (e.g., progress bars)
    // This ensures we process each line refresh separately and keep only the last value
    let lines: Vec<&str> = text.split(['\n', '\r']).collect();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse numbers from the line without making assumptions about format
        // Find all numbers in the line and use the preceding text as the label
        extract_numbers_from_line(line, results, metrics);
    }
}

// Extract numbers from a line, using preceding text as labels.
// Numbers following alphanumeric chars (e.g., "F1") are skipped to avoid false matches.
// Limitation: This may incorrectly parse numbers in complex contexts like version strings.
fn extract_numbers_from_line(
    line: &str,
    results: &mut HashMap<String, String>,
    metrics: &[String],
) {
    let mut search_start = 0; // Position to start searching for the next number
    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    while i < chars.len() {
        // Check if we're at the start of a number
        // A number should not be preceded by an alphanumeric character (to avoid parsing "F1" as having number "1")
        let is_num_start = (chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()))
            && (i == 0 || !chars[i - 1].is_alphanumeric());

        if is_num_start {
            // Found the start of a number
            let num_start = i;
            let mut num_end = i;
            let mut has_dot = chars[i] == '.';

            // If we started with a dot, move past it
            if has_dot {
                num_end = i + 1;
                i += 1;
            }

            // Collect digits (and at most one decimal point)
            while i < chars.len() {
                if chars[i].is_ascii_digit() {
                    num_end = i + 1;
                    i += 1;
                } else if chars[i] == '.'
                    && !has_dot
                    && i + 1 < chars.len()
                    && chars[i + 1].is_ascii_digit()
                {
                    has_dot = true;
                    num_end = i + 1;
                    i += 1;
                } else {
                    break;
                }
            }

            // Extract the number string
            let num_str: String = chars[num_start..num_end].iter().collect();

            // Validate it's a proper number
            if let Ok(_parsed_num) = num_str.parse::<f64>() {
                // Extract the label (everything from search_start to num_start)
                let label: String = chars[search_start..num_start].iter().collect();

                // Use the label as-is, or "value" if empty
                let label = if label.is_empty() {
                    "value".to_string()
                } else {
                    label
                };

                // Check if label matches metrics (if specified)
                if should_keep_label(&label, metrics) {
                    // Keep the last value if metric appears multiple times
                    results.insert(label, num_str);
                }
            }

            // Update search_start for the next number
            search_start = num_end;
        } else {
            i += 1;
        }
    }
}

fn should_keep_label(label: &str, metrics: &[String]) -> bool {
    if metrics.is_empty() {
        return true;
    }

    metrics
        .iter()
        .any(|m| label.to_lowercase().contains(&m.to_lowercase()))
}

fn save_results(
    results: &[ExperimentResult],
    param_names: &[String],
    filename: &str,
    options: &Options,
) -> Result<(), String> {
    let mut file =
        File::create(filename).map_err(|e| format!("Failed to create results file: {}", e))?;

    if results.is_empty() {
        return Ok(());
    }

    // Use the provided param_names order instead of sorting
    // Build header using the shared helper function
    let headers = build_csv_headers(
        param_names,
        &options.metrics,
        options.preserve_output,
        options.stdout_only,
        options.stderr_only,
    );

    // Pre-compute lowercase metrics to avoid repeated allocations in the loop
    let metric_columns_lower: Vec<String> = options
        .metrics
        .iter()
        .map(|m| m.to_lowercase())
        .collect();

    // Write CSV header
    let header_csv = headers
        .iter()
        .map(|h| escape_csv_field(h))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(file, "{}", header_csv).map_err(|e| format!("Failed to write to file: {}", e))?;

    // Write data rows
    for result in results {
        let mut values: Vec<String> = Vec::new();

        // Add parameter values
        for name in param_names {
            let val = result.params.get(name).map(|s| s.as_str()).unwrap_or("");
            values.push(escape_csv_field(val));
        }

        // Add metric values (find matching metric for each metric name)
        for metric_lower in &metric_columns_lower {
            // Find the metric that matches this metric name (case-insensitive)
            let val = result
                .metrics
                .iter()
                .find(|(label, _)| label.to_lowercase().contains(metric_lower))
                .map(|(_, v)| v.as_str())
                .unwrap_or("");
            values.push(escape_csv_field(val));
        }

        // Add stdout/stderr only if preserve_output is enabled
        if options.preserve_output {
            if options.stdout_only {
                values.push(escape_csv_field(&result.stdout));
            } else if options.stderr_only {
                values.push(escape_csv_field(&result.stderr));
            } else {
                values.push(escape_csv_field(&result.stdout));
                values.push(escape_csv_field(&result.stderr));
            }
        }

        writeln!(file, "{}", values.join(","))
            .map_err(|e| format!("Failed to write to file: {}", e))?;
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

fn build_csv_headers(
    param_names: &[String],
    metrics: &[String],
    preserve_output: bool,
    stdout_only: bool,
    stderr_only: bool,
) -> Vec<String> {
    let mut headers = param_names.to_vec();
    headers.extend_from_slice(metrics);
    
    if preserve_output {
        if stdout_only {
            headers.push("stdout".to_string());
        } else if stderr_only {
            headers.push("stderr".to_string());
        } else {
            headers.push("stdout".to_string());
            headers.push("stderr".to_string());
        }
    }
    
    headers
}

fn load_existing_results(
    filename: &str,
    expected_params: &[String],
    expected_metrics: &[String],
    preserve_output: bool,
    stdout_only: bool,
    stderr_only: bool,
) -> Result<Vec<ExperimentResult>, String> {
    let contents =
        fs::read_to_string(filename).map_err(|_| format!("Could not read file: {}", filename))?;

    let records = parse_csv(&contents)?;

    if records.is_empty() {
        return Err("Empty results file".to_string());
    }

    let column_names = &records[0];

    // Build expected header using the shared helper function
    let expected_headers = build_csv_headers(
        expected_params,
        expected_metrics,
        preserve_output,
        stdout_only,
        stderr_only,
    );

    // Compare headers
    if column_names != &expected_headers {
        let file_header = column_names.join(",");
        let expected_header = expected_headers.join(",");
        return Err(format!(
            "Header mismatch.\nExpected: {}\nFound:    {}",
            expected_header, file_header
        ));
    }

    let num_params = expected_params.len();
    let num_metrics = expected_metrics.len();
    let data_columns_end = num_params + num_metrics;

    // Parse the results
    let mut results = Vec::new();

    for values in &records[1..] {
        if values.len() != column_names.len() {
            continue;
        }

        let mut params = HashMap::new();
        let mut metrics = HashMap::new();
        let mut stdout = String::new();
        let mut stderr = String::new();

        for (idx, (name, value)) in column_names.iter().zip(values.iter()).enumerate() {
            if name == "stdout" {
                stdout = value.clone();
            } else if name == "stderr" {
                stderr = value.clone();
            } else if idx < num_params {
                // It's a parameter
                params.insert(name.to_string(), value.to_string());
            } else if idx < data_columns_end {
                // It's a metric - store with metric name as key
                metrics.insert(name.to_string(), value.to_string());
            }
        }

        results.push(ExperimentResult {
            params,
            metrics,
            stdout,
            stderr,
        });
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
        } else if c == '"' {
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
    fn test_parse_output_formats() {
        let metrics: Vec<String> = vec![];
        let mut results = HashMap::new();

        // Basic colon-space format
        parse_output("accuracy: 0.95", &mut results, &metrics);
        assert_eq!(results.get("accuracy: "), Some(&"0.95".to_string()));

        // No space after colon
        parse_output("time:2.3ms", &mut results, &metrics);
        assert_eq!(results.get("time:"), Some(&"2.3".to_string()));

        // With units
        parse_output("latency: 4.5us", &mut results, &metrics);
        assert_eq!(results.get("latency: "), Some(&"4.5".to_string()));

        // Equals sign
        parse_output("result=42", &mut results, &metrics);
        assert_eq!(results.get("result="), Some(&"42".to_string()));

        // Space-separated
        parse_output("count(items) 99", &mut results, &metrics);
        assert_eq!(results.get("count(items) "), Some(&"99".to_string()));
    }

    #[test]
    fn test_parse_output_special_cases() {
        let metrics: Vec<String> = vec![];

        // Carriage return (progress bar simulation) - keep last value
        let mut results = HashMap::new();
        parse_output(
            "progress: 10\rprogress: 50\rprogress: 100",
            &mut results,
            &metrics,
        );
        assert_eq!(results.get("progress: "), Some(&"100".to_string()));

        // Multiple values with same label - keep last
        let mut results = HashMap::new();
        parse_output("score: 10\nscore: 20\nscore: 30", &mut results, &metrics);
        assert_eq!(results.get("score: "), Some(&"30".to_string()));

        // Complex line with multiple numbers
        let mut results = HashMap::new();
        parse_output(
            "simulated 73us in 2.8s, 6000 events resolved",
            &mut results,
            &metrics,
        );
        assert_eq!(results.get("simulated "), Some(&"73".to_string()));
        assert_eq!(results.get("us in "), Some(&"2.8".to_string()));
        assert_eq!(results.get("s, "), Some(&"6000".to_string()));
    }

    #[test]
    fn test_parse_output_labels_preserved() {
        let mut results = HashMap::new();
        let metrics: Vec<String> = vec![];

        parse_output(
            "Test-Accuracy: 0.95\ntrain_loss: 1.234\nF1-Score (macro): 0.88",
            &mut results,
            &metrics,
        );

        assert_eq!(results.get("Test-Accuracy: "), Some(&"0.95".to_string()));
        assert_eq!(results.get("train_loss: "), Some(&"1.234".to_string()));
        assert_eq!(results.get("F1-Score (macro): "), Some(&"0.88".to_string()));
    }

    #[test]
    fn test_parse_output_metric_filtering() {
        let mut results = HashMap::new();
        let metrics = vec!["accuracy".to_string()];

        parse_output("accuracy: 0.95\nloss: 1.234", &mut results, &metrics);

        assert_eq!(results.get("accuracy: "), Some(&"0.95".to_string()));
        assert_eq!(results.get("loss: "), None);
    }

    #[test]
    fn test_load_existing_results_compatible() {
        use std::io::Write;

        // Create a temporary CSV file using std::env::temp_dir() for portability
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_runexp_compatible.csv");
        {
            let mut file = File::create(&temp_path).unwrap();
            writeln!(file, "BATCHSIZE,GPU,accuracy,stdout,stderr").unwrap();
            writeln!(file, "32,1,0.95,\"output\",\"error\"").unwrap();
        }

        let expected_params = vec!["BATCHSIZE".to_string(), "GPU".to_string()];
        let expected_metrics = vec!["accuracy".to_string()];

        let result = load_existing_results(
            temp_path.to_str().unwrap(),
            &expected_params,
            &expected_metrics,
            true,  // preserve_output
            false, // stdout_only
            false, // stderr_only
        );

        // Clean up
        let _ = fs::remove_file(&temp_path);

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].params.get("BATCHSIZE"), Some(&"32".to_string()));
        assert_eq!(results[0].params.get("GPU"), Some(&"1".to_string()));
    }

    #[test]
    fn test_load_existing_results_incompatible_params() {
        use std::io::Write;

        // Create a temporary CSV file with different parameters
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_runexp_incompatible_params.csv");
        {
            let mut file = File::create(&temp_path).unwrap();
            writeln!(file, "BATCHSIZE,GPU,stdout,stderr").unwrap();
            writeln!(file, "32,1,\"output\",\"error\"").unwrap();
        }

        // Expect different parameters (3 instead of 2)
        let expected_params = vec!["BATCHSIZE".to_string(), "GPU".to_string(), "LR".to_string()];
        let expected_metrics: Vec<String> = vec![];

        let result = load_existing_results(
            temp_path.to_str().unwrap(),
            &expected_params,
            &expected_metrics,
            true,  // preserve_output
            false, // stdout_only
            false, // stderr_only
        );

        // Clean up
        let _ = fs::remove_file(&temp_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Header mismatch"));
    }

    #[test]
    fn test_load_existing_results_incompatible_metrics() {
        use std::io::Write;

        // Create a temporary CSV file with accuracy metric
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_runexp_incompatible_metrics.csv");
        {
            let mut file = File::create(&temp_path).unwrap();
            writeln!(file, "BATCHSIZE,GPU,accuracy,stdout,stderr").unwrap();
            writeln!(file, "32,1,0.95,\"output\",\"error\"").unwrap();
        }

        let expected_params = vec!["BATCHSIZE".to_string(), "GPU".to_string()];
        // Expect different metrics
        let expected_metrics = vec!["loss".to_string()];

        let result = load_existing_results(
            temp_path.to_str().unwrap(),
            &expected_params,
            &expected_metrics,
            true,  // preserve_output
            false, // stdout_only
            false, // stderr_only
        );

        // Clean up
        let _ = fs::remove_file(&temp_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Header mismatch"));
    }

    #[test]
    fn test_load_existing_results_preserve_output_mismatch() {
        use std::io::Write;

        // Create a temporary CSV file WITH stdout/stderr columns
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_runexp_preserve_output.csv");
        {
            let mut file = File::create(&temp_path).unwrap();
            writeln!(file, "BATCHSIZE,GPU,accuracy,stdout,stderr").unwrap();
            writeln!(file, "32,1,0.95,\"output\",\"error\"").unwrap();
        }

        let expected_params = vec!["BATCHSIZE".to_string(), "GPU".to_string()];
        let expected_metrics = vec!["accuracy".to_string()];

        // Try to load WITHOUT preserve_output (should fail)
        let result = load_existing_results(
            temp_path.to_str().unwrap(),
            &expected_params,
            &expected_metrics,
            false, // preserve_output = false but file has output columns
            false, // stdout_only
            false, // stderr_only
        );

        // Clean up
        let _ = fs::remove_file(&temp_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Header mismatch"));
    }

    #[test]
    fn test_load_existing_results_without_output_columns() {
        use std::io::Write;

        // Create a temporary CSV file WITHOUT stdout/stderr columns
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_runexp_no_output.csv");
        {
            let mut file = File::create(&temp_path).unwrap();
            writeln!(file, "BATCHSIZE,GPU,accuracy").unwrap();
            writeln!(file, "32,1,0.95").unwrap();
        }

        let expected_params = vec!["BATCHSIZE".to_string(), "GPU".to_string()];
        let expected_metrics = vec!["accuracy".to_string()];

        // Load WITHOUT preserve_output (should succeed)
        let result = load_existing_results(
            temp_path.to_str().unwrap(),
            &expected_params,
            &expected_metrics,
            false, // preserve_output = false and file has no output columns
            false, // stdout_only
            false, // stderr_only
        );

        // Clean up
        let _ = fs::remove_file(&temp_path);

        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].params.get("BATCHSIZE"), Some(&"32".to_string()));
        assert_eq!(results[0].params.get("GPU"), Some(&"1".to_string()));
        assert_eq!(results[0].metrics.get("accuracy"), Some(&"0.95".to_string()));
    }
}
