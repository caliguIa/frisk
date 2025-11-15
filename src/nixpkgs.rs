use crate::element::{Element, ElementList};
use crate::error::{Error, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Hits,
}

#[derive(Debug, Deserialize)]
struct Hits {
    hits: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct Hit {
    #[serde(rename = "_source")]
    source: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    package_attr_name: String,
    #[serde(rename = "package_pversion")]
    package_version: Option<String>,
}

fn get_search_url() -> Result<String> {
    let resp = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?
        .get("https://raw.githubusercontent.com/NixOS/nixos-search/main/version.nix")
        .send()?;
    let text = resp.text()?;

    let frontend = text
        .lines()
        .find(|line| line.contains("frontend"))
        .and_then(|line| line.split('"').nth(1))
        .ok_or_else(|| Error::new("Cannot parse frontend version"))?;

    Ok(format!(
        "https://search.nixos.org/backend/latest-{}-nixos-unstable/_search",
        frontend
    ))
}

pub fn search_nixpkgs(query: &str) -> Result<ElementList> {
    let mut elements = ElementList::new();

    if query.is_empty() {
        return Ok(elements);
    }

    let url = get_search_url()?;

    // Simple wildcard query - just search package names
    let search_body = format!(
        r#"{{
            "size": 50,
            "query": {{
                "bool": {{
                    "filter": [{{"term": {{"type": "package"}}}}],
                    "must": [{{"wildcard": {{"package_attr_name": "*{}*"}}}}]
                }}
            }}
        }}"#,
        query.to_lowercase()
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()?;

    let resp = client
        .post(&url)
        .header(
            "Authorization",
            "Basic YVdWU0FMWHBadjpYOGdQSG56TDUyd0ZFZWt1eHNmUTljU2g=",
        )
        .header("Content-Type", "application/json")
        .body(search_body)
        .send()?;

    if !resp.status().is_success() {
        return Err(Error::new(format!("Search failed: {}", resp.status())));
    }

    let search_response: SearchResponse = resp.json()?;

    for hit in search_response.hits.hits {
        let pkg = hit.source;

        let display = if let Some(version) = &pkg.package_version {
            format!("{} v{}", pkg.package_attr_name, version)
        } else {
            pkg.package_attr_name.clone()
        };

        elements.add(Element::new_nix_package(display, pkg.package_attr_name));
    }

    Ok(elements)
}
