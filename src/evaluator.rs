use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Combination {
    pub params: HashMap<String, String>,
}

pub fn evaluate_params(params: &[(String, String)]) -> Result<Vec<Combination>, String> {
    // Build combinations incrementally, evaluating each parameter in context
    let mut combinations: Vec<HashMap<String, String>> = vec![HashMap::new()];
    
    for (name, value) in params {
        let mut new_combinations = Vec::new();
        
        for combo in &combinations {
            // Normalize context keys to uppercase for case-insensitive lookup
            let normalized_context: HashMap<String, String> = combo.iter()
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
    
    Ok(combinations.into_iter().map(|params| Combination { params }).collect())
}

fn evaluate_expression(expr: &str, context: &HashMap<String, String>) -> Result<Vec<String>, String> {
    // Split by comma for multiple values
    let parts: Vec<&str> = expr.split(',').collect();
    let mut results = Vec::new();
    
    for part in parts {
        let part = part.trim();
        
        // Check for range (e.g., "1:4" or "1:10:2")
        if part.contains(':') {
            let range_parts: Vec<&str> = part.split(':').collect();
            if range_parts.len() == 2 {
                // Format: start:end (step defaults to 1)
                let start = parse_int_expr(range_parts[0].trim(), context)?;
                let end = parse_int_expr(range_parts[1].trim(), context)?;
                if start < end {
                    for i in start..end {
                        results.push(i.to_string());
                    }
                } else {
                    for i in (end..start).rev() {
                        results.push(i.to_string());
                    }
                }
                continue;
            } else if range_parts.len() == 3 {
                // Format: start:end:step
                let start = parse_int_expr(range_parts[0].trim(), context)?;
                let end = parse_int_expr(range_parts[1].trim(), context)?;
                let step = parse_int_expr(range_parts[2].trim(), context)?;
                
                if step == 0 {
                    return Err("Step cannot be zero in range expression".to_string());
                }
                
                if step > 0 && start < end {
                    let mut i = start;
                    while i < end {
                        results.push(i.to_string());
                        i += step;
                    }
                } else if step < 0 && start > end {
                    let mut i = start;
                    while i > end {
                        results.push(i.to_string());
                        i += step;
                    }
                } else {
                    return Err("Invalid range: step direction doesn't match start/end order".to_string());
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
        return value.parse::<i64>().map_err(|_| format!("Variable {} is not a number", expr));
    }
    
    // Try to parse as literal number
    expr.parse::<i64>().map_err(|_| format!("Cannot parse as number: {}", expr))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_values() {
        let params = vec![
            ("GPU".to_string(), "1,2,4".to_string()),
            ("BATCHSIZE".to_string(), "32,64".to_string()),
        ];
        
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 6); // 3 * 2
    }
    
    #[test]
    fn test_range() {
        // Test basic range start:end
        let params = vec![
            ("N".to_string(), "1:4".to_string()),
        ];
        
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 3); // 1, 2, 3
        assert_eq!(combos[0].params.get("N").unwrap(), "1");
        assert_eq!(combos[1].params.get("N").unwrap(), "2");
        assert_eq!(combos[2].params.get("N").unwrap(), "3");
        
        // Test range with step
        let params_step = vec![
            ("N".to_string(), "1:10:2".to_string()),
        ];
        
        let combos_step = evaluate_params(&params_step).unwrap();
        assert_eq!(combos_step.len(), 5); // 1, 3, 5, 7, 9
        assert_eq!(combos_step[0].params.get("N").unwrap(), "1");
        assert_eq!(combos_step[1].params.get("N").unwrap(), "3");
        assert_eq!(combos_step[2].params.get("N").unwrap(), "5");
        assert_eq!(combos_step[3].params.get("N").unwrap(), "7");
        assert_eq!(combos_step[4].params.get("N").unwrap(), "9");
    }
    
    #[test]
    fn test_expression() {
        let params = vec![
            ("N".to_string(), "1,2".to_string()),
            ("GPU".to_string(), "n".to_string()),
            ("BATCHSIZE".to_string(), "32n".to_string()),
        ];
        
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 2);
        assert_eq!(combos[0].params.get("N").unwrap(), "1");
        assert_eq!(combos[0].params.get("GPU").unwrap(), "1");
        assert_eq!(combos[0].params.get("BATCHSIZE").unwrap(), "32");
        assert_eq!(combos[1].params.get("N").unwrap(), "2");
        assert_eq!(combos[1].params.get("GPU").unwrap(), "2");
        assert_eq!(combos[1].params.get("BATCHSIZE").unwrap(), "64");
    }
    
    #[test]
    fn test_operator_precedence() {
        let params = vec![
            ("N".to_string(), "2".to_string()),
            ("VALUE".to_string(), "n+3*2".to_string()), // Should be 2+6=8, not (2+3)*2=10
        ];
        
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 1);
        assert_eq!(combos[0].params.get("VALUE").unwrap(), "8");
        
        let params2 = vec![
            ("N".to_string(), "2".to_string()),
            ("VALUE".to_string(), "n+n^2".to_string()), // Should be 2+4=6, not (2+2)^2=16
        ];
        
        let combos2 = evaluate_params(&params2).unwrap();
        assert_eq!(combos2.len(), 1);
        assert_eq!(combos2[0].params.get("VALUE").unwrap(), "6");
    }
}
