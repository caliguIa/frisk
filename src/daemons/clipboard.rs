use crate::element::Element;
use crate::error::Result;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use std::collections::VecDeque;
use std::thread;
use std::time::{Duration, SystemTime};

const MAX_HISTORY: usize = 100;
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Clipboard history entry
#[derive(Clone, bincode::Encode, bincode::Decode)]
struct ClipboardEntry {
    content: String,
    timestamp: u64, // Unix timestamp
}

/// Monitor clipboard and save history
pub fn run() -> Result<()> {
    eprintln!("[clipboard daemon] Starting...");

    let pasteboard = NSPasteboard::generalPasteboard();
    let mut last_change_count = pasteboard.changeCount();
    let mut history: VecDeque<ClipboardEntry> = VecDeque::new();

    // Load existing history if available
    if let Some(existing) = load_clipboard_history()? {
        history = existing;
        eprintln!(
            "[clipboard daemon] Loaded {} existing entries",
            history.len()
        );
    }

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
                    && !history
                        .front()
                        .map(|e| e.content == content_str)
                        .unwrap_or(false)
                {
                    let timestamp = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let entry = ClipboardEntry {
                        content: content_str.clone(),
                        timestamp,
                    };

                    history.push_front(entry);

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

/// Load clipboard history from binary cache
fn load_clipboard_history() -> Result<Option<VecDeque<ClipboardEntry>>> {
    let cache_path = crate::cache::cache_dir()?.join("clipboard.bin");

    if !cache_path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(&cache_path)?;
    let config = bincode::config::standard();
    let (entries, _): (Vec<ClipboardEntry>, usize) =
        bincode::decode_from_slice(&bytes, config)?;

    Ok(Some(entries.into_iter().collect()))
}

/// Save clipboard history to binary cache
fn save_clipboard_history(history: &VecDeque<ClipboardEntry>) -> Result<()> {
    let vec: Vec<ClipboardEntry> = history.iter().cloned().collect();
    crate::cache::save_cache("clipboard.bin", &vec)?;
    Ok(())
}

/// Convert clipboard history to Elements for picker
pub fn history_to_elements(history: &VecDeque<ClipboardEntry>) -> Vec<Element> {
    history
        .iter()
        .map(|entry| {
            // Truncate long content for display
            let display = if entry.content.len() > 80 {
                format!("{}...", &entry.content[..77])
            } else {
                entry.content.clone()
            };

            Element::new_clipboard_entry(display, entry.content.clone())
        })
        .collect()
}
