use std::time::Instant;

mod cache;
mod calculator;
mod cli;
mod commands;
mod config;
mod daemons;
mod element;
mod error;
mod gui;
mod loader;
mod service;

#[macro_use]
mod log;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use error::Result;
use loader::{load_binary_file, load_binary_source};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Service { command }) => {
            service::handle_service_command(command)?;
            Ok(())
        }
        Some(Commands::Daemon { command }) => {
            use cli::DaemonCommands;
            match command {
                DaemonCommands::Apps => daemons::apps::run(),
                DaemonCommands::Homebrew => daemons::homebrew::run(),
                DaemonCommands::Clipboard => daemons::clipboard::run(),
                DaemonCommands::Nixpkgs { force } => daemons::nixpkgs::run(force),
            }
        }
        None => run_gui(cli),
    }
}

fn run_gui(cli: Cli) -> Result<()> {
    let start = Instant::now();

    let config = if let Some(config_path) = cli.config {
        Config::load(Some(config_path))?
    } else {
        Config::load(None)?
    };

    // Override prompt if specified
    let config = if let Some(prompt) = cli.prompt {
        let mut config = config;
        config.prompt = prompt;
        config
    } else {
        config
    };

    let after_config = Instant::now();

    // Load elements from binary sources
    let mut elements = element::ElementList::new();

    // Load apps from apps.bin if requested
    if cli.apps {
        if let Some(apps) = load_binary_source("apps.bin")? {
            let count = apps.len();
            for app in apps {
                elements.add(app);
            }
            crate::log!("Loaded {} apps from apps.bin", count);
        } else {
            eprintln!("Warning: --apps specified but apps.bin not found");
            eprintln!("Run: kickoff service install apps && kickoff service start apps");
        }
    }

    // Load homebrew from homebrew.bin if requested
    if cli.homebrew {
        if let Some(homebrew_apps) = load_binary_source("homebrew.bin")? {
            let count = homebrew_apps.len();
            for app in homebrew_apps {
                elements.add(app);
            }
            crate::log!("Loaded {} homebrew packages from homebrew.bin", count);
        } else {
            eprintln!("Warning: --homebrew specified but homebrew.bin not found");
            eprintln!("Run: kickoff service install homebrew && kickoff service start homebrew");
        }
    }

    // Load clipboard from clipboard.bin if requested
    if cli.clipboard {
        if let Some(clipboard_items) = load_binary_source("clipboard.bin")? {
            let count = clipboard_items.len();
            for item in clipboard_items {
                elements.add(item);
            }
            crate::log!("Loaded {} clipboard items from clipboard.bin", count);
        } else {
            eprintln!("Warning: --clipboard specified but clipboard.bin not found");
            eprintln!("Run: kickoff service install clipboard && kickoff service start clipboard");
        }
    }

    // Load additional custom sources
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

    // Load custom commands from config
    match commands::CommandsConfig::load() {
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

    let after_discovery = Instant::now();

    crate::log!(
        "Loaded {} items (apps + system commands), estimated memory: ~{} KB",
        elements.len(),
        elements.len() * 60 / 1024 // rough estimate
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

    gui::run(config, elements)?;

    Ok(())
}
