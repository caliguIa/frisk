use crate::element::{Element, ElementList};
use crate::error::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CratesResponse {
    crates: Vec<CrateInfo>,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    name: String,
    #[serde(rename = "max_version")]
    version: String,
    downloads: u64,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    homepage: Option<String>,
    #[allow(dead_code)]
    repository: Option<String>,
    #[allow(dead_code)]
    documentation: Option<String>,
}

/// Search crates.io API for Rust crates
///
/// Uses the official crates.io API: https://crates.io/api/v1/crates
/// Returns up to 100 results, sorted by relevance
pub fn search_crates(query: &str) -> Result<ElementList> {
    if query.is_empty() {
        return Ok(ElementList::new());
    }

    // Simple URL encoding for query parameter
    let encoded = query
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else if c == ' ' {
                "+".to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect::<String>();

    let url = format!(
        "https://crates.io/api/v1/crates?page=1&per_page=100&q={}",
        encoded
    );

    crate::log!("Searching crates.io: {}", url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .user_agent("kickoff-darwin/0.1.0")
        .build()?;

    let response = client.get(&url).send()?;
    let crates_response: CratesResponse = response.json()?;

    let mut elements = ElementList::new();

    for crate_info in crates_response.crates {
        // Format display name with version and download count
        let name = format!(
            "{} v{} (↓ {})",
            crate_info.name,
            crate_info.version,
            format_downloads(crate_info.downloads)
        );

        // Value is the crates.io URL for the crate
        let value = format!("https://crates.io/crates/{}", crate_info.name);

        let element = Element::new_rust_crate(name, value);
        elements.add(element);
    }

    crate::log!("Found {} crates", elements.len());
    Ok(elements)
}

/// Format download count with K/M suffixes
fn format_downloads(downloads: u64) -> String {
    if downloads >= 1_000_000 {
        format!("{:.1}M", downloads as f64 / 1_000_000.0)
    } else if downloads >= 1_000 {
        format!("{:.1}K", downloads as f64 / 1_000.0)
    } else {
        downloads.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_downloads() {
        assert_eq!(format_downloads(500), "500");
        assert_eq!(format_downloads(1_500), "1.5K");
        assert_eq!(format_downloads(1_500_000), "1.5M");
        assert_eq!(format_downloads(42_123_456), "42.1M");
    }
}
