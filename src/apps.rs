use crate::element::{Element, ElementList};
use anyhow::Result;
use log::debug;
use std::fs;
use std::path::Path;

pub fn discover_applications() -> Result<ElementList> {
    let mut elements = ElementList::new();

    let dirs = [
        "/Applications",
        "/System/Applications",
        "/System/Library/CoreServices/Applications",
    ];

    if let Ok(home) = std::env::var("HOME") {
        scan_directory(Path::new(&format!("{}/Applications", home)), &mut elements);
    }

    for dir in &dirs {
        scan_directory(Path::new(dir), &mut elements);
    }

    debug!("Discovered {} applications", elements.len());
    Ok(elements)
}

fn scan_directory(dir: &Path, elements: &mut ElementList) {
    if !dir.exists() {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.extension().and_then(|s| s.to_str()) == Some("app") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let app = Element::new(name.to_string(), path.display().to_string());
                    elements.add(app);
                }
            } else {
                scan_directory(&path, elements);
            }
        }
    }
}
