use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Combination {
    pub params: HashMap<String, String>,
    pub param_order: Vec<String>, // Preserve the order of parameters
}

pub fn evaluate_params(params: &[(String, String)]) -> Result<Vec<Combination>, String> {
    // Topologically sort parameters based on dependencies
    let sorted_params = topological_sort(params)?;
    
    // Store the original order for output
    let param_order: Vec<String> = params.iter().map(|(name, _)| name.clone()).collect();
    
    // Build combinations incrementally, evaluating each parameter in dependency order
    let mut combinations: Vec<HashMap<String, String>> = vec![HashMap::new()];

    for name in &sorted_params {
        // Find the value expression for this parameter
        let value = params
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
            .ok_or_else(|| format!("Parameter {} not found", name))?;
        
        let mut new_combinations = Vec::new();

        for combo in &combinations {
            // Normalize context keys to uppercase for case-insensitive lookup
            let normalized_context: HashMap<String, String> = combo
                .iter()
                .map(|(k, v)| (k.to_uppercase(), v.clone()))
                .collect();

            // Evaluate the expression in the context of this combination
            let values = evaluate_expression(value, &normalized_context)?;

            for val in values {
                let mut new_combo = combo.clone();
                new_combo.insert(name.clone(), val);
                new_combinations.push(new_combo);
            }
        }

        combinations = new_combinations;
    }

    Ok(combinations
        .into_iter()
        .map(|params| Combination { params, param_order: param_order.clone() })
        .collect())
}

// Topologically sort parameters based on their dependencies
fn topological_sort(params: &[(String, String)]) -> Result<Vec<String>, String> {
    // Build dependency graph
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    let param_names: HashSet<String> = params.iter().map(|(name, _)| name.clone()).collect();
    
    for (name, value) in params {
        let dependencies = extract_variables(value);
        // Only include dependencies that are actually parameters
        let filtered_deps: HashSet<String> = dependencies
            .into_iter()
            .filter(|dep| param_names.contains(dep))
            .collect();
        deps.insert(name.clone(), filtered_deps);
    }
    
    // Perform topological sort using Kahn's algorithm
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for name in &param_names {
        in_degree.insert(name.clone(), 0);
    }
    
    // Calculate in-degrees: for each parameter, its in-degree is the number of parameters it depends on
    for (name, dependencies) in &deps {
        *in_degree.get_mut(name).unwrap() = dependencies.len();
    }
    
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(name, _)| name.clone())
        .collect();
    
    // Sort the initial queue by the original parameter order to maintain stability
    let param_positions: HashMap<String, usize> = params
        .iter()
        .enumerate()
        .map(|(i, (name, _))| (name.clone(), i))
        .collect();
    queue.sort_by_key(|name| param_positions.get(name).unwrap_or(&usize::MAX));
    
    let mut result = Vec::new();
    
    while !queue.is_empty() {
        let node = queue.remove(0); // Take from front to maintain order
        result.push(node.clone());
        
        // Find all parameters that depend on this node
        for (name, dependencies) in &deps {
            if dependencies.contains(&node) {
                let degree = in_degree.get_mut(name).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push(name.clone());
                }
            }
        }
        
        // Keep queue sorted by original order
        queue.sort_by_key(|name| param_positions.get(name).unwrap_or(&usize::MAX));
    }
    
    if result.len() != param_names.len() {
        // Circular dependency detected
        return Err("Circular dependency detected in parameter definitions".to_string());
    }
    
    Ok(result)
}

// Extract variable names from an expression
fn extract_variables(expr: &str) -> HashSet<String> {
    let mut variables = HashSet::new();
    
    // Split by comma first
    for part in expr.split(',') {
        let part = part.trim();
        
        // Skip ranges (contain ':')
        if part.contains(':') {
            // Still need to check for variables in range bounds
            for range_part in part.split(':') {
                extract_variables_from_term(range_part.trim(), &mut variables);
            }
            continue;
        }
        
        extract_variables_from_term(part, &mut variables);
    }
    
    variables
}

