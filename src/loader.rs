use crate::element::Element;
use crate::error::Result;
use bincode::config;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Load binary source from cache directory by name
pub fn load_binary_source(name: &str) -> Result<Option<Vec<Element>>> {
    let cache_dir = crate::cache::cache_dir()?;
    let path = cache_dir.join(name);
    
    if !path.exists() {
        return Ok(None);
    }
    
    load_binary_file(&path).map(Some)
}

/// Load binary file from arbitrary path
pub fn load_binary_file(path: &Path) -> Result<Vec<Element>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    
    let config = config::standard();
    let (elements, _): (Vec<Element>, usize) = bincode::decode_from_slice(&bytes, config)?;
    
    Ok(elements)
}
