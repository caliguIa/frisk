use super::view::CustomView;
use crate::config::Config;
use crate::element::ElementList;
use anyhow::{anyhow, Result};
use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSPopUpMenuWindowLevel, NSScreen, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};

pub fn create_window(
    mtm: MainThreadMarker,
    config: Config,
    elements: ElementList,
) -> Result<Retained<BorderlessKeyWindow>> {
    let active_screen =
        NSScreen::mainScreen(mtm).ok_or_else(|| anyhow!("Failed to find main screen"))?;
    let window_rect = calculate_window_rect(&active_screen);

    let window: Retained<BorderlessKeyWindow> = unsafe {
        msg_send![
            mtm.alloc::<BorderlessKeyWindow>(),
            initWithContentRect: window_rect,
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Retained,
            defer: false
        ]
    };

    window.setAlphaValue(config.window_opacity as f64);
    window.setBackgroundColor(Some(&config.background_color));
    window.setOpaque(false);
    window.setHasShadow(false);

    let custom_view = CustomView::new(
        config,
        elements,
        window_rect.size.height,
        active_screen.frame().size.height - active_screen.visibleFrame().size.height,
        mtm,
    );

    window.setContentView(Some(&custom_view));

    window.setLevel(NSPopUpMenuWindowLevel);
    window.makeKeyAndOrderFront(None);
    window.makeKeyWindow();
    window.makeMainWindow();
    window.orderFrontRegardless();
    window.setAccessibilityFrontmost(true);
    window.setAccessibilityFocused(true);

    Ok(window)
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
            crate::log!("canBecomeKeyWindow called");
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            crate::log!("canBecomeMainWindow called");
            true
        }
    }
);
