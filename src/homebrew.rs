use crate::cache;
use crate::element::{Element, ElementList};
use crate::error::Result;
use bincode::{Decode, Encode};
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
struct FormulaInfo {
    name: String,
    #[serde(rename = "versions")]
    versions: Option<Versions>,
    homepage: Option<String>,
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
    homepage: Option<String>,
}

#[derive(Debug, Clone, Encode, Decode)]
struct BrewPackage {
    name: String,
    version: Option<String>,
    is_cask: bool,
    homepage: Option<String>,
}

static HOMEBREW_CACHE: OnceLock<Vec<BrewPackage>> = OnceLock::new();

const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60);

pub fn search_homebrew(query: &str) -> Result<ElementList> {
    if query.is_empty() {
        return Ok(ElementList::new());
    }

    let packages = HOMEBREW_CACHE.get_or_init(|| {
        if let Some(cached) = cache::load_cache("homebrew.cache", CACHE_DURATION) {
            return cached;
        }

        crate::log!("Downloading Homebrew data...");
        match download_homebrew_data() {
            Ok(data) => {
                if let Err(e) = cache::save_cache("homebrew.cache", &data.to_vec()) {
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

    for pkg in packages {
        if pkg.name.to_lowercase().contains(&query_lower) {
            let display = if let Some(ver) = &pkg.version {
                if pkg.is_cask {
                    format!("{} (cask) v{}", pkg.name, ver)
                } else {
                    format!("{} v{}", pkg.name, ver)
                }
            } else {
                if pkg.is_cask {
                    format!("{} (cask)", pkg.name)
                } else {
                    pkg.name.clone()
                }
            };

            let url = if let Some(home) = &pkg.homepage {
                home.clone()
            } else if pkg.is_cask {
                format!("https://formulae.brew.sh/cask/{}", pkg.name)
            } else {
                format!("https://formulae.brew.sh/formula/{}", pkg.name)
            };

            elements.add(Element::new_homebrew_package(display, url));
        }
    }

    Ok(elements)
}

fn download_homebrew_data() -> Result<Vec<BrewPackage>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("kickoff/0.1.0")
        .build()?;

    let mut packages = Vec::new();

    let response = client
        .get("https://formulae.brew.sh/api/formula.json")
        .send()?;
    let formulae: Vec<FormulaInfo> = response.json()?;

    for formula in formulae {
        let version = formula.versions.and_then(|v| v.stable);
        packages.push(BrewPackage {
            name: formula.name,
            version,
            is_cask: false,
            homepage: formula.homepage,
        });
    }

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
        packages.push(BrewPackage {
            name: display_name,
            version: cask.version,
            is_cask: true,
            homepage: cask.homepage,
        });
    }

    crate::log!("Downloaded {} packages", packages.len());
    Ok(packages)
}
