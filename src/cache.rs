use crate::error::Result;
use bincode::{Decode, Encode};
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub fn cache_dir() -> Result<PathBuf> {
    let dir = PathBuf::from(
        env::var("XDG_CACHE_HOME")
            .or_else(|_| env::var("HOME").map(|home| format!("{}/.cache", home)))
            .map_err(|_| crate::error::Error::new("Could not determine cache directory"))?,
    )
    .join("kickoff");

    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn is_cache_valid(path: &PathBuf, ttl: Duration) -> bool {
    if !path.exists() {
        return false;
    }

    if let Ok(metadata) = fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                return elapsed < ttl;
            }
        }
    }

    false
}

pub fn load_cache<T>(name: &str, ttl: Duration) -> Option<T>
where
    T: Decode<()>,
{
    let path = cache_dir().ok()?.join(name);

    if !is_cache_valid(&path, ttl) {
        return None;
    }

    let mut file = File::open(&path).ok()?;
    let config = bincode::config::standard();

    match bincode::decode_from_std_read::<T, _, _>(&mut file, config) {
        Ok(data) => {
            crate::log!("Loaded cache from {}", name);
            Some(data)
        }
        Err(e) => {
            crate::log!("Failed to load cache {}: {}", name, e);
            None
        }
    }
}

pub fn save_cache<T>(name: &str, data: &T) -> Result<()>
where
    T: Encode,
{
    let path = cache_dir()?.join(name);
    let mut file = File::create(&path)?;
    let config = bincode::config::standard();

    bincode::encode_into_std_write(data, &mut file, config)?;
    crate::log!("Saved cache to {}", name);
    Ok(())
}