// Extract variables from a single term (no commas)
fn extract_variables_from_term(term: &str, variables: &mut HashSet<String>) {
    // Parse through the expression looking for variable names
    // Variables are alphabetic identifiers that aren't just numbers
    
    // Split by operators but keep track of tokens
    let mut current_token = String::new();
    
    for ch in term.chars() {
        if ch.is_alphabetic() || ch == '_' {
            current_token.push(ch);
        } else {
            if !current_token.is_empty() {
                // Normalize to uppercase for consistency
                variables.insert(current_token.to_uppercase());
                current_token.clear();
            }
        }
    }
    
    if !current_token.is_empty() {
        variables.insert(current_token.to_uppercase());
    }
}

fn evaluate_expression(
    expr: &str,
    context: &HashMap<String, String>,
) -> Result<Vec<String>, String> {
    // Split by comma for multiple values
    let parts: Vec<&str> = expr.split(',').collect();
    let mut results = Vec::new();

    for part in parts {
        let part = part.trim();

        // Check for range (e.g., "1:4" or "1:10:2")
        if part.contains(':') {
            let range_parts: Vec<&str> = part.split(':').collect();
            if range_parts.len() == 2 {
                let start = parse_int_expr(range_parts[0].trim(), context)?;
                let end = parse_int_expr(range_parts[1].trim(), context)?;
                if start >= end {
                    return Err(format!(
                        "Empty range {}:{} (start must be less than end)",
                        start, end
                    ));
                }
                for i in start..end {
                    results.push(i.to_string());
                }
                continue;
            } else if range_parts.len() == 3 {
                let start = parse_int_expr(range_parts[0].trim(), context)?;
                let end = parse_int_expr(range_parts[1].trim(), context)?;
                let step = parse_int_expr(range_parts[2].trim(), context)?;

                if step == 0 {
                    return Err("Range step cannot be zero".to_string());
                }

                if (step > 0 && start >= end) || (step < 0 && start <= end) {
                    return Err(format!("Invalid range {}:{}:{}", start, end, step));
                }

                let mut i = start;
                while (step > 0 && i < end) || (step < 0 && i > end) {
                    results.push(i.to_string());
                    i += step;
                }
                continue;
            }
        }

        // Try to parse as expression
        match parse_expr(part, context) {
            Ok(val) => results.push(val),
            Err(_) => {
                // If parsing fails, treat as literal string
                results.push(part.to_string());
            }
        }
    }

    Ok(results)
}

fn parse_expr(expr: &str, context: &HashMap<String, String>) -> Result<String, String> {
    let expr = expr.trim();

    // Try to parse as integer expression first
    match parse_int_expr(expr, context) {
        Ok(val) => Ok(val.to_string()),
        Err(_) => {
            // Not a numeric expression, return as-is
            Ok(expr.to_string())
        }
    }
}

fn parse_int_expr(expr: &str, context: &HashMap<String, String>) -> Result<i64, String> {
    let expr = expr.trim();

    // Handle addition (lowest precedence)
    if expr.contains('+') {
        let parts: Vec<&str> = expr.split('+').collect();
        let mut sum = 0;
        for part in parts {
            sum += parse_mult_expr(part.trim(), context)?;
        }
        return Ok(sum);
    }

    parse_mult_expr(expr, context)
}

fn parse_mult_expr(expr: &str, context: &HashMap<String, String>) -> Result<i64, String> {
    let expr = expr.trim();

    // Handle multiplication with explicit *
    if expr.contains('*') {
        let parts: Vec<&str> = expr.split('*').collect();
        let mut product = 1;
        for part in parts {
            product *= parse_exp_expr(part.trim(), context)?;
        }
        return Ok(product);
    }

    parse_exp_expr(expr, context)
}

fn parse_exp_expr(expr: &str, context: &HashMap<String, String>) -> Result<i64, String> {
    let expr = expr.trim();

    // Handle exponentiation (highest precedence for binary operators)
    if expr.contains('^') {
        let parts: Vec<&str> = expr.split('^').collect();
        if parts.len() == 2 {
            let base = parse_atom_expr(parts[0].trim(), context)?;
            let exp = parse_exp_expr(parts[1].trim(), context)?; // Right associative
            return Ok(base.pow(exp as u32));
        }
    }

    parse_atom_expr(expr, context)
}

