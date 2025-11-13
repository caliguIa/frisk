use super::rendering::{draw_cursor, draw_text, measure_text_width, nscolor_from_config};
use super::state::AppState;
use crate::config::Config;
use crate::element::ElementList;
use log::{debug, info};
use objc2::rc::Retained;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBezierPath, NSEvent, NSView};
use objc2_foundation::NSRect;
use std::cell::RefCell;

const KEY_ESCAPE: u16 = 53;
const KEY_ENTER: u16 = 36;
const KEY_TAB: u16 = 48;
const KEY_BACKSPACE: u16 = 51;
const KEY_LEFT: u16 = 123;
const KEY_RIGHT: u16 = 124;
const KEY_DOWN: u16 = 125;
const KEY_UP: u16 = 126;

pub struct Ivars {
    state: RefCell<AppState>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[ivars = Ivars]
    #[name = "KickoffCustomView"]
    pub struct CustomView;

    impl CustomView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let state = self.ivars().state.borrow();

            let bounds = self.bounds();

            let bg_color = nscolor_from_config(&state.config.background_color());
            bg_color.setFill();
            NSBezierPath::fillRect(bounds);

            let padding = state.config.spacing.window_padding as f64;
            let prompt_y = bounds.size.height - padding - state.menubar_height;
            let prompt_text = format!("{}{}", state.config.prompt, state.query);
            
            draw_text(
                &prompt_text,
                padding,
                prompt_y,
                &state.config.query_color(),
                state.config.font_size as f64,
                &state.config.font_family,
            );

            let text_before_cursor = format!("{}{}", state.config.prompt, &state.query[..state.cursor_position]);
            let cursor_x = padding + measure_text_width(&text_before_cursor, state.config.font_size as f64, &state.config.font_family);
            draw_cursor(
                cursor_x,
                prompt_y,
                &state.config.caret_color(),
                state.config.font_size as f64
            );

            let line_height = state.config.font_size as f64 + state.config.spacing.item_spacing as f64;
            let results_start_y = prompt_y - state.config.spacing.prompt_to_items as f64;

            let visible_elements = state.filtered_elements
                .iter()
                .skip(state.scroll_offset)
                .take(state.dynamic_max_results);

            for (display_i, (actual_i, element)) in visible_elements
                .enumerate()
                .map(|(display_i, element)| (display_i, (state.scroll_offset + display_i, element)))
            {
                let y = results_start_y - (display_i as f64 * line_height);
                let text_color = if actual_i == state.selected_index {
                    &state.config.selected_item_color()
                } else {
                    &state.config.items_color()
                };

                draw_text(
                    &element.name,
                    padding,
                    y,
                    text_color,
                    state.config.font_size as f64,
                    &state.config.font_family,
                );
            }

            if state.filtered_elements.is_empty() {
                draw_text(
                    "No results",
                    padding,
                    results_start_y,
                    &state.config.items_color(),
                    state.config.font_size as f64,
                    &state.config.font_family,
                );
            }
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            debug!("View became first responder");
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let key_code = event.keyCode();
            let modifiers = event.modifierFlags();
            
            let ctrl = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Control);
            
            info!("Key: code={}, ctrl={}", key_code, ctrl);

            match key_code {
                KEY_ESCAPE => {
                    self.ivars().state.borrow().terminate();
                    return;
                }
                KEY_ENTER => {
                    let mut state = self.ivars().state.borrow_mut();
                    if let Err(e) = state.execute_selected() {
                        log::error!("Failed to execute: {}", e);
                    }
                    if state.should_exit {
                        drop(state);
                        self.ivars().state.borrow().terminate();
                    }
                    return;
                }
                KEY_UP => {
                    self.ivars().state.borrow_mut().nav_up();
                    self.setNeedsDisplay(true);
                    return;
                }
                KEY_DOWN => {
                    self.ivars().state.borrow_mut().nav_down();
                    self.setNeedsDisplay(true);
                    return;
                }
                KEY_TAB => {
                    self.ivars().state.borrow_mut().autocomplete();
                    self.setNeedsDisplay(true);
                    return;
                }
                KEY_BACKSPACE => {
                    let mut state = self.ivars().state.borrow_mut();
                    if ctrl {
                        state.delete_word();
                    } else {
                        state.delete_char();
                    }
                    drop(state);
                    self.setNeedsDisplay(true);
                    return;
                }
                KEY_LEFT => {
                    self.ivars().state.borrow_mut().move_cursor_left();
                    self.setNeedsDisplay(true);
                    return;
                }
                KEY_RIGHT => {
                    self.ivars().state.borrow_mut().move_cursor_right();
                    self.setNeedsDisplay(true);
                    return;
                }
                _ => {}
            }

            if let Some(characters) = event.characters() {
                let text = characters.to_string();
                let mut state = self.ivars().state.borrow_mut();
                for c in text.chars() {
                    if !c.is_control() && c != '\u{007f}' {
                        state.insert_char(c);
                    }
                }
                drop(state);
                self.setNeedsDisplay(true);
            }
        }
    }
);

impl CustomView {
    pub fn new(config: Config, elements: ElementList, window_height: f64, menubar_height: f64, mtm: objc2::MainThreadMarker) -> Retained<Self> {
        let state = AppState::new(config, elements, window_height, menubar_height);
        let this = Self::alloc(mtm).set_ivars(Ivars {
            state: RefCell::new(state),
        });
        // SAFETY: Calling super's init method is required for proper NSView initialization.
        // The msg_send![super(...), init] pattern is standard for Objective-C subclass initialization.
        unsafe { msg_send![super(this), init] }
    }
}
