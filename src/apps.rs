use crate::element::{Element, ElementList};
use anyhow::Result;
use path::PathBuf;
use process::Command;
use std::{
    env,
    fs::{self, File},
    path, process, time,
};
use time::{Duration, SystemTime};

const CACHE_TTL: Duration = Duration::from_secs(86400); // 24 hour

fn cache_path() -> PathBuf {
    let config_home_path = env::var("XDG_CONFIG_HOME")
        .or_else(|_| env::var("HOME").map(|home| format!("{}/.cache", home)))
        .unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(config_home_path)
        .join("kickoff")
        .join("apps.cache")
}

fn is_cache_valid(path: &PathBuf) -> bool {
    if !path.exists() {
        return false;
    }

    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                return elapsed < CACHE_TTL;
            }
        }
    }

    false
}

pub fn discover_applications() -> Result<ElementList> {
    let cache = cache_path();
    let cache_config = bincode::config::standard();

    if is_cache_valid(&cache) {
        if let Ok(mut file) = File::open(&cache) {
            if let Ok(elements) =
                bincode::decode_from_std_read::<Vec<Element>, _, _>(&mut file, cache_config)
            {
                crate::log!("Loaded {} apps from cache", elements.len());
                let mut list = ElementList::new();
                for element in elements {
                    list.add(element);
                }
                return Ok(list);
            }
        }
    }

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

        // Cache for next time
        if let Some(parent) = cache.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = File::create(&cache) {
            let _ = bincode::encode_into_std_write(&elements.inner, &mut file, cache_config);
        }
    }

    crate::log!("Discovered {} applications", elements.len());
    Ok(elements)
}
