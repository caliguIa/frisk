use crate::core::element::Element;
use crate::core::error::Result;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};

use std::thread;
use std::time::Duration;

const MAX_HISTORY: usize = 1000;
const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn run() -> Result<()> {
    eprintln!("[clipboard daemon] Starting...");

    let pasteboard = NSPasteboard::generalPasteboard();
    let mut last_change_count = pasteboard.changeCount();
    let mut last_content: Option<String> = None;

    eprintln!("[clipboard daemon] Monitoring clipboard...");

    loop {
        let current_count = pasteboard.changeCount();

        if current_count != last_change_count {
            last_change_count = current_count;

            if let Some(content) = pasteboard.stringForType(unsafe { NSPasteboardTypeString }) {
                let content_str = content.to_string();
                let trimmed = content_str.trim();

                if !trimmed.is_empty()
                    && last_content
                        .as_ref()
                        .map(|e| e == &content_str)
                        .unwrap_or(false)
                        == false
                {
                    last_content = Some(content_str.clone());

                    // Save to file
                    if let Err(e) = append_clipboard_entry(&content_str) {
                        eprintln!("[clipboard daemon] Failed to save: {}", e);
                    } else {
                        eprintln!("[clipboard daemon] Saved entry");
                    }
                }
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

fn append_clipboard_entry(content: &str) -> Result<()> {
    // Load existing history
    let mut elements: Vec<Element> =
        crate::loader::load_binary_source("clipboard.bin")?.unwrap_or_default();

    // Create new element
    let normalized: String = content
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if !normalized.is_empty() {
        let display = if normalized.len() > 80 {
            format!("{}...", &normalized[..77])
        } else {
            normalized.clone()
        };

        let new_element = Element::new_clipboard_entry(display, content.to_string());

        // Add to front and trim to max size
        elements.insert(0, new_element);
        elements.truncate(MAX_HISTORY);

        crate::cache::save_cache("clipboard.bin", &elements)?;
    }

    Ok(())
}
