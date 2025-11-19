use super::view::CustomView;
use crate::core::config::Config;
use crate::core::element::ElementList;
use crate::core::error::{Error, Result};
use crate::ipc::IpcMessage;
use objc2::rc::Retained;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAccessibility, NSBackingStoreType, NSPopUpMenuWindowLevel, NSScreen, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::sync::mpsc::Receiver;

pub fn create_window(
    mtm: MainThreadMarker,
    config: Config,
    elements: ElementList,
    ipc_rx: Option<Receiver<IpcMessage>>,
) -> Result<Retained<BorderlessKeyWindow>> {
    let active_screen =
        NSScreen::mainScreen(mtm).ok_or_else(|| Error::new("Failed to find main screen"))?;
    let window_rect = calculate_window_rect(&active_screen);

    let window: Retained<BorderlessKeyWindow> = unsafe {
        msg_send![
            mtm.alloc::<BorderlessKeyWindow>(),
            initWithContentRect: window_rect,
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered,
            defer: false
        ]
    };

    window.setBackgroundColor(Some(&config.background_color));
    window.setOpaque(false);
    window.setHasShadow(false);

    let custom_view = CustomView::new(
        config,
        elements,
        window_rect.size.height,
        active_screen.frame().size.height - active_screen.visibleFrame().size.height,
        ipc_rx,
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
