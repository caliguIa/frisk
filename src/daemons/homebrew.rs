use crate::element::Element;
use crate::error::Result;
use serde::Deserialize;

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

fn fetch_homebrew() -> Result<Vec<Element>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("frisk/0.1.0")
        .build()?;

    let mut elements = Vec::new();

    eprintln!("[homebrew daemon] Fetching formulae...");
    let response = client
        .get("https://formulae.brew.sh/api/formula.json")
        .send()?;
    let formulae: Vec<FormulaInfo> = response.json()?;

    for formula in formulae {
        let version = formula.versions.and_then(|v| v.stable);
        let display = if let Some(ver) = &version {
            format!("{} v{}", formula.name, ver)
        } else {
            formula.name.clone()
        };

        let url = formula
            .homepage
            .unwrap_or_else(|| format!("https://formulae.brew.sh/formula/{}", formula.name));

        elements.push(Element::new_homebrew_package(display, url));
    }

    eprintln!("[homebrew daemon] Fetching casks...");
    let response = client
        .get("https://formulae.brew.sh/api/cask.json")
        .send()?;
    let casks: Vec<CaskInfo> = response.json()?;

    for cask in casks {
        let display_name = if !cask.name.is_empty() {
            &cask.name[0]
        } else {
            &cask.token
        };

        let display = if let Some(ver) = &cask.version {
            format!("{} (cask) v{}", display_name, ver)
        } else {
            format!("{} (cask)", display_name)
        };

        let url = cask
            .homepage
            .unwrap_or_else(|| format!("https://formulae.brew.sh/cask/{}", cask.token));

        elements.push(Element::new_homebrew_package(display, url));
    }

    eprintln!("[homebrew daemon] Fetched {} packages", elements.len());
    Ok(elements)
}

fn save_homebrew(elements: &[Element]) -> Result<()> {
    let cache_path = crate::cache::cache_dir()?.join("homebrew.bin");
    let vec = elements.to_vec();
    crate::cache::save_cache("homebrew.bin", &vec)?;
    eprintln!(
        "[homebrew daemon] Saved {} packages to {:?}",
        elements.len(),
        cache_path
    );
    Ok(())
}

pub fn run() -> Result<()> {
    eprintln!("[homebrew daemon] Starting...");

    match fetch_homebrew() {
        Ok(packages) => {
            save_homebrew(&packages)?;
            eprintln!("[homebrew daemon] Complete");
        }
        Err(e) => {
            eprintln!("[homebrew daemon] Failed to fetch: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
