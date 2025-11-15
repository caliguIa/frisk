use crate::element::{Element, ElementList};
use anyhow::Result;
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 100;

fn get_history_file() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".config");
    path.push("kickoff-macos");
    path.push("clipboard_history.txt");
    path
}

pub fn load_clipboard_history() -> Result<VecDeque<String>> {
    let path = get_history_file();
    
    if !path.exists() {
        return Ok(VecDeque::new());
    }
    
    let content = fs::read_to_string(path)?;
    let mut history = VecDeque::new();
    
    for line in content.lines() {
        if !line.is_empty() {
            history.push_back(line.to_string());
        }
    }
    
    Ok(history)
}

pub fn save_clipboard_history(history: &VecDeque<String>) -> Result<()> {
    let path = get_history_file();
    
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let content = history.iter()
        .take(MAX_HISTORY)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    
    fs::write(path, content)?;
    Ok(())
}

pub fn add_to_clipboard_history(text: String) -> Result<()> {
    let mut history = load_clipboard_history()?;
    
    // Remove duplicate if it exists
    history.retain(|s| s != &text);
    
    // Add to front
    history.push_front(text);
    
    // Limit size
    while history.len() > MAX_HISTORY {
        history.pop_back();
    }
    
    save_clipboard_history(&history)?;
    Ok(())
}

pub fn load_clipboard_history_elements() -> Result<ElementList> {
    let mut elements = ElementList::new();
    
    let history = load_clipboard_history()?;
    
    if history.is_empty() {
        elements.add(Element::new_clipboard_entry(
            "No clipboard history available".to_string(),
            "".to_string(),
        ));
    } else {
        for (i, entry) in history.iter().enumerate() {
            // Truncate long entries for display
            let display = if entry.len() > 100 {
                format!("{}...", &entry[..100])
            } else {
                entry.clone()
            };
            
            // Replace newlines with spaces for display
            let display = display.replace('\n', " ").replace('\r', " ");
            
            elements.add(Element::new_clipboard_entry(
                format!("{}. {}", i + 1, display),
                entry.clone(),
            ));
        }
    }
    
    Ok(elements)
}

// Hook to track clipboard changes - should be called when app exits
pub fn track_clipboard_on_exit() -> Result<()> {
    let pasteboard = NSPasteboard::generalPasteboard();
    if let Some(text) = unsafe { pasteboard.stringForType(NSPasteboardTypeString) } {
        let text_str = text.to_string();
        if !text_str.is_empty() && text_str.len() < 10000 {
            // Only track reasonable-sized text
            add_to_clipboard_history(text_str)?;
        }
    }
    Ok(())
}
