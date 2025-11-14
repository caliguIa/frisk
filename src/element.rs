use nucleo::{Config as NucleoConfig, Nucleo};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub enum ElementType {
    Application,
    CalculatorResult,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Element {
    pub name: String,
    pub value: String,
    pub element_type: ElementType,
}

impl Element {
    pub fn new(name: String, value: String) -> Self {
        Self {
            name,
            value,
            element_type: ElementType::Application,
        }
    }

    pub fn new_calculator_result(expression: String, result: String) -> Self {
        let display_name = if result.starts_with("= ") || result.starts_with("≈ ") {
            format!("{} {}", expression, result)
        } else {
            format!("{} = {}", expression, result)
        };

        let value = result
            .trim_start_matches("= ")
            .trim_start_matches("≈ ")
            .trim()
            .to_string();

        Self {
            name: display_name,
            value,
            element_type: ElementType::CalculatorResult,
        }
    }
}

pub struct ElementList {
    inner: Vec<Element>,
    nucleo: Nucleo<Element>,
}

impl ElementList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            nucleo: Nucleo::new(NucleoConfig::DEFAULT, Arc::new(|| {}), None, 1),
        }
    }

    pub fn add(&mut self, element: Element) {
        self.inner.push(element);
    }

    pub fn search(&mut self, query: &str) -> Vec<&Element> {
        if query.is_empty() {
            return self.inner.iter().collect();
        }

        self.nucleo.restart(false);
        let injector = self.nucleo.injector();

        for element in &self.inner {
            injector.push(element.clone(), |el, cols| {
                cols[0] = el.name.clone().into();
            });
        }

        self.nucleo.pattern.reparse(
            0,
            query,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
            false,
        );

        self.nucleo.tick(10);

        let snapshot = self.nucleo.snapshot();
        let mut results = Vec::new();

        for idx in 0..snapshot.matched_item_count() {
            if let Some(item) = snapshot.get_matched_item(idx) {
                if let Some(el) = self.inner.iter().find(|el| el.name == item.data.name) {
                    results.push(el);
                }
            }
        }

        results
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
