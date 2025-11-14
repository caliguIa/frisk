use bincode::{Decode, Encode};
use nucleo::{Config as NucleoConfig, Nucleo};
use std::sync::Arc;

#[derive(Clone, Encode, Decode)]
pub enum ElementType {
    Application,
    CalculatorResult,
}

#[derive(Clone, Encode, Decode)]
pub struct Element {
    pub name: Arc<str>,
    pub value: Arc<str>,
    pub element_type: ElementType,
}

impl Element {
    pub fn new(name: String, value: String) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
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
            name: display_name.into(),
            value: value.into(),
            element_type: ElementType::CalculatorResult,
        }
    }
}

pub struct ElementList {
    pub inner: Vec<Element>,
    nucleo: Nucleo<usize>,
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

    pub fn search(&mut self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return (0..self.inner.len()).collect();
        }

        self.nucleo.restart(false);
        let injector = self.nucleo.injector();

        for (idx, _element) in self.inner.iter().enumerate() {
            injector.push(idx, |idx, cols| {
                cols[0] = self.inner[*idx].name.as_ref().into();
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
        let mut results = Vec::with_capacity(snapshot.matched_item_count() as usize);

        for idx in 0..snapshot.matched_item_count() {
            if let Some(item) = snapshot.get_matched_item(idx) {
                results.push(*item.data);
            }
        }

        results
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
