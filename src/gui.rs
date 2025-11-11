#![deny(unsafe_op_in_unsafe_fn)]

use crate::calculator::Calculator;
use crate::config::{Color, Config};
use crate::element::{Element, ElementList, ElementType};
use anyhow::Result;
use log::{debug, info};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSBezierPath, NSColor,
    NSEvent, NSFont, NSScreen, NSView, NSWindow, NSWindowStyleMask, NSPasteboard, NSPasteboardTypeString,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use cocoa::foundation::{NSRect as CocoaNSRect, NSRange};
use std::cell::RefCell;
use std::os::raw::c_void;
use std::process::Command;

// Application state
struct AppState {
    config: Config,
    elements: ElementList,
    filtered_elements: Vec<Element>,
    selected_index: usize,
    scroll_offset: usize,
    query: String,
    cursor_position: usize,  // Cursor position in query string (byte index)
    should_exit: bool,
    window_height: f64,
    dynamic_max_results: usize,
    menubar_height: f64,  // Height to offset drawing from top
    calculator: Option<Calculator>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("elements", &self.elements)
            .field("filtered_elements", &self.filtered_elements)
            .field("selected_index", &self.selected_index)
            .field("scroll_offset", &self.scroll_offset)
            .field("query", &self.query)
            .field("cursor_position", &self.cursor_position)
            .field("should_exit", &self.should_exit)
            .field("window_height", &self.window_height)
            .field("dynamic_max_results", &self.dynamic_max_results)
            .field("menubar_height", &self.menubar_height)
            .field("calculator", &"<Calculator>")
            .finish()
    }
}

impl AppState {
    fn new(config: Config, elements: ElementList, window_height: f64, menubar_height: f64) -> Self {
        let font_size = config.font_size as f64; // Extract font_size before moving config
        let calculator = Calculator::new().ok(); // Initialize calculator, ignore errors
        let mut state = Self {
            config,
            elements,
            filtered_elements: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            query: String::new(),
            cursor_position: 0,
            should_exit: false,
            window_height,
            dynamic_max_results: Self::calculate_max_results(window_height, font_size, menubar_height),
            menubar_height,
            calculator,
        };
        state.update_search();
        state
    }

    fn calculate_max_results(window_height: f64, font_size: f64, menubar_height: f64) -> usize {
        // Calculate based on actual layout measurements
        let line_height = font_size + 15.0; // Match item_spacing in config (default 15.0)
        
        // Actual space used - optimized to fit more items:
        // - Top padding: 20 (window_padding_y)
        // - Prompt height: ~font_size
        // - Prompt to items: ~40 (accounting for some overlap/optimization)
        let overhead = 20.0 + font_size + 40.0; // Reduced from 60 to 40
        let available_height = window_height - overhead - menubar_height;
        let max_results = (available_height / line_height).floor() as usize;
        
        info!("Max results calculation: window_height={}, font_size={}, menubar_height={}, line_height={}, overhead={}, available_height={}, max_results={}",
              window_height, font_size, menubar_height, line_height, overhead, available_height, max_results);
        
        max_results.max(3).min(25) // Minimum 3, maximum 25 results
    }

