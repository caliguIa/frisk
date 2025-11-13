mod rendering;
mod state;
mod view;
mod window;

use crate::config::Config;
use crate::element::ElementList;
use anyhow::Result;
use log::info;
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2::{msg_send, ClassType};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEvent, NSScreen, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use rendering::nscolor_from_config;
use view::CustomView;
use window::CustomWindow;

pub fn run(config: Config, elements: ElementList) -> Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("Must be called from main thread"))?;

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let active_screen = find_active_screen(mtm);
    let window_rect = calculate_window_rect(&active_screen);
    let menubar_height = calculate_menubar_height(&active_screen);

    info!(
        "Window: ({}, {}), {}x{}, menubar={}",
        window_rect.origin.x,
        window_rect.origin.y,
        window_rect.size.width,
        window_rect.size.height,
        menubar_height
    );

    let window = create_window(window_rect, &config)?;
    let custom_view = CustomView::new(config, elements, window_rect.size.height, menubar_height);

    setup_window_content(&window, custom_view.clone(), window_rect);
    activate_window(&app, &window, &custom_view);

    app.run();
    Ok(())
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
    let visible_rect = screen.visibleFrame();
    let full_rect = screen.frame();

    NSRect::new(
        NSPoint::new(full_rect.origin.x, full_rect.origin.y),
        NSSize::new(full_rect.size.width, visible_rect.size.height),
    )
}

fn calculate_menubar_height(screen: &NSScreen) -> f64 {
    screen.frame().size.height - screen.visibleFrame().size.height
}

fn create_window(rect: NSRect, config: &Config) -> Result<Retained<NSWindow>> {
    let window: Retained<NSWindow> = unsafe {
        let cls = CustomWindow::class();
        msg_send![
            msg_send![cls, alloc],
            initWithContentRect: rect,
            styleMask: NSWindowStyleMask::Borderless,
            backing: NSBackingStoreType::Buffered,
            defer: false
        ]
    };

    window.setLevel(0);

    let opacity = (config.styles.window_opacity as f64 / 100.0).clamp(0.0, 1.0);
    window.setAlphaValue(opacity);

    let bg_color = config.background_color();
    window.setBackgroundColor(Some(&nscolor_from_config(&bg_color)));
    window.setOpaque(false);
    window.setHasShadow(true);
    window.setAcceptsMouseMovedEvents(true);
    window.setIgnoresMouseEvents(false);
    window.setFrame_display(rect, false);

    unsafe {
        let () = msg_send![&window, setMovable: false];
        let () = msg_send![&window, setCollectionBehavior: 1 | 4];
    }

    if let Some(content_view) = window.contentView() {
        content_view.setWantsLayer(true);
        if let Some(layer) = content_view.layer() {
            layer.setCornerRadius(12.0);
        }
    }

    Ok(window)
}

fn setup_window_content(window: &NSWindow, custom_view: Retained<CustomView>, rect: NSRect) {
    unsafe {
        let () = msg_send![&custom_view, setFrame: rect];
        window.setContentView(Some(&custom_view));
    }
}

fn activate_window(app: &NSApplication, window: &NSWindow, custom_view: &CustomView) {
    app.activateIgnoringOtherApps(true);
    window.makeKeyAndOrderFront(None);
    window.orderFrontRegardless();
    window.makeKeyWindow();

    unsafe {
        let () = msg_send![window, makeFirstResponder: custom_view];
    }
}
