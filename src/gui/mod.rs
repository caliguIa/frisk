mod rendering;
mod state;
mod view;
mod window;

use crate::config::Config;
use crate::element::ElementList;
use anyhow::Result;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSAccessibility, NSApplication, NSApplicationActivationPolicy};
use window::create_window;

pub fn run(config: Config, elements: ElementList) -> Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("Must be called from main thread"))?;

    match create_window(mtm, config, elements) {
        Ok(window) => window,
        Err(error) => panic!("Error creating the window: {error:?}"),
    };

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();
    app.setAccessibilityFrontmost(true);

    app.run();
    Ok(())
}
