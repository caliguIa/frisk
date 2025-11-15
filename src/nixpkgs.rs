use crate::element::{Element, ElementList};
use anyhow::Result;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize, Clone)]
struct Package {
    package_attr_name: String,
    package_description: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchQuery {
    size: usize,
    sort: Vec<serde_json::Value>,
    query: QueryBody,
}

#[derive(Debug, Serialize)]
struct QueryBody {
    bool: BoolQuery,
}

#[derive(Debug, Serialize)]
struct BoolQuery {
    filter: Vec<serde_json::Value>,
    must: Vec<serde_json::Value>,
}

fn get_search_url() -> Result<String> {
    let resp = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?
        .get("https://raw.githubusercontent.com/NixOS/nixos-search/main/version.nix")
        .send()?;
    let text = resp.text()?;

    let re = regex::Regex::new(r#"frontend\s*=\s*"([^"]+)"\s*;"#)?;
    let frontend = re
        .captures(&text)
        .and_then(|cap| cap.get(1))
        .ok_or_else(|| anyhow::anyhow!("Cannot parse frontend version"))?
        .as_str();

    Ok(format!(
        "https://search.nixos.org/backend/latest-{}-nixos-unstable/_search",
        frontend
    ))
}

// Search with full query fields like Raycast - called on every keystroke
pub fn search_nixpkgs(query: &str) -> Result<ElementList> {
    let mut elements = ElementList::new();

    if query.is_empty() {
        return Ok(elements);
    }

    let url = get_search_url()?;

    // Full query fields from Raycast extension
    let query_fields = vec![
        "package_attr_name^9",
        "package_attr_name.edge^9",
        "package_pname^6",
        "package_pname.edge^6",
        "package_attr_name_query^4",
        "package_attr_name_query.edge^4",
        "package_description^1.3",
        "package_description.edge^1.3",
        "package_longDescription^1",
        "package_longDescription.edge^1",
        "flake_name^0.5",
        "flake_name.edge^0.5",
        "package_attr_name_reverse^7.2",
        "package_attr_name_reverse.edge^7.2",
        "package_pname_reverse^4.800000000000001",
        "package_pname_reverse.edge^4.800000000000001",
        "package_attr_name_query_reverse^3.2",
        "package_attr_name_query_reverse.edge^3.2",
        "package_description_reverse^1.04",
        "package_description_reverse.edge^1.04",
        "package_longDescription_reverse^0.8",
        "package_longDescription_reverse.edge^0.8",
        "flake_name_reverse^0.4",
        "flake_name_reverse.edge^0.4",
    ];

    let reversed_query: String = query.chars().rev().collect();

    let search_query = SearchQuery {
        size: 50,
        sort: vec![
            serde_json::json!({"_score": "desc"}),
            serde_json::json!({"package_attr_name": "desc"}),
            serde_json::json!({"package_pversion": "desc"}),
        ],
        query: QueryBody {
            bool: BoolQuery {
                filter: vec![serde_json::json!({
                    "term": {
                        "type": {
                            "value": "package",
                            "_name": "filter_packages"
                        }
                    }
                })],
                must: vec![serde_json::json!({
                    "dis_max": {
                        "tie_breaker": 0.7,
                        "queries": [
                            {
                                "multi_match": {
                                    "type": "cross_fields",
                                    "query": query,
                                    "analyzer": "whitespace",
                                    "auto_generate_synonyms_phrase_query": false,
                                    "operator": "and",
                                    "_name": format!("multi_match_{}", query),
                                    "fields": query_fields.clone(),
                                }
                            },
                            {
                                "multi_match": {
                                    "type": "cross_fields",
                                    "query": reversed_query,
                                    "analyzer": "whitespace",
                                    "auto_generate_synonyms_phrase_query": false,
                                    "operator": "and",
                                    "_name": format!("multi_match_{}", reversed_query),
                                    "fields": query_fields,
                                }
                            },
                            {
                                "wildcard": {
                                    "package_attr_name": {
                                        "value": format!("*{}*", query)
                                    }
                                }
                            }
                        ]
                    }
                })],
            },
        },
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(800)) // 800ms timeout - fast enough to feel responsive
        .build()?;

    let resp = client
        .post(&url)
        .header(
            "Authorization",
            "Basic YVdWU0FMWHBadjpYOGdQSG56TDUyd0ZFZWt1eHNmUTljU2g=",
        )
        .header("Content-Type", "application/json")
        .json(&search_query)
        .send()?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Search failed: {}", resp.status()));
    }

    let search_response: SearchResponse = resp.json()?;

    for hit in search_response.hits.hits {
        let pkg = hit.source;
        let display = if let Some(desc) = &pkg.package_description {
            let desc_short = if desc.len() > 80 {
                format!("{}...", &desc[..80])
            } else {
                desc.clone()
            };
            format!("{} - {}", pkg.package_attr_name, desc_short)
        } else {
            pkg.package_attr_name.clone()
        };

        elements.add(Element::new_nix_package(display, pkg.package_attr_name));
    }

    Ok(elements)
}
