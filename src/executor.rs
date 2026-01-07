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
}

pub fn execute_experiments(
    combinations: &[Combination],
    command: &[String],
    options: &Options,
) -> Result<(), String> {
    let mut results = Vec::new();
    let existing_results = if let Some(ref continue_file) = options.continue_from {
        load_existing_results(continue_file)?
    } else {
        Vec::new()
    };
    
    for (idx, combo) in combinations.iter().enumerate() {
        // Skip if already exists in continue_from results
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
            Ok(metrics) => {
                let result = ExperimentResult {
                    params: combo.params.clone(),
                    metrics,
                };
                results.push(result);
                // Store results immediately
                save_results(&results, "runexp_results.txt")?;
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
) -> Result<HashMap<String, String>, String> {
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
    
    // Check exit status
    if !output.status.success() {
        return Err(format!("Command failed with exit code: {:?}", output.status.code()));
    }
    
    // Parse output
    let mut parsed = HashMap::new();
    
    if options.stdout_only || !options.stderr_only {
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_output(&stdout, &mut parsed, &options.keywords);
    }
    
    if options.stderr_only || (!options.stdout_only && !options.stderr_only) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        parse_output(&stderr, &mut parsed, &options.keywords);
    }
    
    Ok(parsed)
}

fn parse_output(text: &str, results: &mut HashMap<String, String>, keywords: &[String]) {
    for line in text.lines() {
        // Look for patterns like "label: number" or "label number"
        let parts: Vec<&str> = line.split(&[':', ' ', '\t'][..]).collect();
        
        for i in 0..parts.len() {
            if let Ok(num) = parts[i].trim().parse::<f64>() {
                // Found a number, use preceding text as label
                let label = if i > 0 {
                    parts[..i].join(" ").trim().to_string()
                } else {
                    "value".to_string()
                };
                
                // Check if label matches keywords (if specified)
                if !keywords.is_empty() {
                    let matches = keywords.iter().any(|kw| label.to_lowercase().contains(&kw.to_lowercase()));
                    if !matches {
                        continue;
                    }
                }
                
                results.insert(label, num.to_string());
            }
        }
    }
}

fn save_results(results: &[ExperimentResult], filename: &str) -> Result<(), String> {
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
    
    // Write header
    let mut headers = param_names.clone();
    headers.extend(metric_names.clone());
    let header = headers.join("\t");
    writeln!(file, "{}", header).map_err(|e| format!("Failed to write to file: {}", e))?;
    
    // Write data
    for result in results {
        let mut values: Vec<String> = param_names.iter()
            .map(|name| result.params.get(name).unwrap_or(&String::new()).clone())
            .collect();
        
        let metric_values: Vec<String> = metric_names.iter()
            .map(|name| result.metrics.get(name).unwrap_or(&String::new()).clone())
            .collect();
        
        values.extend(metric_values);
        writeln!(file, "{}", values.join("\t")).map_err(|e| format!("Failed to write to file: {}", e))?;
    }
    
    Ok(())
}

fn load_existing_results(filename: &str) -> Result<Vec<ExperimentResult>, String> {
    let contents = fs::read_to_string(filename)
        .map_err(|_| format!("Could not read file: {}", filename))?;
    
    let mut lines = contents.lines();
    let header = lines.next().ok_or("Empty results file")?;
    let column_names: Vec<&str> = header.split('\t').collect();
    
    let mut results = Vec::new();
    for line in lines {
        let values: Vec<&str> = line.split('\t').collect();
        if values.len() != column_names.len() {
            continue;
        }
        
        // We need to determine which columns are params and which are metrics
        // For simplicity, we'll treat all columns as either params or metrics
        // In practice, we should track which are which
        let mut params = HashMap::new();
        let mut metrics = HashMap::new();
        
        for (name, value) in column_names.iter().zip(values.iter()) {
            // Heuristic: uppercase names are parameters
            if name.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
                params.insert(name.to_string(), value.to_string());
            } else {
                metrics.insert(name.to_string(), value.to_string());
            }
        }
        
        results.push(ExperimentResult { params, metrics });
    }
    
    Ok(results)
}

fn result_exists(existing: &[ExperimentResult], combo: &Combination) -> bool {
    existing.iter().any(|r| r.params == combo.params)
}