    fn update_search(&mut self) {
        let mut results = self.elements.search(&self.query);
        let mut filtered: Vec<Element> = results.into_iter().cloned().collect();
        
        // Try to evaluate as calculator expression if we have a calculator
        if let Some(calc) = &mut self.calculator {
            if !self.query.is_empty() {
                if let Some(result) = calc.evaluate(&self.query) {
                    // Add calculator result at the beginning
                    let calc_element = Element::new_calculator_result(
                        self.query.clone(),
                        result,
                    );
                    filtered.insert(0, calc_element);
                }
            }
        }
        
        self.filtered_elements = filtered;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    fn nav_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            // Adjust scroll offset if we're going above the visible area
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    fn nav_down(&mut self) {
        if self.selected_index < self.filtered_elements.len().saturating_sub(1) {
            self.selected_index += 1;
            // Adjust scroll offset if we're going below the visible area
            let visible_end = self.scroll_offset + self.dynamic_max_results;
            if self.selected_index >= visible_end {
                self.scroll_offset = self.selected_index - self.dynamic_max_results + 1;
            }
        }
    }

    fn execute_selected(&mut self) -> Result<()> {
        if let Some(element) = self.filtered_elements.get(self.selected_index) {
            match element.element_type {
                ElementType::Application => {
                    info!("Launching: {}", element.name);

                    let mut cmd = Command::new("open");
                    cmd.arg("-a").arg(&element.value);

                    match cmd.spawn() {
                        Ok(_) => {
                            self.should_exit = true;
                            Ok(())
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to launch application: {}", e)),
                    }
                }
                ElementType::CalculatorResult => {
                    info!("Copying calculator result to clipboard: {}", element.value);
                    
                    // Copy to clipboard using NSPasteboard
                    unsafe {
                        let pasteboard = NSPasteboard::generalPasteboard();
                        pasteboard.clearContents();
                        let ns_string = NSString::from_str(&element.value);
                        let success = pasteboard.setString_forType(&ns_string, NSPasteboardTypeString);
                        
                        if success {
                            self.should_exit = true;
                            Ok(())
                        } else {
                            Err(anyhow::anyhow!("Failed to copy to clipboard"))
                        }
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            // Find the character boundary before cursor
            let byte_pos = self.cursor_position;
            let mut new_pos = byte_pos;
            while new_pos > 0 && !self.query.is_char_boundary(new_pos - 1) {
                new_pos -= 1;
            }
            if new_pos > 0 {
                new_pos -= 1;
                while new_pos > 0 && !self.query.is_char_boundary(new_pos) {
                    new_pos -= 1;
                }
            }
            self.query.remove(new_pos);
            self.cursor_position = new_pos;
            self.update_search();
        }
    }

    fn delete_word(&mut self) {
        if self.cursor_position == 0 {
            return;
        }
        
        let byte_pos = self.cursor_position;
        let before_cursor = &self.query[..byte_pos];
        
        // Find the start of the word to delete
        let trimmed = before_cursor.trim_end();
        if trimmed.is_empty() {
            // Only whitespace before cursor, delete it all
            self.query.drain(..byte_pos);
            self.cursor_position = 0;
        } else {
            // Find last word boundary
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
            self.query.drain(word_start..byte_pos);
            self.cursor_position = word_start;
        }
        self.update_search();
    }

    fn delete_to_start(&mut self) {
        if self.cursor_position > 0 {
            self.query.drain(..self.cursor_position);
            self.cursor_position = 0;
            self.update_search();
        }
    }

    fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
        self.update_search();
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            let mut new_pos = self.cursor_position - 1;
            while new_pos > 0 && !self.query.is_char_boundary(new_pos) {
                new_pos -= 1;
            }
            self.cursor_position = new_pos;
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.query.len() {
            let mut new_pos = self.cursor_position + 1;
            while new_pos < self.query.len() && !self.query.is_char_boundary(new_pos) {
                new_pos += 1;
            }
            self.cursor_position = new_pos;
        }
    }

    fn autocomplete_with_selected(&mut self) {
        if let Some(element) = self.filtered_elements.get(self.selected_index) {
            self.query = element.name.clone();
            self.cursor_position = self.query.len();
            self.update_search();
        }
    }
}

// Custom View using old objc ClassDecl
struct CustomView;

impl CustomView {
    const NAME: &'static str = "KickoffCustomView";

    fn define_class() -> &'static Class {
        let mut decl = ClassDecl::new(Self::NAME, class!(NSView))
            .unwrap_or_else(|| panic!("Unable to register {} class", Self::NAME));

        // Add ivar for state
        decl.add_ivar::<*mut c_void>("state_ptr");

        unsafe {
            // Override drawRect:
            decl.add_method(
                sel!(drawRect:),
                Self::draw_rect as extern "C" fn(&Object, Sel, CocoaNSRect),
            );

            // Override acceptsFirstResponder
            decl.add_method(
                sel!(acceptsFirstResponder),
                Self::accepts_first_responder as extern "C" fn(&Object, Sel) -> bool,
            );

            // Override becomeFirstResponder
            decl.add_method(
                sel!(becomeFirstResponder),
                Self::become_first_responder as extern "C" fn(&Object, Sel) -> bool,
            );

            // Override keyDown:
            decl.add_method(
                sel!(keyDown:),
                Self::key_down as extern "C" fn(&mut Object, Sel, *mut Object),
            );
        }

        decl.register()
    }

    extern "C" fn draw_rect(this: &Object, _sel: Sel, _dirty_rect: CocoaNSRect) {
        unsafe {
            let state_ptr: *mut c_void = *this.get_ivar("state_ptr");
            if state_ptr.is_null() {
                return;
            }
            let state = &*(state_ptr as *const RefCell<AppState>);
            let state = state.borrow();

            // Get bounds (convert from CocoaNSRect to objc2 NSRect)
            let bounds_cocoa: CocoaNSRect = msg_send![this, bounds];
            let bounds = NSRect::new(
                NSPoint::new(bounds_cocoa.origin.x, bounds_cocoa.origin.y),
                NSSize::new(bounds_cocoa.size.width, bounds_cocoa.size.height),
            );

            // Fill background
            let bg_color = nscolor_from_config(&state.config.background_color().unwrap_or_else(|_| Color::rgb(40, 44, 52)));
            bg_color.setFill();
            NSBezierPath::fillRect(bounds);
            
            // Simple graphics context setup - removed complex operations that might cause crashes

            // Draw prompt and query at the top, offset by menubar height
            let padding = state.config.spacing.window_padding_x as f64;
            let prompt_y = bounds.size.height - state.config.spacing.window_padding_y as f64 - state.menubar_height;
            let prompt_text = format!("{}{}", state.config.prompt, state.query);
            draw_text(
                &prompt_text,
                padding,
                prompt_y,
                &state.config.query_color().unwrap_or_else(|_| Color::rgb(224, 108, 117)),
                state.config.font_size as f64,
                &state.config.font_family,
            );

            // Draw cursor - position based on cursor_position in query
            let text_before_cursor = format!("{}{}", state.config.prompt, &state.query[..state.cursor_position]);
            let cursor_x = padding + measure_text_width(&text_before_cursor, state.config.font_size as f64, &state.config.font_family);
            draw_cursor(cursor_x, prompt_y, &state.config.caret_color().unwrap_or_else(|_| Color::rgb(224, 108, 117)), state.config.font_size as f64);

            // Draw results with scrolling - using config spacing
            let max_results = state.dynamic_max_results;
            let line_height = state.config.font_size as f64 + state.config.spacing.item_spacing as f64;
            let results_start_y = prompt_y - state.config.spacing.prompt_to_items as f64;

            let visible_elements = state.filtered_elements
                .iter()
                .skip(state.scroll_offset)
                .take(max_results);

            for (display_i, (actual_i, element)) in visible_elements
                .enumerate()
                .map(|(display_i, element)| (display_i, (state.scroll_offset + display_i, element)))
            {
                let y = results_start_y - (display_i as f64 * line_height);

                // Determine text color based on selection
                let text_color = if actual_i == state.selected_index {
                    &state.config.selected_item_color().unwrap_or_else(|_| Color::rgb(97, 175, 239))
                } else {
                    &state.config.items_color().unwrap_or_else(|_| Color::rgb(255, 255, 255))
                };

                // Draw item text
                draw_text(
                    &element.name,
                    padding,
                    y,
                    text_color,
                    state.config.font_size as f64,
                    &state.config.font_family,
                );
            }

            // Draw "no results" message if needed
            if state.filtered_elements.is_empty() {
                draw_text(
                    "No results found",
                    padding + 5.0,
                    results_start_y,
                    &state.config.items_color().unwrap_or_else(|_| Color::rgb(255, 255, 255)),
                    state.config.font_size as f64,
                    &state.config.font_family,
                );
            }
        }
    }

    extern "C" fn accepts_first_responder(_this: &Object, _sel: Sel) -> bool {
        true
    }

    extern "C" fn become_first_responder(_this: &Object, _sel: Sel) -> bool {
        debug!("View became first responder");
        true
    }

    extern "C" fn key_down(this: &mut Object, _sel: Sel, event: *mut Object) {
        unsafe {
            let state_ptr: *mut c_void = *this.get_ivar("state_ptr");
            if state_ptr.is_null() {
                return;
            }
            let state_cell = &*(state_ptr as *const RefCell<AppState>);

            // Get event as NSEvent
            let event = event as *const NSEvent;
            let event = &*event;

            let key_code = event.keyCode();
            let modifiers = event.modifierFlags();
            let ctrl_pressed = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Control);
            let shift_pressed = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Shift);
            let alt_pressed = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Option);
            let cmd_pressed = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Command);
            
            info!("Key down: keyCode={}, ctrl={}, shift={}, alt={}, cmd={}", 
                  key_code, ctrl_pressed, shift_pressed, alt_pressed, cmd_pressed);

            let mut state = state_cell.borrow_mut();

            // Check keymap bindings
            let config = &state.config;
            
            // Exit
            if config.check_binding(&config.keymap.exit, key_code, ctrl_pressed, shift_pressed, alt_pressed, cmd_pressed) {
                state.should_exit = true;
                let mtm = MainThreadMarker::new().unwrap();
                let app = NSApplication::sharedApplication(mtm);
                app.terminate(None);
                return;
            }
            
            // Execute
            if config.check_binding(&config.keymap.execute, key_code, ctrl_pressed, shift_pressed, alt_pressed, cmd_pressed) {
                if let Err(e) = state.execute_selected() {
                    log::error!("Failed to execute: {}", e);
                }
                if state.should_exit {
                    let mtm = MainThreadMarker::new().unwrap();
                    let app = NSApplication::sharedApplication(mtm);
                    app.terminate(None);
                }
                return;
            }
            
            // Nav up
            if config.check_binding(&config.keymap.nav_up, key_code, ctrl_pressed, shift_pressed, alt_pressed, cmd_pressed) {
                state.nav_up();
                drop(state);
                let _: () = msg_send![this, setNeedsDisplay: true];
                return;
            }
            
            // Nav down
            if config.check_binding(&config.keymap.nav_down, key_code, ctrl_pressed, shift_pressed, alt_pressed, cmd_pressed) {
                state.nav_down();
                drop(state);
                let _: () = msg_send![this, setNeedsDisplay: true];
                return;
            }
            
            // Autocomplete
            if config.check_binding(&config.keymap.autocomplete, key_code, ctrl_pressed, shift_pressed, alt_pressed, cmd_pressed) {
                state.autocomplete_with_selected();
                drop(state);
                let _: () = msg_send![this, setNeedsDisplay: true];
                return;
            }
            
            // Hardcoded essential keys (not configurable)
            match key_code {
                51 => {
                    // Backspace - delete character
                    if ctrl_pressed {
                        // Ctrl+Backspace - delete word
                        state.delete_word();
                    } else {
                        state.delete_char();
                    }
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                123 => {
                    // Left arrow - move cursor left
                    state.move_cursor_left();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                124 => {
                    // Right arrow - move cursor right
                    state.move_cursor_right();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                13 if ctrl_pressed => {
                    // Ctrl+W - delete word
                    state.delete_word();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                32 if ctrl_pressed => {
                    // Ctrl+U - delete to start
                    state.delete_to_start();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                _ => {}
            }

            // Handle text input - accept all graphic characters and space (not just ASCII)
            if let Some(characters) = event.characters() {
                let text = characters.to_string();
                for c in text.chars() {
                    // Accept all printable characters including unicode (e.g., £, €, etc.)
                    if !c.is_control() && c != '\u{007f}' {
                        state.insert_char(c);
                    }
                }
                drop(state);
                let _: () = msg_send![this, setNeedsDisplay: true];
            }
        }
    }

    fn class() -> &'static Class {
        Class::get(Self::NAME).unwrap_or_else(Self::define_class)
    }

    fn new(config: Config, elements: ElementList, window_height: f64, menubar_height: f64) -> *mut Object {
        unsafe {
            let view: *mut Object = msg_send![Self::class(), alloc];
            let view: *mut Object = msg_send![view, init];

            // Create state and box it
            let state = AppState::new(config, elements, window_height, menubar_height);
            let state = Box::new(RefCell::new(state));
            let state_ptr = Box::into_raw(state) as *mut c_void;

            // Store state pointer in ivar
            (*view).set_ivar("state_ptr", state_ptr);

            view
        }
    }
}

// Custom Window using old objc ClassDecl - using NSPanel for better overlay behavior
struct CustomWindow;

impl CustomWindow {
    const NAME: &'static str = "KickoffCustomWindow";

    fn define_class() -> &'static Class {
        // Use NSPanel instead of NSWindow - panels respect menubar better
        let mut decl = ClassDecl::new(Self::NAME, class!(NSPanel))
            .unwrap_or_else(|| panic!("Unable to register {} class", Self::NAME));

        unsafe {
            decl.add_method(
                sel!(canBecomeKeyWindow),
                Self::can_become_key_window as extern "C" fn(&Object, Sel) -> bool,
            );

            decl.add_method(
                sel!(canBecomeMainWindow),
                Self::can_become_main_window as extern "C" fn(&Object, Sel) -> bool,
            );
        }

        decl.register()
    }

    extern "C" fn can_become_key_window(_this: &Object, _sel: Sel) -> bool {
        debug!("canBecomeKeyWindow called");
        true
    }

    extern "C" fn can_become_main_window(_this: &Object, _sel: Sel) -> bool {
        debug!("canBecomeMainWindow called");
        true
    }

    fn class() -> &'static Class {
        Class::get(Self::NAME).unwrap_or_else(Self::define_class)
    }
}

// Helper functions for drawing
fn nscolor_from_config(color: &Color) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        color.r as f64,
        color.g as f64,
        color.b as f64,
        color.a as f64,
    )
}

