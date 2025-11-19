use crate::core::calculator::Calculator;
use crate::core::config::Config;
use crate::core::element::{Element, ElementList, ElementType};
use crate::core::error::{Error, Result};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use std::process::Command;

pub struct AppState {
    pub config: Config,
    pub elements: ElementList,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub query: String,
    pub cursor_position: usize,
    pub should_exit: bool,
    pub dynamic_max_results: usize,
    pub menubar_height: f64,
    calculator: Option<Calculator>,
    pub calculator_result: Option<Element>,
    pub prompt_query_cache: String,
    pub cursor_text_cache: String,
}

impl AppState {
    pub fn new(
        config: Config,
        elements: ElementList,
        window_height: f64,
        menubar_height: f64,
    ) -> Self {
        let font_size = config.font_size as f64;
        let max_results = Self::calculate_max_results(window_height, font_size, menubar_height);

        let mut state = Self {
            config,
            elements,
            filtered_indices: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            query: String::with_capacity(32),
            cursor_position: 0,
            should_exit: false,
            dynamic_max_results: max_results,
            menubar_height,
            calculator: None,
            calculator_result: None,
            prompt_query_cache: String::with_capacity(64),
            cursor_text_cache: String::with_capacity(64),
        };
        state.update_search();
        state
    }

    fn calculate_max_results(window_height: f64, font_size: f64, menubar_height: f64) -> usize {
        let line_height = font_size + 15.0;
        let overhead = 20.0 + font_size + 40.0;
        let available_height = window_height - overhead - menubar_height;
        let max_results = (available_height / line_height).floor() as usize;

        crate::log!(
            "Max results: window={}, font={}, available={}, max={}",
            window_height,
            font_size,
            available_height,
            max_results
        );

        max_results.clamp(3, 25)
    }

