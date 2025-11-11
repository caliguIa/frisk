use anyhow::Result;
use log::debug;
use numbat::{
    module_importer::BuiltinModuleImporter, resolver::CodeSource, Context,
};

pub struct Calculator {
    ctx: Context,
}

impl Calculator {
    pub fn new() -> Result<Self> {
        let module_importer = BuiltinModuleImporter::default();
        let mut ctx = Context::new(module_importer);

        // Load standard prelude modules
        let _ = ctx.interpret("use prelude", CodeSource::Internal)?;

        Ok(Self { ctx })
    }

    pub fn evaluate(&mut self, expression: &str) -> Option<String> {
        // Skip if expression is empty or doesn't look like a calculation
        if expression.trim().is_empty() {
            return None;
        }

        // Try to evaluate
        match self.ctx.interpret(expression, CodeSource::Text) {
            Ok((statements, result)) => {
                // Convert result to markup and format as plain text
                let markup = result.to_markup(
                    statements.last(),
                    self.ctx.dimension_registry(),
                    true,
                    true,
                );
                let formatted = numbat::markup::plain_text_format(&markup, false);
                let formatted = formatted.trim();
                
                debug!("Calculator - expression: {:?}, formatted: {:?}", expression, formatted);
                
                // Filter out trivial results:
                // - Empty results
                // - Results that just echo the input
                // - Results that are just "= <same as input>" (e.g., "1" -> "= 1")
                if formatted.is_empty() {
                    return None;
                }
                
                // Check if it's just echoing the input with "= " prefix
                if formatted.starts_with("= ") {
                    let after_equals = formatted.trim_start_matches("= ").trim();
                    if after_equals == expression.trim() {
                        // It's just "= 1" for input "1" - not a real calculation
                        return None;
                    }
                }
                
                // Check if the result is identical to input (shouldn't happen but be safe)
                if formatted == expression {
                    return None;
                }
                
                Some(formatted.to_string())
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let mut calc = Calculator::new().unwrap();
        
        let result = calc.evaluate("2 + 2");
        assert!(result.is_some());
        
        let result = calc.evaluate("10 * 5");
        assert!(result.is_some());
    }

    #[test]
    fn test_units() {
        let mut calc = Calculator::new().unwrap();
        
        let result = calc.evaluate("5 km to miles");
        assert!(result.is_some());
    }
    
    #[test]
    fn test_trivial_inputs_filtered() {
        let mut calc = Calculator::new().unwrap();
        
        // Single number should be filtered out (just echoes input)
        let result = calc.evaluate("1");
        assert!(result.is_none(), "Single number '1' should not show result");
        
        let result = calc.evaluate("42");
        assert!(result.is_none(), "Single number '42' should not show result");
    }
    
    #[test]
    fn test_incomplete_expressions_filtered() {
        let mut calc = Calculator::new().unwrap();
        
        // Incomplete expressions should fail to parse
        let result = calc.evaluate("2 +");
        assert!(result.is_none(), "Incomplete expression '2 +' should not show result");
        
        let result = calc.evaluate("* 5");
        assert!(result.is_none(), "Invalid expression '* 5' should not show result");
    }
    
    #[test]
    fn test_valid_calculations_shown() {
        let mut calc = Calculator::new().unwrap();
        
        // Valid calculations should show results
        let result = calc.evaluate("2 + 1");
        assert!(result.is_some(), "Valid calculation '2 + 1' should show result");
        
        let result = calc.evaluate("10 / 2");
        assert!(result.is_some(), "Valid calculation '10 / 2' should show result");
        
        let result = calc.evaluate("sqrt(16)");
        assert!(result.is_some(), "Valid calculation 'sqrt(16)' should show result");
    }
}