fn nscolor_opaque_text(color: &Color) -> Retained<NSColor> {
    // Force text colors to be fully opaque (alpha = 1.0)
    NSColor::colorWithSRGBRed_green_blue_alpha(
        color.r as f64,
        color.g as f64,
        color.b as f64,
        1.0, // Always fully opaque for text
    )
}

// PROPER NSAttributedString approach using cacao-style constants with font family support
fn draw_text(text: &str, x: f64, y: f64, color: &Color, font_size: f64, font_name: &str) {
    unsafe {
        // Create NSMutableAttributedString
        let ns_text = NSString::from_str(text);
        let attributed_string_class = class!(NSMutableAttributedString);
        let attr_string: *mut Object = msg_send![attributed_string_class, alloc];
        let attr_string: *mut Object = msg_send![attr_string, initWithString: Retained::as_ptr(&ns_text) as *mut Object];
        
        // Get the proper NSForegroundColorAttributeName constant
        extern "C" {
            static NSForegroundColorAttributeName: *mut Object;
            static NSFontAttributeName: *mut Object;
        }
        
        // Create font with family support - try custom font first, fallback to system
        let font = if !font_name.is_empty() && font_name != "system" {
            let font_name_ns = NSString::from_str(font_name);
            if let Some(custom_font) = NSFont::fontWithName_size(&font_name_ns, font_size) {
                custom_font
            } else {
                NSFont::systemFontOfSize(font_size)
            }
        } else {
            NSFont::systemFontOfSize(font_size)
        };
        
        let text_color = nscolor_from_config(color);
        
        // Get string length for range
        let string_length = ns_text.length();
        let full_range = NSRange::new(0, string_length as u64);
        
        // Add font attribute using proper constant
        let _: () = msg_send![attr_string,
            addAttribute: NSFontAttributeName
            value: Retained::as_ptr(&font) as *mut Object
            range: full_range
        ];
        
        // Add color attribute using proper constant
        let _: () = msg_send![attr_string,
            addAttribute: NSForegroundColorAttributeName
            value: Retained::as_ptr(&text_color) as *mut Object
            range: full_range
        ];
        
        // Draw the attributed string
        let point = NSPoint::new(x, y);
        let _: () = msg_send![attr_string, drawAtPoint: point];
        
        // Clean up
        let _: () = msg_send![attr_string, release];
    }
}

