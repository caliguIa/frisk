use nucleo::{Config as NucleoConfig, Nucleo};
use serde::{Deserialize, Serialize};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::path::PathBuf;

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Application,
    CalculatorResult,
}

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub name: String,
    pub value: String,
    pub base_score: usize,
    pub icon_path: Option<PathBuf>,
    pub app_bundle_path: Option<PathBuf>,
    pub element_type: ElementType,
}

impl Element {
    pub fn new(name: String, value: String) -> Self {
        Self {
            name,
            value,
            base_score: 0,
            icon_path: None,
            app_bundle_path: None,
            element_type: ElementType::Application,
        }
    }

    pub fn new_calculator_result(expression: String, result: String) -> Self {
        // Numbat's formatted output can be:
        // "= 3" (exact result)
        // "≈ 3.10686 mi" (approximate result)
        // We want to display nicely without double symbols
        
        let display_name = if result.starts_with("= ") || result.starts_with("≈ ") {
            // Result already has "=" or "≈", just show "expression result"
            format!("{} {}", expression, result)
        } else {
            // Result doesn't have prefix, add "="
            format!("{} = {}", expression, result)
        };
        
        // For clipboard, strip the prefix symbols but keep the rest
        let value = result
            .trim_start_matches("= ")
            .trim_start_matches("≈ ")
            .trim()
            .to_string();
        
        Self {
            name: display_name,
            value,
            base_score: 1000, // High priority for calculator results
            icon_path: None,
            app_bundle_path: None,
            element_type: ElementType::CalculatorResult,
        }
    }

    pub fn with_icon(mut self, icon_path: Option<PathBuf>) -> Self {
        self.icon_path = icon_path;
        self
    }

    pub fn with_bundle_path(mut self, bundle_path: Option<PathBuf>) -> Self {
        self.app_bundle_path = bundle_path;
        self
    }

    pub fn with_base_score(mut self, score: usize) -> Self {
        self.base_score = score;
        self
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.base_score.cmp(&self.base_score) {
            Ordering::Equal => self.name.cmp(&other.name),
            e => e,
        }
    }
}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct ElementList {
    inner: Vec<Element>,
    nucleo: Nucleo<Element>,
}

impl std::fmt::Debug for ElementList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementList")
            .field("inner", &self.inner)
            .field("nucleo", &"<Nucleo>")
            .finish()
    }
}

impl ElementList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            nucleo: Nucleo::new(
                NucleoConfig::DEFAULT,
                std::sync::Arc::new(|| {}),
                None,
                1
            ),
        }
    }

    pub fn add(&mut self, element: Element) {
        self.inner.push(element);
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Element>,
    {
        self.inner.extend(iter);
    }

    pub fn search(&mut self, query: &str) -> Vec<&Element> {
        if query.is_empty() {
            return self.inner.iter().collect();
        }

        // Clear and re-inject all elements into nucleo
        self.nucleo.restart(false);
        let injector = self.nucleo.injector();
        for element in &self.inner {
            injector.push(element.clone(), |el, cols| {
                cols[0] = el.name.clone().into();
            });
        }

        // Update nucleo with the query
        self.nucleo.pattern.reparse(
            0,
            query,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
            false,
        );

        // Tick to process matches (10ms timeout)
        self.nucleo.tick(10);

        // Get snapshot of results (already sorted by score via nucleo)
        let snapshot = self.nucleo.snapshot();
        let match_count = snapshot.matched_item_count();
        
        // Collect matched items - nucleo returns them pre-sorted by score
        let mut results: Vec<&Element> = Vec::new();
        for idx in 0..match_count {
            if let Some(item) = snapshot.get_matched_item(idx) {
                // Find the element in our inner vec by matching the data
                if let Some(el) = self.inner.iter().find(|el| {
                    el.name == item.data.name && el.value == item.data.value
                }) {
                    results.push(el);
                }
            }
        }
        
        results
    }

    pub fn sort_by_score(&mut self) {
        self.inner.sort();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<Element> {
        self.inner.iter()
    }

    pub fn as_slice(&self) -> &[Element] {
        &self.inner
    }
}

impl IntoIterator for ElementList {
    type Item = Element;
    type IntoIter = std::vec::IntoIter<Element>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a ElementList {
    type Item = &'a Element;
    type IntoIter = std::slice::Iter<'a, Element>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}