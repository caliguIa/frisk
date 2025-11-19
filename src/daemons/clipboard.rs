use crate::element::Element;
use crate::error::Result;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

const MAX_HISTORY: usize = 1000;
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Monitor clipboard and save history
pub fn run() -> Result<()> {
    eprintln!("[clipboard daemon] Starting...");

    let pasteboard = NSPasteboard::generalPasteboard();
    let mut last_change_count = pasteboard.changeCount();
    let mut history: VecDeque<String> = VecDeque::new();

    eprintln!("[clipboard daemon] Monitoring clipboard...");

    loop {
        let current_count = pasteboard.changeCount();

        if current_count != last_change_count {
            last_change_count = current_count;

            // Try to get string content
            if let Some(content) = pasteboard.stringForType(unsafe { NSPasteboardTypeString }) {
                let content_str = content.to_string();

                // Skip if empty or same as last entry
                if !content_str.is_empty()
                    && history.front().map(|e| e == &content_str).unwrap_or(false) == false
                {
                    history.push_front(content_str.clone());

                    // Trim to max size
                    while history.len() > MAX_HISTORY {
                        history.pop_back();
                    }

                    // Save to file
                    if let Err(e) = save_clipboard_history(&history) {
                        eprintln!("[clipboard daemon] Failed to save: {}", e);
                    } else {
                        eprintln!(
                            "[clipboard daemon] Saved entry ({} total)",
                            history.len()
                        );
                    }
                }
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

/// Save clipboard history to binary cache (as Elements for direct loading)
fn save_clipboard_history(history: &VecDeque<String>) -> Result<()> {
    // Convert to Elements before saving
    let elements: Vec<Element> = history
        .iter()
        .map(|content| {
            // Normalize whitespace: replace newlines, tabs, and multiple spaces with single space
            let normalized: String = content
                .chars()
                .map(|c| if c.is_whitespace() { ' ' } else { c })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            
            // Truncate long content for display
            let display = if normalized.len() > 80 {
                format!("{}...", &normalized[..77])
            } else {
                normalized.clone()
            };
            
            // Use original content as value so paste preserves formatting
            Element::new_clipboard_entry(display, content.clone())
        })
        .collect();
    
    crate::cache::save_cache("clipboard.bin", &elements)?;
    Ok(())
}
