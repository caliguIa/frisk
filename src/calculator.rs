use anyhow::Result;
use log::debug;
use numbat::{module_importer::BuiltinModuleImporter, resolver::CodeSource, Context};

pub struct Calculator {
    ctx: Context,
}

impl Calculator {
    pub fn new() -> Result<Self> {
        let module_importer = BuiltinModuleImporter::default();
        let mut ctx = Context::new(module_importer);
        let _ = ctx.interpret("use prelude", CodeSource::Internal)?;
        Ok(Self { ctx })
    }

    pub fn evaluate(&mut self, expression: &str) -> Option<String> {
        if expression.trim().is_empty() {
            return None;
        }

        match self.ctx.interpret(expression, CodeSource::Text) {
            Ok((statements, result)) => {
                let markup =
                    result.to_markup(statements.last(), self.ctx.dimension_registry(), true, true);
                let formatted = numbat::markup::plain_text_format(&markup, false);
                let formatted = formatted.trim();

                debug!("Calculator: {:?} -> {:?}", expression, formatted);

                if formatted.is_empty() {
                    return None;
                }

                if formatted.starts_with("= ") {
                    let after_equals = formatted.trim_start_matches("= ").trim();
                    if after_equals == expression.trim() {
                        return None;
                    }
                }

                if formatted == expression {
                    return None;
                }

                Some(formatted.to_string())
            }
            Err(_) => None,
        }
    }
}
