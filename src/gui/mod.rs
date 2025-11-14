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
    use log::info;
    use std::time::Instant;
    
    let gui_start = Instant::now();
    
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("Must be called from main thread"))?;

    info!("GUI: Creating window...");
    let window_start = Instant::now();
    match create_window(mtm, config, elements) {
        Ok(window) => window,
        Err(error) => panic!("Error creating the window: {error:?}"),
    };
    info!("GUI: Window created in {:?}", window_start.elapsed());

    info!("GUI: Setting up NSApplication...");
    let app_start = Instant::now();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();
    app.setAccessibilityFrontmost(true);
    info!("GUI: NSApplication setup in {:?}", app_start.elapsed());
    
    info!("GUI: Total GUI setup took {:?}, entering run loop", gui_start.elapsed());
    app.run();
    Ok(())
}
