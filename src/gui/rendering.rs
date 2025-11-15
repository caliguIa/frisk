use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSBezierPath, NSColor, NSFont};
use objc2_foundation::{NSPoint, NSRange, NSRect, NSSize, NSString};

pub fn draw_text(text: &str, x: f64, y: f64, color: &Retained<NSColor>, font: &Retained<NSFont>) {
    let ns_text = NSString::from_str(text);
    let char_count = ns_text.length();
    let full_range = NSRange::new(0, char_count);

    unsafe {
        extern "C" {
            static NSForegroundColorAttributeName: &'static AnyObject;
            static NSFontAttributeName: &'static AnyObject;
        }

        // Create opaque version of the color
        let opaque_color: Retained<NSColor> = msg_send![
            color,
            colorWithAlphaComponent: 1.0
        ];

        let attr_string: Retained<AnyObject> = msg_send![
            msg_send![objc2::class!(NSMutableAttributedString), alloc],
            initWithString: &*ns_text
        ];

        let () = msg_send![
            &*attr_string,
            addAttribute: NSFontAttributeName,
            value: &**font,
            range: full_range
        ];

        let () = msg_send![
            &*attr_string,
            addAttribute: NSForegroundColorAttributeName,
            value: &*opaque_color,
            range: full_range
        ];

        let point = NSPoint::new(x, y);
        let () = msg_send![&*attr_string, drawAtPoint: point];
    }
}

pub fn draw_cursor(x: f64, y: f64, color: &Retained<NSColor>, font_size: f64) {
    color.setFill();

    let cursor_height = font_size * 0.9;
    let cursor_width = 2.0;
    let cursor_y_offset = font_size * 0.15;

    let cursor_rect = NSRect::new(
        NSPoint::new(x, y + cursor_y_offset),
        NSSize::new(cursor_width, cursor_height),
    );
    NSBezierPath::fillRect(cursor_rect);
}

pub fn measure_text_width(text: &str, font: &Retained<NSFont>) -> f64 {
    let ns_text = NSString::from_str(text);
    let char_count = ns_text.length();
    let full_range = NSRange::new(0, char_count);

    unsafe {
        extern "C" {
            static NSFontAttributeName: &'static AnyObject;
        }

        let attr_string: Retained<AnyObject> = msg_send![
            msg_send![objc2::class!(NSMutableAttributedString), alloc],
            initWithString: &*ns_text
        ];

        let () = msg_send![
            &*attr_string,
            addAttribute: NSFontAttributeName,
            value: &**font,
            range: full_range
        ];

        let size: NSSize = msg_send![&*attr_string, size];
        size.width
    }
}
