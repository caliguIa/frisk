use crate::config::Color;
use cocoa::foundation::NSRange;
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};
use objc2::rc::Retained;
use objc2_app_kit::{NSBezierPath, NSColor, NSFont};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

pub fn nscolor_from_config(color: &Color) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        color.r as f64,
        color.g as f64,
        color.b as f64,
        color.a as f64,
    )
}

pub fn draw_text(text: &str, x: f64, y: f64, color: &Color, font_size: f64, font_name: &str) {
    unsafe {
        let ns_text = NSString::from_str(text);
        let attributed_string_class = class!(NSMutableAttributedString);
        let attr_string: *mut Object = msg_send![attributed_string_class, alloc];
        let attr_string: *mut Object = msg_send![attr_string, initWithString: Retained::as_ptr(&ns_text) as *mut Object];
        
        extern "C" {
            static NSForegroundColorAttributeName: *mut Object;
            static NSFontAttributeName: *mut Object;
        }
        
        let font = create_font(font_name, font_size);
        let text_color = nscolor_from_config(color);
        let string_length = ns_text.length();
        let full_range = NSRange::new(0, string_length as u64);
        
        let _: () = msg_send![attr_string,
            addAttribute: NSFontAttributeName
            value: Retained::as_ptr(&font) as *mut Object
            range: full_range
        ];
        
        let _: () = msg_send![attr_string,
            addAttribute: NSForegroundColorAttributeName
            value: Retained::as_ptr(&text_color) as *mut Object
            range: full_range
        ];
        
        let point = NSPoint::new(x, y);
        let _: () = msg_send![attr_string, drawAtPoint: point];
        let _: () = msg_send![attr_string, release];
    }
}

pub fn draw_cursor(x: f64, y: f64, color: &Color, font_size: f64) {
    let cursor_color = nscolor_from_config(color);
    cursor_color.setFill();
    
    let cursor_height = font_size * 0.9;
    let cursor_width = 2.0;
    let cursor_y_offset = font_size * 0.1;
    
    let cursor_rect = NSRect::new(
        NSPoint::new(x, y + cursor_y_offset),
        NSSize::new(cursor_width, cursor_height),
    );
    NSBezierPath::fillRect(cursor_rect);
}

pub fn measure_text_width(text: &str, font_size: f64, font_name: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    
    unsafe {
        let ns_text = NSString::from_str(text);
        let attributed_string_class = class!(NSMutableAttributedString);
        let attr_string: *mut Object = msg_send![attributed_string_class, alloc];
        let attr_string: *mut Object = msg_send![attr_string, initWithString: Retained::as_ptr(&ns_text) as *mut Object];
        
        extern "C" {
            static NSFontAttributeName: *mut Object;
        }
        
        let font = create_font(font_name, font_size);
        let string_length = ns_text.length();
        let full_range = NSRange::new(0, string_length as u64);
        
        let _: () = msg_send![attr_string,
            addAttribute: NSFontAttributeName
            value: Retained::as_ptr(&font) as *mut Object
            range: full_range
        ];
        
        let size: NSSize = msg_send![attr_string, size];
        let _: () = msg_send![attr_string, release];
        
        size.width
    }
}

fn create_font(font_name: &str, font_size: f64) -> Retained<NSFont> {
    if !font_name.is_empty() && font_name != "system" {
        let font_name_ns = NSString::from_str(font_name);
        if let Some(custom_font) = NSFont::fontWithName_size(&font_name_ns, font_size) {
            return custom_font;
        }
    }
    NSFont::systemFontOfSize(font_size)
}
