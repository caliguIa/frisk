pub mod apps;
pub mod clipboard;
pub mod homebrew;
pub mod nixpkgs;

use crate::error::Result;

/// Common utilities for daemons
pub fn cache_dir() -> Result<std::path::PathBuf> {
    crate::cache::cache_dir()
}
