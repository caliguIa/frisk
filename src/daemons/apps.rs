use crate::element::Element;
use crate::error::Result;
use notify_debouncer_full::{new_debouncer, notify::*, DebounceEventResult};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::Duration;

const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

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
    let vec = elements.to_vec(); // Convert slice to Vec for serialization
    crate::cache::save_cache("apps.bin", &vec)?;
    eprintln!(
        "[apps daemon] Saved {} apps to {:?}",
        elements.len(),
        cache_path
    );
    Ok(())
}

pub fn run() -> Result<()> {
    eprintln!("[apps daemon] Starting...");

    let apps = discover_applications()?;
    save_apps(&apps)?;

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(
        DEBOUNCE_DURATION,
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => {
                for event in events {
                    eprintln!("[apps daemon] FS event: {:?}", event);
                }
                let _ = tx.send(());
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("[apps daemon] Watch error: {:?}", error);
                }
            }
        },
    )?;

    for dir in watch_dirs() {
        let path = Path::new(dir);
        if path.exists() {
            match debouncer.watcher().watch(path, RecursiveMode::Recursive) {
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
            match debouncer.watcher().watch(path, RecursiveMode::Recursive) {
                Ok(_) => eprintln!("[apps daemon] Watching {}", user_apps),
                Err(e) => eprintln!("[apps daemon] Failed to watch {}: {}", user_apps, e),
            }
        }
    }

    eprintln!("[apps daemon] Ready, watching for changes...");

    loop {
        match rx.recv() {
            Ok(_) => {
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
            Err(e) => {
                eprintln!("[apps daemon] Channel error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
