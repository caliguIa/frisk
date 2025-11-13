use crate::config::Color;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSBezierPath, NSColor, NSFont};
use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};

pub fn nscolor_from_config(color: &Color) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        color.r as f64,
        color.g as f64,
        color.b as f64,
        color.a as f64,
    )
}

pub fn draw_text(text: &str, x: f64, y: f64, color: &Color, font_size: f64, font_name: &str) {
    // SAFETY: NSMutableAttributedString creation and manipulation via msg_send! is required
    // because objc2-foundation doesn't provide safe wrappers for mutable attributed strings.
    // The operations are:
    // 1. Allocate and initialize NSMutableAttributedString with text
    // 2. Add font and color attributes via NSFontAttributeName/NSForegroundColorAttributeName
    // 3. Draw at the specified point
    // All Retained<T> objects are properly managed and the string is valid for the operation.
    unsafe {
        let ns_text = NSString::from_str(text);
        let attr_string: Retained<AnyObject> = msg_send![
            msg_send![objc2::class!(NSMutableAttributedString), alloc],
            initWithString: &*ns_text
        ];

        extern "C" {
            static NSForegroundColorAttributeName: &'static AnyObject;
            static NSFontAttributeName: &'static AnyObject;
        }

        let font = create_font(font_name, font_size);
        let text_color = nscolor_from_config(color);
        let string_length = ns_text.len();
        let full_range = NSRange::new(0, string_length);

        let () = msg_send![
            &*attr_string,
            addAttribute: NSFontAttributeName,
            value: &*font,
            range: full_range
        ];

        let () = msg_send![
            &*attr_string,
            addAttribute: NSForegroundColorAttributeName,
            value: &*text_color,
            range: full_range
        ];

        let point = NSPoint::new(x, y);
        let () = msg_send![&*attr_string, drawAtPoint: point];
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

    // SAFETY: NSMutableAttributedString size measurement via msg_send! is required
    // because objc2-foundation doesn't provide safe wrappers for attributed string sizing.
    // This creates an attributed string with the font and queries its size.
    unsafe {
        let ns_text = NSString::from_str(text);
        let attr_string: Retained<AnyObject> = msg_send![
            msg_send![objc2::class!(NSMutableAttributedString), alloc],
            initWithString: &*ns_text
        ];

        extern "C" {
            static NSFontAttributeName: &'static AnyObject;
        }

        let font = create_font(font_name, font_size);
        let string_length = ns_text.len();
        let full_range = NSRange::new(0, string_length);

        let () = msg_send![
            &*attr_string,
            addAttribute: NSFontAttributeName,
            value: &*font,
            range: full_range
        ];

        let size: NSSize = msg_send![&*attr_string, size];

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
