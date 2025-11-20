use crate::core::element::Element;
use crate::core::error::Result;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct NixpkgsSearchResult {
    package_attr_name: String,
    package_pname: String,
    package_pversion: String,
    package_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NixpkgsSearchResponse {
    hits: NixpkgsSearchHits,
}

#[derive(Debug, Deserialize)]
struct NixpkgsSearchHits {
    hits: Vec<NixpkgsSearchHit>,
}

#[derive(Debug, Deserialize)]
struct NixpkgsSearchHit {
    #[serde(rename = "_source")]
    source: NixpkgsSearchResult,
    sort: Option<Vec<serde_json::Value>>,
}

fn get_search_url() -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("frisk/0.1.0")
        .build()?;

    let response = client
        .get("https://raw.githubusercontent.com/NixOS/nixos-search/main/version.nix")
        .send()?;

    let text = response.text()?;

    // Extract frontend = "44"; (simple string search)
    let frontend = text
        .lines()
        .find(|line| line.contains("frontend"))
        .and_then(|line| {
            // Extract number between quotes
            line.split('"').nth(1)
        })
        .ok_or_else(|| {
            crate::core::error::Error::new("Could not parse frontend version from version.nix")
        })?;

    let url = format!(
        "https://search.nixos.org/backend/latest-{}-nixos-unstable/_search",
        frontend
    );

    eprintln!("[nixpkgs daemon] Using search URL: {}", url);
    Ok(url)
}

fn fetch_nixpkgs_batch(
    url: &str,
    search_after: Option<Vec<serde_json::Value>>,
    size: usize,
) -> Result<(Vec<Element>, Option<Vec<serde_json::Value>>)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("frisk/0.1.0")
        .build()?;

    let mut search_body = serde_json::json!({
        "size": size,
        "sort": [
            { "package_attr_name": "asc" }
        ],
        "query": {
            "bool": {
                "filter": [{ "term": { "type": { "value": "package" } } }]
            }
        }
    });

    if let Some(after) = search_after {
        search_body["search_after"] = serde_json::json!(after);
    }

    let response = client
        .post(url)
        .header(
            "Authorization",
            "Basic YVdWU0FMWHBadjpYOGdQSG56TDUyd0ZFZWt1eHNmUTljU2g=",
        )
        .header("Content-Type", "application/json")
        .json(&search_body)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Could not read body".to_string());
        eprintln!("[nixpkgs daemon] API error {}: {}", status, body);
        return Err(crate::core::error::Error::new(format!(
            "API returned {}",
            status
        )));
    }

    let search_response: NixpkgsSearchResponse = response.json()?;

    let mut elements = Vec::new();
    let mut last_sort = None;

    for hit in search_response.hits.hits {
        let result = hit.source;
        let display_name = if !result.package_pversion.is_empty() {
            format!("{} v{}", result.package_pname, result.package_pversion)
        } else {
            result.package_pname.clone()
        };

        elements.push(Element::new_nix_package(
            display_name,
            result.package_attr_name,
        ));

        last_sort = hit.sort;
    }

    Ok((elements, last_sort))
}

fn fetch_nixpkgs() -> Result<Vec<Element>> {
    let url = get_search_url()?;
    let mut all_packages = Vec::new();
    let mut search_after = None;
    let size = 10000;
    let mut batch_num = 0;

    eprintln!("[nixpkgs daemon] Fetching packages...");

    loop {
        batch_num += 1;
        eprintln!("[nixpkgs daemon] Fetching batch {}...", batch_num);
        let (batch, next_search_after) = fetch_nixpkgs_batch(&url, search_after, size)?;
        let count = batch.len();
        all_packages.extend(batch);

        if count < size || next_search_after.is_none() {
            break; // Last page
        }
        search_after = next_search_after;
    }

    eprintln!(
        "[nixpkgs daemon] Fetched {} packages total",
        all_packages.len()
    );
    Ok(all_packages)
}

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

pub fn run() -> Result<()> {
    eprintln!("[nixpkgs daemon] Starting...");

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
