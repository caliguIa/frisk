use std::time::Instant;

use crate::cli::Cli;
use crate::core::config::Config;
use crate::core::element::ElementList;
use crate::core::error::Result;
use crate::instance;
use crate::ipc;
use crate::loader::{load_binary_file, load_binary_source};
use crate::picker;

pub fn run(cli: Cli) -> Result<()> {
    let sent_ipc = instance::check_single_instance(&cli)?;
    if sent_ipc {
        crate::log!("Sent reload message to existing instance");
        return Ok(());
    }

    let ipc_rx = ipc::start_listener()?;

    let start = Instant::now();

    let config = if let Some(ref config_path) = cli.config {
        Config::load(Some(config_path.clone()))?
    } else {
        Config::load(None)?
    };

    let config = if let Some(ref prompt) = cli.prompt {
        let mut config = config;
        config.prompt = prompt.clone();
        config
    } else {
        config
    };

    let after_config = Instant::now();

    let elements = load_elements(&cli)?;

    let after_discovery = Instant::now();

    crate::log!(
        "Loaded {} items (apps + system commands), estimated memory: ~{} KB",
        elements.len(),
        elements.len() * 60 / 1024
    );

    crate::log!("⏱️  Timing breakdown:");
    crate::log!(
        "  Config loading:  {:>6.2}ms",
        (after_config - start).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  Data loading:    {:>6.2}ms",
        (after_discovery - after_config).as_secs_f64() * 1000.0
    );
    crate::log!(
        "  Total to GUI:    {:>6.2}ms",
        (after_discovery - start).as_secs_f64() * 1000.0
    );

    picker::run(config, elements, Some(ipc_rx))?;

    Ok(())
}

fn load_elements(cli: &Cli) -> Result<ElementList> {
    let mut elements = ElementList::new();

    if cli.apps {
        if let Some(apps) = load_binary_source("apps.bin")? {
            let count = apps.len();
            for app in apps {
                elements.add(app);
            }
            crate::log!("Loaded {} apps from apps.bin", count);
        } else {
            eprintln!("Warning: --apps specified but apps.bin not found");
            eprintln!("Run: frisk service install apps && frisk service start apps");
        }
    }

    if cli.homebrew {
        if let Some(homebrew_apps) = load_binary_source("homebrew.bin")? {
            let count = homebrew_apps.len();
            for app in homebrew_apps {
                elements.add(app);
            }
            crate::log!("Loaded {} homebrew packages from homebrew.bin", count);
        } else {
            eprintln!("Warning: --homebrew specified but homebrew.bin not found");
            eprintln!("Run: frisk service install homebrew && frisk service start homebrew");
        }
    }

    if cli.clipboard {
        if let Some(clipboard_items) = load_binary_source("clipboard.bin")? {
            let count = clipboard_items.len();
            for item in clipboard_items {
                elements.add(item);
            }
            crate::log!("Loaded {} clipboard items from clipboard.bin", count);
        } else {
            eprintln!("Warning: --clipboard specified but clipboard.bin not found");
            eprintln!("Run: frisk service install clipboard && frisk service start clipboard");
        }
    }

    for source_path in &cli.source {
        match load_binary_file(source_path) {
            Ok(items) => {
                let count = items.len();
                for item in items {
                    elements.add(item);
                }
                crate::log!("Loaded {} items from {:?}", count, source_path);
            }
            Err(e) => {
                crate::log!("Failed to load source {:?}: {}", source_path, e);
            }
        }
    }

    if cli.commands {
        match crate::core::commands::CommandsConfig::load() {
            Ok(commands_config) => {
                for cmd in commands_config.to_elements() {
                    elements.add(cmd);
                }
                crate::log!("Loaded {} custom commands", commands_config.command.len());
            }
            Err(e) => {
                eprintln!("Warning: Failed to load commands config: {}", e);
            }
        }
    }

    if cli.nixpkgs {
        if let Some(nixpkgs) = load_binary_source("nixpkgs.bin")? {
            let count = nixpkgs.len();
            for pkg in nixpkgs {
                elements.add(pkg);
            }
            crate::log!("Loaded {} nixpkgs packages from nixpkgs.bin", count);
        } else {
            eprintln!("Warning: --nixpkgs specified but nixpkgs.bin not found");
            eprintln!("Run: frisk daemon nixpkgs");
        }
    }

    Ok(elements)
}
