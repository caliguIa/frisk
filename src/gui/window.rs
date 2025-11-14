use super::view::CustomView;
use crate::config::Config;
use crate::element::ElementList;
use anyhow::Result;
use log::{debug, info};
use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSEvent, NSPopUpMenuWindowLevel, NSScreen, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

pub fn create_window(
    mtm: MainThreadMarker,
    config: Config,
    elements: ElementList,
) -> Result<Retained<BorderlessKeyWindow>> {
    use std::time::Instant;
    info!("Window: Finding active screen...");
    let start = Instant::now();
    let active_screen = find_active_screen(mtm);
    let window_rect = calculate_window_rect(&active_screen);
    info!("Window: Screen detection took {:?}", start.elapsed());
    info!("Window: Creating NSWindow...");
    let window_start = Instant::now();
    let window: Retained<BorderlessKeyWindow> = unsafe {
        msg_send![
            mtm.alloc::<BorderlessKeyWindow>(),
            initWithContentRect: window_rect,
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered,
            defer: false
        ]
    };
    info!("Window: NSWindow created in {:?}", window_start.elapsed());

    info!("Window: Setting window properties...");
    let props_start = Instant::now();
    window.setAlphaValue(config.window_opacity as f64);
    window.setBackgroundColor(Some(&config.background_color));
    window.setOpaque(false);
    window.setHasShadow(false);
    info!("Window: Properties set in {:?}", props_start.elapsed());

    info!("Window: Creating custom view...");
    let view_start = Instant::now();
    let custom_view = CustomView::new(
        config,
        elements,
        window_rect.size.height,
        active_screen.frame().size.height - active_screen.visibleFrame().size.height,
        mtm,
    );
    info!("Window: Custom view created in {:?}", view_start.elapsed());

    info!("Window: Setting content view...");
    let content_start = Instant::now();
    window.setContentView(Some(&custom_view));
    info!("Window: Content view set in {:?}", content_start.elapsed());

    info!("Window: Making window visible...");
    let visible_start = Instant::now();
    window.setLevel(NSPopUpMenuWindowLevel);
    window.makeKeyAndOrderFront(None);
    window.makeKeyWindow();
    window.makeMainWindow();
    window.orderFrontRegardless();
    window.setAccessibilityFrontmost(true);
    window.setAccessibilityFocused(true);
    info!("Window: Made visible in {:?}", visible_start.elapsed());

    Ok(window)
}

fn find_active_screen(mtm: MainThreadMarker) -> Retained<NSScreen> {
    let mouse_location = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);

    for i in 0..screens.count() {
        let screen = screens.objectAtIndex(i);
        let frame = screen.frame();
        if mouse_location.x >= frame.origin.x
            && mouse_location.x <= frame.origin.x + frame.size.width
            && mouse_location.y >= frame.origin.y
            && mouse_location.y <= frame.origin.y + frame.size.height
        {
            return screen;
        }
    }

    NSScreen::mainScreen(mtm).unwrap()
}

fn calculate_window_rect(screen: &NSScreen) -> NSRect {
    let full_rect = screen.frame();

    NSRect::new(
        NSPoint::new(full_rect.origin.x, full_rect.origin.y),
        NSSize::new(full_rect.size.width, screen.visibleFrame().size.height),
    )
}

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    #[name = "BorderlessKeyWindow"]
    pub struct BorderlessKeyWindow;

    impl BorderlessKeyWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            debug!("canBecomeKeyWindow called");
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            debug!("canBecomeMainWindow called");
            true
        }
    }
);