fn draw_cursor(x: f64, y: f64, color: &Color, font_size: f64) {
    let cursor_color = nscolor_from_config(color);
    cursor_color.setFill();
    
    // Increase cursor height for better visibility
    let cursor_height = font_size * 0.9; // 90% of font size (increased from 80%)
    let cursor_width = 2.0;
    
    // Slightly offset cursor vertically for better alignment
    let cursor_y_offset = font_size * 0.1; // Small vertical offset
    let cursor_rect = NSRect::new(
        NSPoint::new(x, y + cursor_y_offset), 
        NSSize::new(cursor_width, cursor_height)
    );
    NSBezierPath::fillRect(cursor_rect);
}

fn measure_text_width(text: &str, font_size: f64, font_name: &str) -> f64 {
    unsafe {
        if text.is_empty() {
            return 0.0;
        }
        
        // Use actual NSAttributedString to measure text width accurately with same font as rendering
        let ns_text = NSString::from_str(text);
        let attributed_string_class = class!(NSMutableAttributedString);
        let attr_string: *mut Object = msg_send![attributed_string_class, alloc];
        let attr_string: *mut Object = msg_send![attr_string, initWithString: Retained::as_ptr(&ns_text) as *mut Object];
        
        extern "C" {
            static NSFontAttributeName: *mut Object;
        }
        
        // Use the SAME font selection logic as draw_text
        let font = if !font_name.is_empty() && font_name != "system" {
            let font_name_ns = NSString::from_str(font_name);
            if let Some(custom_font) = NSFont::fontWithName_size(&font_name_ns, font_size) {
                custom_font
            } else {
                NSFont::systemFontOfSize(font_size)
            }
        } else {
            NSFont::systemFontOfSize(font_size)
        };
        
        let string_length = ns_text.length();
        let full_range = NSRange::new(0, string_length as u64);
        
        let _: () = msg_send![attr_string,
            addAttribute: NSFontAttributeName
            value: Retained::as_ptr(&font) as *mut Object
            range: full_range
        ];
        
        // Get the size of the attributed string
        let size: NSSize = msg_send![attr_string, size];
        
        // Clean up
        let _: () = msg_send![attr_string, release];
        
        size.width
    }
}

