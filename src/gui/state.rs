use crate::calculator::Calculator;
use crate::config::Config;
use crate::element::{Element, ElementList, ElementType};
use anyhow::Result;
use log::info;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use std::process::Command;

pub struct AppState {
    pub config: Config,
    pub elements: ElementList,
    pub filtered_elements: Vec<Element>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub query: String,
    pub cursor_position: usize,
    pub should_exit: bool,
    pub dynamic_max_results: usize,
    pub menubar_height: f64,
    calculator: Option<Calculator>,
}

impl AppState {
    pub fn new(
        config: Config,
        elements: ElementList,
        window_height: f64,
        menubar_height: f64,
    ) -> Self {
        let font_size = config.font_size as f64;
        let calculator = Calculator::new().ok();
        Self {
            config,
            elements,
            filtered_elements: Vec::new(), // Start empty - will populate on first keystroke
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            cursor_position: 0,
            should_exit: false,
            dynamic_max_results: Self::calculate_max_results(
                window_height,
                font_size,
                menubar_height,
            ),
            menubar_height,
            calculator,
        }
    }

    fn calculate_max_results(window_height: f64, font_size: f64, menubar_height: f64) -> usize {
        let line_height = font_size + 15.0;
        let overhead = 20.0 + font_size + 40.0;
        let available_height = window_height - overhead - menubar_height;
        let max_results = (available_height / line_height).floor() as usize;

        info!(
            "Max results: window={}, font={}, available={}, max={}",
            window_height, font_size, available_height, max_results
        );

        max_results.clamp(3, 25)
    }

    pub fn update_search(&mut self) {
        let results = self.elements.search(&self.query);
        let mut filtered: Vec<Element> = results.into_iter().cloned().collect();

        if let Some(calc) = &mut self.calculator {
            if !self.query.is_empty() {
                if let Some(result) = calc.evaluate(&self.query) {
                    let calc_element = Element::new_calculator_result(self.query.clone(), result);
                    filtered.insert(0, calc_element);
                }
            }
        }

        self.filtered_elements = filtered;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn nav_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    pub fn nav_down(&mut self) {
        if self.selected_index < self.filtered_elements.len().saturating_sub(1) {
            self.selected_index += 1;
            let visible_end = self.scroll_offset + self.dynamic_max_results;
            if self.selected_index >= visible_end {
                self.scroll_offset = self.selected_index - self.dynamic_max_results + 1;
            }
        }
    }

    pub fn execute_selected(&mut self) -> Result<()> {
        if let Some(element) = self.filtered_elements.get(self.selected_index) {
            match element.element_type {
                ElementType::Application => {
                    info!("Launching: {}", element.name);
                    Command::new("open")
                        .arg("-a")
                        .arg(&element.value)
                        .spawn()
                        .map_err(|e| anyhow::anyhow!("Failed to launch: {}", e))?;
                    self.should_exit = true;
                }
                ElementType::CalculatorResult => {
                    info!("Copying: {}", element.value);
                    let pasteboard = NSPasteboard::generalPasteboard();
                    pasteboard.clearContents();
                    let ns_string = NSString::from_str(&element.value);
                    if unsafe { pasteboard.setString_forType(&ns_string, NSPasteboardTypeString) } {
                        self.should_exit = true;
                    } else {
                        return Err(anyhow::anyhow!("Failed to copy"));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let mut new_pos = self.cursor_position - 1;
            while new_pos > 0 && !self.query.is_char_boundary(new_pos) {
                new_pos -= 1;
            }
            self.query.remove(new_pos);
            self.cursor_position = new_pos;
            self.update_search();
        }
    }

    pub fn delete_word(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let before_cursor = &self.query[..self.cursor_position];
        let trimmed = before_cursor.trim_end();

        if trimmed.is_empty() {
            self.query.drain(..self.cursor_position);
            self.cursor_position = 0;
        } else {
            let mut word_start = trimmed.len();
            for (i, c) in trimmed.char_indices().rev() {
                if c.is_whitespace() {
                    word_start = i + c.len_utf8();
                    break;
                }
                if i == 0 {
                    word_start = 0;
                }
            }
            self.query.drain(word_start..self.cursor_position);
            self.cursor_position = word_start;
        }
        self.update_search();
    }

    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.update_search();
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            let mut new_pos = self.cursor_position - 1;
            while new_pos > 0 && !self.query.is_char_boundary(new_pos) {
                new_pos -= 1;
            }
            self.cursor_position = new_pos;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.query.len() {
            let mut new_pos = self.cursor_position + 1;
            while new_pos < self.query.len() && !self.query.is_char_boundary(new_pos) {
                new_pos += 1;
            }
            self.cursor_position = new_pos;
        }
    }

    pub fn autocomplete(&mut self) {
        if let Some(element) = self.filtered_elements.get(self.selected_index) {
            self.query = element.name.clone();
            self.cursor_position = self.query.len();
            self.update_search();
        }
    }

    pub fn terminate(&self) {
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).terminate(None);
        }
    }
}
