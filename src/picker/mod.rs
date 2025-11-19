mod rendering;
mod state;
mod view;
mod window;

use crate::core::config::Config;
use crate::core::element::ElementList;
use crate::core::error::{Error, Result};
use crate::ipc::IpcMessage;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSAccessibility, NSApplication, NSApplicationActivationPolicy};
use std::sync::mpsc::Receiver;
use window::create_window;

pub fn run(
    config: Config,
    elements: ElementList,
    ipc_rx: Option<Receiver<IpcMessage>>,
) -> Result<()> {
    let mtm =
        MainThreadMarker::new().ok_or_else(|| Error::new("Must be called from main thread"))?;

    let _window = match create_window(mtm, config, elements, ipc_rx) {
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
