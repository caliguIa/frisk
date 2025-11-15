use crate::element::{Element, ElementList};
use crate::error::{Error, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

#[derive(Debug, Deserialize, Clone)]
struct FormulaInfo {
    name: String,
    #[serde(rename = "versions")]
    versions: Option<Versions>,
}

#[derive(Debug, Deserialize, Clone)]
struct Versions {
    stable: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct CaskInfo {
    token: String,
    name: Vec<String>,
    version: Option<String>,
}

// Simple text cache format: one line per package
static HOMEBREW_CACHE: OnceLock<Vec<(String, Option<String>, bool)>> = OnceLock::new();

const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

fn get_cache_path() -> Result<PathBuf> {
    let cache_dir = PathBuf::from(
        env::var("XDG_CACHE_HOME")
            .or_else(|_| env::var("HOME").map(|home| format!("{}/.cache", home)))
            .map_err(|_| Error::new("Could not determine cache directory"))?,
    )
    .join("kickoff-darwin");
    
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("homebrew_cache.txt"))
}

fn load_from_disk() -> Option<Vec<(String, Option<String>, bool)>> {
    let cache_path = get_cache_path().ok()?;
    
    if !cache_path.exists() {
        return None;
    }
    
    let metadata = fs::metadata(&cache_path).ok()?;
    let modified = metadata.modified().ok()?;
    let age = SystemTime::now().duration_since(modified).ok()?;
    
    if age > CACHE_DURATION {
        return None;
    }
    
    let content = fs::read_to_string(&cache_path).ok()?;
    let mut packages = Vec::new();
    
    for line in content.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let version = if parts[1].is_empty() { None } else { Some(parts[1].to_string()) };
            let is_cask = parts[2] == "1";
            packages.push((name, version, is_cask));
        }
    }
    
    crate::log!("Loaded {} packages from disk cache (age: {:?})", packages.len(), age);
    Some(packages)
}

fn save_to_disk(packages: &[(String, Option<String>, bool)]) -> Result<()> {
    let cache_path = get_cache_path()?;
    
    let mut lines = Vec::new();
    for (name, version, is_cask) in packages {
        let version_str = version.as_deref().unwrap_or("");
        let cask_flag = if *is_cask { "1" } else { "0" };
        lines.push(format!("{}\t{}\t{}", name, version_str, cask_flag));
    }
    
    fs::write(&cache_path, lines.join("\n"))?;
    crate::log!("Saved {} packages to disk cache", packages.len());
    Ok(())
}

pub fn search_homebrew(query: &str) -> Result<ElementList> {
    if query.is_empty() {
        return Ok(ElementList::new());
    }

    let packages = HOMEBREW_CACHE.get_or_init(|| {
        if let Some(cached) = load_from_disk() {
            return cached;
        }
        
        crate::log!("Downloading Homebrew data...");
        match download_homebrew_data() {
            Ok(data) => {
                if let Err(e) = save_to_disk(&data) {
                    crate::log!("Failed to save cache: {}", e);
                }
                data
            }
            Err(e) => {
                crate::log!("Failed to download Homebrew data: {}", e);
                Vec::new()
            }
        }
    });

    let mut elements = ElementList::new();
    let query_lower = query.to_lowercase();

    for (name, version, is_cask) in packages {
        if name.to_lowercase().contains(&query_lower) {
            let display = if let Some(ver) = version {
                if *is_cask {
                    format!("{} (cask) v{}", name, ver)
                } else {
                    format!("{} v{}", name, ver)
                }
            } else {
                if *is_cask {
                    format!("{} (cask)", name)
                } else {
                    name.clone()
                }
            };
            
            let url = if *is_cask {
                format!("https://formulae.brew.sh/cask/{}", name)
            } else {
                format!("https://formulae.brew.sh/formula/{}", name)
            };
            
            elements.add(Element::new_homebrew_package(display, url));
        }
    }

    Ok(elements)
}

fn download_homebrew_data() -> Result<Vec<(String, Option<String>, bool)>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("kickoff-darwin/0.1.0")
        .build()?;

    let mut packages = Vec::new();

    // Fetch formulae
    let response = client
        .get("https://formulae.brew.sh/api/formula.json")
        .send()?;
    let formulae: Vec<FormulaInfo> = response.json()?;
    
    for formula in formulae {
        let version = formula.versions.and_then(|v| v.stable);
        packages.push((formula.name, version, false));
    }

    // Fetch casks
    let response = client
        .get("https://formulae.brew.sh/api/cask.json")
        .send()?;
    let casks: Vec<CaskInfo> = response.json()?;
    
    for cask in casks {
        let display_name = if !cask.name.is_empty() {
            cask.name[0].clone()
        } else {
            cask.token.clone()
        };
        packages.push((display_name, cask.version, true));
    }

    crate::log!("Downloaded {} packages", packages.len());
    Ok(packages)
}
