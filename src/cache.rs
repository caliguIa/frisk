use crate::error::Result;
use bincode::Encode;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;

pub fn cache_dir() -> Result<PathBuf> {
    let dir = PathBuf::from(
        env::var("XDG_CACHE_HOME")
            .or_else(|_| env::var("HOME").map(|home| format!("{}/.cache", home)))
            .map_err(|_| crate::error::Error::new("Could not determine cache directory"))?,
    )
    .join("frisk");

    fs::create_dir_all(&dir)?;
    Ok(dir)
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
