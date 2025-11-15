use crate::cache;
use crate::element::{Element, ElementList};
use crate::error::Result;
use std::process::Command;
use std::time::Duration;

const CACHE_TTL: Duration = Duration::from_secs(86400); // 24 hours

pub fn discover_applications() -> Result<ElementList> {
    // Try to load from cache
    if let Some(elements_vec) = cache::load_cache::<Vec<Element>>("apps.cache", CACHE_TTL) {
        let mut list = ElementList::new();
        for element in elements_vec {
            list.add(element);
        }
        return Ok(list);
    }

    // Cache miss - discover applications
    let mut elements = ElementList::new();
    let output = Command::new("mdfind")
        .arg("kMDItemKind == 'Application'")
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let path = line.trim();
            if path.ends_with(".app") {
                if let Some(name) = path.rsplit('/').next().and_then(|s| s.strip_suffix(".app")) {
                    elements.add(Element::new(name.to_string(), path.to_string()));
                }
            }
        }

        // Save to cache
        let _ = cache::save_cache("apps.cache", &elements.inner);
    }

    crate::log!("Discovered {} applications", elements.len());
    Ok(elements)
}