fn parse_atom_expr(expr: &str, context: &HashMap<String, String>) -> Result<i64, String> {
    let expr = expr.trim();

    // Handle implicit multiplication (e.g., "2n", "32gpu")
    // Try to find where number ends and variable begins
    let mut num_end = 0;
    for (i, c) in expr.chars().enumerate() {
        if !c.is_ascii_digit() {
            num_end = i;
            break;
        }
    }

    if num_end > 0 && num_end < expr.len() {
        let num_part = &expr[..num_end];
        let var_part = &expr[num_end..];
        let num = num_part.parse::<i64>().map_err(|_| "Invalid number")?;
        let var_val = parse_atom_expr(var_part, context)?;
        return Ok(num * var_val);
    }

    // Check if it's a variable (context keys are already normalized to uppercase)
    let upper_expr = expr.to_uppercase();
    if let Some(value) = context.get(&upper_expr) {
        return value
            .parse::<i64>()
            .map_err(|_| format!("Variable {} is not a number", expr));
    }

    // Try to parse as literal number
    expr.parse::<i64>()
        .map_err(|_| format!("Cannot parse as number: {}", expr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combinations() {
        let params = vec![
            ("GPU".to_string(), "1,2,4".to_string()),
            ("BATCHSIZE".to_string(), "32,64".to_string()),
        ];
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 6); // 3 * 2
    }

    #[test]
    fn test_ranges() {
        // Basic range
        let combos = evaluate_params(&[("N".to_string(), "1:4".to_string())]).unwrap();
        assert_eq!(combos.len(), 3);
        assert_eq!(combos[0].params.get("N").unwrap(), "1");
        assert_eq!(combos[2].params.get("N").unwrap(), "3");

        // Positive step
        let combos = evaluate_params(&[("N".to_string(), "1:10:2".to_string())]).unwrap();
        assert_eq!(combos.len(), 5);
        assert_eq!(combos[0].params.get("N").unwrap(), "1");
        assert_eq!(combos[4].params.get("N").unwrap(), "9");

        // Negative step
        let combos = evaluate_params(&[("N".to_string(), "10:1:-2".to_string())]).unwrap();
        assert_eq!(combos.len(), 5);
        assert_eq!(combos[0].params.get("N").unwrap(), "10");
        assert_eq!(combos[4].params.get("N").unwrap(), "2");
    }

    #[test]
    fn test_expressions() {
        // Variable reference and implicit multiplication
        let params = vec![
            ("N".to_string(), "1,2".to_string()),
            ("GPU".to_string(), "n".to_string()),
            ("BATCHSIZE".to_string(), "32n".to_string()),
        ];
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 2);
        assert_eq!(combos[0].params.get("BATCHSIZE").unwrap(), "32");
        assert_eq!(combos[1].params.get("BATCHSIZE").unwrap(), "64");

        // Operator precedence: n+3*2 = 2+6 = 8
        let combos = evaluate_params(&[
            ("N".to_string(), "2".to_string()),
            ("VALUE".to_string(), "n+3*2".to_string()),
        ])
        .unwrap();
        assert_eq!(combos[0].params.get("VALUE").unwrap(), "8");

        // Operator precedence: n+n^2 = 2+4 = 6
        let combos = evaluate_params(&[
            ("N".to_string(), "2".to_string()),
            ("VALUE".to_string(), "n+n^2".to_string()),
        ])
        .unwrap();
        assert_eq!(combos[0].params.get("VALUE").unwrap(), "6");
    }

    #[test]
    fn test_literal_strings() {
        // Pure literals
        let combos =
            evaluate_params(&[("ROUTING".to_string(), "source,dest,both".to_string())]).unwrap();
        assert_eq!(combos.len(), 3);
        assert_eq!(combos[0].params.get("ROUTING").unwrap(), "source");

        // Mixed literals and numbers
        let combos =
            evaluate_params(&[("MODE".to_string(), "train,test,1,2".to_string())]).unwrap();
        assert_eq!(combos.len(), 4);
        assert_eq!(combos[0].params.get("MODE").unwrap(), "train");
        assert_eq!(combos[2].params.get("MODE").unwrap(), "1");
    }

    #[test]
    fn test_parameter_order_preserved() {
        // Test that parameter order is preserved in param_order field
        let params = vec![
            ("GPU".to_string(), "1,2".to_string()),
            ("BATCHSIZE".to_string(), "32,64".to_string()),
            ("LR".to_string(), "0.01".to_string()),
        ];
        let combos = evaluate_params(&params).unwrap();
        
        // Check that param_order matches input order
        assert_eq!(combos[0].param_order, vec!["GPU", "BATCHSIZE", "LR"]);
    }

    #[test]
    fn test_forward_references() {
        // Test that parameters can refer to variables defined later
        let params = vec![
            ("BATCHSIZE".to_string(), "32n".to_string()), // Refers to N, defined later
            ("N".to_string(), "1,2".to_string()),
            ("GPU".to_string(), "n".to_string()), // Also refers to N
        ];
        let combos = evaluate_params(&params).unwrap();
        
        assert_eq!(combos.len(), 2);
        
        // Check first combination
        assert_eq!(combos[0].params.get("N").unwrap(), "1");
        assert_eq!(combos[0].params.get("BATCHSIZE").unwrap(), "32");
        assert_eq!(combos[0].params.get("GPU").unwrap(), "1");
        
        // Check second combination
        assert_eq!(combos[1].params.get("N").unwrap(), "2");
        assert_eq!(combos[1].params.get("BATCHSIZE").unwrap(), "64");
        assert_eq!(combos[1].params.get("GPU").unwrap(), "2");
        
        // Check that param_order preserves input order, not dependency order
        assert_eq!(combos[0].param_order, vec!["BATCHSIZE", "N", "GPU"]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        // Test that circular dependencies are detected and reported
        let params = vec![
            ("A".to_string(), "b".to_string()), // A depends on B
            ("B".to_string(), "a".to_string()), // B depends on A - circular!
        ];
        let result = evaluate_params(&params);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency"));
    }

    #[test]
    fn test_loop_order() {
        // Test that first parameter changes least frequently (outer loop)
        let params = vec![
            ("GPU".to_string(), "1,2".to_string()),
            ("BATCHSIZE".to_string(), "32,64".to_string()),
        ];
        let combos = evaluate_params(&params).unwrap();
        
        assert_eq!(combos.len(), 4);
        
        // Expected order: GPU changes slowest (outer loop), BATCHSIZE changes fastest (inner loop)
        // (1,32), (1,64), (2,32), (2,64)
        assert_eq!(combos[0].params.get("GPU").unwrap(), "1");
        assert_eq!(combos[0].params.get("BATCHSIZE").unwrap(), "32");
        
        assert_eq!(combos[1].params.get("GPU").unwrap(), "1");
        assert_eq!(combos[1].params.get("BATCHSIZE").unwrap(), "64");
        
        assert_eq!(combos[2].params.get("GPU").unwrap(), "2");
        assert_eq!(combos[2].params.get("BATCHSIZE").unwrap(), "32");
        
        assert_eq!(combos[3].params.get("GPU").unwrap(), "2");
        assert_eq!(combos[3].params.get("BATCHSIZE").unwrap(), "64");
    }

    #[test]
    fn test_complex_forward_dependency() {
        // Test more complex dependency chains
        let params = vec![
            ("C".to_string(), "a+b".to_string()), // C depends on A and B
            ("B".to_string(), "2a".to_string()),   // B depends on A
            ("A".to_string(), "1,2".to_string()),  // A has no dependencies
        ];
        let combos = evaluate_params(&params).unwrap();
        
        assert_eq!(combos.len(), 2);
        
        // When A=1: B=2, C=1+2=3
        assert_eq!(combos[0].params.get("A").unwrap(), "1");
        assert_eq!(combos[0].params.get("B").unwrap(), "2");
        assert_eq!(combos[0].params.get("C").unwrap(), "3");
        
        // When A=2: B=4, C=2+4=6
        assert_eq!(combos[1].params.get("A").unwrap(), "2");
        assert_eq!(combos[1].params.get("B").unwrap(), "4");
        assert_eq!(combos[1].params.get("C").unwrap(), "6");
        
        // Param order should be preserved
        assert_eq!(combos[0].param_order, vec!["C", "B", "A"]);
    }
}
