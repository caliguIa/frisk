use crate::element::{Element, ElementList};
use anyhow::{Context, Result};

use log::{debug, warn};
use plist::{Dictionary, Value};
use std::path::{Path, PathBuf};
use std::{env, fs, process::Command};
use tokio::task;

pub struct AppDiscovery {
    pub applications_dir: Vec<PathBuf>,
    pub include_path: bool,
}

impl Default for AppDiscovery {
    fn default() -> Self {
        Self {
            applications_dir: vec![
                PathBuf::from("/Applications"),
                PathBuf::from("/System/Applications"),
                PathBuf::from("/System/Library/CoreServices/Applications"),
            ],
            include_path: true,
        }
    }
}

impl AppDiscovery {
    pub fn new(include_path: bool) -> Self {
        let mut discovery = Self::default();
        discovery.include_path = include_path;
        
        // Add user Applications directory
        if let Ok(home) = env::var("HOME") {
            discovery.applications_dir.push(PathBuf::from(home).join("Applications"));
        }
        
        discovery
    }

    pub async fn discover_all(&self) -> Result<ElementList> {
        let mut elements = ElementList::new();

        // Discover macOS applications
        let apps = self.discover_applications().await?;
        elements.extend(apps);

        // Discover PATH executables if enabled
        if self.include_path {
            let path_executables = self.discover_path_executables().await?;
            elements.extend(path_executables);
        }

        elements.sort_by_score();
        debug!("Discovered {} total items", elements.len());
        
        Ok(elements)
    }

    pub async fn discover_apps_only(&self) -> Result<ElementList> {
        let mut elements = ElementList::new();

        // Discover only macOS applications
        let apps = self.discover_applications().await?;
        elements.extend(apps);

        elements.sort_by_score();
        debug!("Discovered {} applications", elements.len());
        
        Ok(elements)
    }

    pub async fn discover_path_only(&self) -> Result<ElementList> {
        let mut elements = ElementList::new();

        // Discover only PATH executables
        let path_executables = self.discover_path_executables().await?;
        elements.extend(path_executables);

        elements.sort_by_score();
        debug!("Discovered {} PATH executables", elements.len());
        
        Ok(elements)
    }

    async fn discover_applications(&self) -> Result<Vec<Element>> {
        let dirs = self.applications_dir.clone();
        
        task::spawn_blocking(move || {
            let mut apps = Vec::new();
            
            for dir in &dirs {
                if !dir.exists() {
                    continue;
                }
                
                debug!("Scanning directory: {}", dir.display());
                
                Self::scan_directory_recursive(dir, &mut apps);
            }
            
            debug!("Found {} applications", apps.len());
            apps
        })
        .await
        .context("Failed to discover applications")
    }

    fn scan_directory_recursive(dir: &Path, apps: &mut Vec<Element>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if path.extension().map_or(false, |ext| ext == "app") {
                        // This is an app bundle
                        if let Some(app) = Self::parse_app_bundle(&path) {
                            apps.push(app);
                        }
                    } else {
                        // This might be a subdirectory containing apps, recurse into it
                        Self::scan_directory_recursive(&path, apps);
                    }
                }
            }
        }
    }

    fn parse_app_bundle(path: &Path) -> Option<Element> {
        if !path.is_dir() || !path.extension().map_or(false, |ext| ext == "app") {
            return None;
        }

        let info_plist_path = path.join("Contents").join("Info.plist");
        if !info_plist_path.exists() {
            return None;
        }

        let plist = match plist::from_file(&info_plist_path) {
            Ok(Value::Dictionary(dict)) => dict,
            Ok(_) => {
                warn!("Info.plist is not a dictionary: {}", info_plist_path.display());
                return None;
            }
            Err(e) => {
                warn!("Failed to parse Info.plist at {}: {}", info_plist_path.display(), e);
                return None;
            }
        };

        let bundle_name = plist
            .get("CFBundleName")
            .or_else(|| plist.get("CFBundleDisplayName"))
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown App")
            });

        let bundle_identifier = plist
            .get("CFBundleIdentifier")
            .and_then(|v| v.as_string())
            .unwrap_or("");

        let icon_path = Self::find_app_icon(path, &plist);

        let element = Element::new(bundle_name.to_string(), path.display().to_string())
            .with_bundle_path(Some(path.to_path_buf()))
            .with_icon(icon_path);

        debug!("Found app: {} ({})", bundle_name, bundle_identifier);
        
        Some(element)
    }

    fn find_app_icon(app_path: &Path, plist: &Dictionary) -> Option<PathBuf> {
        let resources_dir = app_path.join("Contents").join("Resources");
        if !resources_dir.exists() {
            return None;
        }

        // Try to get icon name from Info.plist
        let icon_file = plist
            .get("CFBundleIconFile")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .or_else(|| {
                // Try common icon files
                let common_names = ["icon.icns", "app.icns", "AppIcon.icns"];
                for name in &common_names {
                    let icon_path = resources_dir.join(name);
                    if icon_path.exists() {
                        return Some(name.to_string());
                    }
                }
                None
            })?;

        let icon_path = if icon_file.ends_with(".icns") {
            resources_dir.join(&icon_file)
        } else {
            resources_dir.join(format!("{}.icns", icon_file))
        };

        if icon_path.exists() {
            Some(icon_path)
        } else {
            None
        }
    }

    async fn discover_path_executables(&self) -> Result<Vec<Element>> {
        task::spawn_blocking(|| {
            let mut executables = Vec::new();
            
            if let Ok(path_env) = env::var("PATH") {
                for dir_str in path_env.split(':') {
                    let dir = PathBuf::from(dir_str);
                    if !dir.exists() || !dir.is_dir() {
                        continue;
                    }

                    if let Ok(entries) = fs::read_dir(&dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Ok(metadata) = fs::metadata(&path) {
                                    let permissions = metadata.permissions();
                                    #[cfg(unix)]
                                    {
                                        use std::os::unix::fs::PermissionsExt;
                                        if permissions.mode() & 0o111 != 0 {
                                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                                let element = Element::new(
                                                    name.to_string(),
                                                    name.to_string()
                                                );
                                                executables.push(element);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            debug!("Found {} PATH executables", executables.len());
            executables
        })
        .await
        .context("Failed to discover PATH executables")
    }
}

pub async fn launch_application(element: &Element) -> Result<()> {
    let command = element.value.clone();
    
    task::spawn_blocking(move || {
        debug!("Launching: {}", command);
        
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&command);
        
        match cmd.spawn() {
            Ok(mut child) => {
                // Detach the child process
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                Ok(())
            }
            Err(e) => {
                warn!("Failed to launch '{}': {}", command, e);
                Err(anyhow::anyhow!("Failed to launch application: {}", e))
            }
        }
    })
    .await
    .context("Failed to spawn launch task")?
}