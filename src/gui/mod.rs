mod rendering;
mod state;
mod view;
mod window;

use crate::config::Config;
use crate::element::ElementList;
use crate::error::{Error, Result};
use crate::ipc::IpcMessage;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSAccessibility, NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::NSTimer;
use block2::RcBlock;
use std::sync::mpsc::Receiver;
use window::create_window;

pub fn run(config: Config, elements: ElementList, ipc_rx: Option<Receiver<IpcMessage>>) -> Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| Error::new("Must be called from main thread"))?;

    let window = match create_window(mtm, config, elements, ipc_rx) {
        Ok(window) => window,
        Err(error) => panic!("Error creating the window: {error:?}"),
    };
    
    // Get the content view for periodic redraws
    if let Some(view) = window.contentView() {
        // Create a timer that fires every 50ms to trigger redraws
        // This allows debounced searches to be checked periodically
        let block = RcBlock::new(move |_timer: std::ptr::NonNull<NSTimer>| {
            view.setNeedsDisplay(true);
        });
        
        let _timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_repeats_block(
                0.05, // 50ms = 20fps
                true,
                &block,
            )
        };
    }

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();
    app.setAccessibilityFrontmost(true);

    app.run();
    Ok(())
}
