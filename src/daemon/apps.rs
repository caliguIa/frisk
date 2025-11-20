use crate::core::element::Element;
use crate::core::error::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

const DEBOUNCE_DURATION: Duration = Duration::from_secs(2);

fn watch_dirs() -> Vec<&'static str> {
    vec![
        "/Applications",
        "/System/Applications",
        "/System/Applications/Utilities",
    ]
}

fn discover_applications() -> Result<Vec<Element>> {
    let mut elements = Vec::new();

    let output = Command::new("mdfind")
        .arg("kMDItemKind == 'Application'")
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let path = line.trim();
            if path.ends_with(".app") {
                if let Some(name) = path.rsplit('/').next().and_then(|s| s.strip_suffix(".app")) {
                    elements.push(Element::new(name.to_string(), path.to_string()));
                }
            }
        }
    }

    eprintln!("[apps daemon] Discovered {} applications", elements.len());
    Ok(elements)
}

fn save_apps(elements: &[Element]) -> Result<()> {
    let cache_path = crate::cache::cache_dir()?.join("apps.bin");
    let vec = elements.to_vec();
    crate::cache::save_cache("apps.bin", &vec)?;
    eprintln!(
        "[apps daemon] Saved {} apps to {:?}",
        elements.len(),
        cache_path
    );
    Ok(())
}

fn handle_events(rx: Receiver<notify::Result<Event>>) {
    let mut last_update = Instant::now();

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                // Only process create/remove/modify events
                if !matches!(
                    event.kind,
                    notify::EventKind::Create(_)
                        | notify::EventKind::Remove(_)
                        | notify::EventKind::Modify(_)
                ) {
                    continue;
                }

                let now = Instant::now();
                if now.duration_since(last_update) < DEBOUNCE_DURATION {
                    continue;
                }

                last_update = now;
                eprintln!("[apps daemon] Change detected, rescanning...");

                match discover_applications() {
                    Ok(apps) => {
                        if let Err(e) = save_apps(&apps) {
                            eprintln!("[apps daemon] Failed to save: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("[apps daemon] Failed to discover apps: {}", e);
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("[apps daemon] Watch error: {:?}", e);
            }
            Err(e) => {
                eprintln!("[apps daemon] Channel error: {}", e);
                break;
            }
        }
    }
}

pub fn run() -> Result<()> {
    eprintln!("[apps daemon] Starting...");

    // Discover and save apps, then immediately drop the Vec
    {
        let apps = discover_applications()?;
        save_apps(&apps)?;
    } // apps Vec dropped here

    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    )?;

    for dir in watch_dirs() {
        let path = Path::new(dir);
        if path.exists() {
            match watcher.watch(path, RecursiveMode::Recursive) {
                Ok(_) => eprintln!("[apps daemon] Watching {}", dir),
                Err(e) => eprintln!("[apps daemon] Failed to watch {}: {}", dir, e),
            }
        } else {
            eprintln!("[apps daemon] Directory not found: {}", dir);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_apps = format!("{}/Applications", home);
        let path = Path::new(&user_apps);
        if path.exists() {
            match watcher.watch(path, RecursiveMode::Recursive) {
                Ok(_) => eprintln!("[apps daemon] Watching {}", user_apps),
                Err(e) => eprintln!("[apps daemon] Failed to watch {}: {}", user_apps, e),
            }
        }
    }

    eprintln!("[apps daemon] Ready, watching for changes...");

    handle_events(rx);

    Ok(())
}
