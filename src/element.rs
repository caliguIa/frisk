use bincode::{Decode, Encode};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config as MatcherConfig, Matcher, Utf32Str,
};

#[derive(Clone, Encode, Decode)]
pub enum ElementType {
    Application,
    CalculatorResult,
    SystemCommand,
    ClipboardHistory,
    NixPackage,
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

    pub fn new_system_command(name: String, command: String) -> Self {
        Self {
            name: name.into_boxed_str(),
            value: command.into_boxed_str(),
            element_type: ElementType::SystemCommand,
        }
    }

    pub fn new_clipboard_entry(name: String, value: String) -> Self {
        Self {
            name: name.into_boxed_str(),
            value: value.into_boxed_str(),
            element_type: ElementType::ClipboardHistory,
        }
    }

    pub fn new_nix_package(name: String, value: String) -> Self {
        Self {
            name: name.into_boxed_str(),
            value: value.into_boxed_str(),
            element_type: ElementType::NixPackage,
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
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut matches: Vec<(usize, u32)> = Vec::with_capacity(20);

        for (idx, element) in self.inner.iter().enumerate() {
            let haystack = Utf32Str::new(&element.name, &mut self.char_buf);

            if let Some(score) = pattern.score(haystack, &mut self.matcher) {
                matches.push((idx, score));
            }
        }

        matches.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        matches.into_iter().map(|(idx, _)| idx).collect()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
