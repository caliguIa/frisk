use super::rendering::{draw_cursor, draw_text, measure_text_width};
use super::state::AppState;
use crate::core::config::Config;
use crate::core::element::ElementList;
use crate::ipc::IpcMessage;
use objc2::rc::Retained;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBezierPath, NSEvent, NSView};
use objc2_foundation::NSRect;
use std::cell::RefCell;
use std::sync::mpsc::Receiver;

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
    ipc_rx: RefCell<Option<Receiver<IpcMessage>>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[ivars = Ivars]
    #[name = "KickoffCustomView"]
    pub struct CustomView;

    impl CustomView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            if let Some(rx) = self.ivars().ipc_rx.borrow().as_ref() {
                if let Ok(msg) = rx.try_recv() {
                    self.handle_ipc_message(msg);
                }
            }

            let mut state = self.ivars().state.borrow_mut();

            state.update_string_caches();

            let bounds = self.bounds();

            state.config.background_color.setFill();
            NSBezierPath::fillRect(bounds);

            let padding = state.config.window_padding as f64;
            let prompt_y = bounds.size.height - padding - state.menubar_height;

            draw_text(
                &state.prompt_query_cache,
                padding,
                prompt_y,
                &state.config.query_color,
                &state.config.font,
            );

            let cursor_text = state.cursor_text_cache.clone();
            let cursor_x = padding + measure_text_width(&cursor_text, &state.config.font);
            draw_cursor(
                cursor_x,
                prompt_y,
                &state.config.caret_color,
                state.config.font_size as f64
            );

            let line_height = state.config.font_size as f64 + state.config.item_spacing as f64;
            let results_start_y = prompt_y - state.config.prompt_to_items as f64;

            let mut display_idx = 0;

            if let Some(calc_result) = &state.calculator_result {
                if display_idx >= state.scroll_offset && display_idx < state.scroll_offset + state.dynamic_max_results {
                    let y = results_start_y - ((display_idx - state.scroll_offset) as f64 * line_height);
                    let text_color = if display_idx == state.selected_index {
                        &state.config.selected_item_color
                    } else {
                        &state.config.items_color
                    };

                    draw_text(
                        &calc_result.name,
                        padding,
                        y,
                        text_color,
                        &state.config.font,
                    );
                }
                display_idx += 1;
            }

            for &elem_idx in &state.filtered_indices {
                if display_idx >= state.scroll_offset && display_idx < state.scroll_offset + state.dynamic_max_results {
                    if let Some(element) = state.elements.inner.get(elem_idx) {
                        let y = results_start_y - ((display_idx - state.scroll_offset) as f64 * line_height);
                        let text_color = if display_idx == state.selected_index {
                            &state.config.selected_item_color
                        } else {
                            &state.config.items_color
                        };

                        draw_text(
                            &element.name,
                            padding,
                            y,
                            text_color,
                            &state.config.font,
                        );
                    }
                }
                display_idx += 1;
                if display_idx >= state.scroll_offset + state.dynamic_max_results {
                    break;
                }
            }

            let has_results = state.calculator_result.is_some() || !state.filtered_indices.is_empty();
            if !has_results && !state.query.is_empty() {
                draw_text(
                    "No results",
                    padding,
                    results_start_y,
                    &state.config.items_color,
                    &state.config.font,
                );
            }
        }

        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            crate::log!("View became first responder");
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let key_code = event.keyCode();
            let modifiers = event.modifierFlags();

            let ctrl = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Control);
            let cmd = modifiers.contains(objc2_app_kit::NSEventModifierFlags::Command);

            crate::log!("Key: code={}, ctrl={}, cmd={}", key_code, ctrl, cmd);

            if ctrl && !cmd {
                if let Some(characters) = event.charactersIgnoringModifiers() {
                    let text = characters.to_string();
                    crate::log!("Ctrl+key: {:?}", text);
                    match text.to_lowercase().as_str() {
                        "w" => {
                            self.ivars().state.borrow_mut().delete_word();
                            self.setNeedsDisplay(true);
                            return;
                        }
                        "u" => {
                            self.ivars().state.borrow_mut().delete_to_start();
                            self.setNeedsDisplay(true);
                            return;
                        }
                        "n" => {
                            self.ivars().state.borrow_mut().nav_down();
                            self.setNeedsDisplay(true);
                            return;
                        }
                        "p" => {
                            self.ivars().state.borrow_mut().nav_up();
                            self.setNeedsDisplay(true);
                            return;
                        }
                        "y" => {
                            let mut state = self.ivars().state.borrow_mut();
                            if let Err(e) = state.execute_selected() {
                                eprintln!("[kickoff] Failed to execute: {}", e);
                            }
                            if state.should_exit {
                                drop(state);
                                self.ivars().state.borrow().terminate();
                            }
                            return;
                        }
                        _ => {}
                    }
                }
            }

            if cmd && !ctrl {
                if let Some(characters) = event.charactersIgnoringModifiers() {
                    if characters.to_string().to_lowercase() == "v" {
                        self.ivars().state.borrow_mut().paste();
                        self.setNeedsDisplay(true);
                        return;
                    }
                }
            }

            match key_code {
                KEY_ESCAPE => {
                    self.ivars().state.borrow().terminate();
                    return;
                }
                KEY_ENTER => {
                    let mut state = self.ivars().state.borrow_mut();
                    if let Err(e) = state.execute_selected() {
                        eprintln!("[kickoff] Failed to execute: {}", e);
                    }
                    if state.should_exit {
                        drop(state);
                        self.ivars().state.borrow().terminate();
                    }
                    self.setNeedsDisplay(true);
                    return;
                }
                KEY_UP => {
                    if !ctrl {
                        self.ivars().state.borrow_mut().nav_up();
                        self.setNeedsDisplay(true);
                    }
                    return;
                }
                KEY_DOWN => {
                    if !ctrl {
                        self.ivars().state.borrow_mut().nav_down();
                        self.setNeedsDisplay(true);
                    }
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
    pub fn new(
        config: Config,
        elements: ElementList,
        window_height: f64,
        menubar_height: f64,
        ipc_rx: Option<Receiver<IpcMessage>>,
        mtm: objc2::MainThreadMarker,
    ) -> Retained<Self> {
        unsafe {
            msg_send![
                super(Self::alloc(mtm).set_ivars(Ivars {
                    state: RefCell::new(AppState::new(
                        config,
                        elements,
                        window_height,
                        menubar_height,
                    )),
                    ipc_rx: RefCell::new(ipc_rx),
                })),
                init
            ]
        }
    }

    fn handle_ipc_message(&self, msg: IpcMessage) {
        match msg {
            IpcMessage::Reload {
                apps,
                homebrew,
                clipboard,
                commands,
                nixpkgs,
                sources,
                prompt,
            } => {
                let mut state = self.ivars().state.borrow_mut();
                state.handle_reload(
                    apps, homebrew, clipboard, commands, nixpkgs, sources, prompt,
                );
            }
            IpcMessage::Search { .. } => {}
        }
    }
}