pub fn run(config: Config, elements: ElementList) -> Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("Must be called from the main thread"))?;

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Get the screen where the mouse cursor is located (active screen)
    let active_screen = unsafe {
        // Get mouse location in screen coordinates
        let mouse_location = NSEvent::mouseLocation();
        
        // Find the screen containing the mouse cursor
        let screens = NSScreen::screens(mtm);
        let mut found_screen = None;
        
        for i in 0..screens.count() {
            let screen = screens.objectAtIndex(i);
            let frame = screen.frame();
            if mouse_location.x >= frame.origin.x 
                && mouse_location.x <= frame.origin.x + frame.size.width
                && mouse_location.y >= frame.origin.y 
                && mouse_location.y <= frame.origin.y + frame.size.height {
                found_screen = Some(screen);
                break;
            }
        }
        
        // Fall back to main screen if no screen contains mouse
        found_screen.unwrap_or_else(|| NSScreen::mainScreen(mtm).unwrap())
    };

    // Get visible frame of active screen (excludes menubar and dock automatically)
    // visibleFrame() returns the area excluding menubar at top and dock
    // NOTE: macOS coordinates have Y=0 at bottom of screen
    let visible_rect = active_screen.visibleFrame();
    let full_rect = active_screen.frame();
    
    info!("Screen full frame: origin=({}, {}), size=({}, {})", 
          full_rect.origin.x, full_rect.origin.y, 
          full_rect.size.width, full_rect.size.height);
    info!("Screen visible frame: origin=({}, {}), size=({}, {})", 
          visible_rect.origin.x, visible_rect.origin.y, 
          visible_rect.size.width, visible_rect.size.height);
    
    // Calculate menubar height from the difference
    let menubar_height = full_rect.size.height - visible_rect.size.height;
    info!("Calculated menubar height: {}", menubar_height);
    
    // IMPORTANT: visibleFrame() on some systems returns origin (0,0) incorrectly
    // We need to manually position the window to start at Y=0 (bottom) 
    // and height = visible_height so top stops at (full_height - menubar_height)
    // This ensures the window top edge is at the bottom of the menubar
    let window_rect = NSRect::new(
        NSPoint::new(full_rect.origin.x, full_rect.origin.y),  // Start at screen bottom (Y=0)
        NSSize::new(full_rect.size.width, visible_rect.size.height)  // Height excludes menubar
    );
    
    info!("Window frame set to: origin=({}, {}), size=({}, {})",
          window_rect.origin.x, window_rect.origin.y,
          window_rect.size.width, window_rect.size.height);

    let style_mask = NSWindowStyleMask::Borderless;

    // Create custom window using our custom class
    let window: *mut Object = unsafe {
        let window: *mut Object = msg_send![CustomWindow::class(), alloc];
        msg_send![window,
            initWithContentRect: window_rect
            styleMask: style_mask
            backing: NSBackingStoreType::Buffered
            defer: false
        ]
    };

    // Convert to Retained<NSWindow>
    let window = unsafe { Retained::from_raw(window as *mut NSWindow).unwrap() };

    // Configure window
    // NSNormalWindowLevel = 0 (respects menubar)
    // NSFloatingWindowLevel = 3 (can overlap menubar)
    // Use normal level so window stays below menubar
    window.setLevel(0); // Normal window level - respects menubar
    
    // Set window opacity from config
    let opacity = config.styles.window_opacity.max(0.0).min(1.0);
    window.setAlphaValue(opacity as f64);
    
    // Use background color from styles config (no alpha in color itself)
    let bg_color = config.background_color().unwrap_or_else(|_| Color::rgb(40, 44, 52));
    window.setBackgroundColor(Some(&nscolor_from_config(&bg_color)));
    window.setOpaque(false);
    window.setHasShadow(true);
    window.setAcceptsMouseMovedEvents(true);
    window.setIgnoresMouseEvents(false);
    
    // Explicitly constrain window to visible frame
    window.setFrame_display(window_rect, false);
    
    // Prevent window from moving into menubar area
    unsafe {
        let window_ptr = window.as_ref() as *const NSWindow as *mut Object;
        // Disable auto-positioning that might move window
        let _: () = msg_send![window_ptr, setMovable: false];
    }
    
    // Critical: Set collection behavior to allow this window to be key/main
    unsafe {
        let window_ptr = window.as_ref() as *const NSWindow as *mut Object;
        // NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary
        let _: () = msg_send![window_ptr, setCollectionBehavior: 1 | 4];
    }

    // Set corner radius - using a fixed modern value since window config is removed
    if let Some(content_view) = window.contentView() {
        content_view.setWantsLayer(true);
        if let Some(layer) = content_view.layer() {
            layer.setCornerRadius(12.0); // Fixed corner radius
        }
    }

    // Create custom view with menubar height offset
    let custom_view = CustomView::new(config, elements, window_rect.size.height, menubar_height);
    
    // Set frame to full screen
    unsafe {
        let _: () = msg_send![custom_view, setFrame: window_rect];
    }

    // Set as content view
    unsafe {
        let custom_view_nsview = custom_view as *mut NSView;
        let custom_view_retained = Retained::from_raw(custom_view_nsview).unwrap();
        window.setContentView(Some(&custom_view_retained));
        // Don't drop custom_view_retained, let window manage it
        std::mem::forget(custom_view_retained);
    }

    // Critical activation sequence
    // 1. First activate the app with ignoring other apps
    app.activateIgnoringOtherApps(true);
    
    // 2. Order window front
    window.makeKeyAndOrderFront(None);
    window.orderFrontRegardless();
    
    // 3. Make it key window
    window.makeKeyWindow();
    
    // 4. Set first responder to our custom view for keyboard input
    unsafe {
        let window_ptr = window.as_ref() as *const NSWindow as *mut Object;
        let _: () = msg_send![window_ptr, makeFirstResponder: custom_view];
    }

    // Run event loop
    app.run();

    Ok(())
}
