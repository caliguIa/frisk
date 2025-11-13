use super::rendering::{draw_cursor, draw_text, measure_text_width, nscolor_from_config};
use super::state::AppState;
use crate::config::{Color, Config};
use crate::element::ElementList;
use cocoa::foundation::NSRect as CocoaNSRect;
use log::{debug, info};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use objc2_app_kit::{NSBezierPath, NSEvent};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::cell::RefCell;
use std::os::raw::c_void;

const KEY_ESCAPE: u16 = 53;
const KEY_ENTER: u16 = 36;
const KEY_TAB: u16 = 48;
const KEY_BACKSPACE: u16 = 51;
const KEY_LEFT: u16 = 123;
const KEY_RIGHT: u16 = 124;
const KEY_DOWN: u16 = 125;
const KEY_UP: u16 = 126;

pub struct CustomView;

impl CustomView {
    const NAME: &'static str = "KickoffCustomView";

    fn define_class() -> &'static Class {
        let mut decl = ClassDecl::new(Self::NAME, class!(NSView))
            .unwrap_or_else(|| panic!("Unable to register {} class", Self::NAME));

        decl.add_ivar::<*mut c_void>("state_ptr");

        unsafe {
            decl.add_method(
                sel!(drawRect:),
                Self::draw_rect as extern "C" fn(&Object, Sel, CocoaNSRect),
            );

            decl.add_method(
                sel!(acceptsFirstResponder),
                Self::accepts_first_responder as extern "C" fn(&Object, Sel) -> bool,
            );

            decl.add_method(
                sel!(becomeFirstResponder),
                Self::become_first_responder as extern "C" fn(&Object, Sel) -> bool,
            );

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

            let bounds_cocoa: CocoaNSRect = msg_send![this, bounds];
            let bounds = NSRect::new(
                NSPoint::new(bounds_cocoa.origin.x, bounds_cocoa.origin.y),
                NSSize::new(bounds_cocoa.size.width, bounds_cocoa.size.height),
            );

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
            let event = &*(event as *const NSEvent);
            let key_code = event.keyCode();
            let modifiers = event.modifierFlags();
            
            let ctrl = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Control);
            
            info!("Key: code={}, ctrl={}", key_code, ctrl);

            let mut state = state_cell.borrow_mut();
            
            match key_code {
                KEY_ESCAPE => {
                    drop(state);
                    state_cell.borrow().terminate();
                    return;
                }
                KEY_ENTER => {
                    if let Err(e) = state.execute_selected() {
                        log::error!("Failed to execute: {}", e);
                    }
                    if state.should_exit {
                        drop(state);
                        state_cell.borrow().terminate();
                    }
                    return;
                }
                KEY_UP => {
                    state.nav_up();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                KEY_DOWN => {
                    state.nav_down();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                KEY_TAB => {
                    state.autocomplete();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                KEY_BACKSPACE => {
                    if ctrl {
                        state.delete_word();
                    } else {
                        state.delete_char();
                    }
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                KEY_LEFT => {
                    state.move_cursor_left();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                KEY_RIGHT => {
                    state.move_cursor_right();
                    drop(state);
                    let _: () = msg_send![this, setNeedsDisplay: true];
                    return;
                }
                _ => {}
            }

            if let Some(characters) = event.characters() {
                let text = characters.to_string();
                for c in text.chars() {
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

    pub fn new(config: Config, elements: ElementList, window_height: f64, menubar_height: f64) -> *mut Object {
        unsafe {
            let view: *mut Object = msg_send![Self::class(), alloc];
            let view: *mut Object = msg_send![view, init];

            let state = AppState::new(config, elements, window_height, menubar_height);
            let state = Box::new(RefCell::new(state));
            let state_ptr = Box::into_raw(state) as *mut c_void;

            (*view).set_ivar("state_ptr", state_ptr);

            view
        }
    }
}
