use crate::error::Result;

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

        match evalexpr::eval(expr) {
            Ok(result) => {
                let formatted = match result {
                    evalexpr::Value::Int(i) => i.to_string(),
                    evalexpr::Value::Float(f) => {
                        if f.fract() == 0.0 && f.abs() < 1e15 {
                            format!("{}", f as i64)
                        } else {
                            let s = format!("{}", f);
                            if s.len() > 10 {
                                format!("{:.6}", f)
                            } else {
                                s
                            }
                        }
                    }
                    evalexpr::Value::Boolean(b) => b.to_string(),
                    _ => return None,
                };

                crate::log!("Calculator: {:?} -> {:?}", expr, formatted);

                if formatted == expr {
                    return None;
                }

                Some(formatted)
            }
            Err(_) => None,
        }
    }
}
