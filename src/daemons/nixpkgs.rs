use crate::element::Element;
use crate::error::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct NixPackage {
    package_attr_name: String,
    package_pname: String,
    package_pversion: String,
    package_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NixSearchResponse {
    packages: Vec<NixPackage>,
}

/// Fetch nixpkgs and save to binary cache
fn fetch_nixpkgs() -> Result<Vec<Element>> {
    eprintln!("[nixpkgs daemon] Fetching packages from search.nixos.org...");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("kickoff/0.1.0")
        .build()?;

    // Fetch most popular packages (limited to keep size reasonable)
    let response = client
        .get("https://search.nixos.org/packages")
        .query(&[
            ("channel", "unstable"),
            ("size", "5000"), // Limit to 5000 packages
            ("sort", "relevance"),
        ])
        .send()?;

    let search_response: NixSearchResponse = response.json()?;

    let mut elements = Vec::new();
    for pkg in search_response.packages {
        let display = format!("{} v{}", pkg.package_pname, pkg.package_pversion);
        let url = format!(
            "https://search.nixos.org/packages?channel=unstable&query={}",
            pkg.package_attr_name
        );
        elements.push(Element::new_nix_package(display, url));
    }

    eprintln!("[nixpkgs daemon] Fetched {} packages", elements.len());
    Ok(elements)
}

/// Save nixpkgs to binary cache
fn save_nixpkgs(elements: &[Element]) -> Result<()> {
    let cache_path = crate::cache::cache_dir()?.join("nixpkgs.bin");
    let vec = elements.to_vec();
    crate::cache::save_cache("nixpkgs.bin", &vec)?;
    eprintln!(
        "[nixpkgs daemon] Saved {} packages to {:?}",
        elements.len(),
        cache_path
    );
    Ok(())
}

/// Run the nixpkgs daemon (one-shot execution)
pub fn run(_force: bool) -> Result<()> {
    eprintln!("[nixpkgs daemon] Starting...");

    // Fetch and save
    match fetch_nixpkgs() {
        Ok(packages) => {
            save_nixpkgs(&packages)?;
            eprintln!("[nixpkgs daemon] Complete");
        }
        Err(e) => {
            eprintln!("[nixpkgs daemon] Failed to fetch: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
