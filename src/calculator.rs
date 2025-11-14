use anyhow::Result;
use log::debug;

pub struct Calculator;

impl Calculator {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn evaluate(&mut self, expression: &str) -> Option<String> {
        let expr = expression.trim();
        if expr.is_empty() {
            return None;
        }

        match meval::eval_str(expr) {
            Ok(result) => {
                let formatted = if result.fract() == 0.0 && result.abs() < 1e15 {
                    format!("{}", result as i64)
                } else {
                    let s = format!("{}", result);
                    if s.len() > 10 {
                        format!("{:.6}", result)
                    } else {
                        s
                    }
                };

                debug!("Calculator: {:?} -> {:?}", expr, formatted);

                if formatted == expr {
                    return None;
                }

                Some(formatted)
            }
            Err(_) => None,
        }
    }
}
