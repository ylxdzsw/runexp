use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Combination {
    pub params: HashMap<String, String>,
}

pub fn evaluate_params(params: &[(String, String)]) -> Result<Vec<Combination>, String> {
    // Build combinations incrementally, evaluating each parameter in context
    let mut combinations = vec![HashMap::new()];
    
    for (name, value) in params {
        let mut new_combinations = Vec::new();
        
        for combo in &combinations {
            // Evaluate the expression in the context of this combination
            let values = evaluate_expression(value, combo)?;
            
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
        
        // Check for range (e.g., "1..4")
        if part.contains("..") {
            let range_parts: Vec<&str> = part.split("..").collect();
            if range_parts.len() == 2 {
                let start = parse_int_expr(range_parts[0].trim(), context)?;
                let end = parse_int_expr(range_parts[1].trim(), context)?;
                for i in start..end {
                    results.push(i.to_string());
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
    
    // Handle exponentiation (highest precedence)
    if expr.contains('^') {
        let parts: Vec<&str> = expr.split('^').collect();
        if parts.len() == 2 {
            let base = parse_int_expr(parts[0].trim(), context)?;
            let exp = parse_int_expr(parts[1].trim(), context)?;
            return Ok(base.pow(exp as u32));
        }
    }
    
    // Handle addition
    if expr.contains('+') {
        let parts: Vec<&str> = expr.split('+').collect();
        let mut sum = 0;
        for part in parts {
            sum += parse_int_expr(part.trim(), context)?;
        }
        return Ok(sum);
    }
    
    // Handle multiplication with explicit *
    if expr.contains('*') {
        let parts: Vec<&str> = expr.split('*').collect();
        let mut product = 1;
        for part in parts {
            product *= parse_int_expr(part.trim(), context)?;
        }
        return Ok(product);
    }
    
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
        let var_val = parse_int_expr(var_part, context)?;
        return Ok(num * var_val);
    }
    
    // Check if it's a variable (lowercase version)
    let lower_expr = expr.to_lowercase();
    if context.contains_key(&lower_expr) {
        let val = &context[&lower_expr];
        return val.parse::<i64>().map_err(|_| format!("Variable {} is not a number", expr));
    }
    
    // Check if it's a variable (uppercase version)
    let upper_expr = expr.to_uppercase();
    if context.contains_key(&upper_expr) {
        let val = &context[&upper_expr];
        return val.parse::<i64>().map_err(|_| format!("Variable {} is not a number", expr));
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
        let params = vec![
            ("N".to_string(), "1..4".to_string()),
        ];
        
        let combos = evaluate_params(&params).unwrap();
        assert_eq!(combos.len(), 3); // 1, 2, 3
        assert_eq!(combos[0].params.get("N").unwrap(), "1");
        assert_eq!(combos[1].params.get("N").unwrap(), "2");
        assert_eq!(combos[2].params.get("N").unwrap(), "3");
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
}