    pub fn update_search(&mut self) {
        // Normal fuzzy search with calculator
        let indices = self.elements.search(&self.query);

        if self.calculator.is_none() && !self.query.is_empty() {
            self.calculator = Calculator::new().ok();
        }

        self.calculator_result = None;
        if let Some(calc) = &mut self.calculator {
            if !self.query.is_empty() {
                if let Some(result) = calc.evaluate(&self.query) {
                    self.calculator_result = Some(Element {
                        name: result.clone().into_boxed_str(),
                        value: result.into_boxed_str(),
                        element_type: crate::core::element::ElementType::CalculatorResult,
                    });
                }
            }
        }

        self.filtered_indices = indices;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn update_string_caches(&mut self) {
        self.prompt_query_cache.clear();
        self.prompt_query_cache.push_str(&self.config.prompt);
        self.prompt_query_cache.push_str(&self.query);

        self.cursor_text_cache.clear();
        self.cursor_text_cache.push_str(&self.config.prompt);
        self.cursor_text_cache
            .push_str(&self.query[..self.cursor_position]);
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
        let total_results =
            (self.calculator_result.is_some() as usize) + self.filtered_indices.len();
        if self.selected_index < total_results.saturating_sub(1) {
            self.selected_index += 1;
            let visible_end = self.scroll_offset + self.dynamic_max_results;
            if self.selected_index >= visible_end {
                self.scroll_offset = self.selected_index - self.dynamic_max_results + 1;
            }
        }
    }

    pub fn execute_selected(&mut self) -> Result<()> {
        if self.selected_index == 0 && self.calculator_result.is_some() {
            if let Some(calc_result) = &self.calculator_result {
                crate::log!("Copying calculator result: {}", calc_result.value);
                let pasteboard = NSPasteboard::generalPasteboard();
                pasteboard.clearContents();
                let ns_string = NSString::from_str(&calc_result.value);
                if unsafe { pasteboard.setString_forType(&ns_string, NSPasteboardTypeString) } {
                    self.should_exit = true;
                } else {
                    return Err(Error::new("Failed to copy"));
                }
            }
        } else {
            let app_idx = if self.calculator_result.is_some() {
                self.selected_index - 1
            } else {
                self.selected_index
            };

            if let Some(&idx) = self.filtered_indices.get(app_idx) {
                if let Some(element) = self.elements.inner.get(idx) {
                    match element.element_type {
                        ElementType::Application => {
                            crate::log!("Launching: {}", element.name);
                            Command::new("open")
                                .arg("-a")
                                .arg(element.value.as_ref())
                                .spawn()
                                .map_err(|e| Error::new(format!("Failed to launch: {}", e)))?;
                            self.should_exit = true;
                        }
                        ElementType::CalculatorResult => {
                            crate::log!("Copying: {}", element.value);
                            let pasteboard = NSPasteboard::generalPasteboard();
                            pasteboard.clearContents();
                            let ns_string = NSString::from_str(&element.value);
                            if unsafe {
                                pasteboard.setString_forType(&ns_string, NSPasteboardTypeString)
                            } {
                                self.should_exit = true;
                            } else {
                                return Err(Error::new("Failed to copy"));
                            }
                        }
                        ElementType::SystemCommand => {
                            crate::log!("Executing system command: {}", element.name);
                            let command = element.value.as_ref();

                            if command.trim().starts_with("frisk ") || command.trim() == "frisk" {
                                // Parse frisk arguments and reload current instance
                                let args: Vec<&str> = command.split_whitespace().skip(1).collect();
                                let mut apps = false;
                                let mut homebrew = false;
                                let mut clipboard = false;
                                let mut commands = false;
                                let mut nixpkgs = false;
                                let sources = vec![];
                                let mut prompt = None;
                                let mut i = 0;

                                while i < args.len() {
                                    match args[i] {
                                        "--apps" => apps = true,
                                        "--homebrew" => homebrew = true,
                                        "--clipboard" => clipboard = true,
                                        "--commands" => commands = true,
                                        "--nixpkgs" => nixpkgs = true,
                                        "--prompt" | "-p" => {
                                            if i + 1 < args.len() {
                                                i += 1;
                                                // Remove quotes if present
                                                let p = args[i];
                                                let p = p.trim_matches(|c| c == '"' || c == '\'');
                                                prompt = Some(p.to_string());
                                            }
                                        }
                                        _ => {}
                                    }
                                    i += 1;
                                }

                                self.handle_reload(apps, homebrew, clipboard, commands, nixpkgs, sources, prompt);
                                return Ok(());
                            }

                            Command::new("sh")
                                .arg("-c")
                                .arg(command)
                                .spawn()
                                .map_err(|e| Error::new(format!("Failed to execute: {}", e)))?;
                            self.should_exit = true;
                        }
                        ElementType::ClipboardHistory => {
                            crate::log!("Copying clipboard history entry: {}", element.value);
                            let pasteboard = NSPasteboard::generalPasteboard();
                            pasteboard.clearContents();
                            let ns_string = NSString::from_str(&element.value);
                            if unsafe {
                                pasteboard.setString_forType(&ns_string, NSPasteboardTypeString)
                            } {
                                self.should_exit = true;
                            } else {
                                return Err(Error::new("Failed to copy"));
                            }
                        }
                        ElementType::NixPackage => {
                            crate::log!("Copying nixpkg name: {}", element.value);
                            let pasteboard = NSPasteboard::generalPasteboard();
                            pasteboard.clearContents();
                            let ns_string = NSString::from_str(&element.value);
                            if unsafe {
                                pasteboard.setString_forType(&ns_string, NSPasteboardTypeString)
                            } {
                                self.should_exit = true;
                            } else {
                                return Err(Error::new("Failed to copy"));
                            }
                        }
                        ElementType::RustCrate => {
                            crate::log!("Opening crate URL: {}", element.value);
                            Command::new("open")
                                .arg(element.value.as_ref())
                                .spawn()
                                .map_err(|e| Error::new(format!("Failed to open URL: {}", e)))?;
                            self.should_exit = true;
                        }
                        ElementType::HomebrewPackage => {
                            crate::log!("Opening homebrew URL: {}", element.value);
                            Command::new("open")
                                .arg(element.value.as_ref())
                                .spawn()
                                .map_err(|e| Error::new(format!("Failed to open URL: {}", e)))?;
                            self.should_exit = true;
                        }
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

    pub fn delete_to_start(&mut self) {
        if self.cursor_position > 0 {
            self.query.drain(..self.cursor_position);
            self.cursor_position = 0;
            self.update_search();
        }
    }

    pub fn paste(&mut self) {
        let pasteboard = NSPasteboard::generalPasteboard();
        if let Some(text) = unsafe { pasteboard.stringForType(NSPasteboardTypeString) } {
            let paste_str = text.to_string();
            for c in paste_str.chars() {
                if !c.is_control() || c == '\n' || c == '\t' {
                    // Convert newlines and tabs to spaces
                    if c == '\n' || c == '\t' {
                        self.query.insert(self.cursor_position, ' ');
                        self.cursor_position += 1;
                    } else {
                        self.query.insert(self.cursor_position, c);
                        self.cursor_position += c.len_utf8();
                    }
                }
            }
            self.update_search();
        }
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
        if let Some(&idx) = self.filtered_indices.get(self.selected_index) {
            if let Some(element) = self.elements.inner.get(idx) {
                self.query.clear();
                self.query.push_str(&element.name);
                self.cursor_position = self.query.len();
                self.update_search();
            }
        }
    }

    pub fn terminate(&self) {
        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).terminate(None);
        }
    }

    pub fn handle_reload(
        &mut self,
        apps: bool,
        homebrew: bool,
        clipboard: bool,
        commands: bool,
        nixpkgs: bool,
        sources: Vec<String>,
        prompt: Option<String>,
    ) {
        crate::log!(
            "Reloading with: apps={}, homebrew={}, clipboard={}, commands={}, nixpkgs={}, sources={:?}, prompt={:?}",
            apps,
            homebrew,
            clipboard,
            commands,
            nixpkgs,
            sources,
            prompt
        );

        // Update prompt if provided
        if let Some(new_prompt) = prompt {
            self.config.prompt = new_prompt;
        }

        // Clear query on reload
        self.query.clear();
        self.cursor_position = 0;

        let mut new_elements = crate::core::element::ElementList::new();

        if apps {
            if let Ok(Some(app_list)) = crate::loader::load_binary_source("apps.bin") {
                for app in app_list {
                    new_elements.add(app);
                }
            }
        }

        if homebrew {
            if let Ok(Some(brew_list)) = crate::loader::load_binary_source("homebrew.bin") {
                for item in brew_list {
                    new_elements.add(item);
                }
            }
        }

        if clipboard {
            if let Ok(Some(clip_list)) = crate::loader::load_binary_source("clipboard.bin") {
                for item in clip_list {
                    new_elements.add(item);
                }
            }
        }

        if nixpkgs {
            if let Ok(Some(nix_list)) = crate::loader::load_binary_source("nixpkgs.bin") {
                for item in nix_list {
                    new_elements.add(item);
                }
            }
        }

        for source_path in sources {
            if let Ok(items) =
                crate::loader::load_binary_file(&std::path::PathBuf::from(source_path))
            {
                for item in items {
                    new_elements.add(item);
                }
            }
        }

        if commands {
            if let Ok(commands_config) = crate::core::commands::CommandsConfig::load() {
                for cmd in commands_config.to_elements() {
                    new_elements.add(cmd);
                }
            }
        }

        self.elements = new_elements;
        self.update_search();
        crate::log!("Reloaded {} elements", self.elements.len());
    }
}
