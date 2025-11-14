use bincode::{Decode, Encode};
use nucleo_matcher::{Matcher, Config as MatcherConfig, pattern::{Pattern, CaseMatching, Normalization}, Utf32Str};

#[derive(Clone, Encode, Decode)]
pub enum ElementType {
    Application,
    CalculatorResult,
}

#[derive(Clone, Encode, Decode)]
pub struct Element {
    pub name: Box<str>,
    pub value: Box<str>,
    pub element_type: ElementType,
}

impl Element {
    pub fn new(name: String, value: String) -> Self {
        Self {
            name: name.into_boxed_str(),
            value: value.into_boxed_str(),
            element_type: ElementType::Application,
        }
    }

    pub fn new_calculator_result(expression: String, result: String) -> Self {
        let mut display_name = String::with_capacity(expression.len() + result.len() + 3);
        display_name.push_str(&expression);

        if result.starts_with("= ") || result.starts_with("≈ ") {
            display_name.push(' ');
            display_name.push_str(&result);
        } else {
            display_name.push_str(" = ");
            display_name.push_str(&result);
        }

        let value = result
            .trim_start_matches("= ")
            .trim_start_matches("≈ ")
            .trim()
            .to_string();

        Self {
            name: display_name.into_boxed_str(),
            value: value.into_boxed_str(),
            element_type: ElementType::CalculatorResult,
        }
    }
}

pub struct ElementList {
    pub inner: Vec<Element>,
    matcher: Matcher,
    char_buf: Vec<char>,
}

impl ElementList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            matcher: Matcher::new(MatcherConfig::DEFAULT),
            char_buf: Vec::with_capacity(256),
        }
    }

    pub fn add(&mut self, element: Element) {
        self.inner.push(element);
    }

    pub fn search(&mut self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();  // Don't show all apps when query is empty
        }

        // Parse the pattern from the query string
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

        let mut matches: Vec<(usize, u32)> = Vec::with_capacity(20);  // Pre-allocate for typical result count
        
        for (idx, element) in self.inner.iter().enumerate() {
            // Create UTF-32 representation of haystack
            let haystack = Utf32Str::new(&element.name, &mut self.char_buf);
            
            if let Some(score) = pattern.score(haystack, &mut self.matcher) {
                matches.push((idx, score));
            }
        }

        // Sort by score descending
        matches.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        matches.into_iter().map(|(idx, _)| idx).collect()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
