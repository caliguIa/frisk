use crate::element::{Element, ElementList};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FormulaInfo {
    name: String,
    #[serde(rename = "full_name")]
    full_name: Option<String>,
    desc: Option<String>,
    #[allow(dead_code)]
    homepage: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CaskInfo {
    token: String,
    name: Vec<String>,
    desc: Option<String>,
    #[allow(dead_code)]
    homepage: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedData {
    timestamp: SystemTime,
    formulae: Vec<FormulaInfo>,
    casks: Vec<CaskInfo>,
}

// Global cache for downloaded Homebrew data
static HOMEBREW_CACHE: OnceLock<(Vec<FormulaInfo>, Vec<CaskInfo>)> = OnceLock::new();

const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

fn get_cache_path() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
        .join("kickoff-darwin");
    
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("homebrew_cache.json"))
}

fn load_from_disk() -> Option<(Vec<FormulaInfo>, Vec<CaskInfo>)> {
    let cache_path = get_cache_path().ok()?;
    
    if !cache_path.exists() {
        crate::log!("No disk cache found");
        return None;
    }
    
    let cache_data = fs::read_to_string(&cache_path).ok()?;
    let cached: CachedData = serde_json::from_str(&cache_data).ok()?;
    
    // Check if cache is still valid (< 24 hours old)
    let age = SystemTime::now().duration_since(cached.timestamp).ok()?;
    if age > CACHE_DURATION {
        crate::log!("Disk cache expired (age: {:?})", age);
        return None;
    }
    
    crate::log!("Loaded from disk cache (age: {:?})", age);
    Some((cached.formulae, cached.casks))
}

fn save_to_disk(formulae: &[FormulaInfo], casks: &[CaskInfo]) -> Result<()> {
    let cache_path = get_cache_path()?;
    
    let cached = CachedData {
        timestamp: SystemTime::now(),
        formulae: formulae.to_vec(),
        casks: casks.to_vec(),
    };
    
    let json = serde_json::to_string(&cached)?;
    fs::write(&cache_path, json)?;
    
    crate::log!("Saved to disk cache: {:?}", cache_path);
    Ok(())
}

/// Search Homebrew formulae and casks
/// 
/// Uses 24-hour disk cache if available, otherwise downloads from formulae.brew.sh
/// Caches results in memory for subsequent searches (fast!)
/// Returns matching items based on client-side filtering
pub fn search_homebrew(query: &str) -> Result<ElementList> {
    if query.is_empty() {
        return Ok(ElementList::new());
    }

    crate::log!("Searching Homebrew for: {}", query);

    // Get or initialize cache (checks disk first, then downloads)
    let (formulae, casks) = HOMEBREW_CACHE.get_or_init(|| {
        // Try disk cache first
        if let Some(cached) = load_from_disk() {
            return cached;
        }
        
        // Disk cache miss - download from API
        crate::log!("Cache miss - downloading Homebrew data");
        match download_homebrew_data() {
            Ok(data) => {
                // Save to disk for next time
                if let Err(e) = save_to_disk(&data.0, &data.1) {
                    crate::log!("Failed to save cache to disk: {}", e);
                }
                data
            }
            Err(e) => {
                crate::log!("Failed to download Homebrew data: {}", e);
                (Vec::new(), Vec::new())
            }
        }
    });

    crate::log!("Using cached data: {} formulae, {} casks", formulae.len(), casks.len());

    // Filter based on query
    let mut elements = ElementList::new();
    let query_lower = query.to_lowercase();

    // Search formulae
    for formula in formulae {
        if matches_query(&formula.name, &formula.desc, &query_lower) {
            let display_name = format_formula_name(formula);
            let url = format!("https://formulae.brew.sh/formula/{}", formula.name);
            elements.add(Element::new_homebrew_package(display_name, url));
        }
    }

    // Search casks
    for cask in casks {
        if matches_cask_query(cask, &query_lower) {
            let display_name = format_cask_name(cask);
            let url = format!("https://formulae.brew.sh/cask/{}", cask.token);
            elements.add(Element::new_homebrew_package(display_name, url));
        }
    }

    crate::log!("Found {} matching items", elements.len());
    Ok(elements)
}

fn download_homebrew_data() -> Result<(Vec<FormulaInfo>, Vec<CaskInfo>)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(5000)) // Longer timeout for downloading full lists
        .user_agent("kickoff-darwin/0.1.0")
        .build()?;

    crate::log!("Downloading formula.json...");
    let formulae = fetch_formulae(&client)?;
    crate::log!("Downloaded {} formulae", formulae.len());

    crate::log!("Downloading cask.json...");
    let casks = fetch_casks(&client)?;
    crate::log!("Downloaded {} casks", casks.len());

    Ok((formulae, casks))
}

fn fetch_formulae(client: &reqwest::blocking::Client) -> Result<Vec<FormulaInfo>> {
    let response = client
        .get("https://formulae.brew.sh/api/formula.json")
        .send()?;
    let formulae: Vec<FormulaInfo> = response.json()?;
    Ok(formulae)
}

fn fetch_casks(client: &reqwest::blocking::Client) -> Result<Vec<CaskInfo>> {
    let response = client
        .get("https://formulae.brew.sh/api/cask.json")
        .send()?;
    let casks: Vec<CaskInfo> = response.json()?;
    Ok(casks)
}

fn matches_query(name: &str, desc: &Option<String>, query: &str) -> bool {
    // Match on name (most important)
    if name.to_lowercase().contains(query) {
        return true;
    }
    
    // Match on description
    if let Some(desc) = desc {
        if desc.to_lowercase().contains(query) {
            return true;
        }
    }
    
    false
}

fn matches_cask_query(cask: &CaskInfo, query: &str) -> bool {
    // Match on token
    if cask.token.to_lowercase().contains(query) {
        return true;
    }
    
    // Match on any of the names
    for name in &cask.name {
        if name.to_lowercase().contains(query) {
            return true;
        }
    }
    
    // Match on description
    if let Some(desc) = &cask.desc {
        if desc.to_lowercase().contains(query) {
            return true;
        }
    }
    
    false
}

fn format_formula_name(formula: &FormulaInfo) -> String {
    let name = formula.full_name.as_ref().unwrap_or(&formula.name);
    
    if let Some(desc) = &formula.desc {
        // Truncate long descriptions
        let short_desc = if desc.len() > 60 {
            format!("{}...", &desc[..57])
        } else {
            desc.clone()
        };
        format!("{} - {}", name, short_desc)
    } else {
        name.clone()
    }
}

fn format_cask_name(cask: &CaskInfo) -> String {
    let display_name = if !cask.name.is_empty() {
        cask.name[0].clone()
    } else {
        cask.token.clone()
    };
    
    if let Some(desc) = &cask.desc {
        // Truncate long descriptions
        let short_desc = if desc.len() > 60 {
            format!("{}...", &desc[..57])
        } else {
            desc.clone()
        };
        format!("{} (cask) - {}", display_name, short_desc)
    } else {
        format!("{} (cask)", display_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_query() {
        assert!(matches_query("firefox", &Some("Browser".to_string()), "fire"));
        assert!(matches_query("python", &Some("Language".to_string()), "lang"));
        assert!(!matches_query("rust", &Some("Language".to_string()), "python"));
    }
}
